# RBS Structure Inspector Plan

> **Execution note:** Completed inline. This focused asset-workbench spike does not alter game runtime code.

**Goal:** Turn the stockpile's `RBNDL` + `RBSDL` pair into the first bounded, evidence-backed mesh preview, transparent catalogue thumbnail, and GLB export; use the mapping to define the next model-decoder boundary.

**Architecture:** Add standalone readers for the observed `RBNDL` material/link stream and `RBSDL` geometry block stream, then join material records to indexed meshes. Decode the confirmed `RBNGeneral` SNORM16 positions and primary UVs using the material scales, validate extents, and emit an independent WebGL preview, offline PNG thumbnail, and GLB. Keep game textures, lighting, normals/tangents, and other binary variants explicitly unsupported.

**Tech Stack:** Python standard library, existing local extraction and HTML preview tooling, ignored generated output under `target/content-index`.

**Spec:** `docs/superpowers/specs/2026-11-01-wasteland-asset-workbench-design.md`

## Scope and constraints

- Accept only the observed `RBSDL` signature; report other RBS variants as unsupported.
- Bound file size, block count, buffer counts, vertex/index counts, and every byte span before reading.
- Validate all declared data spans and the per-block end marker before emitting a structural report.
- Do not label guessed semantics as facts; keep the extra per-mesh count field as unknown metadata.
- Keep extracted game files and generated reports under ignored `target/content-index`.
- No live-game calls, runtime code changes, or asset redistribution. Offline rendering is limited to the selected local sample.

## Completion

- [x] Implement bounded `RBSDL` and observed `RBNGeneral` v7 readers.
- [x] Validate stream/index bounds, markers, material-to-mesh mapping, and decoded vertex extents.
- [x] Generate interactive flat-shaded preview, transparent thumbnail, and GLB for seven project meshes across stockpiles, maggot farm, Armory, Super Fuel, and Super Water.
- [x] Keep unsupported Scrap Crew and cleanup-crew material hashes visible as unsupported rather than guessing their layout.
- [x] Shorten generated metadata/report names when required by Windows path limits while retaining the original game resource path in metadata.
- [x] Expose preview image, model export, and structure summary in the local gallery manifest.
- [x] Refresh the research note and tool documentation with sample facts and external references.
- [x] Rebuild the local 38,467-resource gallery and verify generated JavaScript/GLB structure.

## Files

- Create `tools/asset_workbench/rbs_inspector.py` for format structures and parsing.
- Modify `tools/asset_workbench/prepare_preview.py` to recognize `.rbs` and render structural details.
- Modify `tools/asset_workbench/README.md` and `docs/research/game-content-inventory-2026-11-01.md` with the supported boundary and source attribution.
- Generated per-resource reports remain in `target/content-index/previews/`.

## Steps

1. Implement a little-endian bounded cursor and `RBSDL` parser. Report header version, bounds, block hashes/versions, vertex-buffer stride/count/size, index count, observed extra count, and terminator. Reject truncation, impossible values, index values outside a block’s vertex count, trailing bytes, and unknown terminators.
2. Add an HTML structural view for validated `.rbs` resources; retain the raw binary string fallback only for unsupported variants.
3. Parse the `RBNGeneral` material constants, texture string slots, geometry reference hash, and mesh index; preserve empty texture slots.
4. Join the selected stockpile material records to RBS mesh indices and decode SNORM16 position/UV triplets; verify resulting bounds against the serialized geometry bounds.
5. Generate the selected stockpile WebGL preview, transparent thumbnail, and GLB; make those discoverable from the local gallery.
6. Refresh the research note and workbench docs with exact sample values and citations to the format implementation/reference.

## Review focus

- Truncated or corrupted length fields must fail before any allocation or output is published.
- A valid but unsupported RBS signature must remain opaque, not be mislabeled as RBSDL.
- Indices must be interpreted only after the parser has established a valid mesh buffer layout.
- Bounds/order in the header must be labeled according to the external reader’s declared `max`/`min` fields, not reinterpreted from appearance.
