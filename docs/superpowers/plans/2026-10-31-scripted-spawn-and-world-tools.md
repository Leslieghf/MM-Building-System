# Scripted Spawn Workbench and World Tools Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans for inline implementation. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Evolve the verified stockpile spawn into a script-owned, catalog-driven spawn workbench and build a defensible path toward Toolgun-style removal and broader object support.

**Architecture:** Rhai packages orchestrate copied raycast results, panels, gizmos, and bounded typed effects. The Bevy host validates package capability/generation and catalog IDs; game-native calls remain at the existing exclusive callback boundary. Catalog discovery and native weapon/map-removal research are separate from whether an entry is currently safe to spawn.

**Tech Stack:** Rust, Bevy ECS, Rhai, Dear ImGui, existing Mad Max native ABI bridge, TOML/JSON package/catalog metadata.

**Spec:** `docs/superpowers/specs/2026-10-31-scripted-spawn-and-world-tools-design.md`

## Global Constraints

- Never expose native pointers or arbitrary asset paths to scripts.
- Every script effect is bounded, generation-attributed, capability-gated, copied, and validated before game callback dispatch.
- Only catalog entries marked `verified` may be spawned by the end-user workbench.
- Keep focus/capture recovery in the host; package reload must not strand game input.
- Separate mod-runtime input observation from native Mad Max input suppression.
- Do not activate the workbench on R until the game's corresponding native action can be suppressed or shown harmlessly.
- Only remove an exact mod-owned instance until game-object identity and map lifecycle are proven.
- A native Toolgun weapon slot is optional and follows evidence about registration, assets, and input.

## Review Focus

- A script hot-reload between request and game callback must drop stale work; owned-spawn attribution must include package ID and generation.
- Malformed IDs, non-finite transforms, missing capabilities, and full queues must fail without reaching native code.
- Script raycasts are deferred; no-hit/unavailable results must disable placement cleanly.
- An archive-found composition must not be shown as a verified single prop.
- Removing a map-authored object must remain impossible until a validated object identity/lifecycle adapter exists.

---

### Task 1: Evidence-tagged catalog format and seed

**Files:** Create `assets/catalog/entities.json`; create `docs/research/entity-catalog.md`; modify `docs/research/static-prop-asset-candidates-2026-10-31.md` only for new confirmed classifications.

**Interfaces:** Each record has `id`, `name`, `category`, `asset_key`, `kind`, `motion`, `evidence`, `spawn_readiness`, `sources`, and `notes`. Asset key is the exact entity key without `.ee`; catalog ID is independent and stable.

- [ ] Seed only the known observed stockpile and inspected composition candidates.
- [ ] Mark the stockpile `spawn_observed`; mark untested compositions `definition_inspected` and `candidate` or `blocked`.
- [ ] Add source/provenance and a short user-facing category path.
- [ ] Document that `prop` and physical motion are independent fields.
- [ ] Review every seed record against the existing research note; do not bulk-import uninspected keys as spawnable.

### Task 2: Bounded script spawn request contract

**Files:** `src/api/rhai/catalog.rs`, `src/api/rhai/bindings.rs`, `src/api/scripts/manager/invocation.rs`, script dispatch/runtime effect resources, and the game callback/native effect adapter.

**Interfaces:** Add `ctx.entities.request(catalog_id, transform) -> request_id`, guarded by `game.spawn`. The script collector accepts one bounded request per callback, resolves only a catalog entry marked verified, requires a finite 4x4 matrix, and emits an owned effect attributed to package ID and generation.

- [ ] Add request/result types with catalog ID, copied transform, origin script generation, and request ID.
- [ ] Drop stale-generation or unknown catalog requests before native dispatch.
- [ ] Route accepted requests through the current `PendingSpawnIntent` callback path without calling native functions from Rhai.
- [ ] Report queued/submitted/observed/failed status back to the originating script lifecycle channel.
- [ ] Verify API metadata and generated `API.md` describe the exact capability and delay semantics.

### Task 3: Map game input routing and weapon HUD

**Files:** `docs/research/input-and-weapon-hud-2026-10-31.md`; Ghidra `/MadMax.exe`; modify the spec with only proven call paths and boundaries.

**Interfaces:** Separate `ctx.input` (mod-runtime observations) from an optional `ctx.native_input` (native game action policy). The latter is not implemented until one synchronous input path is found and tested. Map keyboard, mouse, DirectInput, XInput/controller, weapon selection, and HUD handling independently.

- [ ] Trace `CKeyboardRouter`, DirectInput/XInput acquisition, and a weapon action into the game callback that consumes it.
- [ ] Determine whether the game needs both down and release transitions and whether a message-based intercept reaches the action.
- [ ] Locate the weapon selector HUD/control state, including any state-container or UI resource owner.
- [ ] Record exactly which action classes can be suppressed and which remain native/unmapped.
- [ ] Prototype selective suppression behind a single known key before exposing it to Rhai.

### Task 4: `debug_spawn_menu/main.rhai` owns menu and ray

**Files:** Create `examples/scripts/debug_spawn_menu/manifest.toml` and `main.rhai`; script UI; `src/devtools/overlay/spawn_menu.rs` only for host activation/focus recovery during migration.

**Interfaces:** The package calls `ctx.raycast.request` using a named verified profile, reads its own ticket result on a later callback, optionally draws the hit indicator with `ctx.gizmos`, builds controls with `ctx.ui`, and submits placement via `ctx.entities.request`. Panel controls configure ray origin/direction source/filter/distance and indicator visibility. The independent `reticle_ray` demo is not a dependency.

- [ ] Build an initial package with the existing stockpile only.
- [ ] Keep the host responsible for activation policy and focus restoration.
- [ ] Expose ray settings and an optional hit marker in a dedicated panel from this package.
- [ ] Use R only after Task 3 validates suppression of its native behavior; otherwise keep the script package dormant/activated by a proven safe path.
- [ ] Allow movement and camera aim through while the workbench is visible; isolate only its UI actions once selective input is proven.
- [ ] Keep the existing native menu available until the script package reaches feature parity.
- [ ] Remove the duplicate native catalog UI only after an in-game check confirms script panel and spawn flow.

### Task 5: Expand catalog discovery and GMod-like browser

**Files:** `assets/catalog/entities.json`, catalog import tooling, script-owned UI package.

**Interfaces:** Browser filters catalog records by category and query; categories group by asset semantics, not inferred physical state. Only verified entries submit spawn effects; candidates show as discoverable but unavailable until checked.

- [ ] Import public/community asset lists as provenance-tagged candidates, never as verified actions.
- [ ] Join exact asset keys against installed archive/file-list evidence.
- [ ] Inspect `.ee` composition definitions and resources for object class, child count, physics, triggers, scripts, and external dependencies.
- [ ] Assign display labels and GMod-like folders; preserve exact original asset key.
- [ ] Add browser paging/search and tile actions within script panel widget and queue limits.
- [ ] Render catalog status consistently: green stable/verified, yellow an attempt that may work or may not, red no usable spawn path; always pair color with a visible label.

### Task 6: Owned-instance Tool mode and despawn validation

**Files:** script workbench modules, owned spawn runtime records, native despawn adapter, lifecycle sampling.

**Interfaces:** Tool mode targets only an owned-spawn native identity that the current build's spawn tracker has observed. Native remove remains unavailable until callback ABI and post-call lifecycle are confirmed.

- [ ] Single-launch validate spawn observation, candidate despawn call, despawn observation, and repeated cycles.
- [ ] Handle area change, reload, missing observation, and package unload as unknown/retained states, never as permission to guess an ID.
- [ ] Add tool selection/removal UI for tracked owned entities only.
- [ ] Keep ray hit object selection separate from spawn identity.

### Task 7: Map-authored deletion and native weapon investigation

**Files:** `docs/research/` evidence notes and Ghidra project; implementation only after API evidence.

- [ ] Trace world-query result to `CGameObject` identity and component ownership.
- [ ] Trace map/zone streaming and object removal across reload and save behavior.
- [ ] Audit `CWeaponHolder`, `CWeaponSequencer`, type tables, creation paths, assets, animations, and UI before judging slot injection.
- [ ] Prefer a script-owned Tool mode unless a supported registration/insertion route is proven.
- [ ] Design a separate gated implementation plan once both contracts are understood.

### Task 8: Weapon HUD and custom Toolgun feasibility

**Files:** Ghidra and `docs/research/input-and-weapon-hud-2026-10-31.md`; product code only after registration points are proven.

- [ ] Identify weapon selector presentation owner and whether it is hideable through a game-owned API.
- [ ] Trace `CWeaponHolder` and `CWeaponSequencer` registration/enumeration; identify any extension point for a new weapon type or slot.
- [ ] Prefer a script-owned tool mode plus separate UI if the game has no stable weapon insertion interface.
- [ ] Do not inject a new slot or replace HUD state through guessed offsets.

## Validation policy

No automated tests are added or run unless requested. For each implementation task, run formatting/build checks and inspect the diff. Game-dependent spawn/despawn and input checks require Mad Max to be closed for install and then manually launched by the user; record submission and in-game observation as distinct outcomes.
