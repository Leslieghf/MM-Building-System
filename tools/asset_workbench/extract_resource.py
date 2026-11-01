"""Extract one indexed Mad Max archive resource into ignored local analysis output."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import pathlib
import struct
import zlib


DEFAULT_GAME_ROOT = pathlib.Path(r"C:\Program Files (x86)\Steam\steamapps\common\Mad Max")
INDEX_ROOT = pathlib.Path("target/content-index")
OUTPUT_ROOT = INDEX_ROOT / "extracted"
MAX_COMPRESSED_BYTES = 128 * 1024 * 1024
MAX_UNCOMPRESSED_BYTES = 256 * 1024 * 1024
MASK32 = 0xFFFFFFFF


def hash_jenkins(value: str) -> int:
    data = value.encode("ascii")
    a = b = c = (0xDEADBEEF + len(data)) & MASK32
    i = 0

    def rot(value: int, count: int) -> int:
        return ((value << count) & MASK32) | (value >> (32 - count))

    while i + 12 < len(data):
        a = (a + int.from_bytes(data[i : i + 4], "little")) & MASK32
        b = (b + int.from_bytes(data[i + 4 : i + 8], "little")) & MASK32
        c = (c + int.from_bytes(data[i + 8 : i + 12], "little")) & MASK32
        a = (a - c) & MASK32
        a ^= rot(c, 4)
        c = (c + b) & MASK32
        b = (b - a) & MASK32
        b ^= rot(a, 6)
        a = (a + c) & MASK32
        c = (c - b) & MASK32
        c ^= rot(b, 8)
        b = (b + a) & MASK32
        a = (a - c) & MASK32
        a ^= rot(c, 16)
        c = (c + b) & MASK32
        b = (b - a) & MASK32
        b ^= rot(a, 19)
        a = (a + c) & MASK32
        c = (c - b) & MASK32
        c ^= rot(b, 4)
        b = (b + a) & MASK32
        i += 12

    for index, byte in enumerate(data[i:]):
        if index < 4:
            a = (a + (byte << (8 * index))) & MASK32
        elif index < 8:
            b = (b + (byte << (8 * (index - 4)))) & MASK32
        else:
            c = (c + (byte << (8 * (index - 8)))) & MASK32

    c ^= b
    c = (c - rot(b, 14)) & MASK32
    a ^= c
    a = (a - rot(c, 11)) & MASK32
    b ^= a
    b = (b - rot(a, 25)) & MASK32
    c ^= b
    c = (c - rot(b, 16)) & MASK32
    a ^= c
    a = (a - rot(c, 4)) & MASK32
    b ^= a
    b = (b - rot(a, 14)) & MASK32
    c ^= b
    c = (c - rot(b, 24)) & MASK32
    return c


def read_path_record(logical_path: str) -> dict[str, str]:
    index_path = INDEX_ROOT / "path-census.csv"
    if not index_path.is_file():
        raise RuntimeError(f"Missing {index_path}; build the local content inventory first.")
    wanted = logical_path.replace("\\", "/").strip("/").casefold()
    with index_path.open(encoding="utf-8-sig", newline="") as source:
        for row in csv.DictReader(source):
            if row["path"].casefold() == wanted:
                return row
    raise ValueError(f"Path is not present in the installed name index: {logical_path}")


def archive_set_for_tab(tab_path: pathlib.Path, game_root: pathlib.Path) -> str:
    relative = tab_path.relative_to(game_root)
    parts = [part.casefold() for part in relative.parts]
    if parts[0] == "archives_win64":
        return "base"
    if parts[0] == "patch_win64":
        return "patch"
    if parts[0] == "dlc_win64":
        return f"dlc:{parts[1]}" if len(parts) > 2 else "dlc:unknown"
    return "unknown"


def read_tab(tab_path: pathlib.Path, game_root: pathlib.Path) -> dict[int, list[dict[str, object]]]:
    data = tab_path.read_bytes()
    if len(data) < 8:
        raise ValueError(f"Truncated archive table: {tab_path}")
    _magic, chunk_table_count = struct.unpack_from("<II", data, 0)
    pos = 8
    chunks_by_hash: dict[int, list[tuple[int, int]]] = {}
    for _ in range(chunk_table_count):
        if pos + 8 > len(data):
            raise ValueError(f"Truncated chunk header: {tab_path}")
        entry_hash, count = struct.unpack_from("<II", data, pos)
        pos += 8
        size = count * 8
        if pos + size > len(data):
            raise ValueError(f"Truncated chunk map: {tab_path}")
        chunks = [struct.unpack_from("<II", data, pos + i * 8) for i in range(count)]
        chunks_by_hash[entry_hash] = chunks
        pos += size

    entries: dict[int, list[dict[str, object]]] = {}
    if (len(data) - pos) % 16 != 0:
        raise ValueError(f"Archive record table has a partial record: {tab_path}")
    while pos < len(data):
        entry_hash, offset, compressed, uncompressed = struct.unpack_from("<IIII", data, pos)
        pos += 16
        record = {
            "archive": str(tab_path.with_suffix(".arc")),
            "archive_set": archive_set_for_tab(tab_path, game_root),
            "table": str(tab_path),
            "offset": offset,
            "compressed": compressed,
            "uncompressed": uncompressed,
            "chunks": chunks_by_hash.get(entry_hash, []),
        }
        entries.setdefault(entry_hash, []).append(record)
    return entries


def inflate_raw(payload: bytes, expected_size: int) -> bytes:
    decoder = zlib.decompressobj(-15)
    decoded = decoder.decompress(payload, expected_size + 1)
    if len(decoded) > expected_size or decoder.unconsumed_tail:
        raise ValueError("Raw-deflate payload exceeds its declared output size")
    decoded += decoder.flush()
    if len(decoded) != expected_size:
        raise ValueError(f"Inflated length {len(decoded)} does not match {expected_size}")
    return decoded


def read_candidate(record: dict[str, object]) -> bytes:
    compressed = int(record["compressed"])
    uncompressed = int(record["uncompressed"])
    if compressed > MAX_COMPRESSED_BYTES:
        raise ValueError(f"Compressed resource exceeds {MAX_COMPRESSED_BYTES} byte limit")
    if uncompressed > MAX_UNCOMPRESSED_BYTES:
        raise ValueError(f"Resource exceeds {MAX_UNCOMPRESSED_BYTES} byte output limit")
    archive = pathlib.Path(str(record["archive"]))
    archive_size = archive.stat().st_size
    offset = int(record["offset"])
    if offset < 0 or offset + compressed > archive_size:
        raise ValueError(f"Archive span is outside {archive}")
    chunks = record["chunks"]
    with archive.open("rb") as source:
        if not chunks:
            source.seek(offset)
            packed = source.read(compressed)
            if len(packed) != compressed:
                raise ValueError("Short read from archive")
            payload = packed if compressed == uncompressed else inflate_raw(packed, uncompressed)
            if len(payload) != uncompressed:
                raise ValueError("Unchunked resource length mismatch")
            return payload

        result = bytearray()
        for index, (uncompressed_offset, compressed_offset) in enumerate(chunks):
            next_uncompressed = chunks[index + 1][0] if index + 1 < len(chunks) else uncompressed
            next_compressed = chunks[index + 1][1] if index + 1 < len(chunks) else compressed
            if next_uncompressed < uncompressed_offset or next_compressed < compressed_offset:
                raise ValueError("Archive chunk offsets are not monotonic")
            source.seek(offset + compressed_offset)
            packed = source.read(next_compressed - compressed_offset)
            if len(packed) != next_compressed - compressed_offset:
                raise ValueError("Short read from archive chunk")
            result.extend(inflate_raw(packed, next_uncompressed - uncompressed_offset))
            if len(result) > MAX_UNCOMPRESSED_BYTES:
                raise ValueError("Chunked resource exceeded output limit")
    if len(result) != uncompressed:
        raise ValueError(f"Chunked length {len(result)} does not match {uncompressed}")
    return bytes(result)


def select_record(
    records: list[dict[str, object]], preferred_sets: list[str], archive_name: str | None
) -> tuple[dict[str, object], bytes, list[dict[str, object]]]:
    if archive_name:
        matches = [row for row in records if pathlib.Path(str(row["archive"])).name.casefold() == archive_name.casefold()]
        if len(matches) != 1:
            raise ValueError(f"--archive {archive_name!r} matched {len(matches)} archive records")
        selected = matches[0]
        return selected, read_candidate(selected), records

    by_set: dict[str, list[dict[str, object]]] = {}
    for record in records:
        archive_set = str(record["archive_set"])
        if archive_set in preferred_sets:
            by_set.setdefault(archive_set, []).append(record)
    if not by_set:
        available = sorted({str(row["archive_set"]) for row in records})
        raise ValueError(f"No installed archive locator in {preferred_sets}; available: {available}")

    priorities = ["patch", *[value for value in preferred_sets if value.startswith("dlc:")], "base"]
    chosen_set = min(by_set, key=lambda value: priorities.index(value) if value in priorities else len(priorities))
    candidates = sorted(by_set[chosen_set], key=lambda row: str(row["archive"]).casefold())
    payloads = [(row, read_candidate(row)) for row in candidates]
    hashes = {hashlib.sha256(payload).hexdigest() for _, payload in payloads}
    if len(hashes) != 1:
        raise ValueError(
            "Multiple different resources share this basename hash in the preferred archive set; "
            "choose one with --archive."
        )
    return payloads[0][0], payloads[0][1], records


def safe_output_path(logical_path: str) -> pathlib.Path:
    parts = pathlib.PurePosixPath(logical_path.replace("\\", "/")).parts
    if not parts or any(part in ("", ".", "..") for part in parts) or pathlib.PurePosixPath(logical_path).is_absolute():
        raise ValueError("Invalid logical resource path")
    root = OUTPUT_ROOT.resolve()
    destination = (root / pathlib.Path(*parts)).resolve()
    if destination != root and root not in destination.parents:
        raise ValueError("Output path escapes the local extraction directory")
    return destination


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("logical_path", help="Exact logical resource path from master.dirlist")
    parser.add_argument("--game-root", type=pathlib.Path, default=DEFAULT_GAME_ROOT)
    parser.add_argument("--archive", help="Select an archive filename when duplicate hash records differ")
    args = parser.parse_args()

    game_root = args.game_root.resolve()
    logical = args.logical_path.replace("\\", "/").strip("/")
    record = read_path_record(logical)
    entry_hash = int(record["hash"], 16)
    if hash_jenkins(pathlib.PurePosixPath(logical).name) != entry_hash:
        raise ValueError("Indexed basename hash does not match the path-census record")
    preferred_sets = [value for value in record["archives"].split(";") if value]
    tabs = sorted(
        [*game_root.glob("archives_win64/*.tab"), *game_root.glob("patch_win64/*.tab"), *game_root.glob("dlc_win64/**/*.tab")],
        key=lambda path: str(path).casefold(),
    )
    locators: list[dict[str, object]] = []
    for table in tabs:
        if archive_set_for_tab(table, game_root) not in preferred_sets:
            continue
        locators.extend(read_tab(table, game_root).get(entry_hash, []))
    if not locators:
        raise ValueError(f"No archive entry for {logical!r} ({entry_hash:08X})")

    selected, payload, all_locators = select_record(locators, preferred_sets, args.archive)
    destination = safe_output_path(logical)
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = destination.with_suffix(destination.suffix + ".tmp")
    temporary.write_bytes(payload)
    temporary.replace(destination)
    provenance = {
        "logical_path": logical,
        "basename_hash": f"0x{entry_hash:08X}",
        "archive_set_precedence": preferred_sets,
        "selected_archive_set": selected["archive_set"],
        "selected_archive": selected["archive"],
        "selected_table": selected["table"],
        "archive_offset": selected["offset"],
        "compressed_bytes": selected["compressed"],
        "uncompressed_bytes": selected["uncompressed"],
        "chunk_count": len(selected["chunks"]),
        "sha256": hashlib.sha256(payload).hexdigest(),
        "other_matching_archive_records": [
            {
                "archive_set": row["archive_set"],
                "archive": row["archive"],
                "compressed_bytes": row["compressed"],
                "uncompressed_bytes": row["uncompressed"],
            }
            for row in all_locators
            if row is not selected
        ],
    }
    destination.with_suffix(destination.suffix + ".asset.json").write_text(
        json.dumps(provenance, indent=2), encoding="utf-8"
    )
    print(json.dumps({"written": str(destination), "bytes": len(payload), "sha256": provenance["sha256"]}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
