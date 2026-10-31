# General-Purpose Rhai Raycast API Implementation Plan

> **For agentic workers:** Implement inline in this task; no subagents. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add bounded asynchronous world raycasts to Rhai, use the camera ray in the reticle example, and show only its optional hit position.

**Architecture:** Rhai emits owned ray requests with ticket and script-generation attribution. The native callback validates ownership, executes bounded requests using named filter profiles, and returns copied ticket results for later script callbacks. Camera position/direction are copied from the existing camera matrix sampling path.

**Tech Stack:** Rust, Bevy ECS, Rhai, existing Mad Max native ABI adapter, Windows MSVC release build.

**Spec:** `docs/superpowers/specs/2026-10-31-general-purpose-raycast-rhai-design.md`

## Global Constraints

- Native pointers and game calls remain inside `api::game::native`.
- Query requests contain finite owned vectors, range, named filter, ticket, and script-generation attribution only.
- Initial filters are `static_only` and `player_aim`; raw masks and vtables are not script-facing.
- Per script callback, accept at most one request; per game callback, execute at most eight requests in deterministic dispatch order.
- Query maximum distance is `(0, 500]`; coordinates are within ±100000; invalid or non-finite inputs are rejected.
- Results are copied, owner/generation-scoped, expire after 120 accepted game callbacks, and become observable on a later callback.
- Do not run or add tests in this pass; use formatting, compilation, release build, API generation consistency, and deployment hash checks.

## Review Focus

- Missing or stale camera matrix: expose `()` and let the reticle script skip the query.
- Invalid filter name or vector/range: reject before native handoff with a script-facing error.
- Callback failure, unload, or reload: discard queued work and prevent old-generation result reads.
- Miss, unavailable query base, and pending ticket: preserve optional position and explicit result state.
- Queue saturation and result expiry: bound memory/work and return explicit full/expired states.

---

### Task 1: Camera snapshot and Rhai values

**Files:** `src/api/ecs/resources/frame.rs`, `src/api/game/native/sampling/telemetry.rs`, `src/api/game/native/sampling/mod.rs`, `src/api/game/native/sampling/actors.rs`, `src/api/rhai/bindings.rs`, `src/api/rhai/catalog.rs`.

**Interfaces:**
- Add optional copied `camera_position` and normalized `camera_direction` to `GameSnapshot`.
- Expose them as `ctx.camera.position` and `ctx.camera.direction`, each `vector | ()` and gated by `game.read`.

- [x] Copy camera translation and normalized view direction from the current camera matrix sample; preserve stale/missing handling.
- [x] Add Rhai property registration and API catalog entries for both camera values.
- [x] Run `cargo fmt --all -- --check` and `cargo check --target x86_64-pc-windows-msvc --locked`.

### Task 2: Owned Rhai request and ticket result contract

**Files:** `src/api/rhai/bindings.rs`, `src/api/rhai/catalog.rs`, `src/api/scripts/manager/invocation.rs`, `src/api/scripts/manager.rs`, `src/api/scripts/package.rs`.

**Interfaces:**
- Add capability `game.raycast`.
- Add `ctx.raycast.request(origin, direction, max_distance, filter_name) -> ticket`.
- Add `ctx.raycast.result(ticket) -> map` with `state`, `has_hit`, `hit_position`, `fraction`, and `state_present` fields.
- Add owned `ScriptRaycastRequest` and `ScriptRaycastResult` records carrying ticket and script-generation identity.

- [x] Add input validation and request/ticket allocation; one request per script invocation.
- [x] Build results available to the invoking generation only; distinguish pending, complete, unavailable, expired, and unknown tickets.
- [x] Export only requests from successful callbacks and currently active generations; enforce the phase cap of eight.
- [x] Regenerate `docs/rhai-api.md`; run format and Windows-target compilation checks.

### Task 3: Bounded native query handoff

**Files:** `src/api/ecs/systems/script_dispatch.rs`, `src/api/ecs/runtime/mod.rs`, `src/api/ecs/runtime/effects.rs`, `src/api/game/native/callback.rs`, `src/api/game/native/sampling/reticle.rs`, `src/api/game/native/sampling/telemetry.rs`, `src/api/game/native/sampling/mod.rs`.

**Interfaces:**
- `GameRuntime::take_script_raycast_requests() -> Vec<ScriptRaycastRequest>` drains requests after the post-simulation schedule.
- `GameRuntime::complete_script_raycast(result)` records a copied result for its still-active originating generation.
- A native query helper accepts one owned ray plus a named `RaycastFilterProfile` and returns copied availability/state/fraction/position values.

- [x] Carry callback-produced requests through bounded ECS storage with generation attribution.
- [x] Execute each accepted request after scripts in the accepted game callback; build only the known native filter contexts.
- [x] Reconstruct hit positions from that exact origin, normalized direction, distance, and fraction; preserve misses as optional positions.
- [x] Store ticket results in bounded owner-scoped retention and expire them after 120 accepted game callbacks.
- [x] Remove the old fixed player-relative double-filter probe once the script-request path supplies the reticle result; ensure no duplicate ray is issued.
- [x] Run format and Windows-target compilation checks.

### Task 4: Camera reticle consumer and hit-position-only presentation

**Files:** `examples/scripts/reticle_ray/manifest.toml`, `examples/scripts/reticle_ray/main.rhai`, `src/devtools/presentation.rs`, `docs/rhai-api.md`, `docs/research/raycast-filter-mappings.md`.

- [x] Update `reticle_ray` to request the camera-position/direction ray with `static_only`, poll its ticket, and separately compose raycast data with debug drawing for visual diagnosis.
- [x] Reduce the Raycast debug section to exactly one `Hit position` row, showing the current reticle result or `--` when unavailable/missed.
- [x] Remove obsolete static-probe wording and document the general request/result contract and camera-ray result provenance.
- [ ] Confirm the deployed Rhai package compiles at runtime; this awaits the user's next game launch.

### Task 5: Build and deployment

**Files:** `docs/research/install-history.md`; installed ASI and `MadMaxModding` package/API files.

- [x] Build the Windows MSVC release artifact and record its SHA-256.
- [x] Check Mad Max is closed before deployment; preserve the current ASI, API document, and `reticle_ray` package as timestamped backups.
- [x] Install the ASI, API reference, and updated example package; verify deployed hashes match repository/build hashes.
- [x] Record the installed artifact hashes and verification results in install history.
