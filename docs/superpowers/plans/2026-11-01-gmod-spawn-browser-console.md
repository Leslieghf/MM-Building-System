# GMod-Style Spawn Browser and Copyable Console Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the cramped spawn panel with a compact three-pane GMod-style browser and report spawn lifecycle outcomes in a multi-row-selectable console.

**Architecture:** Keep the generated catalog and native spawn boundary unchanged. Rework the overlay into a top-tab/folder/grid/right-panel composition, extend owned-spawn tracking so asynchronous game observations retain catalog identity, and render bounded console events through a single read-only multiline editor.

**Tech Stack:** Rust 2024, Bevy ECS, Dear ImGui through `hudhook`/`imgui`, Rhai runtime, generated JSON catalog.

**Spec:** `docs/superpowers/specs/2026-11-01-gmod-spawn-browser-console-design.md`

## Global Constraints

- Preserve the allowlisted catalog ID-to-asset-key boundary; never accept script-supplied arbitrary paths.
- Preserve status meaning: green verified, yellow attemptable/unverified, red no known safe spawn path.
- Preserve hold-Q capture, F1/F2/F3 behavior, UI opacity, and independent script panels.
- Keep console output and catalog work bounded; the grid must continue clipping offscreen tiles.
- Do not report native submission as proof the object spawned.

## Review Focus

- A game spawn ID observed as failed is matched to the right pending catalog request and reports failure once.
- A spawn ID reported spawned updates the corresponding record and reports success once.
- Requests that have only been submitted remain pending/unknown rather than showing false success.
- Empty category/folder and empty search states remain legible in the narrow center pane.
- Console multiline selection and copying still work while the prompt retains keyboard focus behavior and command history.

---

### Task 1: Spawn lifecycle reporting

**Files:**
- Modify: `src/api/rhai/bindings.rs`
- Modify: `src/api/scripts/manager.rs`
- Modify: `src/api/ecs/systems/script_dispatch.rs`
- Modify: `src/api/ecs/runtime/mod.rs`
- Modify: `src/api/ecs/runtime/effects.rs`
- Modify: `src/api/ecs/intents.rs` only if a distinct observed-failure result is needed.

**Interfaces:**
- Carry `catalog_id` beside `asset_key` in the script spawn effect and `PendingSpawnIntent`.
- Retain request ID, native ID, catalog ID, and lifecycle attribution in each tracked spawn.
- Emit `SPAWN:` console events for queued/submitted, game-observed spawned, and game-observed failed phases.

- [x] Trace existing native outcome/report call sites before edits.
- [x] Add catalog identity to the owned effect path without changing native ABI fields.
- [x] Emit and correlate console events for observed success/failure without duplicating the current submission line.
- [x] Run formatting, `cargo check`, release build, and inspect generated event text.

### Task 2: Three-pane compact GMod-style spawn workspace

**Files:**
- Modify: `src/devtools/overlay/spawn_menu.rs`
- Modify: `src/devtools/state/overlay.rs` or a focused spawn-menu state module if persistent UI state belongs outside thread-local rendering state.
- Modify: `src/devtools/overlay/mod.rs` only if the layout needs viewport-aware placement or capture integration.

**Interfaces:**
- Main tabs select the existing top-level category.
- Left folder tree selects nested `CatalogEntry.category` paths.
- Center grid renders only matched/visible entries.
- Right `Tools` / `Settings` tabs expose the current placement and removal controls plus a compact/medium/large tile-size setting.

- [x] Add folder-tree/filter state with stable IDs and maintain category counts.
- [x] Replace fixed 440x510 layout with viewport-aware sizing and three compact panes.
- [x] Move placement controls and owned-spawn rows into Tools; move tile density options into Settings.
- [x] Rework cards to small descriptive icons and tight calibrated spacing by default while retaining status colors.
- [x] Run formatting, `cargo check`, release build, and inspect the diff for preserved input/focus behavior.

### Task 3: Single selectable console transcript

**Files:**
- Modify: `src/devtools/overlay/panel.rs`
- Modify: `src/devtools/state/console.rs`
- Modify: `src/api/scripts/console/output.rs` only if a public transcript formatter is needed.

**Interfaces:**
- Build a bounded plain-text transcript from retained `ConsoleEvent`s with the current command/script headers, indented response lines, and explicit severity labels.
- Render it using one read-only multiline ImGui text field, preserving scroll-follow controls and command prompt/history.

- [x] Replace row-per-event selection and custom Ctrl+C logic with the single read-only multiline text field.
- [x] Retain horizontal scrolling and native text selection/copy; omit separate copy controls per user direction.
- [x] Reduce margins and controls while preserving command/source grouping.
- [x] Run formatting, `cargo check`, release build, and inspect grouped transcript construction.

## Manual acceptance

- In-game, browse each top tab and folder; verify Props shows only currently classified props, and center tiles remain compact/readable.
- Spawn a known working stockpile, then attempt one yellow candidate. Confirm console distinguishes submitted, observed success, and game-reported failure without calling submission a success.
- Drag-select across several console records and copy them; confirm copied text contains headers and their associated output.

Implementation and installation are complete. Formatting, locked check, Windows release build, and diff whitespace checks passed. Manual in-game acceptance remains pending.
