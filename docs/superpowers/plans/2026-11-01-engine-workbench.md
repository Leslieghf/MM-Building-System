# Engine Workbench Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` to implement task-by-task. This plan is executed inline in the current session.

**Goal:** Add one compact script-owned in-game panel for currently available camera, player, and vehicle controls, with explicit weapon and upgrade research gaps.

**Architecture:** Add a new Rhai package using the existing `ctx.ui` panel API and copied game snapshots. A module selector changes the widgets published in one window; camera override and vehicle velocity writes remain in their established bounded adapters.

**Tech Stack:** Existing Rust host, Rhai scripts, TOML script manifest, generated API reference.

**Spec:** `docs/superpowers/specs/2026-11-01-engine-workbench-design.md`

## Constraints

- One package panel, one stable panel ID, at most 48 widgets.
- No new native APIs or raw process memory access.
- Camera pose is an override on the game's camera getter, not a detached camera.
- Vehicle writes use existing `game.vehicle.write` capability and bounded velocity adapter.
- Weapon damage, vehicle tuning, and Max upgrades remain unavailable until their native routes are mapped.
- Do not launch Mad Max, install a build, or add/run automated tests unless requested.

## Task 1: Script package

**Files:** create `examples/scripts/engine_workbench/main.rhai` and `manifest.toml`; modify `docs/rhai-api.md` only if the script reveals API-documentation drift.

- [ ] Declare `game.read`, `game.camera.control`, `game.vehicle.write`, and `ui.panels` capabilities.
- [ ] Store a persistent selected module and bounded camera/velocity control state.
- [ ] Consume copied UI events before publishing the frame's one panel.
- [x] Publish a compact page selector and only the active page's widgets.
- [x] Refresh or clear the camera override on every callback based on its toggle.
- [x] Apply one-shot vehicle velocity/jump actions only when vehicle data is available.
- [x] Show live player, vehicle, and camera state using the shared fixed-precision formatter.
- [x] Show explicit unavailable status and concrete research targets on Vehicle tuning, Weapons, and Max pages.

## Task 2: User-facing documentation

**Files:** create `examples/scripts/engine_workbench/README.md`; update `docs/superpowers/specs/2026-11-01-engine-workbench-design.md` only if runtime details require an adjustment.

- [x] Document load/reload, module navigation, camera semantics, and vehicle write behavior.
- [x] Record weapon damage, vehicle tuning, and Max upgrades as unmapped API work, not UI bugs.

## Task 3: Verify artifact and package contract

- [ ] Compile the script package through the game's script loader on a later in-game validation pass.
- [x] Confirm panel IDs/widget IDs are unique and widget count is below 48 by inspecting the published script construction.
- [x] Review `git diff --check`; do not commit, push, deploy, or install.

## Review focus

- Camera override must clear on disable and unload.
- Vehicle write must not issue when no current vehicle sample exists.
- UI changes arrive on a later callback, so actions must be one-shot and not repeated while a widget remains selected.
- Module selection must not make unrelated camera state disappear or duplicate panels.
- Unsupported weapon/upgrades pages must not imply live controls exist.
