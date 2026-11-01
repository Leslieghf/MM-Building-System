"""Extract one indexed SARC member from a selected .ee entity bundle."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import pathlib
import subprocess
import sys


REPOSITORY_ROOT = pathlib.Path(__file__).resolve().parents[2]
TOOL_DIRECTORY = pathlib.Path(__file__).resolve().parent
INDEX_ROOT = REPOSITORY_ROOT / "target/content-index"
EXTRACTED_ROOT = INDEX_ROOT / "extracted"
MEMBER_ROOT = EXTRACTED_ROOT / "bundle-members"
MEMBER_INDEX = INDEX_ROOT / "entity-bundle-members.csv"
MAX_MEMBER_BYTES = 128 * 1024 * 1024


def member_record(bundle: str, member: str) -> dict[str, str]:
    if not MEMBER_INDEX.is_file():
        raise RuntimeError(f"Missing {MEMBER_INDEX}; build the local SARC member index first.")
    wanted_bundle = bundle.replace("\\", "/").strip("/").casefold()
    wanted_member = member.replace("\\", "/").strip("/").casefold()
    with MEMBER_INDEX.open(encoding="utf-8-sig", newline="") as source:
        matches = [
            row
            for row in csv.DictReader(source)
            if row["entity"].casefold() == wanted_bundle and row["member"].casefold() == wanted_member
        ]
    if len(matches) != 1:
        raise ValueError(f"Expected one indexed member, found {len(matches)} for {bundle} :: {member}")
    return matches[0]


def safe_member_output(bundle: str, member: str) -> pathlib.Path:
    bundle_parts = pathlib.PurePosixPath(bundle.replace("\\", "/")).parts
    member_parts = pathlib.PurePosixPath(member.replace("\\", "/")).parts
    if (
        not bundle_parts
        or not member_parts
        or any(part in ("", ".", "..") for part in (*bundle_parts, *member_parts))
        or pathlib.PurePosixPath(bundle).is_absolute()
        or pathlib.PurePosixPath(member).is_absolute()
    ):
        raise ValueError("Invalid bundle or member path")
    bundle_stem = bundle_output_segment(bundle)
    root = MEMBER_ROOT.resolve()
    destination = (root / bundle_stem / pathlib.Path(*member_parts)).resolve()
    if destination == root or root not in destination.parents:
        raise ValueError("Member output path escapes the local extraction directory")
    return destination


def bundle_output_segment(bundle: str) -> str:
    name = pathlib.PurePosixPath(bundle.replace("\\", "/")).stem
    digest = hashlib.sha256(bundle.casefold().encode("utf-8")).hexdigest()[:10]
    return f"{name}-{digest}"


def sidecar_output_path(destination: pathlib.Path, member: str) -> pathlib.Path:
    sidecar = destination.with_suffix(destination.suffix + ".member.json")
    if len(str(sidecar)) <= 240:
        return sidecar
    digest = hashlib.sha256(member.casefold().encode("utf-8")).hexdigest()[:16]
    return destination.parent / f"member-{digest}.json"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("bundle", help="Exact .ee path from the local path census")
    parser.add_argument("member", help="Exact internal member path from entity-bundle-members.csv")
    parser.add_argument("--game-root", type=pathlib.Path)
    args = parser.parse_args()

    bundle = args.bundle.replace("\\", "/").strip("/")
    member = args.member.replace("\\", "/").strip("/")
    if pathlib.PurePosixPath(bundle).suffix.casefold() != ".ee":
        raise ValueError("The selected bundle path must end in .ee")
    indexed = member_record(bundle, member)
    size = int(indexed["size"])
    offset = int(indexed["offset"])
    if size > MAX_MEMBER_BYTES:
        raise ValueError(f"Member exceeds the {MAX_MEMBER_BYTES}-byte output limit")

    command = [sys.executable, str(TOOL_DIRECTORY / "extract_resource.py"), bundle]
    if args.game_root:
        command.extend(["--game-root", str(args.game_root)])
    subprocess.run(command, cwd=REPOSITORY_ROOT, check=True)

    bundle_path = EXTRACTED_ROOT.joinpath(*pathlib.PurePosixPath(bundle).parts)
    bundle_bytes = bundle_path.read_bytes()
    if offset < 0 or offset + size > len(bundle_bytes):
        raise ValueError(
            f"Indexed member span 0x{offset:X}+0x{size:X} is outside the {len(bundle_bytes)}-byte bundle"
        )
    payload = bundle_bytes[offset : offset + size]
    destination = safe_member_output(bundle, member)
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = destination.with_suffix(destination.suffix + ".tmp")
    temporary.write_bytes(payload)
    temporary.replace(destination)
    provenance = {
        "bundle": bundle,
        "member": member,
        "member_offset": offset,
        "member_bytes": size,
        "sha256": hashlib.sha256(payload).hexdigest(),
        "bundle_archive_provenance": json.loads(
            bundle_path.with_suffix(bundle_path.suffix + ".asset.json").read_text(encoding="utf-8")
        ),
    }
    sidecar_path = sidecar_output_path(destination, member)
    sidecar_path.parent.mkdir(parents=True, exist_ok=True)
    sidecar_path.write_text(
        json.dumps(provenance, indent=2), encoding="utf-8"
    )
    print(json.dumps({"written": str(destination), "bytes": size, "sha256": provenance["sha256"]}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
