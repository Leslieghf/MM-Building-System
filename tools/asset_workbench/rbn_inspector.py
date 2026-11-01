"""Read the observed RBNDL material and mesh-link records."""

from __future__ import annotations

import dataclasses
import hashlib
import math
import struct

from rbs_inspector import Cursor, RbsFormatError


MAX_FILE_BYTES = 64 * 1024 * 1024
MAX_BLOCKS = 4096
MAX_TEXTURES_PER_BLOCK = 256
MAX_STRING_BYTES = 16 * 1024
RBN_GENERAL_HASH = 0x0826F6EE
RBN_GENERAL_VERSION = 7
RBN_GENERAL_PROPERTY_BYTES = 84
END_BLOCK = 0x123BCDEF


@dataclasses.dataclass(frozen=True)
class RbnMaterial:
    block_index: int
    render_block_hash: int
    render_block_version: int
    vertex_scale: float
    uv_scales: tuple[tuple[float, float], ...]
    additional_uv_sets: int
    texture_slots: tuple[str, ...]
    geometry_reference_hash: int
    mesh_index: int
    end_marker: int

    @property
    def supported_vertex_layout(self) -> bool:
        return (
            self.render_block_hash == RBN_GENERAL_HASH
            and self.render_block_version == RBN_GENERAL_VERSION
            and self.additional_uv_sets <= 2
        )


def inspect_rbn(data: bytes, source_path: str = "") -> dict[str, object]:
    if len(data) > MAX_FILE_BYTES:
        raise RbsFormatError(f"RBN file exceeds inspection cap ({MAX_FILE_BYTES} bytes)")
    if len(data) < 33:
        raise RbsFormatError("File is too short to contain an RBNDL header")

    cursor = Cursor(data)
    prefix_length = cursor.u32("identifier length")
    if prefix_length != 5:
        raise RbsFormatError(f"Unsupported identifier length {prefix_length}; expected 5")
    signature = bytes(cursor.take(prefix_length, "identifier")).decode("ascii", errors="replace")
    if signature != "RBNDL":
        raise RbsFormatError(f"Unsupported material signature {signature!r}; only RBNDL is decoded")

    version_major = cursor.u32("header major version")
    version_minor = cursor.u32("header minor version")
    revision = cursor.u32("header revision")
    block_count = cursor.u32("material block count")
    geometry_reference_hash = cursor.u32("geometry reference hash")
    if block_count > MAX_BLOCKS:
        raise RbsFormatError(f"Material block count {block_count} exceeds cap {MAX_BLOCKS}")

    path_length = cursor.u32("geometry path length")
    if path_length > MAX_STRING_BYTES:
        raise RbsFormatError(f"Geometry path is too long ({path_length} bytes)")
    try:
        geometry_path = bytes(cursor.take(path_length, "geometry path")).decode("utf-8")
    except UnicodeDecodeError as error:
        raise RbsFormatError("Geometry path is not UTF-8") from error

    materials: list[RbnMaterial] = []
    for block_index in range(block_count):
        render_block_hash = cursor.u32(f"material {block_index} render-block hash")
        render_block_version = cursor.u8(f"material {block_index} render-block version")
        if render_block_hash != RBN_GENERAL_HASH or render_block_version != RBN_GENERAL_VERSION:
            raise RbsFormatError(
                f"Material {block_index} uses unsupported render block "
                f"0x{render_block_hash:08X} v{render_block_version}"
            )

        properties = cursor.take(RBN_GENERAL_PROPERTY_BYTES, f"material {block_index} RBNGeneral properties")
        vertex_scale = struct.unpack_from("<f", properties, 8)[0]
        uv_scales = tuple(struct.unpack_from("<2f", properties, 12 + uv_index * 8) for uv_index in range(3))
        additional_uv_sets = struct.unpack_from("<I", properties, 48)[0]
        if not math.isfinite(vertex_scale) or vertex_scale <= 0:
            raise RbsFormatError(f"Material {block_index} has invalid vertex scale {vertex_scale}")
        if additional_uv_sets > 2:
            raise RbsFormatError(f"Material {block_index} declares unsupported UV set count {additional_uv_sets}")

        texture_count = cursor.u32(f"material {block_index} texture slot count")
        if texture_count > MAX_TEXTURES_PER_BLOCK:
            raise RbsFormatError(f"Material {block_index} declares too many texture slots: {texture_count}")
        texture_slots: list[str] = []
        for slot_index in range(texture_count):
            text_length = cursor.u32(f"material {block_index} texture slot {slot_index} length")
            if text_length > MAX_STRING_BYTES:
                raise RbsFormatError(f"Material {block_index} texture path exceeds length cap")
            try:
                texture_slots.append(bytes(cursor.take(text_length, "texture path")).decode("utf-8"))
            except UnicodeDecodeError as error:
                raise RbsFormatError(f"Material {block_index} texture slot {slot_index} is not UTF-8") from error

        block_geometry_hash = cursor.u32(f"material {block_index} geometry hash")
        mesh_index = cursor.u32(f"material {block_index} mesh index")
        end_marker = cursor.u32(f"material {block_index} end marker")
        if block_geometry_hash != geometry_reference_hash:
            raise RbsFormatError(
                f"Material {block_index} geometry hash 0x{block_geometry_hash:08X} "
                f"does not match header 0x{geometry_reference_hash:08X}"
            )
        if end_marker != END_BLOCK:
            raise RbsFormatError(
                f"Material {block_index} has unexpected end marker 0x{end_marker:08X} "
                f"at byte 0x{cursor.offset - 4:X}"
            )
        materials.append(
            RbnMaterial(
                block_index,
                render_block_hash,
                render_block_version,
                vertex_scale,
                uv_scales,
                additional_uv_sets,
                tuple(texture_slots),
                block_geometry_hash,
                mesh_index,
                end_marker,
            )
        )

    if cursor.offset != len(data):
        raise RbsFormatError(f"Unexpected {len(data) - cursor.offset} trailing bytes at 0x{cursor.offset:X}")
    mesh_indices = [material.mesh_index for material in materials]
    if len(mesh_indices) != len(set(mesh_indices)):
        raise RbsFormatError("Multiple materials map to the same mesh index")

    return {
        "format": "RBNDL",
        "source_path": source_path,
        "file_size": len(data),
        "sha256": hashlib.sha256(data).hexdigest(),
        "header_version": {"major": version_major, "minor": version_minor, "revision": revision},
        "geometry_reference_hash": geometry_reference_hash,
        "geometry_path": geometry_path,
        "material_count": block_count,
        "materials": [dataclasses.asdict(material) | {"supported_vertex_layout": material.supported_vertex_layout} for material in materials],
        "decoder_status": "RBNGeneral material constants and texture slots decoded; texture semantics remain unknown.",
    }
