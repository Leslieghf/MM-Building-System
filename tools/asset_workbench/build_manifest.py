"""Merge the local path, package, and entity indexes into an asset-workbench JSONL manifest."""

from __future__ import annotations

import collections
import csv
import json
import pathlib


ROOT = pathlib.Path("target/content-index")
PREVIEW_ROOT = ROOT / "previews"
OUTPUT = ROOT / "asset-workbench-resources.jsonl"
SUMMARY = ROOT / "asset-workbench-summary.json"

TEXT_CANDIDATE_EXTENSIONS = {
    "txt", "xml", "json", "ini", "cfg", "h", "hpp", "c", "cpp", "rhai", "py", "xpy", "glsl", "hlsl", "shader"
}


def rows(name: str):
    with (ROOT / name).open(encoding="utf-8-sig", newline="") as source:
        yield from csv.DictReader(source)


def normal(path: str) -> str:
    return path.replace("\\", "/").strip("/").casefold()


def extension(path: str) -> str:
    name = path.rsplit("/", 1)[-1]
    return name.rsplit(".", 1)[-1].casefold() if "." in name else ""


def resource_families(path: str, ext: str) -> list[str]:
    lower = path.casefold()
    result: list[str] = []
    if ext in {"ee", "epe"}:
        result.append("entity_definition" if ext == "epe" else "entity_package")
    if ext in {"model_xml", "lod", "rbs", "rbn"}:
        result.append("model_candidate")
    if ext in {"ddsc", "tga", "dds", "png", "jpg", "jpeg"}:
        result.append("texture_candidate")
    if ext in {"pfx", "pfxc", "pfic", "pfs"}:
        result.append("collision_or_effect_candidate")
    if ext in {"xpy", "xvmc"} or "/scripts/" in lower:
        result.append("script_candidate")
    if ext in {"gsr", "gsrc", "btc", "ccenemy", "ccenemyc"} or "/graphs/" in lower or "/ai/bts/" in lower:
        result.append("graph_or_behavior_candidate")
    if ext in {"ban", "asb", "afsmb", "skeleton", "ragdoll"} or "/animations/" in lower:
        result.append("animation_or_rig_candidate")
    if ext in {"fsb", "fev"} or "/sound/" in lower:
        result.append("audio_candidate")
    if ext in {"effc", "effc_link"} or "/effects/" in lower:
        result.append("effect_or_vfx_candidate")
    if ext == "mdic" or "/locations/" in lower or "/terrain/" in lower:
        result.append("world_or_streaming_candidate")
    if ext in {"vengine", "venginec", "vpgeneral", "vpgeneralc", "vpsolver", "vpsolverc"}:
        result.append("vehicle_configuration_candidate")
    if not result:
        result.append("unknown")
    return result


def new_record(
    path: str, *, path_kind: str = "logical", normalized_path: str | None = None
) -> dict[str, object]:
    ext = extension(path)
    normalized_path = normalized_path or normal(path)
    id_kind = "path" if path_kind == "logical" else path_kind
    return {
        "schema_version": 1,
        "resource_id": f"game-{id_kind}:{normal(path)}",
        "path": path,
        "path_kind": path_kind,
        "normalized_path": normalized_path,
        "extension": ext,
        "families": resource_families(path, ext),
        "discovered_as": [],
        "archive": {"present": False, "sets": [], "hits": 0, "hash": None},
        "compiled_member": {"present": False, "owners": [], "size_min": None, "size_max": None},
        "references": {"count": 0, "entities": [], "node_classes": [], "property_hashes": [], "compiled_candidates": []},
        "catalog_entries": [],
        "preview": None,
        "entity_profile": None,
        "capabilities": {
            "source_text_candidate": ext in TEXT_CANDIDATE_EXTENSIONS,
            "structure_decoded": False,
            "preview_ready": False,
            "spawn_route_known": False,
            "spawn_observed": False,
        },
    }


def main() -> None:
    resources: dict[str, dict[str, object]] = {}

    def get(path: str, *, path_kind: str = "logical") -> dict[str, object]:
        normalized = normal(path)
        key = normalized if path_kind == "logical" else f"@{path_kind}/{normalized}"
        if key not in resources:
            resources[key] = new_record(
                path.replace("\\", "/"),
                path_kind=path_kind,
                normalized_path=key,
            )
        return resources[key]

    for row in rows("path-census.csv"):
        path_kind = "logical" if row["path"] else "basename"
        record = get(row["path"] or row["basename"], path_kind=path_kind)
        record["discovered_as"].append("logical_path_index")
        record["archive"] = {
            "present": row["installed"].casefold() == "true",
            "sets": [value for value in row["archives"].split(";") if value],
            "hits": int(row["archive_hits"] or 0),
            "hash": row["hash"],
        }

    owner_sets: dict[str, set[str]] = collections.defaultdict(set)
    size_ranges: dict[str, list[int]] = collections.defaultdict(list)
    for row in rows("entity-bundle-members.csv"):
        path = row["member"].replace("\\", "/")
        record = get(path)
        if "compiled_bundle_member" not in record["discovered_as"]:
            record["discovered_as"].append("compiled_bundle_member")
        owner_sets[normal(path)].add(row["entity"])
        size_ranges[normal(path)].append(int(row["size"]))

    catalog_entries: dict[str, list[dict[str, str]]] = collections.defaultdict(list)
    for filename in ("assets/catalog/entities.json", "assets/catalog/community-candidates.json"):
        with pathlib.Path(filename).open(encoding="utf-8") as source:
            entries = json.load(source)["entries"]
        for entry in entries:
            package_path = entry.get("asset_key", "") + ".ee"
            if package_path != ".ee":
                catalog_entries[normal(package_path)].append(
                    {"id": entry["id"], "name": entry["name"], "kind": entry["kind"], "readiness": entry["spawn_readiness"]}
                )

    for row in rows("entity-structural-profiles.csv"):
        package_path = row["entity"]
        record = get(package_path)
        record["entity_profile"] = {
            "root_classification": row["classification_candidate"],
            "direct_root_count": int(row["top_level_count"]),
            "direct_root_classes": json.loads(row["top_level_classes"]),
            "catalog_evidence": row["catalog_evidence"] or None,
        }
        record["capabilities"]["structure_decoded"] = True
        for entry in catalog_entries.get(normal(package_path), []):
            record["catalog_entries"].append(entry)
        record["capabilities"]["spawn_route_known"] = bool(record["catalog_entries"])
        record["capabilities"]["spawn_observed"] = any(
            entry["readiness"] == "verified" for entry in record["catalog_entries"]
        )
        record["discovered_as"].append("parsed_entity_profile")

    source_to_compiled: dict[str, set[str]] = collections.defaultdict(set)
    for row in rows("epe-reference-compiled-member-candidates.csv"):
        source = row["source_reference"].replace("\\", "/")
        compiled = row["compiled_member_candidate"].replace("\\", "/")
        source_to_compiled[normal(source)].add(compiled)

    reference_owners: dict[str, set[str]] = collections.defaultdict(set)
    reference_classes: dict[str, set[str]] = collections.defaultdict(set)
    reference_hashes: dict[str, set[str]] = collections.defaultdict(set)
    reference_counts: collections.Counter[str] = collections.Counter()
    for row in rows("epe-embedded-path-references.csv"):
        source = row["value"].replace("\\", "/")
        key = normal(source)
        record = get(source)
        if "entity_source_reference" not in record["discovered_as"]:
            record["discovered_as"].append("entity_source_reference")
        reference_counts[key] += 1
        reference_owners[key].add(row["entity"])
        if row["node_class"]:
            reference_classes[key].add(row["node_class"])
        if row["property_hash"]:
            reference_hashes[key].add(row["property_hash"])

    for key, record in resources.items():
        record["compiled_member"]["owners"] = sorted(owner_sets.get(key, set()))
        record["compiled_member"]["present"] = bool(owner_sets.get(key))
        sizes = size_ranges.get(key, [])
        if sizes:
            record["compiled_member"]["size_min"] = min(sizes)
            record["compiled_member"]["size_max"] = max(sizes)
        if reference_counts[key]:
            record["references"] = {
                "count": reference_counts[key],
                "entities": sorted(reference_owners[key]),
                "node_classes": sorted(reference_classes[key]),
                "property_hashes": sorted(reference_hashes[key]),
                "compiled_candidates": sorted(source_to_compiled.get(key, set())),
            }
        if record["families"] == ["unknown"]:
            record["families"] = resource_families(record["path"], record["extension"])
        if record["extension"] == "epe" and "parsed_entity_profile" in record["discovered_as"]:
            record["capabilities"]["structure_decoded"] = True

    previewed_paths: set[str] = set()
    if PREVIEW_ROOT.is_dir():
        for sidecar_path in sorted(PREVIEW_ROOT.rglob("*.preview.json"), key=lambda path: path.as_posix()):
            preview = json.loads(sidecar_path.read_text(encoding="utf-8"))
            key = normal(preview["logical_path"])
            record = resources.get(key)
            if record is None:
                continue
            record["capabilities"]["preview_ready"] = True
            record["preview"] = {
                "representation": preview["representation"],
                "report": preview["report"],
                "image": preview.get("image"),
                "model_export": preview.get("model_export"),
                "dependencies": preview.get("dependencies", []),
                "structure": {
                    key: preview["structure"][key]
                    for key in ("format", "block_count", "vertex_count", "index_count", "bounds", "decoded_bounds", "decoder_status", "geometry_preview")
                    if preview.get("structure") and key in preview["structure"]
                } or None,
                "source_bundle": preview.get("bundle"),
            }
            previewed_paths.add(key)

    counts = collections.Counter()
    family_counts = collections.Counter()
    path_kind_counts = collections.Counter()
    for record in resources.values():
        counts.update(record["discovered_as"])
        family_counts.update(record["families"])
        path_kind_counts.update([record["path_kind"]])

    with OUTPUT.open("w", encoding="utf-8", newline="\n") as destination:
        for record in sorted(resources.values(), key=lambda item: item["normalized_path"]):
            destination.write(json.dumps(record, separators=(",", ":"), ensure_ascii=True) + "\n")

    summary = {
        "schema_version": 1,
        "resource_count": len(resources),
        "path_kind_counts": dict(sorted(path_kind_counts.items())),
        "preview_records": len(previewed_paths),
        "discovery_counts": dict(sorted(counts.items())),
        "family_memberships": dict(sorted(family_counts.items())),
        "relationships": {
            "source_reference_edges": sum(reference_counts.values()),
            "unique_source_reference_paths": len(reference_counts),
            "same_stem_compiled_candidates": sum(len(values) for values in source_to_compiled.values()),
        },
        "notes": [
            "Exact normalized paths are merged; same-stem source and compiled paths remain separate records.",
            "Format-family names are candidates and do not imply a successful decoder or preview.",
            "Game payloads and extracted bytes are not embedded in this manifest.",
        ],
    }
    SUMMARY.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
