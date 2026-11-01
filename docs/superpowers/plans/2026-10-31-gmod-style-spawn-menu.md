# GMod-Style Spawn Menu Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the configurator's one-shot stockpile test with a compact hold-Q GMod-style spawn menu.

**Architecture:** Add a focused overlay mode and Q-key state at the DX11 input hook boundary. Render a dedicated compact panel from copied snapshot data and submit the existing typed spawn intent on each valid tile click.

**Tech Stack:** Rust, Bevy ECS resources, hudhook, Dear ImGui.

**Spec:** `docs/superpowers/specs/2026-10-31-gmod-style-spawn-menu-design.md`

## Global Constraints

- Preserve the existing F1/F2/F3 behavior.
- Use only the verified stockpile key and current typed spawn-intent path.
- Keep game reads and native spawn effects on existing boundaries.
- Do not add despawn or arbitrary asset-key input.

## Review Focus

- Confirm in-game that the Q hold does not trigger an unwanted native action; hudhook processes its window-message queue asynchronously, so the capture transition needs a manual timing check.
- Focus loss while Q is down must not leave the menu or input capture stuck.
- Missing reticle hit/player transform must prevent spawning.
- Each tile click must enqueue exactly one separate request.
- Inactive categories must not look like populated or actionable tabs.

---

### Task 1: Hold-Q input mode

**Files:** `src/devtools/state/overlay.rs`, `src/devtools/overlay/mod.rs`

**Interfaces:** Add `OverlayMode::SpawnMenu`; add Q-down/capture state and intercept Q window messages before they reach the game. Spawn-menu mode always captures game input. On release/focus loss, return to HUD and release the temporary capture.

- [x] Test that `SpawnMenu` captures input even when F1 override is false, and that switching back to HUD restores the ordinary override behavior.
- [x] Verify the new test fails before implementation.
- [x] Implement Q down/up handling, per-frame mode transitions, and focus-loss reset.
- [x] Run the targeted state test and formatter.

### Task 2: Dedicated spawn-menu presentation

**Files:** Create `src/devtools/overlay/spawn_menu.rs`; modify `src/devtools/overlay/mod.rs`.

**Interfaces:** `draw_spawn_menu(runtime, ui, preferences)` renders the compact title/tab/grid layout. The first item is a drawn thumbnail and `Scrap Stockpile`; only it is active. It uses the latest snapshot reticle hit and player transform, then submits one spawn intent on click.

- [x] Keep the placement test for the fixed asset key and hit-position-plus-0.5-up transform.
- [x] Render the scrollable grid, active Props tab, inactive Vehicles/NPCs tabs, and Q-release hint.
- [x] Remove `draw_static_prop_test_control` and `SpawnTestUiState` from the configurator/app resources.
- [x] Run relevant tests and formatter.

### Task 3: Verify build and prepare visual check

**Files:** `docs/research/install-history.md`

- [x] Run `cargo test`, `cargo fmt --all -- --check`, and `cargo build --release --locked`.
- [x] Confirm Mad Max was closed before installing; preserve the previous ASI.
- [x] Record the build/install result; in-game checks remain pending: Q hold/release, native Q action, click stockpile, missing-hit behavior, and repeated click behavior.
