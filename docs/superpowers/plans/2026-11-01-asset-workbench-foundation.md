# Asset Workbench Foundation Implementation Plan

> **For agentic workers:** Execute inline and in order. Keep the task scoped to foundations; viewers are follow-up plans per resource family.

**Goal:** Establish safe Q-menu focus behavior and a provenance-preserving offline resource preparation path for future in-game asset viewers.

**Architecture:** Keep the injected overlay limited to input state and prepared asset metadata. Add an offline, selected-resource reader that resolves a logical game path through the installed file index and archive tables, plus a generated asset manifest that keeps source, compiled member, relationship, and capability status distinct.

**Tech Stack:** Rust 2024 overlay using `hudhook`/Dear ImGui and Bevy resources; existing local Python research tooling for the first offline archive-reader spike; JSON/CSV metadata under ignored `target/content-index`.

**Spec:** `docs/superpowers/specs/2026-11-01-wasteland-asset-workbench-design.md`

## Global Constraints

- Preserve hold-Q behavior when no excluded UI item has input focus.
- Q interactions must be decided using an atomic UI snapshot; the WndProc hook must not call ImGui APIs.
- An active text field or hovered non-allowlisted UI item locks the Q toggle; browser tabs and spawn tiles are the explicit allowlist.
- A locked menu remains visible after release if the UI interaction took ownership; a fresh Q press/release over an allowlisted browser target closes it.
- The overlay must not scan archives, extract resources, decode media, or render thumbnails in its frame callback.
- Offline extraction accepts an exact path from the local name index and writes only under ignored `target/content-index/extracted`.
- Keep original logical paths and archive/member provenance; do not check game payloads or previews into version control.
- Do not claim resource formats are readable or previewable before a representative decoder establishes that status.

## Review Focus

- Q release while a search field is active does not close the menu or leak Q to Mad Max; typing Q remains available to the UI.
- Q press/release over a top tab or spawn tile can close a pinned menu; focus loss clears all held/pinned state.
- Exact-path archive lookup handles duplicate entries without silently choosing incompatible payloads.
- Chunked and unchunked resource payloads are bounded and decompressed to the declared output size.
- Unsafe path input cannot write outside the ignored extraction directory.

---

### Task 1: Q-toggle interaction lock

**Files:**
- Modify: `src/devtools/overlay/mod.rs`
- Modify: `src/devtools/overlay/spawn_menu.rs`

**Interfaces:**
- Publish one atomic `spawn_menu_toggle_locked` value from the render frame based on `ui.io().want_text_input`, active/hovered UI items, and explicit allowed-hover state from top-level tabs and spawn tiles.
- Track whether Q was consumed and whether the menu was pinned open by a locked interaction. Clear both on window focus loss, script unload, and non-spawn overlay transitions.
- In WndProc, when Q is locked, pass the key event to ImGui; when Q has already been consumed, always consume its release and pin the menu if the lock is active.

- [x] Inspect existing menu mode transitions and hook ordering before edits.
- [x] Add the atomic lock/pin state and update it only from the completed ImGui frame.
- [x] Track allowed hover from browser tabs and individual catalog tiles; other UI items remain excluded.
- [x] Update Q keydown/keyup and menu mode transitions so locked release pins, and an unlocked fresh Q cycle closes.
- [x] Format and compile the overlay; inspect the final state transitions and focus-loss paths.

### Task 2: Safe selected-resource extraction spike

**Files:**
- Create: `tools/asset_workbench/extract_resource.py`
- Create: `tools/asset_workbench/README.md`
- Output only: `target/content-index/extracted/`

**Interfaces:**
- CLI accepts one exact case-insensitive logical path from `master.dirlist`, optional archive-set selection, and an output mode limited to `target/content-index/extracted`.
- Resolve the basename Jenkins hash in local `.tab` tables, retain every matching archive locator, and reject unresolved or conflicting size records.
- Read chunked or unchunked archive payloads, raw-deflate when needed, enforce configured compressed/uncompressed caps, validate declared sizes, and emit a small provenance sidecar.

- [x] Reuse the proven Jenkins and tab-table rules from the current research parsers, keeping archive parsing isolated.
- [x] Add path validation before opening output files; preserve logical subfolders below the extraction cache.
- [x] Implement exact hash lookup with explicit duplicate handling and archive-set selection.
- [x] Implement bounded chunk and non-chunk reads with declared-length validation.
- [x] Document local-only use, supported inputs, limits, and how later viewers consume extracted samples.
- [x] Run formatter/static compilation if available; do not add tests in this autonomous pass unless the user requests them.

### Task 3: Shared asset-workbench manifest

**Files:**
- Create: `tools/asset_workbench/build_manifest.py`
- Create output: `target/content-index/asset-workbench-index.json`
- Update: `docs/research/game-content-inventory-2026-11-01.md`

**Interfaces:**
- Each resource row has a stable key, logical path, extension/family, archive presence, compiled-member candidates, owning entity/package links, capability flags (`source_readable`, `structure_decoded`, `preview_ready`, `spawn_route_known`, `spawn_observed`), and provenance.
- The manifest must distinguish authoring/source references from compiled SARC member paths and preserve many-to-many links.
- Initial flags default to unknown/false; only existing evidence promotes them.

- [x] Merge the path census, bundle members, property-reference profile, structural profiles, and both JSON catalogs.
- [x] Merge by normalized exact path only; do not merge source and compiled resources solely because of same stems.
- [x] Include catalog/entity relationship rows and preserve source-reference relationships.
- [x] Emit deterministic JSONL with stable path IDs and a summary of rows and relationship counts.
- [x] Add the manifest format and generated output to the research note.

### Task 4: Selected-resource preview report

**Files:**
- Create: `tools/asset_workbench/prepare_preview.py`
- Update: `tools/asset_workbench/README.md`
- Output only: `target/content-index/previews/`

- [x] Extract a single explicitly selected path through the bounded archive reader.
- [x] Render valid UTF-8 resources as escaped, read-only text; decode DDS candidates only when an installed image decoder accepts them.
- [x] Show printable strings for compiled binaries with an explicit “not source code” label; identify FSB5 banks without claiming audio playback support.
- [x] Keep previews, reports, and extracted payloads in ignored local output.

### Task 5: Searchable offline gallery

**Files:**
- Create: `tools/asset_workbench/build_gallery.py`
- Update: `tools/asset_workbench/README.md`
- Output only: `target/content-index/asset-workbench.html`

- [x] Build a self-contained local page from compact manifest metadata; keep media and game files outside the page.
- [x] Add family, search, preview, and spawn-readiness filters with paginated results.
- [x] Preserve the green/yellow/red spawn-status meanings and avoid applying them to unrelated viewer capability.
- [x] Link only to previews already generated; show the selected-resource command for named paths.
- [x] Syntax-check the generated inline JavaScript and keep generated output ignored.

### Task 6: Bounded bundle-member extraction

**Files:**
- Create: `tools/asset_workbench/extract_bundle_member.py`
- Update: `tools/asset_workbench/prepare_preview.py`, `tools/asset_workbench/README.md`
- Output only: `target/content-index/extracted/bundle-members/`, `target/content-index/previews/`

- [x] Resolve an exact `.ee` and exact member pair through the existing local SARC member index.
- [x] Extract only the indexed byte span after checking it against the locally extracted outer bundle and the configured size cap.
- [x] Preserve both bundle and member paths in provenance and keep output under ignored local directories.
- [x] Prepare a real preview of a texture stored inside the known stockpile package.

### Task 7: Model dependency surfacing

**Files:**
- Update: `tools/asset_workbench/prepare_preview.py`, `tools/asset_workbench/build_manifest.py`, `tools/asset_workbench/build_gallery.py`
- Update: `docs/research/game-content-inventory-2026-11-01.md`

- [x] Inspect all three stockpile `.rbn` LOD resources and confirm the observed `RBNDL` signature and dependency paths.
- [x] Resolve each inspected `.rbn` path list against same-package SARC members; all 13 dependencies per sample are present.
- [x] Preserve typed geometry/texture dependency paths in preview sidecars and the merged manifest.
- [x] Surface those links in the local gallery without claiming the `.rbs` geometry is decoded.

## Deferred viewer plans

After the foundation, create separate small plans for: (1) readable text and compiled script/graph inspection, (2) texture decoding/preview, (3) sound audition, (4) model preview and thumbnail baking, (5) skeleton/animation inspection and playback, and (6) effects/VFX and world-streaming views. Do not pretend these formats are supported before each plan demonstrates representative assets.
