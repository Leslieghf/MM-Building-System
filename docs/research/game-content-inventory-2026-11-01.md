# Mad Max game-content inventory

## Purpose

Build a broad, evidence-backed map of shipped content and its relationships, separately from the curated spawn browser. The inventory should help answer:

- What named resources exist in this game build, and which installed archive contains each one?
- Which entries are standalone entity definitions, multi-object compositions, encounters, or support resources?
- Which individual vehicles, props, characters/NPCs, models, textures, collision assets, scripts, effects, and audio resources are referenced by each entity?
- Which resources have a plausible spawn route, and which have actually been tested?
- Which assets can later be rendered or extracted for spawn-menu thumbnails?

“Not mapped yet” must remain distinct from “does not exist.” The current 467-row spawn browser is a list of known spawn attempts, not an inventory of the game.

## Evidence already available

- The installed game has **39 base `archives_win64/game*.arc` + `.tab` pairs**, plus separate patch and DLC archive sets. The base archives are around 32 GB in aggregate; the path-to-hash index is the main barrier to broad inspection.
- The current curated catalog has 10 rows; its community import has 456 rows from `gigaHours/MadMaxSpawner`. Those community keys mix vehicles, encounters, loot compositions, missions, projects, and other entities. They are useful discovery leads, not a complete or type-safe catalog.
- A public Mad Max save-research project documents a Gibbed path index containing **27,501 game paths**, along with working readers for the game's `.arc/.tab`, SARC bundles, RTPC entity data, ADF data, and XVM scripts. Its `docs/GAME-FILES.md` records how its author used the index and parsers to inspect real archive entries.
- The index maps `basename:logical/path` (examples include `...ee:editor/entities/...`). The archive table stores a Jenkins hash of the basename, so joining by basename can be ambiguous; preserve every candidate path and verify the actual archive lookup rather than assuming each hash has one unique logical path.
- Existing local research has already inspected several `.ee` bundles. It shows why filename semantics alone are insufficient: `a01_static_wreck_0006_birds` contains multiple gameplay components and many resources, while the user-observed stockpile bundle contains one `CRigidObject` with a model and Havok collision dependency. That stockpile is currently the only green/verified spawn entry.

## First local archive census

The upstream `master.dirlist` is now present as a local input under the ignored build-output directory. A basename-hash join against every installed `.tab` table produced:

- **27,501 / 27,501** path records had a matching hash in the installed archive sets.
- Scanned **44** tables: 39 base-game tables, one patch table, and four DLC tables.
- The path list itself has **no repeated basenames** and **no different paths sharing a hash**. The archive tables contain 47,603 resource records and 47,162 distinct hashes; many table entries are not named by this path list.
- This is strong installed-build name/index coverage. A basename hash hit is still not a decoded semantic definition or proof that a resource can be spawned.

The local generated index is `target/content-index/path-census.csv` (all named paths, basename-only rows, file extensions, top-level folder, and archive-set hit) and `target/content-index/census-summary.json`. It contains **26,685 full logical paths and 816 basename-only entries**. These are local generated analysis outputs; the upstream path list and game payloads are not added to version control.

### Workbench preparation and representative format probes

The initial offline workbench now has three local-only pieces under `tools/asset_workbench/`: exact-path archive extraction, a merged resource manifest, and selected-resource preview reports. Extraction uses the existing path census and archive tables, resolves basename hashes, handles chunked and unchunked raw-deflate entries, applies output caps, and writes payloads plus provenance only beneath ignored `target/content-index/extracted/`. The JSONL inventory is generated at `target/content-index/asset-workbench-resources.jsonl`; it currently has **38,467 distinct rows**, including 816 basename-only records. Full paths use `game-path:<case-folded normalized logical path>` as their stable local `resource_id`, while basename-only entries use separate `game-basename:<case-folded basename>` IDs. The manifest output stays local and contains metadata/relationships rather than game bytes.

Four representative paths were extracted and inspected locally:

| Resource | Confirmed representation | What this establishes |
| --- | --- | --- |
| `textures/00_general/vfx/a00_blood_erosion_alpha_dif.ddsc` | Starts with the standard `DDS ` signature, reports DXT5 in its pixel format, and is accepted by Pillow as a 1024×256 RGBA image; a PNG and standalone HTML preview were generated. | This `.ddsc` sample is a conventional DDS payload that can be previewed by the offline pipeline. It does not prove every `.ddsc` variant is supported. |
| `models/a00/environment/vegstone/a00_env_vegstone_s_03.lod` | UTF-8 text listing `_lod1.rbn`, `_lod2.rbn`, and `_lod3.rbn` paths, followed by two `-` fields and `5.0`. | At least this `.lod` is a small model/LOD manifest, not geometry bytes. Its referenced `.rbn` files remain separate resources to inspect. |
| Stockpile `.lod` and its three `.rbn` members | The package `.lod` lists three LOD-specific `.rbn` paths, two `-` fields, and trailing float `2.29999995232`. Each of the three corresponding `.rbn` members has ASCII `RBNDL` at byte offset 4 and contains one LOD-specific `.rbs` path plus 12 `.ddsc` texture paths. All 13 referenced members for each inspected `.rbn` exist in that same `.ee` package. | This is a concrete model dependency chain: entity package → `.lod` → `.rbn` resource links → geometry `.rbs` and texture `.ddsc` members. The trailing float's meaning and the actual RBS geometry format are still unknown. |
| `scripts/upgrades/acceleration.xvmc` | Compiled binary with 52 printable string runs, including symbols such as `GetEngineWeightAdjustment`, `GetEnginePowerPeakRpm`, and `ActivateEnhacementLevel`. The preview labels it compiled data, not source code. | The compiled script carries useful names, but this extraction does not recover the original script or establish executable semantics. |
| `sound/fmod/chaos_destruction_bone.fsb` | Begins with `FSB5`; the preview identifies it as an FMOD bank and shows printable strings only. No audio playback is claimed. | Bank recognition is not audio decoding. A separate compatible decoder is still required for auditioning. |
| `editor/entities/a00/propsenc/expentity/a00_propsenc_projects_stockpile_small_01.ee` → `textures/a00/props/storagepiles/a00_pr_storagepiles_alphatest_dif.ddsc` | The bundle-member index resolves the texture to byte offset 2,134,584 and size 349,672. The bounded nested extractor wrote the exact member and Pillow decoded it as a 1024×512 RGBA preview. | We can now reach and preview actual compiled resources inside a known `.ee` SARC, not only standalone outer-archive paths. The bundle and member remain separately recorded in provenance. |

These are selected, representative probes rather than format-wide compatibility claims. The preview command writes standalone HTML reports and sidecars below ignored `target/content-index/previews/`, referencing local images and extraction provenance. The manifest can merge those sidecars to mark inspected rows as preview-ready and now carries the `.rbn` dependency paths; nine selected resources currently have previews. A self-contained gallery at ignored `target/content-index/asset-workbench.html` can search/filter the full inventory and open prepared reports without loading archives or media in-game. The initial extractor and preview utilities use the repository's existing Python research tooling; the in-game overlay remains Rust and is intended to consume prepared records/previews only.

### First validated RBS mesh structure

The stockpile model chain has now been followed through to its actual geometry member:
`a00_propsenc_projects_stockpile_small_01.ee` → `...stockpile_small_lod1.rbn` →
`a00_propsenc_projects_stockpile_small_lod1ge0ge2ge0.rbs`.

The selected `.rbs` is **58,436 bytes** (`SHA-256 04215e7bb9ead88a550e1b70b08371702f8f727c0b44bbeddd86ccedcebd94be`). It has the five-byte `RBSDL` signature, header version `2.0.0`, and three geometry blocks. The independent inspector validates **2,027 vertices**, **4,656 16-bit indices**, all block end markers (`0x456BCDEF`), and every index against its block's vertex count. The three blocks use render-block hash `0x0826F6EE`, version 7, with stream layouts:

| Block | Vertex streams (stride × vertex count) | Indices | Observed index range |
| ---: | --- | ---: | --- |
| 0 | `8 × 1,828`; `16 × 1,828` | 4,032 | `0–1,827` |
| 1 | `8 × 79`; `20 × 79` | 306 | `0–78` |
| 2 | `8 × 120`; `16 × 120` | 318 | `0–119` |

The file declares two three-component bounds vectors. In this sample their component order is numerically inverted relative to the usual max/min labels; therefore the inspector retains the serialized vectors as **first** and **second** and reports componentwise lower/upper values separately. Componentwise bounds are `[-2.41927, -0.400694, -2.35449]` to `[0.0128682, 1.39704, 3.30597]`.

This structural parse was compared with the public [ApexLib `LoadRBSDL` implementation](https://github.com/PredatorCZ/ApexLib/blob/master/src/RBM.cpp), whose reader describes the block sequence: render-block hash/version, vertex-buffer stride/count and payloads, an additional count field, 16-bit index payload, then the block terminator. ApexLib's README lists Mad Max `RBN` and `RBS` among the supported formats and notes that they are roughly converted into ADF form ([repository README](https://github.com/PredatorCZ/ApexLib)). Its source is GPL-3.0; no ApexLib code is included in this project. The new Python inspector independently reads only the bounded structural envelope. Vertex attribute packing, material constants/scales, UV decoding, and actual mesh rendering remain unsolved. In particular, **2,027/4,656 counts are structural results, not proof of visually correct topology**.

The local report is now usable from the gallery: prepared image previews appear as small row thumbnails, image detail is shown in the selection pane, and the stockpile `.rbs` entry displays its validated block and vertex/index counts. This remains an offline prepared-data workflow; no archive parsing or decoding was added to Mad Max's render callback.

### Resource-family counts

These are path counts, not object counts. Unknown extensions and extensionless entries remain included in the full CSV.

| Family / extension | Paths | What it currently tells us |
| --- | ---: | --- |
| `animations/` | 12,919 | The largest named content family; its formats and actor links need separate mapping. |
| `.ban` | 10,664 | Animation-related resources by path/name; exact semantics not assigned to every file. |
| `.ddsc` | 2,479 | Texture resources across the full index, including paths outside `textures/`. |
| `locations/` | 3,777 | Region/location content; likely a major route into streamed world compositions. |
| `terrain/` | 1,856 | Terrain family; relationships to world decoration and stream chunks remain to map. |
| `.bl` / `.blo` | 1,460 / 931 | Bundle and RTPC-like families; representative `.bl`/`.blo` structures are documented upstream, but the entire set is not yet validated. |
| `.ee` / `.epe` | 467 / 388 | Entity bundles and entity-definition resources; these are the highest-priority object/encounter census targets. |
| `.gsrc` / `.xvmc` | 428 / 576 | Graph and gameplay-script families. |
| `models/` | 256 | Relatively small named model-resource family; these are dependencies, not necessarily standalone spawn definitions. |
| `.rbs` / `.rbn` / `.lod` | 164 / 31 / 41 | Model/geometry-related resources based on inspected assets; contracts still need bulk confirmation. |
| `.pfxc` | 19 | Collision/physics resources by extension; inspect and verify each before assigning behavior. |
| `.mdic` | 38 | Streaming/decorative map-index family, already identified as a promising world-placement lead. |

Top-level path counts include `animations` 12,919; `locations` 3,777; `terrain` 1,856; `textures` 1,450; `editor` 1,211; `effects` 1,127; `datamaps` 1,024; `missions` 904; `ai` 643; `scripts` 575; and `graphs` 428. The `.ddsc` count exceeds the `textures/` directory count, so texture files also live under other logical path roots.

The namespace counts show this is not mainly an “objects folder”: there are more than 12,900 animation paths, 3,777 location paths, 1,856 terrain paths, and 1,127 effects paths. Other notable roots are `datamaps` (1,024), `missions` (904), `ai` (643), `scripts` (575), `graphs` (428), `models` (256), `gui` (138), `dlc` (133), and `sound` (77). The game content is spread across authored entity definitions plus extensive world, behavior, presentation, and streaming support data.

Complete extension-count snapshot from the index (path counts): `.ban` 10,664; `.ddsc` 2,479; `.dat` 1,855; `.bl` 1,460; `.nl` 1,395; `.asb` 1,126; `.afsmb` 1,116; `.effc_link` 1,099; `.blo` 931; no extension 816; `.fl` 713; `.xvmc` 576; `.navmeshc` 541; `.toc` 512; `.ee` 467; `.gsrc` 428; `.epe` 388; `.stringlookup` 207; `.rbs` 164; `.btc` 101; `.bin` 64; `.guixc` 45; `.lod` 41; `.mdic` 38; `.pfic` 32; `.rbn` 31; `.pfs` 31; `.fsb` 30; `.revc` 29; `.effc` 27; `.pfxc` 19; `.brd` 12; `.fev` 11; `.rawc` 5; and `.font` 5. These are extension labels and counts only; several proprietary suffixes still need format confirmation.

### Entity-bundle breakdown

The 467 `.ee` records divide by logical namespace as follows:

| Namespace | Count | Initial interpretation |
| --- | ---: | --- |
| `editor/entities/a00/encounters/**` | 213 | Encounter-oriented packages; inspect their definitions for child objects and actor/vehicle composition. |
| `editor/entities/a01/encounters/**` | 88 | Same broad family for another area/content set. |
| `editor/entities/a00/activities/**` | 99 | Activities, including convoy leaders/compositions and 40 deathrun entries. |
| `editor/entities/a00/exported/**` | 43 | 40 `a00_vehicle_empty_*` entries plus three named side-mission vehicle entries: a strong standalone-vehicle candidate family. |
| `editor/entities/a00/propsenc/expentity/**` | 22 | Project/propsenc bundles, including locked/unlocked variants; not automatically standalone props. |
| Other mission/DLC paths | 2 | One mission entity and one DLC entity outside the main `editor/entities` namespace. |

The index contains **466 `.ee` paths under `editor/entities/`**, plus one DLC path. The spawn catalog now has one row per local `.ee` package, but the two sets still differ in meaning: community keys can name compositions and other entity kinds, and local presence does not establish spawn behavior.

There is not an obvious top-level `characters/` model family. Character leads show up in filenames such as `..._driver.ee`, `..._boarders.ee`, and `..._driver_db.ee`, mostly under encounter/activity folders. That supports your suspicion that the current inventory over-represents assembled encounters; it does **not** prove the game lacks individual NPC definitions. The embedded RTPC class tree now gives us a way to distinguish character definitions from compositions; the next step is to connect their resource references to the wider asset inventory.

The 22 project bundles include paired unlocked/locked entries for armory, cleanup crew, doghouse, maggot farm, scrap crew, stockpiles, super fuel, super water, and survey crew. The inventory should preserve these variants but group them as related project definitions rather than presenting each as a generic prop.

### What is inside the `.ee` bundles

All **467 of 467** `.ee` archive entries have now had their SARC v2 member indexes read. This reads the bundle directory tables without writing/extracting the game payloads. The bundles contain **657,984 member records** in total and report about **20.14 GB compressed / 38.94 GB uncompressed** across archive entries. These totals count repeated dependencies many times; they are not unique asset counts.

Every indexed bundle contains exactly one `.epe` member. The other member records show that an `.ee` is a dependency package, not a single model file: the index records **84,749 model-like members**, **13,164 collision-resource members**, **44,413 textures**, **65,485 script/graph-like members**, plus large animation, effect, sound, localization, and material families. These are also repeated references across bundles, not unique resources. The biggest convoy packages have over **2,100 members** and around **150 MB uncompressed** each. By contrast, the tested small stockpile bundle has **37 members** and is a compact project entity with a real collision dependency. This strongly supports keeping “entity package,” “its referenced assets,” and “number of spawned world objects” as separate inventory fields.

Deduplicating those member records yields **10,944 unique compiled member paths** across the 467 bundles (657,984 total bundle-to-resource occurrences). The biggest unique-path families are 4,670 localized `stringlookup` paths, 1,398 `.ban` animation assets, 1,057 `.rbn` resources, 1,029 `.rbs` geometry/model candidates, 585 `.ddsc` textures, 467 embedded `.epe` definitions, 398 `.lod` model candidates, 276 `.pfxc` collision resources, and 274 `.xvmc` compiled scripts. By occurrence, `.ban` assets dominate at 326,005 bundle references; compiled scripts occur 58,444 times, textures 44,413, `.rbs` 34,278, and collision `.pfxc` 13,164. These counts make dependency reuse visible: a small set of compiled assets is shared across hundreds of entity packages.

### `.epe` definitions: structural RTPC parse

The public Gibbed parser identifies `.epe` as an RTPC property container: `RTPC` magic, version 1, a node table with hashed names and child/property counts, typed property records, and data offsets relative to the file. The parser also documents the variant tags used here. We implemented a read-only decoder from that format description and validated it against the known stockpile definition, including its `CRigidObject` class, `filename`, and world-transform properties. The `_class_hash` values we could compare matched the Jenkins hash of `_class` in every decoded bundle. See [Gibbed's PropertyContainerFile.cs](https://github.com/gibbed/Gibbed.MadMax/blob/master/Gibbed.MadMax.PropertyFormats/PropertyContainerFile.cs) and [StringHelpers.cs](https://github.com/gibbed/Gibbed.MadMax/blob/master/Gibbed.MadMax.FileFormats/StringHelpers.cs).

The package-embedded definition is the important corpus: all **467 of 467** `.ee` bundles contain exactly one `.epe`, and all 467 embedded definitions now parse structurally with **zero failures**. Together they contain **649,417 RTPC nodes**, of which **648,950** carry a resolved class name; there are **138 distinct class names**. The artificial RTPC root has 7,901 direct, class-tagged children across the corpus, classified into **41 root-level classes**. Examples include `CCharacter` (960), `CCar` (471), `CRigidObject` (224), `CStaticDecalObject` (597), and `CEventTrigger` (1,237). These are direct children in entity definitions, not a count of every world object or proof that each definition is independently spawnable. Only **29 definitions** have one direct class-tagged child: 16 `CCar`, 11 `CRigidObject`, and 2 `CCharacter`. The other **438** contain multiple direct records, often assemblies or gameplay systems. This is strong structural evidence that most `.ee` entries are compositions rather than single spawnable objects.

The most frequent nested class tags include `SEventTriggerComponent` (98,848), `CEventTrigger` (94,146), `CBoneOffsetAlias` (37,795), `CScriptGameObject` (37,452), and `CEffectSpawner` (29,729). This shows why a flat “component names per file” list is insufficient: the definitions include large nested gameplay/interaction graphs as well as top-level characters, cars, rigid props, triggers, and other objects. The structural tree now distinguishes those levels.

String-valued RTPC properties yielded **142,639 path-like reference edges** across all 467 definitions, pointing to **1,056 distinct source-style path strings**. The largest extension families are `.xpy` scripts (61,555 edges), `.gsr` graphs (21,323), `.model_xml` models (19,194), `.al` animation layers (16,282), and `.pfx` collision resources (8,498). These are references, not unique resources or proof of runtime loading. Their source node class and raw property hash are retained so we can resolve property semantics instead of guessing. The stockpile's `CRigidObject.filename` edge is one validated example.

The literal source-style paths do not appear as exact names in the installed path census or `.ee` member index because bundles store compiled variants. A same-stem comparison against real SARC member paths found a compiled-member candidate for **962 of 1,056** unique references, covering **135,071 of 142,639** reference edges. Observed suffix correspondences include `.xpy` → `.xvmc`, `.gsr` → `.gsrc`, `.pfx` → `.pfxc`, `.bts` → `.btc`, `.ccenemy` → `.ccenemyc`, and `.vengine` → `.venginec`; model and animation references can have multiple packed forms (`.model_xml` → `.lod`/`.rbs`, `.al` → `.asb`/`.afsmb`/`.ban`). These are high-value dependency candidates because their stems agree with real packaged members; they still need per-class validation before treating every mapping as a loader rule. The remaining 94 source paths have no same-stem member candidate under this first-pass mapping.

We joined those candidate paths back to the SARC member index. For all **135,071 matched reference edges**, at least one compiled member candidate is present in the referring `.ee` package itself. This confirms the relationship is package-local, while the exact suffix mapping and property meaning remain candidates rather than loader guarantees. The expansion creates **151,353 candidate edge rows** because some source paths (notably model and animation resources) have more than one same-stem compiled form.

### Property-hash and resource-role profile

Grouping each path reference by `(node class, raw property hash, extension)` yields **50 recurring class/hash/extension combinations**. This gives us a better route to naming RTPC fields: compare a hash across many definitions and classes, then validate it against an inspected known-good entity. It is not safe to assume a hash has one universal meaning across all node classes.

The strongest current patterns are:

| Class / property hash | Repeated value family | Evidence and interpretation |
| --- | --- | --- |
| `CRigidObject` / `0x32B409E0` | `.model_xml` | 3,093 edges across 401 definitions; includes the stockpile-small model field already validated in the working spawn package. Treat as a high-confidence model-field mapping for this class. |
| `CRigidObject` / `0xF16ECA7D` | `.pfx` | 4,039 edges across 409 definitions, with collision-named resources such as `_col.pfx`; strong collision-resource field candidate. |
| `CPartProp` / `0x0F94740B` | `.model_xml` | 6,335 edges across 353 definitions; repeated part-model field pattern. |
| `CPartProp` / `0x536C28F0` | `.pfx` | 3,629 edges across 353 definitions; repeated part-collision field pattern. |
| `CScriptGameObject` / `0xCD2F3871` | `.xpy` | 37,452 edges across 431 definitions; strong script-reference field candidate. |
| `CGraphScriptGameObject` / `0xE1E4AD4A` | `.gsr` | 21,323 edges across 460 definitions; strong graph-reference field candidate. |
| `CCar` / `0xAB0E5193` | `.vpgeneral` | 471 edges across 353 definitions; vehicle physics configuration candidate. |
| `CCar` / `0xB72DC753` | `.vengine` | 471 edges across 353 definitions; vehicle engine configuration candidate. |

These patterns are substantially stronger than suffix-only matching, because the same class/hash pair recurs across hundreds of packages and its values cluster into one resource family. The stockpile check anchors the `CRigidObject` model property to an already-inspected definition. The remaining rows are semantic candidates until we inspect the corresponding RTPC nodes and compare more known entities; they are not yet new public API contracts.

The generated `epe-property-reference-profile.csv` keeps all 50 groups with edge counts, definition counts, unique paths, and examples. It is an analysis index, not a replacement for the raw edge table.

### First structural catalog profile

The structural profiles join all 467 local entity definitions to their top-level RTPC classes, archive/member sizes, path-reference counts, and the curated/community spawn catalog. All **467 current catalog entries** match local `.ee` definitions after adding the Rockstar ATV candidate: `dlc/rockstar/editor/entities/a00/exported/a00_vehicle_empty_rockstaratv.ee`, a one-root `CCar` package.

The 29 one-root definitions are the clearest simple-root candidates, but “one root” means only one direct RTPC record, not one physical part or a known safe standalone spawn. The 11 one-root `CRigidObject` definitions are all stronghold-project assets (armory, cleanup crew, doghouse, maggot farm, scrap crew, three stockpile sizes, super fuel, super water, survey crew). Some still contain multiple rigid-object records below that root: stockpile medium has four `CRigidObject` nodes, doghouse has three, and armory has two. In the stockpile-small definition, the root rigid object’s `filename` reference resolves to the stockpile model candidate and its sibling collision path resolves to the compiled `.pfxc` member in the same bundle. They are useful rigid-object references, but they do not represent the full loose-prop catalog. The two one-root `CCharacter` definitions are critter encounters (`encounter_lizard_01` and `encounter_rat_01`). The 16 one-root `CCar` definitions each contain exactly one `CCar` node and provide a compact vehicle-focused follow-up set.

Most packages are compositions: **438 definitions have multiple direct records**. Of all 467 definitions, **309 have both top-level `CCar` and `CCharacter` records**, consistent with many vehicle packages carrying their drivers/crew together. **97** contain a top-level `CRigidObject`; only 11 have one direct `CRigidObject` root. This explains why a spawn key that sounds like a vehicle or prop may actually load an assembly with characters, triggers, scripts, sounds, decals, and placement markers. Catalog kind/readiness should not be inferred from one nested class or filename token.

Before this pass, the 466-entry catalog labeled 418 entries `composition`, 40 `vehicle`, 4 `encounter`, 3 `unknown`, and 1 `prop`. Adding the Rockstar ATV brings the catalog to **467 entries**. Its local package has one direct `CCar` root and Rockstar-specific vehicle physics, with engine and solver resources shared with other vehicles; spawn behavior is unverified. The 40 prior catalog vehicles also have a top-level `CCar`; 39 have only one direct root record, while one also has a top-level `CCharacter`. The sole verified `prop` remains the small stockpile.

There are **388 standalone `.epe` files** as well as 467 bundle-embedded definitions. The 388 loose paths all correspond to an embedded member, but the copies should not be treated as interchangeable: in this comparison no loose copy was byte-identical, and only 6 of 388 had the same class-count map. For inventorying a particular `.ee`, its embedded `.epe` is therefore the authoritative package definition; retain the loose file as a potentially different build/variant until provenance explains the mismatch. One convoy comparison confirms that both versions can still have the same high-level composition (`6 CCharacter`, `6 CCar`, and one `CSpawnReferencePosition`) while differing in nested nodes.

Generated local outputs (ignored `target/` data):

- `entity-bundle-index.csv` — one row per `.ee`, archive sizes and SARC member counts by resource family.
- `entity-bundle-members.csv` — all 657,984 member paths, useful for dependency searches and duplicate resolution.
- `compiled-member-resource-index.csv` / `compiled-resource-family-summary.json` — deduplicated compiled resources with bundle reuse counts.
- `epe-archive-index.csv` — archive offset and size for each standalone `.epe`.
- `epe-embedded-rtpc-summary.json` — per-bundle parsed node counts, named top-level nodes/classes, and class composition.
- `epe-embedded-class-frequency.csv` — class node counts and number of bundle definitions containing each class.
- `epe-embedded-path-references.csv` — path-like references with source entity, node class/name, and raw property hash.
- `epe-reference-compiled-member-candidates.csv` — same-stem source-reference to compiled SARC-member candidates, including referring and containing bundle counts.
- `epe-property-reference-profile.csv` — recurring node-class/property-hash/resource-extension patterns for field-name recovery.
- `entity-structural-profiles.csv` — all 467 packages joined to catalog IDs, direct-root classes, nested class counts, bundle sizes, and candidate-reference coverage.
- `single-root-entity-candidates.csv` — the 29 simplest-root candidates with root names, models/collision candidates, and current catalog verification state.
- `entity-resource-candidate-edges.csv` — per-definition source property/reference to same-stem compiled member candidate, including whether the candidate is in that definition's own bundle.
- `entity-structural-inventory-summary.json` — corpus totals and structural candidate classifications.
- `epe-rtpc-tree.json`, `epe-rtpc-summary.json`, `epe-scan-summary.json`, and `epe-string-candidates.json` — standalone-file structural/string scans, useful for variant comparison and leads.
- `path-census.csv` / `census-summary.json` — 27,501 logical paths joined to installed archive tables.

## What this changes

We now have a path/archive census, a complete SARC directory index for every `.ee`, and a structural parse of every package-embedded `.epe`. That is a broad content map plus a real entity/component hierarchy and typed reference edges, rather than just guessed spawn names. It is still not a semantic census of every entity: class names identify record types, not complete runtime behavior, and the source-to-compiled member candidates need per-class validation. Many `.ee` bundles are large and chunked, so selective structure/dependency analysis remains preferable to full extraction.

Sources:

- [MadMax Savegame Editor Project README](https://github.com/Rick-Thompson/MadMax_Savegame_Editor_Project) — file-list count, tools, repository structure, and provenance notes.
- [MadMax Savegame Editor Project `docs/GAME-FILES.md`](https://github.com/Rick-Thompson/MadMax_Savegame_Editor_Project/blob/main/docs/GAME-FILES.md) — archive table layout, path-index use, SARC/RTPC analysis, and examples of identifying entity composition.
- [Gibbed.MadMax `RebuildFileLists`](https://github.com/gigaHours/Gibbed.MadMax/blob/master/RebuildFileLists/Program.cs) — archive tables are joined to project hash lists to recover known paths.
- [`docs/research/entity-catalog.md`](entity-catalog.md) — current spawn-catalog evidence policy.
- [`docs/research/static-prop-asset-candidates-2026-10-31.md`](static-prop-asset-candidates-2026-10-31.md) — local archive and entity-bundle findings.

## Inventory model

Do not force every asset into exactly one GMod-style folder or one “static/dynamic” classification. Store independent dimensions and allow multiple tags where the file genuinely serves multiple roles.

### Resource record

Each discovered archive resource should carry:

- **Identity:** exact logical path when known, basename, extension/type, and archive/table location.
- **Origin:** base, patch, or named DLC archive set; source index and its revision/hash.
- **Resource role:** entity definition, entity bundle, model, texture, collision/physics, animation, gameplay script/graph, effect/particle, audio, world/stream data, UI, localization, or unknown.
- **Content role:** prop, vehicle, character/NPC, weapon, pickup, encounter, project/building, set piece, world decoration, gameplay system, or unknown. This is multi-valued when warranted.
- **Composition:** standalone candidate, multi-object composition, encounter/system package, support asset, or unknown, with the evidence behind that judgment.
- **References:** outgoing resource dependencies and incoming references, preserving exact source identifiers.
- **Runtime behavior:** static, dynamic, keyframed, animated, interactable, destructible, streamed, or unknown. Do not infer these from filenames alone.
- **Spawn/lifecycle status:** no known route, route candidate, spawn attempted, observed working, or observed failing; keep this separate from content classification.
- **Preview status:** not inspected, renderable candidate, extracted, thumbnail generated. This is a later pipeline and must not be conflated with game spawnability.
- **Evidence/provenance:** source file and offsets, parser/version, archive hit, decoded structure, community lead, in-game observation, and confidence.

### Relationship records

Represent references as edges, not flattened text: `entity -> component`, `entity -> model`, `entity -> collision`, `entity -> script`, `model -> texture/material`, and `composition -> child entity`. This is what will let us distinguish an actual reusable object from a package that happens to contain rigid bodies.

### User-facing organization

Keep the exhaustive inventory in a searchable data/index view. The spawn menu remains a curated projection with compact folders such as `Props`, `Vehicles`, `Characters`, and `World Compositions`; it should include only entries for which a spawnable entity key/path is known. An asset's existence in the inventory must not automatically expose it as a spawn button.

## Work sequence

1. **Validate the reference graph:** validate the suffix rules by resource class, check the unresolved 94 paths, and establish property-hash meanings for high-value reference fields (model, collision, scripts/graphs, animation, materials, and effects). Cross-check against stockpile, lootbox, and wreck definitions.
2. **Expand the dependency graph:** link `.epe` entries to `.ee` members and then to model, collision, texture/material, graph/script, animation, effect, and audio dependencies. Deduplicate shared resources without losing the source bundle edges.
3. **Separate individual content from compositions:** identify reusable single-object entity definitions, vehicles, actors/NPCs, projects, and encounter/mission packages from structural composition, not naming heuristics.
4. **Map world placement:** connect location, terrain, MDIC/world-decoration and streaming resources to the entity/dependency graph; establish which assets are baked/streamed versus spawned as individual game objects.
5. **Connect to runtime capability:** annotate spawnability and lifecycle evidence independently. Keep unsupported objects in the inventory as known content even when the current bridge cannot spawn them.
6. **Prepare visual previews:** only after model/material formats and dependency edges are understood, create a separate offline preview pipeline. Use transparent renders and stable catalog IDs for thumbnails; do not run extraction as part of the in-game UI.

## Immediate next step and constraint

The user supplied the upstream `master.dirlist` in ignored `target/content-index/`; archive joining, SARC member indexing, and structural `.epe` parsing are complete. Keep source attribution/license with that input. Do not add game archives or extracted copyrighted payloads to this repository. The next durable deliverable is the validated dependency graph: preserve source reference names and compiled bundle member names as separate identities, verify the candidate suffix mappings, and retain “unknown” where structure has not been decoded.

## First decoded model: stockpile mesh

The first end-to-end mesh decode is now available from the stockpile entity bundle. The dependency chain is:

`editor/entities/a00/propsenc/expentity/a00_propsenc_projects_stockpile_small_01.ee`
→ `models/a00/propsenc/projects/a00_propsenc_projects_stockpile_small_lod1.rbn`
→ `models/a00/propsenc/projects/a00_propsenc_projects_stockpile_small_lod1ge0ge2ge0.rbs`

The `.rbs` is a 58,436-byte `RBSDL` version 2.0.0 file (SHA-256 `04215e7bb9ead88a550e1b70b08371702f8f727c0b44bbeddd86ccedcebd94be`) containing three geometry blocks, 2,027 vertices, and 4,656 16-bit indices. Every block uses render-block hash `0x0826F6EE`, version 7, and end marker `0x456BCDEF`. All parsed index ranges are valid. Stream counts/strides are `(1828, 8)+(1828, 16)`, `(79, 8)+(79, 20)`, and `(120, 8)+(120, 16)` with 4,032, 306, and 318 indices respectively.

The paired `.rbn` is a 1,153-byte `RBNDL` material/link file with three `RBNGeneral` blocks. Each has an 84-byte constants record and the same render-block identity. Its geometry reference hash matches the file header; mesh indices `0`, `1`, and `2` map one-to-one to the three RBS blocks. `vertexScale` is `3.305966377` in each material. `additionalUVSets` is 0, 1, 0 and matches the primary UV stream strides of 16, 20, and 16 bytes. There are eight texture slots per material, including empty slots; twelve paths are populated across all three materials.

The position stream is decoded as three signed normalized 16-bit values per vertex, multiplied by the corresponding material `vertexScale`. The decoded overall range is approximately `x [-2.419216, 0.012813]`, `y [-0.400647, 1.396967]`, `z [-2.354443, 3.305865]`; each axis agrees with the componentwise range from the RBS header to within 0.00011. Primary UVs are decoded from signed normalized 16-bit pairs and scaled by the first UV scale in `RBNGeneralConstants`. This bounds validation is a strong independent confirmation of the position interpretation.

The local tool now emits an interactive flat-shaded WebGL preview, a transparent PNG catalogue thumbnail, a GLB export, and a structural sidecar for this supported pairing. The mesh preview uses neutral per-mesh colors; it does not yet apply game textures/material shaders, recover normals/tangents, or reproduce Mad Max lighting. The generated model and extracted game payload remain in ignored `target/content-index/`; they are not checked into the repository. The readers deliberately reject unrecognized material layouts instead of assuming all game models match this sample.

The independent format work was cross-checked against [ApexLib's RBM/RBS loader](https://github.com/PredatorCZ/ApexLib/blob/master/src/RBM.cpp), [RBN material mapper](https://github.com/PredatorCZ/ApexLib/blob/master/src/RBNClasses.cpp), and [`RBNGeneralConstants` definition](https://github.com/PredatorCZ/ApexLib/blob/master/include/property_classes/RbnMaterial.h). ApexLib is GPL-3.0; no ApexLib source was copied into the workbench.

The position/UV/index contract was then exercised on six more distinct project models from separate `.ee` packages: stockpile large (2 meshes / 1,594 vertices), stockpile medium (3 / 1,600), maggot farm (1 / 1,873), Armory (4 / 4,384), Super Fuel (3 / 4,062), and Super Water (4 / 3,378). Each produced a preview and passed the same structural and decoded-bounds checks. The initial format support therefore covers seven model samples across several prop families, not just the original stockpile.

Two deliberate unsupported cases already show why family-wide guessing is unsafe: Scrap Crew's material block 1 is render-block `0xED8B5331` v4, while the cleanup-crew ghost model is `0x5B2003F6` v5. The current reader reports both and preserves their assets without attempting to interpret their records. The SARC index contains 34,306 `.rbn` and 34,278 `.rbs` member records; all 467 indexed entity bundles contain at least one of each family. These corpus counts are bundle-member inventory facts, not proof that every RBN/RBS pair matches or uses the currently decoded layout.
