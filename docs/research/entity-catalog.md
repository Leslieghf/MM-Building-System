# Entity catalog

The curated file is [`assets/catalog/entities.json`](../../assets/catalog/entities.json). The separate [`assets/catalog/community-candidates.json`](../../assets/catalog/community-candidates.json) imports 456 deduplicated keys from the `empty_vehicle` and `all_entity` arrays in `gigaHours/MadMaxSpawner` commit `4c9af72cd7cd4fd8c40f24f90a4498a66850dae6`.

## Readiness levels

| Evidence | Meaning |
| --- | --- |
| `name_only` | A community list names the key; local archive presence and composition are unverified. |
| `archive_found` | Matching entity data is present in an installed archive; behavior is not necessarily understood. |
| `definition_inspected` | The entity definition and important resources/components have been inspected. This may reveal it is a complex encounter and unsafe for a prop browser. |
| `spawn_observed` | A user observed the entity spawn in game through this bridge. |

`spawn_readiness` is confidence/evidence metadata (`blocked`, `candidate`, or `verified`), not a UI or dispatch gate. A usable safe-relative `asset_key` can be attempted through the generic native entity-spawn route even when readiness is only a candidate or blocked. `kind` describes content semantics; `motion` describes physics behavior. A prop may be static, keyframed, dynamic, or still unknown.

## Shared UI status colors

Use one status vocabulary throughout the spawn browser and script-authored workbenches, always with a visible text label:

- **Green — stable:** verified behavior for the current build.
- **Yellow — experimental attempt:** the generic spawn route and an asset key are known, but the attempt may work, fail, or destabilize the game.
- **Red — no spawn path:** there is no usable asset key to pass to the generic spawn route.

In the spawn browser, green and yellow entries are actionable. The generated ID-to-asset-key table remains authoritative; raw strings from scripts are not accepted. A red row stays visible but cannot reach native spawn dispatch.

## Source import

The community spawner's `all_entity` and `empty_vehicle` arrays are useful discovery sources. They are not curated prop catalogs: they contain road encounters, convoys, vehicles, missions, loot compositions, project states and character groups. Their safe-relative asset keys are yellow spawn attempts with provenance, not claims that the entry exists locally or will spawn successfully.

The 2026-10-31 curated seed includes inspected composition keys and a user-observed stockpile. The 456 community-source keys and 11 curated keys now match local `.ee` packages in the installed archive index. All 467 local `.ee` bundles have an embedded `.epe` RTPC structural parse, and all 467 catalog entries are linked to those profiles. The Rockstar ATV was added as an experimental DLC vehicle from its one-root `CCar` package; it has not been spawn-tested. Community-source records have been updated to `archive_found`; this does not change their yellow experimental spawn status. See [`game-content-inventory-2026-11-01.md`](game-content-inventory-2026-11-01.md) for structural classes, composition counts, and dependency candidates.

## Spawn browser projection

The native spawn browser and script spawn table are generated from both JSON files by `tools/generate_spawn_catalog.py`. Regenerate `src/devtools/overlay/spawn_menu_catalog.rs` and `src/api/scripts/spawn_catalog.rs` after changing either source file. Curated entries take precedence when an asset key appears in both sources. The current projection contains 467 unique entries: 11 curated records and 456 community discoveries. All 467 currently have a safe-relative asset key and can be attempted; only the stockpile is green/verified, and the other 466 entries, including Roadkill Lootbox and the Rockstar ATV, use the same yellow experimental label.

The browser groups entries by their first category (`Props`, `Vehicles`, `World Compositions`, and `Discovered`) and filters the active group by display name, category, or exact asset key. Search and category filtering happen over the small metadata table; the ImGui list clipper renders only visible tile rows. Green verified records and yellow safe-path attempts receive enabled tiles; the generated catalog ID-to-path table remains the final authority. Candidate records are yellow even when their existence or behavior is unknown. Red means no usable path, not merely a complex composition.

## Naming and folders

Use GMod-like folders for discoverability, but do not erase the game's source structure. The user-facing category is an editorial organization (for example `Props/Projects` or `World Compositions/Roadside`); `asset_key` stays byte-for-byte as the source identifier. Avoid naming an encounter “prop” merely because its components include a `CRigidObject`.

## Growth workflow

1. Import source keys with source URL/commit and provisional category.
2. Join the exact entity key against local archives; the current 467 catalog entries all resolve.
3. Use the parsed `.epe` root/component tree and compiled-member dependency candidates to determine whether it is an individual entity or a composition.
4. Give it a display label and classify content kind separately from motion; preserve unknown runtime behavior.
5. A usable safe-relative key is immediately a yellow attempt. Test spawn and lifecycle in game before setting `spawn_readiness` to `verified` and turning the row green.
6. Keep observations and unknowns in research notes; keep the catalog focused on the current best-known mapping.
