"""Extract one selected game resource and prepare a local, read-only preview report."""

from __future__ import annotations

import argparse
import hashlib
import html
import json
import pathlib
import re
import subprocess
import sys

from mesh_viewer import viewer_markup
from extract_bundle_member import bundle_output_segment, sidecar_output_path
from rbn_inspector import RBN_GENERAL_HASH, RBN_GENERAL_VERSION, RbnMaterial, inspect_rbn
from rbs_inspector import RbsFormatError, decode_rbs_meshes, inspect_rbs


REPOSITORY_ROOT = pathlib.Path(__file__).resolve().parents[2]
TOOL_DIRECTORY = pathlib.Path(__file__).resolve().parent
EXTRACTED_ROOT = REPOSITORY_ROOT / "target/content-index/extracted"
PREVIEW_ROOT = REPOSITORY_ROOT / "target/content-index/previews"
MAX_TEXT_BYTES = 1 * 1024 * 1024
MAX_STRINGS = 5000


def printable_runs(data: bytes) -> list[tuple[int, str]]:
    runs: list[tuple[int, str]] = []
    start = 0
    for index, value in enumerate(data + b"\0"):
        if value == 0 or value < 0x20 or value > 0x7E:
            if index - start >= 4:
                runs.append((start, data[start:index].decode("ascii", errors="replace")))
                if len(runs) >= MAX_STRINGS:
                    break
            start = index + 1
    return runs


def text_representation(data: bytes) -> tuple[str | None, bool]:
    if len(data) > MAX_TEXT_BYTES:
        candidate = data[:MAX_TEXT_BYTES]
        truncated = True
    else:
        candidate = data
        truncated = False
    try:
        text = candidate.decode("utf-8-sig")
    except UnicodeDecodeError:
        return None, False
    if not text:
        return "", truncated
    printable = sum(char.isprintable() or char in "\r\n\t" for char in text)
    if printable / len(text) < 0.92:
        return None, False
    return text, truncated


def safe_preview_path(default_path: pathlib.Path, logical_path: str) -> pathlib.Path:
    if len(str(default_path.with_suffix(".mesh.json"))) <= 240:
        return default_path
    digest = hashlib.sha256(logical_path.casefold().encode("utf-8")).hexdigest()[:16]
    return default_path.parent / f"preview-{digest}.preview.html"


def rbn_dependencies(data: bytes) -> list[dict[str, str]]:
    if len(data) < 9 or data[4:9] != b"RBNDL":
        return []
    found: dict[str, dict[str, str]] = {}
    for match in re.finditer(rb"([A-Za-z0-9_./-]+\.(?:rbs|ddsc))", data, re.IGNORECASE):
        path = match.group(1).decode("ascii")
        key = path.casefold()
        found.setdefault(
            key,
            {
                "path": path,
                "kind": "geometry" if path.casefold().endswith(".rbs") else "texture",
            },
        )
    return list(found.values())


def html_report(
    logical_path: str,
    representation: str,
    summary: dict[str, object],
    text_body: str | None,
    strings: list[tuple[int, str]],
    dependencies: list[dict[str, str]],
    image_path: pathlib.Path | None,
    mesh_structure: dict[str, object] | None,
    model_export_name: str | None,
) -> str:
    if text_body is not None:
        body = f'<pre>{html.escape(text_body)}</pre>'
    elif mesh_structure:
        meshes = mesh_structure["meshes"]
        rows = []
        for mesh in meshes:
            buffers = ", ".join(
                f"stride {buffer['stride']} · {buffer['vertex_count']:,} vertices"
                for buffer in mesh["vertex_buffers"]
            ) or "none"
            rows.append(
                "<tr>"
                f"<td>{mesh['block_index']}</td>"
                f"<td><code>0x{mesh['render_block_hash']:08X}</code> · v{mesh['render_block_version']}</td>"
                f"<td>{html.escape(buffers)}</td>"
                f"<td>{mesh['index_count']:,}</td>"
                f"<td>{mesh['minimum_index'] if mesh['minimum_index'] is not None else '—'}"
                f" … {mesh['maximum_index'] if mesh['maximum_index'] is not None else '—'}</td>"
                "</tr>"
            )
        bounds = mesh_structure["bounds"]
        viewer = viewer_markup(mesh_structure)
        dependency_list = ""
        if dependencies:
            items = "".join(
                f"<li><code>{html.escape(item['kind'])}</code> {html.escape(item['path'])}</li>"
                for item in dependencies
            )
            dependency_list = f"<h3>Linked model resources</h3><ul>{items}</ul>"
        body = viewer + (
            '<h3>Validated mesh structure</h3>'
            f"<p>{mesh_structure['block_count']} mesh blocks · "
            f"{mesh_structure['vertex_count']:,} vertices · {mesh_structure['index_count']:,} indices</p>"
            f"<p>Bounds vectors: first {html.escape(str(bounds['header_first']))} · "
            f"second {html.escape(str(bounds['header_second']))}</p>"
            f"<p>Componentwise range: {html.escape(str(bounds['componentwise_min']))} → "
            f"{html.escape(str(bounds['componentwise_max']))}</p>"
            '<div class="table-scroll"><table><thead><tr><th>Block</th><th>Render block</th>'
            '<th>Vertex buffers</th><th>Indices</th><th>Index range</th></tr></thead><tbody>'
            + "".join(rows)
            + '</tbody></table></div>'
            f"<p class=empty>{html.escape(mesh_structure['decoder_status'])}</p>"
            + dependency_list
        )
    elif image_path is not None:
        relative_image = pathlib.Path(image_path.name).as_posix()
        body = f'<img class="texture" src="{html.escape(relative_image)}" alt="Decoded texture preview">'
    elif dependencies:
        items = "".join(f"<li><code>{html.escape(item['kind'])}</code> {html.escape(item['path'])}</li>" for item in dependencies)
        body = f"<h3>Linked model resources</h3><ul>{items}</ul>"
    elif strings:
        body = "<pre>" + html.escape("\n".join(f"0x{offset:08X}  {value}" for offset, value in strings)) + "</pre>"
    else:
        body = '<p class="empty">No readable text, printable string runs, or decoded image were found.</p>'

    provenance = "\n".join(
        f"{key}: {value}" for key, value in summary.items() if key in {
            "bytes", "sha256", "selected_archive_set", "selected_archive", "chunk_count",
            "bundle", "member", "member_offset"
        }
    )
    title = html.escape(pathlib.PurePosixPath(logical_path).name)
    model_link = (
        f'<p><a class="model-export" href="{html.escape(model_export_name)}" download>Download 3D model (.glb)</a></p>'
        if model_export_name
        else ""
    )
    return f"""<!doctype html>
<html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width">
<title>{title} — Mad Max asset preview</title>
<style>
body {{ margin: 0; padding: 18px; color: #dce6ee; background: #101820; font: 13px/1.45 system-ui, sans-serif; }}
h1 {{ margin: 0 0 8px; font-size: 18px; }} .path {{ color: #90a8b9; overflow-wrap: anywhere; }}
.badge {{ display: inline-block; margin: 10px 0; padding: 3px 7px; color: #101820; background: #88c789; font-weight: 700; }}
pre {{ overflow: auto; max-height: 75vh; padding: 12px; border: 1px solid #344657; background: #0a1118; white-space: pre-wrap; overflow-wrap: anywhere; }}
.texture {{ display: block; max-width: min(100%, 1200px); max-height: 76vh; margin: 12px auto; object-fit: contain; image-rendering: auto; background: repeating-conic-gradient(#222 0% 25%, #333 0% 50%) 50% / 18px 18px; }}
.table-scroll {{ overflow: auto; max-width: 100%; }} table {{ width: 100%; border-collapse: collapse; font-size: 12px; }} th,td {{ padding: 6px 8px; border: 1px solid #344657; text-align: left; vertical-align: top; }} th {{ background: #1a2a37; }} td {{ overflow-wrap: anywhere; }}
.mesh-viewer {{ margin: 12px 0; border: 1px solid #344657; background: #0d151d; }} .mesh-viewer-title,.mesh-viewer-help {{ padding: 6px 9px; color: #9bb0bf; font-size: 11px; }} #mesh-canvas {{ display:block;width:100%;height:min(58vh,560px);cursor:grab;touch-action:none }} #mesh-canvas:active {{ cursor:grabbing }} .mesh-error {{ padding:24px;color:#e2c66d }}
.model-export {{ display:inline-block;padding:6px 9px;background:#244155;border:1px solid #3b5263;border-radius:3px;color:#bfe2f6 }}
.empty {{ color: #e2c66d; }} details {{ margin-top: 14px; color: #9bb0bf; }}
</style>
<h1>{title}</h1><div class="path">{html.escape(logical_path)}</div>
<div class="badge">{html.escape(representation)}</div>
{body}
{model_link}
<details><summary>Source and extraction details</summary><pre>{html.escape(provenance)}</pre></details>
</html>"""


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("logical_path", help="Exact logical path or exact bundle-member path")
    parser.add_argument("--bundle", help="Read this member from the exact .ee bundle instead of the outer archive")
    parser.add_argument("--game-root", type=pathlib.Path)
    parser.add_argument("--archive", help="Select an archive filename when duplicate entries differ")
    args = parser.parse_args()

    logical = args.logical_path.replace("\\", "/").strip("/")
    extractor_name = "extract_bundle_member.py" if args.bundle else "extract_resource.py"
    command = [sys.executable, str(TOOL_DIRECTORY / extractor_name)]
    if args.bundle:
        command.append(args.bundle.replace("\\", "/").strip("/"))
    command.append(logical)
    if args.game_root:
        command.extend(["--game-root", str(args.game_root)])
    if args.archive:
        command.extend(["--archive", args.archive])
    subprocess.run(command, cwd=REPOSITORY_ROOT, check=True)

    if args.bundle:
        bundle_segment = bundle_output_segment(args.bundle)
        payload_path = EXTRACTED_ROOT / "bundle-members" / bundle_segment
        payload_path = payload_path.joinpath(*pathlib.PurePosixPath(logical).parts)
        sidecar_path = sidecar_output_path(payload_path, logical)
    else:
        payload_path = EXTRACTED_ROOT.joinpath(*pathlib.PurePosixPath(logical).parts)
        sidecar_path = payload_path.with_suffix(payload_path.suffix + ".asset.json")
    data = payload_path.read_bytes()
    provenance = json.loads(sidecar_path.read_text(encoding="utf-8"))
    suffix = payload_path.suffix.casefold()
    text_body, truncated = text_representation(data)
    strings: list[tuple[int, str]] = []
    dependencies: list[dict[str, str]] = []
    image_path: pathlib.Path | None = None
    mesh_structure: dict[str, object] | None = None
    rbn_structure: dict[str, object] | None = None
    geometry_provenance: dict[str, object] | None = None

    if text_body is not None:
        representation = "Read-only text" + (" (first 1 MiB)" if truncated else "")
    elif suffix in {".ddsc", ".dds"}:
        try:
            from PIL import Image

            with Image.open(payload_path) as image:
                image.load()
                if args.bundle:
                    image_path = PREVIEW_ROOT / "bundle-members" / bundle_segment
                    image_path = image_path.joinpath(
                        *pathlib.PurePosixPath(logical).parts
                    ).with_suffix(".png")
                else:
                    image_path = PREVIEW_ROOT.joinpath(
                        *pathlib.PurePosixPath(logical).parts
                    ).with_suffix(".png")
                image_path.parent.mkdir(parents=True, exist_ok=True)
                image.save(image_path, format="PNG")
                representation = f"Decoded texture · {image.width}×{image.height} · {image.mode}"
        except ImportError:
            representation = "Texture candidate · Pillow is required to decode this format"
        except Exception as error:  # Decoder errors are useful viewer status, not a failed extraction.
            representation = f"Texture candidate · decoder could not open it: {error}"
    elif suffix == ".rbs":
        try:
            mesh_structure = inspect_rbs(data, logical)
            representation = (
                f"RBSDL mesh structure · {mesh_structure['block_count']} blocks · "
                f"{mesh_structure['vertex_count']:,} vertices · {mesh_structure['index_count']:,} indices"
            )
        except RbsFormatError as error:
            representation = f"RBS geometry candidate · structure not decoded: {error}"
    elif suffix == ".rbn" and data[4:9] == b"RBNDL":
        try:
            rbn_structure = inspect_rbn(data, logical)
        except RbsFormatError as error:
            rbn_structure = None
            representation = f"RBNDL resource candidate · material layout not decoded: {error}"
        dependencies = rbn_dependencies(data)
        geometry_count = sum(item["kind"] == "geometry" for item in dependencies)
        texture_count = sum(item["kind"] == "texture" for item in dependencies)
        if rbn_structure:
            representation = f"RBNDL materials · {rbn_structure['material_count']} · {geometry_count} geometry, {texture_count} texture links"
        if rbn_structure and args.bundle:
            geometry_path = str(rbn_structure["geometry_path"])
            try:
                geometry_command = [
                    sys.executable,
                    str(TOOL_DIRECTORY / "extract_bundle_member.py"),
                    args.bundle.replace("\\", "/").strip("/"),
                    geometry_path,
                ]
                if args.game_root:
                    geometry_command.extend(["--game-root", str(args.game_root)])
                subprocess.run(geometry_command, cwd=REPOSITORY_ROOT, check=True)
                geometry_payload = (
                    EXTRACTED_ROOT
                    / "bundle-members"
                    / bundle_segment
                    / pathlib.PurePosixPath(geometry_path)
                )
                geometry_sidecar = geometry_payload.with_suffix(geometry_payload.suffix + ".member.json")
                geometry_provenance = json.loads(geometry_sidecar.read_text(encoding="utf-8"))
                materials = [
                    RbnMaterial(
                        item["block_index"],
                        item["render_block_hash"],
                        item["render_block_version"],
                        item["vertex_scale"],
                        tuple(tuple(scale) for scale in item["uv_scales"]),
                        item["additional_uv_sets"],
                        tuple(item["texture_slots"]),
                        item["geometry_reference_hash"],
                        item["mesh_index"],
                        item["end_marker"],
                    )
                    for item in rbn_structure["materials"]
                ]
                mesh_structure = decode_rbs_meshes(geometry_payload.read_bytes(), materials)
                representation = (
                    f"Flat-shaded model preview · {mesh_structure['block_count']} meshes · "
                    f"{mesh_structure['vertex_count']:,} vertices"
                )
            except (RbsFormatError, OSError, subprocess.CalledProcessError) as error:
                representation += f" · mesh preview unavailable: {error}"
    else:
        strings = printable_runs(data)
        if suffix in {".xvmc", ".gsrc", ".pfxc", ".effc"}:
            representation = f"Compiled binary · {len(strings)} printable string runs; not source code"
        elif data.startswith(b"FSB5"):
            representation = "FMOD FSB5 bank · metadata only; audio decoding is not wired yet"
        else:
            representation = f"Binary data · {len(strings)} printable string runs"

    PREVIEW_ROOT.mkdir(parents=True, exist_ok=True)
    if args.bundle:
        report_path = PREVIEW_ROOT / "bundle-members" / bundle_segment
        report_path = report_path.joinpath(*pathlib.PurePosixPath(logical).parts).with_suffix(".preview.html")
    else:
        report_path = PREVIEW_ROOT.joinpath(*pathlib.PurePosixPath(logical).parts).with_suffix(".preview.html")
    report_path = safe_preview_path(report_path, logical)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    summary = {
        **provenance,
        "bytes": len(data),
        "sha256": provenance.get("sha256"),
        "representation": representation,
        "geometry_sha256": geometry_provenance.get("sha256") if geometry_provenance else None,
    }
    model_export_path: pathlib.Path | None = None
    mesh_sidecar_path: pathlib.Path | None = None
    if mesh_structure and mesh_structure.get("decoded_geometry"):
        from mesh_viewer import write_glb, write_thumbnail

        model_export_path = report_path.with_suffix(".glb")
        model_export_path.write_bytes(write_glb(mesh_structure, pathlib.PurePosixPath(logical).stem))
        thumbnail_path = report_path.with_suffix(".png")
        if write_thumbnail(mesh_structure, thumbnail_path):
            image_path = thumbnail_path
        mesh_sidecar_path = report_path.with_suffix(".mesh.json")
        mesh_sidecar_path.write_text(
            json.dumps(
                {
                    key: value
                    for key, value in mesh_structure.items()
                    if key in {"format", "bounds", "block_count", "vertex_count", "index_count", "decoded_bounds", "decoder_status", "decoded_geometry"}
                },
                separators=(",", ":"),
            ),
            encoding="utf-8",
        )
    report_path.write_text(
        html_report(
            logical,
            representation,
            summary,
            text_body,
            strings,
            dependencies,
            image_path,
            mesh_structure,
            model_export_path.name if model_export_path else None,
        ),
        encoding="utf-8",
    )
    structure_summary = None
    if mesh_structure:
        structure_summary = {
            key: mesh_structure[key]
            for key in ("format", "block_count", "vertex_count", "index_count", "bounds", "decoded_bounds", "decoder_status")
            if key in mesh_structure
        }
        structure_summary["geometry_preview"] = bool(mesh_structure.get("decoded_geometry"))
    preview_sidecar = {
        "logical_path": logical,
        "bundle": args.bundle.replace("\\", "/").strip("/") if args.bundle else None,
        "representation": representation,
        "report": report_path.relative_to(REPOSITORY_ROOT).as_posix(),
        "image": image_path.relative_to(REPOSITORY_ROOT).as_posix() if image_path else None,
        "sha256": provenance.get("sha256"),
        "dependencies": dependencies,
        "structure": structure_summary,
        "materials": rbn_structure,
        "model_export": model_export_path.relative_to(REPOSITORY_ROOT).as_posix() if model_export_path else None,
        "mesh_sidecar": mesh_sidecar_path.relative_to(REPOSITORY_ROOT).as_posix() if mesh_sidecar_path else None,
    }
    report_path.with_suffix(".json").write_text(
        json.dumps(preview_sidecar, indent=2), encoding="utf-8"
    )
    result = {
        "report": str(report_path),
        "representation": representation,
        "decoded_image": str(image_path) if image_path else None,
        "decoded_model": str(model_export_path) if model_export_path else None,
        "printable_string_runs": len(strings),
    }
    print(json.dumps(result, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
