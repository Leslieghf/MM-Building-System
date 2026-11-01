"""Generate bounded sample geometry and offline previews for the asset schema."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import stat

from mesh_viewer import viewer_markup, write_glb, write_thumbnail


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
OUTPUT_ROOT = REPOSITORY_ROOT / "target/content-index/generated/procedural"
SCHEMA_VERSION = 1


def identity_with_translation(x: float, y: float, z: float) -> list[float]:
    return [
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        x, y, z, 1.0,
    ]


def transformed_bounds(vertices: list[list[float]], matrix: list[float]) -> dict[str, list[float]]:
    points = [
        [
            matrix[0] * x + matrix[4] * y + matrix[8] * z + matrix[12],
            matrix[1] * x + matrix[5] * y + matrix[9] * z + matrix[13],
            matrix[2] * x + matrix[6] * y + matrix[10] * z + matrix[14],
        ]
        for x, y, z in vertices
    ]
    return {
        "min": [min(point[axis] for point in points) for axis in range(3)],
        "max": [max(point[axis] for point in points) for axis in range(3)],
    }


def bounds_union(bounds: list[dict[str, list[float]]]) -> dict[str, list[float]]:
    return {
        "min": [min(item["min"][axis] for item in bounds) for axis in range(3)],
        "max": [max(item["max"][axis] for item in bounds) for axis in range(3)],
    }


def make_box() -> dict[str, object]:
    # Separate face vertices keep the preview's generated normals flat.
    faces = [
        ((-0.8, -0.8, 0.8), (0.8, -0.8, 0.8), (0.8, 0.8, 0.8), (-0.8, 0.8, 0.8)),
        ((0.8, -0.8, -0.8), (-0.8, -0.8, -0.8), (-0.8, 0.8, -0.8), (0.8, 0.8, -0.8)),
        ((0.8, -0.8, 0.8), (0.8, -0.8, -0.8), (0.8, 0.8, -0.8), (0.8, 0.8, 0.8)),
        ((-0.8, -0.8, -0.8), (-0.8, -0.8, 0.8), (-0.8, 0.8, 0.8), (-0.8, 0.8, -0.8)),
        ((-0.8, 0.8, 0.8), (0.8, 0.8, 0.8), (0.8, 0.8, -0.8), (-0.8, 0.8, -0.8)),
        ((-0.8, -0.8, -0.8), (0.8, -0.8, -0.8), (0.8, -0.8, 0.8), (-0.8, -0.8, 0.8)),
    ]
    vertices: list[list[float]] = []
    indices: list[int] = []
    for face in faces:
        start = len(vertices)
        vertices.extend([list(point) for point in face])
        indices.extend((start, start + 1, start + 2, start, start + 2, start + 3))
    transform = identity_with_translation(-1.25, 0.0, 0.0)
    return {
        "vertices": vertices,
        "indices": indices,
        "local_transform": transform,
        "material_slots": [{"key": "preview:material/stone"}],
        "triangle_materials": [0] * (len(indices) // 3),
        "bounds": transformed_bounds(vertices, transform),
    }


def make_triangular_prism() -> dict[str, object]:
    vertices = [
        [-0.8, 0.0, -0.55], [0.8, 0.0, -0.55], [0.0, 1.25, -0.55],
        [-0.8, 0.0, 0.55], [0.8, 0.0, 0.55], [0.0, 1.25, 0.55],
    ]
    # Two triangular ends and three rectangular sides, split into triangles.
    indices = [
        0, 1, 2, 3, 5, 4,
        0, 3, 4, 0, 4, 1,
        1, 4, 5, 1, 5, 2,
        2, 5, 3, 2, 3, 0,
    ]
    transform = identity_with_translation(1.25, 0.0, 0.0)
    return {
        "vertices": vertices,
        "indices": indices,
        "local_transform": transform,
        "material_slots": [
            {"key": "preview:material/metal"},
            {"key": "preview:material/accent"},
        ],
        "triangle_materials": [0, 0, 1, 1, 0, 0, 1, 1],
        "bounds": transformed_bounds(vertices, transform),
    }


def preview_mesh(
    primitive: dict[str, object], mesh_index: int, material_index: int
) -> dict[str, object]:
    transform = primitive["local_transform"]
    vertices = primitive["vertices"]
    transformed = [
        [
            transform[0] * point[0] + transform[4] * point[1] + transform[8] * point[2] + transform[12],
            transform[1] * point[0] + transform[5] * point[1] + transform[9] * point[2] + transform[13],
            transform[2] * point[0] + transform[6] * point[1] + transform[10] * point[2] + transform[14],
        ]
        for point in vertices
    ]
    return {
        "mesh_index": mesh_index,
        "positions": transformed,
        "indices": [
            index
            for triangle, slot in enumerate(primitive["triangle_materials"])
            if slot == material_index
            for index in primitive["indices"][triangle * 3 : triangle * 3 + 3]
        ],
        "primary_uvs": [[0.0, 0.0] for _ in transformed],
    }


def create_documents() -> tuple[dict[str, object], dict[str, object], dict[str, object], dict[str, object]]:
    primitives = [make_box(), make_triangular_prism()]
    geometry = {
        "schema_version": SCHEMA_VERSION,
        "key": "preview:geometry/schema_examples",
        "primitives": primitives,
        "bounds": bounds_union([item["bounds"] for item in primitives]),
    }
    descriptor = {
        "key": "preview:geometry/schema_examples",
        "source": {"kind": "generated", "locator": "procedural_geometry.py"},
        "family": "geometry",
        "dependencies": [],
        "decode": "decoded",
        "preview": "available",
        "confidence": "confirmed",
    }
    meshes = []
    for primitive in primitives:
        for material_index in range(len(primitive["material_slots"])):
            meshes.append(preview_mesh(primitive, len(meshes), material_index))
    manifest = {
        "schema_version": SCHEMA_VERSION,
        "generator": "procedural_geometry.py",
        "runtime_support": "preview_only",
        "validation": {"status": "structural_validation_not_performed", "native_runtime_verified": False},
        "source": descriptor["source"],
        "asset_key": geometry["key"],
        "bounds": geometry["bounds"],
        "primitives": [
            {
                "name": name,
                "bounds": primitive["bounds"],
                "material_slots": [slot["key"] for slot in primitive["material_slots"]],
            }
            for name, primitive in zip(("box", "triangular_prism"), primitives)
        ],
        "outputs": ["asset-descriptor.json", "geometry-asset.json", "preview.html", "preview.glb", "thumbnail.png"],
    }
    return descriptor, geometry, {"decoded_geometry": meshes}, manifest


def write_html(path: Path, mesh_data: dict[str, object]) -> None:
    markup = viewer_markup(mesh_data)
    document = f"""<!doctype html>
<html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Procedural geometry schema preview</title>
<style>
html,body{{margin:0;min-height:100%;background:#101820;color:#dce5eb;font:14px system-ui,sans-serif}}
main{{max-width:900px;margin:24px auto;padding:0 16px}}
h1{{font-size:18px;font-weight:600}}.mesh-viewer{{border:1px solid #34434e;background:#17232c;padding:10px}}
.mesh-viewer-title,.mesh-viewer-help{{color:#9db0bd;font-size:12px;padding:4px}}
#mesh-canvas{{display:block;width:100%;height:min(62vh,560px);background:#0e151b}}
</style><main><h1>Procedural geometry · preview only</h1>
<p>One box and one triangular prism. Game rendering, collision, and asset loading are not implied.</p>
{markup}</main></html>"""
    path.write_text(document, encoding="utf-8")


def confined_output_path(requested: Path) -> Path:
    """Resolve output lexically, rejecting directory symlinks/junctions."""
    repository = REPOSITORY_ROOT.resolve()
    allowed_root = repository / "target" / "content-index"
    output = Path(os.path.abspath(requested))
    try:
        relative = output.relative_to(allowed_root)
    except ValueError as error:
        raise ValueError("output must stay under target/content-index") from error

    current = repository
    parts = (*allowed_root.relative_to(repository).parts, *relative.parts)
    reparse_flag = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400)
    for part in parts:
        current = current / part
        if not current.exists() and not current.is_symlink():
            continue
        metadata = current.lstat()
        if current.is_symlink() or getattr(metadata, "st_file_attributes", 0) & reparse_flag:
            raise ValueError(f"output path contains a symlink or junction: {current}")

    resolved_root = allowed_root.resolve()
    resolved_output = output.resolve()
    if not resolved_output.is_relative_to(resolved_root):
        raise ValueError("resolved output escapes target/content-index")
    return output


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=OUTPUT_ROOT, help="ignored output directory (default: target/content-index/generated/procedural)")
    args = parser.parse_args()
    try:
        output = confined_output_path(args.output)
    except ValueError as error:
        parser.error(str(error))

    descriptor, geometry, mesh_data, manifest = create_documents()
    output.mkdir(parents=True, exist_ok=True)
    try:
        output = confined_output_path(output)
    except ValueError as error:
        parser.error(str(error))
    (output / "asset-descriptor.json").write_text(json.dumps(descriptor, indent=2) + "\n", encoding="utf-8")
    (output / "geometry-asset.json").write_text(json.dumps(geometry, indent=2) + "\n", encoding="utf-8")
    write_html(output / "preview.html", mesh_data)
    (output / "preview.glb").write_bytes(write_glb(mesh_data, "procedural-schema-examples"))
    has_thumbnail = write_thumbnail(mesh_data, output / "thumbnail.png")
    if not has_thumbnail:
        manifest["outputs"].remove("thumbnail.png")
    (output / "preview-manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")

    print(f"Generated preview-only box and triangular prism in {output}")
    print("Wrote asset-descriptor.json, geometry-asset.json, preview-manifest.json, preview.html, and preview.glb")
    if has_thumbnail:
        print("Wrote transparent thumbnail.png")
    else:
        print("PNG thumbnail unavailable (install Pillow to enable offline raster thumbnails)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
