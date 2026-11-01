"""Read the structural envelope of Mad Max RBSDL geometry files."""

from __future__ import annotations

import dataclasses
import hashlib
import math
import pathlib
import struct
from collections.abc import Sequence


MAX_FILE_BYTES = 256 * 1024 * 1024
MAX_BLOCKS = 4096
MAX_VERTEX_BUFFERS = 8
MAX_VERTICES_PER_BUFFER = 20_000_000
MAX_INDICES_PER_BLOCK = 60_000_000
MAX_PREVIEW_VERTICES = 250_000
MAX_PREVIEW_INDICES = 750_000
END_BLOCK = 0x456BCDEF


class RbsFormatError(ValueError):
    """The input is unsupported, malformed, or exceeds inspection limits."""


@dataclasses.dataclass(frozen=True)
class BufferInfo:
    stride: int
    vertex_count: int
    byte_length: int


@dataclasses.dataclass(frozen=True)
class MeshInfo:
    block_index: int
    render_block_hash: int
    render_block_version: int
    vertex_buffers: tuple[BufferInfo, ...]
    post_vertex_buffer_count: int
    index_count: int
    minimum_index: int | None
    maximum_index: int | None
    end_marker: int

    @property
    def vertex_count(self) -> int:
        return self.vertex_buffers[0].vertex_count if self.vertex_buffers else 0


@dataclasses.dataclass(frozen=True)
class DecodedMesh:
    mesh_index: int
    render_block_hash: int
    render_block_version: int
    positions: tuple[tuple[float, float, float], ...]
    primary_uvs: tuple[tuple[float, float], ...]
    indices: tuple[int, ...]
    texture_slots: tuple[str, ...]


class Cursor:
    def __init__(self, data: bytes) -> None:
        self.data = data
        self.offset = 0

    def take(self, size: int, label: str) -> memoryview:
        if size < 0 or size > len(self.data) - self.offset:
            raise RbsFormatError(f"Truncated {label} at byte 0x{self.offset:X} (need {size} bytes)")
        start = self.offset
        self.offset += size
        return memoryview(self.data)[start : self.offset]

    def u8(self, label: str) -> int:
        return self.take(1, label)[0]

    def u16(self, label: str) -> int:
        return struct.unpack("<H", self.take(2, label))[0]

    def u32(self, label: str) -> int:
        return struct.unpack("<I", self.take(4, label))[0]

    def i32(self, label: str) -> int:
        return struct.unpack("<i", self.take(4, label))[0]

    def f32s(self, count: int, label: str) -> tuple[float, ...]:
        values = struct.unpack(f"<{count}f", self.take(count * 4, label))
        if any(not math.isfinite(value) for value in values):
            raise RbsFormatError(f"Non-finite float in {label}")
        return values


def inspect_rbs(data: bytes, source_path: str = "") -> dict[str, object]:
    if len(data) > MAX_FILE_BYTES:
        raise RbsFormatError(f"File exceeds inspection cap ({MAX_FILE_BYTES} bytes)")
    if len(data) < 49:
        raise RbsFormatError("File is too short to contain an RBSDL header")

    cursor = Cursor(data)
    prefix_length = cursor.u32("identifier length")
    if prefix_length != 5:
        raise RbsFormatError(f"Unsupported identifier length {prefix_length}; expected 5")
    signature = bytes(cursor.take(prefix_length, "identifier")).decode("ascii", errors="replace")
    if signature != "RBSDL":
        raise RbsFormatError(f"Unsupported render-block signature {signature!r}; only RBSDL is decoded")

    version_major = cursor.u32("header major version")
    version_minor = cursor.u32("header minor version")
    revision = cursor.u32("header revision")
    bounds_first = cursor.f32s(3, "first header bounds vector")
    bounds_second = cursor.f32s(3, "second header bounds vector")
    componentwise_min = tuple(min(first, second) for first, second in zip(bounds_first, bounds_second))
    componentwise_max = tuple(max(first, second) for first, second in zip(bounds_first, bounds_second))
    block_count = cursor.u32("mesh block count")
    if block_count > MAX_BLOCKS:
        raise RbsFormatError(f"Mesh block count {block_count} exceeds cap {MAX_BLOCKS}")

    meshes: list[MeshInfo] = []
    total_vertex_bytes = 0
    total_index_bytes = 0
    for block_index in range(block_count):
        render_block_hash = cursor.u32(f"block {block_index} render-block hash")
        render_block_version = cursor.u8(f"block {block_index} render-block version")
        vertex_buffer_count = cursor.u32(f"block {block_index} vertex-buffer count")
        if vertex_buffer_count > MAX_VERTEX_BUFFERS:
            raise RbsFormatError(f"Block {block_index} declares too many vertex buffers: {vertex_buffer_count}")

        buffers: list[BufferInfo] = []
        for buffer_index in range(vertex_buffer_count):
            stride = cursor.i32(f"block {block_index} buffer {buffer_index} stride")
            vertex_count = cursor.i32(f"block {block_index} buffer {buffer_index} vertex count")
            if stride <= 0 or stride > 4096:
                raise RbsFormatError(f"Block {block_index} buffer {buffer_index} has invalid stride {stride}")
            if vertex_count < 0 or vertex_count > MAX_VERTICES_PER_BUFFER:
                raise RbsFormatError(f"Block {block_index} buffer {buffer_index} has invalid vertex count {vertex_count}")
            byte_length = stride * vertex_count
            total_vertex_bytes += byte_length
            cursor.take(byte_length, f"block {block_index} vertex buffer {buffer_index}")
            buffers.append(BufferInfo(stride, vertex_count, byte_length))

        post_vertex_buffer_count = cursor.u32(f"block {block_index} post-vertex-buffer count")
        index_count = cursor.u32(f"block {block_index} index count")
        if index_count > MAX_INDICES_PER_BLOCK:
            raise RbsFormatError(f"Block {block_index} index count exceeds cap: {index_count}")
        minimum_index: int | None = None
        maximum_index: int | None = None
        index_bytes = cursor.take(index_count * 2, f"block {block_index} 16-bit indices")
        total_index_bytes += len(index_bytes)
        if index_count:
            minimum_index = 0xFFFF
            maximum_index = 0
            for (value,) in struct.iter_unpack("<H", index_bytes):
                minimum_index = min(minimum_index, value)
                maximum_index = max(maximum_index, value)
            if buffers and maximum_index >= buffers[0].vertex_count:
                raise RbsFormatError(
                    f"Block {block_index} index {maximum_index} is outside its first vertex buffer "
                    f"({buffers[0].vertex_count} vertices)"
                )

        end_marker = cursor.u32(f"block {block_index} end marker")
        if end_marker != END_BLOCK:
            raise RbsFormatError(
                f"Block {block_index} has unexpected end marker 0x{end_marker:08X} "
                f"at byte 0x{cursor.offset - 4:X}"
            )
        meshes.append(
            MeshInfo(
                block_index,
                render_block_hash,
                render_block_version,
                tuple(buffers),
                post_vertex_buffer_count,
                index_count,
                minimum_index,
                maximum_index,
                end_marker,
            )
        )

    if cursor.offset != len(data):
        raise RbsFormatError(f"Unexpected {len(data) - cursor.offset} trailing bytes at 0x{cursor.offset:X}")

    return {
        "format": "RBSDL",
        "source_path": source_path,
        "file_size": len(data),
        "sha256": hashlib.sha256(data).hexdigest(),
        "header_version": {"major": version_major, "minor": version_minor, "revision": revision},
        "bounds": {
            "header_first": list(bounds_first),
            "header_second": list(bounds_second),
            "componentwise_min": list(componentwise_min),
            "componentwise_max": list(componentwise_max),
        },
        "block_count": block_count,
        "vertex_count": sum(mesh.vertex_count for mesh in meshes),
        "index_count": sum(mesh.index_count for mesh in meshes),
        "vertex_payload_bytes": total_vertex_bytes,
        "index_payload_bytes": total_index_bytes,
        "meshes": [dataclasses.asdict(mesh) for mesh in meshes],
        "decoder_status": "Structure validated; vertex attributes and materials remain undecoded.",
    }


def inspect_file(path: pathlib.Path) -> dict[str, object]:
    if path.stat().st_size > MAX_FILE_BYTES:
        raise RbsFormatError(f"File exceeds inspection cap ({MAX_FILE_BYTES} bytes)")
    return inspect_rbs(path.read_bytes(), path.as_posix())


def decode_rbs_meshes(data: bytes, materials: Sequence[object]) -> dict[str, object]:
    structure = inspect_rbs(data)
    if structure["vertex_count"] > MAX_PREVIEW_VERTICES or structure["index_count"] > MAX_PREVIEW_INDICES:
        raise RbsFormatError(
            f"Mesh exceeds flat-preview cap ({MAX_PREVIEW_VERTICES:,} vertices, "
            f"{MAX_PREVIEW_INDICES:,} indices); structural counts remain available"
        )
    material_by_index = {material.mesh_index: material for material in materials}
    if not material_by_index:
        raise RbsFormatError("No RBN material-to-mesh links were provided")

    cursor = Cursor(data)
    cursor.u32("identifier length")
    cursor.take(5, "identifier")
    cursor.take(40, "RBSDL header")
    block_count = len(structure["meshes"])
    decoded: list[DecodedMesh] = []
    all_positions: list[tuple[float, float, float]] = []
    for block_index in range(block_count):
        render_block_hash = cursor.u32(f"block {block_index} render-block hash")
        render_block_version = cursor.u8(f"block {block_index} render-block version")
        buffer_count = cursor.u32(f"block {block_index} vertex-buffer count")
        buffers: list[tuple[int, int, memoryview]] = []
        for buffer_index in range(buffer_count):
            stride = cursor.i32(f"block {block_index} buffer {buffer_index} stride")
            vertex_count = cursor.i32(f"block {block_index} buffer {buffer_index} vertex count")
            buffers.append((stride, vertex_count, cursor.take(stride * vertex_count, f"block {block_index} vertex buffer")))
        cursor.u32(f"block {block_index} post-vertex-buffer count")
        index_count = cursor.u32(f"block {block_index} index count")
        index_bytes = cursor.take(index_count * 2, f"block {block_index} index buffer")
        cursor.u32(f"block {block_index} end marker")

        material = material_by_index.get(block_index)
        if material is None:
            raise RbsFormatError(f"No RBNDL material points to RBS mesh block {block_index}")
        if not material.supported_vertex_layout:
            raise RbsFormatError(f"Mesh {block_index} has an unsupported material vertex layout")
        if (render_block_hash, render_block_version) != (
            material.render_block_hash,
            material.render_block_version,
        ):
            raise RbsFormatError(f"RBN/RBS render-block identity mismatch for mesh {block_index}")
        if len(buffers) != 2:
            raise RbsFormatError(f"RBNGeneral mesh {block_index} declares {len(buffers)} vertex streams; expected 2")

        position_stride, vertex_count, position_bytes = buffers[0]
        uv_stride, uv_vertex_count, uv_bytes = buffers[1]
        if position_stride != 8 or uv_vertex_count != vertex_count:
            raise RbsFormatError(f"Mesh {block_index} has an unsupported RBNGeneral position stream")
        expected_uv_stride = 16 + material.additional_uv_sets * 4
        if uv_stride != expected_uv_stride:
            raise RbsFormatError(
                f"Mesh {block_index} UV/normal stream stride {uv_stride} does not match "
                f"the material's expected {expected_uv_stride}"
            )

        positions: list[tuple[float, float, float]] = []
        primary_uvs: list[tuple[float, float]] = []
        uv_scale = material.uv_scales[0]
        for vertex_index in range(vertex_count):
            px, py, pz = struct.unpack_from("<3h", position_bytes, vertex_index * position_stride)
            positions.append(
                (
                    max(-1.0, px / 32767.0) * material.vertex_scale,
                    max(-1.0, py / 32767.0) * material.vertex_scale,
                    max(-1.0, pz / 32767.0) * material.vertex_scale,
                )
            )
            u, v = struct.unpack_from("<2h", uv_bytes, vertex_index * uv_stride)
            primary_uvs.append((max(-1.0, u / 32767.0) * uv_scale[0], max(-1.0, v / 32767.0) * uv_scale[1]))

        indices = tuple(value for (value,) in struct.iter_unpack("<H", index_bytes))
        all_positions.extend(positions)
        decoded.append(
            DecodedMesh(
                block_index,
                render_block_hash,
                render_block_version,
                tuple(positions),
                tuple(primary_uvs),
                indices,
                tuple(material.texture_slots),
            )
        )

    if set(material_by_index) != set(range(block_count)):
        raise RbsFormatError("RBN material mesh indices do not cover the RBS block range exactly")

    header_bounds = structure["bounds"]
    tolerance = max(0.001, materials[0].vertex_scale / 32767.0 * 2)
    for axis in range(3):
        actual_min = min(position[axis] for position in all_positions)
        actual_max = max(position[axis] for position in all_positions)
        declared_min = header_bounds["componentwise_min"][axis]
        declared_max = header_bounds["componentwise_max"][axis]
        if abs(actual_min - declared_min) > tolerance or abs(actual_max - declared_max) > tolerance:
            raise RbsFormatError(
                f"Decoded positions do not fit declared bounds on axis {axis}: "
                f"[{actual_min}, {actual_max}] vs [{declared_min}, {declared_max}]"
            )

    structure["decoded_geometry"] = [dataclasses.asdict(mesh) for mesh in decoded]
    structure["decoded_bounds"] = {
        "min": [min(position[axis] for position in all_positions) for axis in range(3)],
        "max": [max(position[axis] for position in all_positions) for axis in range(3)],
        "within_header_bounds": True,
        "tolerance": tolerance,
    }
    structure["decoder_status"] = "RBNGeneral positions, primary UVs, and 16-bit triangle indices decoded; rendering is flat shaded."
    return structure
