# Static Prop Placement and Cleanup — Design

**Status:** Approved for implementation, gated on asset and lifecycle verification
**Date:** 2026-10-31

## Intent

Establish a reversible, script-driven experiment that places one static world prop using an existing Mad Max asset, verifies that the game instantiated it with its normal collision setup, and removes only the instance created by this mod. This is the first step toward construction and world editing. It deliberately does not remove objects that were authored into the map.

## Current state and evidence

- `src/api/game/native/effects.rs` already constructs the SDK-sized `SSpawnInstanceInfo`, sets an entity-file path, fills candidate spawn fields, and calls the Steam `SpawnSystemSpawn` address during the guarded game callback.
- The only current UI experiment requests `editor/entities/a00/exported/a00_vehicle_empty_buggy`. The user has previously confirmed that a buggy appears, so the path is useful for vehicle spawning.
- The native result currently records `NativeCallSubmitted` and the raw return value. That is not a positive observation that the object exists, and the return value has no confirmed ownership/handle semantics.
- Spawn IDs currently come from the bridge's request counter. The SDK also declares a native ID allocator, but this Rust call path does not use it. Collision with IDs owned by the game has not been ruled out.
- The community SDK declares `SpawnSystemDespawn` and `CSpawnedEntityHandler` tracking arrays, but the current adapter does not call a despawn function or consume those arrays.
- Ghidra recognizes Steam entry points `SpawnSystemSpawn` (`0x14218D5B0`), `SpawnSystemDespawn` (`0x142157080`), the descriptor constructor (`0x141EB2410`), and `SetEntityFile` (`0x142171820`). Static decompilation of these `.xpdata` bodies is invalid, so it does not establish their runtime ABI or lifecycle contract. Runtime code observation is required before treating them as safe to call.
- The game install contains `.arc`/`.tab` archives. No asset list or suitable static-prop key is currently present in the project or mod folder. Gibbed.MadMax's `RebuildFileLists` reconstructs resource paths from archive tables and its project hash lists; use that workflow to identify a real shipped `.ee` prop rather than inventing an asset key.
- Many map placements may be baked into streamed world or region data; this has not yet been verified for all scenery. That possibility does not preclude independently spawnable `.ee` entities: the public spawner includes exported entity keys and submits those through the game-owned spawn system. The entity catalog includes encounter and project-composition paths whose names are not proof of standalone prop assets. Asset classification is therefore a required gate, not an assumption.

## Goals

1. Identify one small, existing static prop asset that loads from the installed game data.
2. Request its placement through a typed, bounded intent consumed at the established game callback boundary.
3. Positively observe successful creation and retain a safe mod-owned identity for cleanup.
4. Remove only the test instance created by this mod and observe that removal completed.
5. Give the experiment a focused Rhai script panel with one place action, one remove action, and clear lifecycle status.
6. Confirm the prop is visible and raycast-collidable; separately record whether its expected physics/material behavior is present.

## Non-goals for this slice

- Removing, hiding, or editing a map-authored object.
- Saving placements across sessions or making them permanent.
- Arbitrary asset browsing, a general spawn menu, custom models, meshes, or textures.
- Direct Havok-world allocation or manual collision-shape construction.
- NPCs, particles, encounters, or vehicle-spawner expansion.
- Claiming that native submission equals successful creation.

## Approaches considered

### A. Spawn and clean up one mod-owned static prop — selected

Use the game's entity spawn system, discover a real static-prop `.ee` key from its shipped archive catalog, then establish an owned instance record and matching cleanup path. It uses the existing native spawn work and keeps failure/recovery scoped to one object. It does not answer map-object deletion yet.

### B. Remove a selected map object first

Use the working raycast to select a map object and remove it. This is closer to world editing, but the current ray result does not expose a validated object identity, and deleting a streamed map object may affect area reload and save state. This should follow after object identity and lifecycle are understood.

### C. Construct Havok objects directly

Clone a runtime object's shape, material, broadphase state, and world registration. This may eventually expose more control, but it bypasses the game-owned spawn/ownership path and currently lacks a verified construction/destruction contract.

The user selected approach A, with map-object removal as a separate follow-up and direct Havok construction deferred.

## Proposed behavior

The first deliverable is a `static_prop_lab` script package and the narrow backend required to support it. Its panel offers **Place test prop** and **Remove test prop**. Placement uses the current camera ray's hit position with a small, fixed upward offset; no hit means the action stays unavailable. The test is limited to one instance at a time. The script shows distinct queued, loading/spawning, active, removing, failed, and unavailable states based on backend observations.

Before exposing removal, the native adapter must identify how the spawn system assigns/returns an instance identity and how `SpawnSystemDespawn` consumes it, or use an equivalent game-owned handler operation with a confirmed ownership key. The mod must not keep an unvalidated raw pointer across callbacks. If the SDK's result arrays are the reliable completion signal, copy their relevant values at the game callback boundary and correlate by the native-generated spawn ID.

The asset key is fixed to a single verified shipped prop for the first experiment. It is not accepted from arbitrary script text. The existing raycast and gizmo APIs remain separate and are orchestrated by the script, not hidden inside the spawn API.

## Safety and limits

- At most one outstanding placement request and one live test object.
- Do all native work at the already-used exclusive post-simulation callback; never call the game from Rhai or the render callback.
- Validate the supported executable, callback exclusivity, asset key, transform finiteness, generated ID, object state, and returned status.
- Do not interpret `SpawnSystemSpawn`'s raw return as a handle until established.
- Do not issue cleanup unless the adapter can prove the identity belongs to this mod's test instance.
- On area transition or loss of tracking, report state as unknown; do not guess at a pointer or delete a nearby object.
- No on-disk persistence in this phase. A game restart may clear the temporary test object through normal game lifecycle.

## Acceptance criteria

1. The named asset path is found in the shipped archive/file list and its entity type is inspected as a static prop before the first call.
2. One placement request produces a confirmed active instance at the requested location, with a valid transform and a working world query collision.
3. The script reports success only after the game confirms the instance, not when the native function merely returns.
4. The remove action affects that exact mod-created instance, and the game confirms it is gone.
5. Repeating place/remove does not accumulate orphaned props or remove a map-authored object.
6. Startup, loading, unavailable area, and failed spawn states do not issue native object calls outside the established callback gate.

## Decisions left to implementation

- Which existing static prop asset is the smallest safe first target.
- Which spawn-completion and spawned-entity tracking fields provide positive identity on this Steam build.
- Whether cleanup uses `SpawnSystemDespawn`, a spawned-entity handler delete queue, or another game-owned removal path.
- How the script receives request IDs and asynchronous completion events without exposing native pointers.
- Whether the first placement offset should be global-up or ray/surface-normal based. The current ray API does not yet provide a verified hit normal, so the proposal uses global-up.

## References

- Local SDK audit: `docs/research/2026-10-27-mm-sdk-rust-port-audit.md`.
- Existing transform/spawn feasibility notes: `docs/research/rust-transform-feasibility.md`.
- Existing spawn adapter: `src/api/game/native/effects.rs`.
- SDK declarations and pinned usage links: `src/api/game/abi/game/spawn.rs` and `docs/research/2026-10-27-mm-sdk-rust-port-audit.md`.
- The [Gibbed.MadMax file-list rebuild tool](https://github.com/gigaHours/Gibbed.MadMax/blob/master/RebuildFileLists/Program.cs) reads `.tab` archive tables and rebuilds known resource-path lists; use it to identify a real asset key.
