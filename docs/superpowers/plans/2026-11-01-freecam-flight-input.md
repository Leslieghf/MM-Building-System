# Free Camera, Hybrid Flight, and Input Boundary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

**Goal:** Add an input-driven free-camera workbench and vehicle position-hold flight workbench using only the existing bounded native adapters.

**Architecture:** The overlay samples its supported keyboard/mouse keys only while F1 capture owns UI input. Rhai scripts consume the copied input frame; freecam writes an expiring camera getter override, and flight updates a script-owned target position then submits bounded vehicle velocity corrections. Player movement and native game binding integration remain explicit WIP.

**Tech Stack:** Existing Rust overlay/input mailbox, Rhai API v1, Bevy ECS resources, `game.camera.control`, `game.vehicle.write`, `input.read`, and script panels.

**Spec:** `docs/superpowers/specs/2026-11-01-freecam-flight-input-design.md`

## Global Constraints

- No guessed native function/vtable calls or arbitrary memory access.
- Input sampling is unavailable unless F1 overlay capture is active.
- Existing `GetCameraMatrix` override is bounded and expiring; no detached renderer camera claim.
- Vehicle writes use the documented `CVehicle::SetVelocity` adapter and remain within ±250 units/s.
- Max transform/velocity writes, collision bypass, XInput suppression, and native Controls-menu bindings are WIP.
- Do not launch Mad Max; user has already closed it for deployment.
- Do not add or run automated tests.

## Review Focus

- F1 capture transitions publish unavailable input promptly when capture ends.
- Key directions are normalized so diagonal movement does not exceed configured speed.
- Large/stale `delta_seconds` cannot produce a large camera or target-position jump.
- Disabling freecam clears the camera override and disabling flight stops writes.
- No vehicle sample or inactive input prevents flight writes; player mode never calls vehicle setters.

### Task 1: Gate mod input sampling on F1 capture

**Files:** Modify `src/devtools/overlay/mod.rs`, `src/devtools/overlay/config.rs`, and `docs/rhai-api.md`/API catalog availability text.

- [x] Build and publish an available `InputFrame` only when runtime sampling is enabled, F1 capture is true, and ImGui is not requesting text input.
- [x] Publish an unavailable empty frame outside capture so key-down state cannot remain stale.
- [x] Preserve existing transition accumulation and document that only overlay keyboard/mouse states are sampled; do not claim XInput suppression.

### Task 2: Turn `freecam_workbench` into keyboard-driven camera movement

**Files:** Modify `examples/scripts/freecam_workbench/main.rhai`, `manifest.toml`, and `README.md`.

- [x] Add input capability and initialize a detached world position from the copied camera when enabled.
- [x] Use WASD, Space/Left Ctrl, and arrow keys for bounded translation and yaw/pitch while input is available.
- [x] Derive the underlying camera position from the previous applied offset, then recompute offset against the desired position to avoid feedback accumulation.
- [x] Cap frame delta used for movement; normalize diagonal move direction; expose speed/turn-rate and status controls.
- [x] Clear the camera override when disabled or when the camera hook is unavailable; clearly label the camera getter dependency.

### Task 3: Add a hybrid vehicle flight workbench

**Files:** Create `examples/scripts/hybrid_flight/main.rhai`, `manifest.toml`, and `README.md`.

- [x] On explicit enable, capture current vehicle position as the script target; never target player mode with a vehicle write.
- [x] Move the target using camera-relative WASD and world vertical keys, with bounded speed and delta time.
- [x] Convert target error to bounded velocity correction and submit at most one write per callback while enabled and a vehicle sample exists.
- [x] Add hold-current-position, stop, and disable behavior; report that collisions/physics remain active and this is not teleport/noclip.
- [x] Show Max movement, object transform setting, and native input-menu integration as WIP with the missing adapter named.

### Task 4: Document, build, install, and verify package copies

**Files:** Update `docs/research/input-and-weapon-hud-2026-10-31.md`, `docs/rhai-api.md`, and `docs/research/install-history.md`; deploy the two script packages and release ASI.

- [x] Run targeted Rust formatting check, `cargo check --locked --target x86_64-pc-windows-msvc`, and a locked Windows release build; do not run test suites.
- [x] Check script widget IDs, package IDs/callback declarations, capabilities, and `git diff --check`.
- [x] Verify Mad Max is not running before installation; preserve the replaced ASI and script directories.
- [x] Compare source/deployed hashes and state that Rhai compilation and movement behavior require the user's next in-game load.

## Plan self-review

The plan covers each requested feature with current adapters, preserves the user’s preference for script-owned movement controls, and marks unavailable native facilities instead of presenting fake controls. The native keybind-menu path is investigation-only because current Ghidra evidence has names but no verified registration contract.
