# Local asset workbench tools

These tools operate on the installed Mad Max files and the ignored indexes under `target/content-index`. They do not modify game archives and must not write extracted game payloads into source-controlled directories.

## Selected resource extraction

`extract_resource.py` extracts one exact logical path already present in `path-census.csv`. It resolves the basename Jenkins hash through installed archive tables, prefers a patch record over a base record when both exist, and writes the selected payload plus provenance into `target/content-index/extracted/`. DLC-only paths select their matching DLC archive set. If multiple records in the selected archive set have different content, pass `--archive <archive-name>` to choose explicitly.

The extractor supports both unchunked and chunked raw-deflate archive entries. It enforces 128 MiB compressed and 256 MiB uncompressed limits. It is a local reverse-engineering aid; the result and sidecar are ignored build output and should not be committed, redistributed, or installed into the live mod directory.

Example, from the repository root:

```powershell
python tools/asset_workbench/extract_resource.py models/a00/propsenc/projects/a00_propsenc_projects_stockpile.model_xml
```

The checked-in tool currently requires the path census generated from the user-provided local `master.dirlist` and scans installed `.tab` files. It does not infer semantics from the file extension. Viewer tools should consume the sidecar provenance and report decode capability independently from archive presence.

## Selected-resource preview

`prepare_preview.py <logical-path> --bundle <exact-ee-path>` extracts one selected resource and writes a standalone read-only HTML report under `target/content-index/previews/`. UTF-8 text is shown as text, `.ddsc`/`.dds` images are converted to PNG when Pillow recognizes them, and compiled binaries show printable strings with an explicit compiled-data label. `rbn_inspector.py` parses the observed `RBNDL` `RBNGeneral` v7 material records; `rbs_inspector.py` validates `RBSDL` block and stream spans, 16-bit indices, and terminators. For this verified material/mesh pairing, the reader decodes SNORM16 positions, primary UVs, and indices, and checks decoded extents against the geometry header. The report provides an interactive flat-shaded WebGL view, an offline transparent thumbnail, and a GLB export. It does not yet apply game textures, recover normals/tangents, or reproduce the game's lighting/material shader. Other RBN/RBS variants remain unsupported until independently mapped. An FSB5 bank is identified as a bank, but the tool does not claim to decode or play its audio.

```powershell
python tools/asset_workbench/prepare_preview.py textures/00_general/vfx/a00_blood_erosion_alpha_dif.ddsc
python tools/asset_workbench/prepare_preview.py scripts/upgrades/acceleration.xvmc
```

The independent readers are informed by the public [ApexLib RBM/RBS loader](https://github.com/PredatorCZ/ApexLib/blob/master/src/RBM.cpp), [RBN material mapper](https://github.com/PredatorCZ/ApexLib/blob/master/src/RBNClasses.cpp), and [`RBNGeneralConstants` layout](https://github.com/PredatorCZ/ApexLib/blob/master/include/property_classes/RbnMaterial.h), checked against the installed stockpile sample. ApexLib is GPL-3.0; this project does not bundle or copy its implementation. The preview labels the two serialized bounds vectors as first/second because their apparent max/min order is reversed in the inspected sample; componentwise lower/upper values are shown separately.

For resources stored inside an `.ee` SARC package, supply the exact package and member path from `entity-bundle-members.csv`:

```powershell
python tools/asset_workbench/prepare_preview.py textures/a00/props/storagepiles/a00_pr_storagepiles_alphatest_dif.ddsc --bundle editor/entities/a00/propsenc/expentity/a00_propsenc_projects_stockpile_small_01.ee
```

`extract_bundle_member.py` first resolves and extracts the selected `.ee`, then slices only the verified member span from the local SARC index. It checks the span against the extracted bundle, caps output at 128 MiB, and stores the payload and provenance under `target/content-index/extracted/bundle-members/`. Neither tool changes installed game files.

## Shared inventory manifest

`build_manifest.py` merges the local path census, compiled bundle members, parsed entity profiles, reference edges, both spawn catalogs, and generated preview sidecars into `asset-workbench-resources.jsonl` with a summary JSON. Run it after preparing previews to refresh `preview_ready` and the local report/image paths. Same-stem source-reference and compiled paths remain separate records. Family labels and viewer capabilities are discovery metadata; they do not assert that a decoder exists or that an asset is spawnable.

Each JSONL record has a stable `resource_id`: `game-path:<case-folded normalized logical path>` for named paths and `game-basename:<case-folded basename>` for the path list's basename-only entries. These 816 basename-only records are kept distinct instead of being merged into one empty path. The generated inventory remains local under ignored `target/content-index/`; it includes metadata and relationships but no archive payloads or extracted previews.

## Searchable local gallery

After building the manifest, generate `target/content-index/asset-workbench.html`:

```powershell
python tools/asset_workbench/build_gallery.py
```

Open that HTML file in a browser to search the whole local inventory, filter by resource family and spawn status, inspect package/reference counts, and open any prepared read-only preview. The generated file embeds only compact metadata; game payloads and preview images remain in the ignored output tree. This offline gallery is the first usable workbench surface while the in-game browser and future Rust desktop companion continue to evolve.
