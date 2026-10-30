# Mad Max Bevy Runtime Bridge Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a lightweight, host-scheduled Bevy ECS runtime to the existing Mad Max ASI, with game and UI ECS worlds, bounded ECS intent handoffs, and one guarded vehicle velocity setter request.

**Architecture:** Keep the existing Rust ASI, MinHook callback, and DX11 ImGui renderer. Add two headless Bevy apps: the post-simulation callback owns the game world, and the ImGui render loop owns the UI world. The native adapter remains the only place allowed to touch game pointers; ECS receives owned snapshots and emits typed requests. Bounded `std::sync::mpsc::sync_channel` queues bridge the worlds, using only `try_send`/`try_recv` in host callbacks.

**Tech Stack:** Rust 2024, existing `hudhook`/MinHook/ImGui stack, lightweight `bevy_app` and `bevy_ecs` only, Windows x64.

**Spec:** `docs/superpowers/specs/2026-10-27-bevy-runtime-bridge-design.md`

## Global Constraints

- Preserve the existing ASI and DX11 overlay integration.
- Add no Bevy renderer, UI renderer, window, physics, or 2D/3D crates.
- Keep game pointers and raw addresses inside the native adapter; ECS receives copied DTOs only.
- Keep game and UI ECS worlds separately owned by their respective host callbacks.
- Use bounded, nonblocking cross-world message queues; report queue overflow.
- Run game reads and writes only from `UpdatePostSim`; do not invoke native game APIs from render/UI systems.
- Apply at most one validated native mutation per game callback; the initial setter is a one-shot echo of copied current velocity.
- The `SetVelocity` candidate slot 38 is SDK-derived and remains an explicitly reported live-game crash risk.
- Do not launch Mad Max. Build the Windows release; deploy only after confirming the game process is closed and preserving the installed ASI backup.
- Do not add or run tests unless the user asks.

## Review Focus

- Callback overlap or callback thread changes must skip native calls and ECS schedule dispatch while still calling the original game function.
- Bounded queue saturation must not block either host callback and must increment a visible diagnostic.
- Missing vehicle, invalid/non-finite velocity, or unavailable setter target must leave the one-shot request pending or report a safe skip.
- UI section toggles must update UI ECS state and never call the game adapter directly.
- UI-originated intent entities must be applied only on a later game callback after re-resolving the current vehicle.

## Files and Responsibilities

- `Cargo.toml`: add only `bevy_app` and `bevy_ecs` with default features disabled where supported.
- `src/runtime.rs`: DTOs, bounded queue endpoints, game/UI apps, resources, components, schedules, systems, counters, and UI presentation model.
- `src/game_probe.rs`: callback ownership/reentrancy gate, pointer-owning native sampling and setter adapter, snapshot publication, and intent outcomes.
- `src/overlay.rs`: own and run the UI app in the ImGui callback; draw the prepared ECS presentation model and spawn UI intent entities and update UI ECS resources.
- `src/lib.rs`: declare runtime module and initialize shared endpoints before hook installation.
- `docs/superpowers/plans/2026-10-27-bevy-runtime-bridge.md`: this plan.

## Tasks

### Task 1: Add lightweight Bevy runtime foundations

**Files:**
- Modify: `Cargo.toml`
- Create: `src/runtime.rs`
- Modify: `src/lib.rs`

- [x] Add pinned compatible `bevy_app` and `bevy_ecs` dependencies with default features disabled; update `Cargo.lock`.
- [x] Define owned `GameSnapshot`, `GameEvent`, `GameIntentEnvelope`, `IntentOutcome`, UI section/readout DTOs, and bounded queue constructors. Queue capacities must be named constants; send/receive APIs must use nonblocking operations and return explicit full/disconnected outcomes.
- [x] Define `MadMaxPostSim` and `MadMaxRender` schedule labels plus independent game/UI app constructors and registration functions.
- [x] Add resources for the latest game snapshot, pending intent entities, intent diagnostics, UI section visibility, UI presentation rows, and callback/schedule counters.
- [x] Wire one shared runtime endpoint bundle during ASI startup, without changing native probe behavior yet.
- [x] Run `cargo fmt --check` and Windows x64 `cargo check --target x86_64-pc-windows-msvc`; resolve dependency feature or compiler incompatibilities.

### Task 2: Drive the game ECS from UpdatePostSim

**Files:**
- Modify: `src/game_probe.rs`
- Modify: `src/runtime.rs`

- [x] Replace the global gameplay sample mutex interface with an owned `GameSnapshot` publication path while retaining existing native reads and displayed values.
- [x] Instrument first callback thread ID, stable serial streak, and reentrancy with atomics; after the original callback, skip probe, schedule, and setter on overlap or thread mismatch.
- [x] Once stable, copy the latest snapshot into the game world, materialize bounded inbound intent entities, run exactly `MadMaxPostSim`, and publish copied outcomes/snapshots to the UI queue.
- [x] Keep callback work nonblocking; wrap Rust callback work in `catch_unwind` so panic does not cross the native ABI.
- [x] Run `cargo fmt --check` and Windows x64 `cargo check --target x86_64-pc-windows-msvc`.

### Task 3: Move overlay state and presentation into the UI ECS

**Files:**
- Modify: `src/overlay.rs`
- Modify: `src/runtime.rs`

- [x] Give `BootstrapNotice` ownership of the UI app and UI-side queue endpoints.
- [x] On each ImGui render callback, drain available snapshots/events, update UI resources/entities, and run exactly `MadMaxRender`.
- [x] Move section visibility, runtime diagnostics, labels, and formatted presentation values into UI ECS resources/components; remove `debug_entries()`'s direct `game_probe` reads.
- [x] Keep ImGui as a renderer/input adapter over the UI presentation model; section picker actions update ECS UI state; UI intent entities are serialized only by the private transport.
- [x] Preserve current appearance, hotkeys, and all existing player, camera, vehicle, and callback readouts.
- [x] Run `cargo fmt --check` and Windows x64 `cargo check --target x86_64-pc-windows-msvc`.

### Task 4: Add the guarded one-shot vehicle setter and diagnostics

**Files:**
- Modify: `src/game_probe.rs`
- Modify: `src/runtime.rs`
- Modify: `src/overlay.rs`

- [x] Add the typed `SetPlayerVehicleVelocity { velocity: [f32; 3] }` ECS intent component and reject non-finite components before native dispatch.
- [x] After callback stability and the first valid vehicle sample, spawn one intent entity carrying an identity echo using the exact copied velocity; never repeat automatically after completion.
- [x] Resolve the current vehicle again at dispatch, validate the candidate vtable slot 38 target as readable/executable, and call the native setter only on the game callback.
- [x] Cap mutation dispatch at one per callback; report pending, applied, invalid, unavailable, dropped, and failed outcomes in UI diagnostics.
- [x] Keep user-created intent entities bounded and typed; UI controls cannot expose pointers or direct native calls.
- [x] Run `cargo fmt --check` and Windows x64 `cargo check --target x86_64-pc-windows-msvc`.

### Task 5: Release build and delivery preparation

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-10-27-bevy-runtime-bridge-design.md`

- [x] Document the schedules, callback ownership gate, ECS intent boundary, setter caveat, and first in-game observation procedure.
- [x] Review the full diff for pointer leakage, blocking queue operations, unsafe native calls outside the adapter, and unintended Bevy features.
- [x] Build the release for `x86_64-pc-windows-msvc` and report exact command/result.
- [x] Before any install/deploy action, verify Mad Max is closed, back up the installed ASI, deploy the matching artifact, and compare hashes. Never launch the game.
- [x] Summarize the candidate setter slot risk and tell the user to observe the first live run and report the runtime diagnostic result.

## Delivery Note

Do not push or publish without a fresh user instruction. The in-game setter observation is user-controlled; this plan does not launch the game.



