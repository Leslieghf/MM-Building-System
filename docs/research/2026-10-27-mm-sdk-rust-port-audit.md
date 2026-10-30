# Mad Max SDK Port Audit: Construction-First

**Date:** 2026-10-27  
**SDK revision:** [`f2dc640769c88aae9c3965a2aca0aa138f0e62ab`](https://github.com/gigaHours/mm_sdk/tree/f2dc640769c88aae9c3965a2aca0aa138f0e62ab)  
**Status:** Source-surface audit; no new native calls or game changes made.

## Decision

A Rust-facing port of the SDK's useful surface is feasible. A literal translation of the repository is neither necessary nor a useful first milestone. At this pinned revision, the repository tree has 61 files and 2,485,103 bytes. Vendored ImGui accounts for 21 files / 2,345,176 bytes and MinHook for 13 files / 92,864 bytes. The game-specific `game/` declarations are 13 files / 14,269 bytes. Much of the apparent size is unrelated UI and hooking implementation that this project already supplies through its Rust ASI/overlay stack.

The construction goal changes the port order: first establish one reversible **spawn → observe → despawn** operation for a known existing asset. Broader SDK coverage should follow demonstrated use cases. Persistence is a later layer: do not save placements until asset identity, area lifecycle, duplicate prevention, and successful spawn completion are understood.

## Surface map

| SDK area | What it describes | Port priority | Evidence / caution |
|---|---|---:|---|
| Spawn system | Spawn descriptor, entity-file string, spawn IDs, spawn/despawn entry points and queues | Highest | Function declarations exist; valid descriptors, asset naming, completion semantics, thread affinity and supported areas are not established by these headers. |
| Game objects and transforms | Object identity, alias/name, children, transform initialization/get/set | High after spawn proof | Native pointers and C++ object lifetime stay inside the adapter. A transform setter is not a substitute for proving object creation. |
| Character / vehicle / camera | Player character, attached vehicle, camera matrix, vehicle reads and setters | Medium | Some calls already exist in this project's bridge. Each native signature/address/slot is build-specific evidence, not a generic binding. |
| Game state and callbacks | Main-menu/state globals, `UpdatePostSim` renderer dispatch and event declarations | Medium | The SDK does not establish callback thread, reentrancy, subscription lifetime, or a general safe plugin lifecycle. The current project owns its callback integration. |
| Core utility types | Hash strings, vectors, strings, weak/shared pointers, matrices | Selectively | C++ container and smart-pointer ABI layouts are traps. Prefer copied Rust DTOs and native-owned opaque storage; do not expose these types to ECS systems. |
| ImGui and MinHook | UI rendering and hook backend | Low / omit | Already-covered infrastructure; avoid duplicating dependencies merely to claim SDK completeness. |

## Spawn path evidence

The pinned [`spawnsystem.h`](https://github.com/gigaHours/mm_sdk/blob/f2dc640769c88aae9c3965a2aca0aa138f0e62ab/game/spawnsystem.h) declares `SSpawnInstanceInfo`, `SpawnSystemSpawn`, `SpawnSystemDespawn`, spawn-ID allocation, and a spawned-entity handler with spawn/delete queues and last-frame spawned/despawned/failed lists. `SSpawnInstanceInfo` is asserted as 0x98 bytes and contains fields such as a spawn entity-file string, object IDs, location hash, flags, radii, and resource-set IDs. It declares native construction/destruction helpers and `SetEntityFile`.

The public [`MadMaxSpawner` example](https://github.com/gigaHours/MadMaxSpawner/blob/main/spawnermenu.cpp#L600-L619) provides a concrete call recipe: it allocates the descriptor through the SDK constructor, assigns the generated spawn hash to both `m_SpawnId` and `m_SpawnPatchId`, sets a 90,000 squared despawn distance and critical priority, appends `.ee` to the selected key, derives a transform from Max's transform, disables spawn limits through the singleton context, calls `SpawnSystemSpawn`, then runs the matching descriptor destructor. Its callback is queued through the mod's game-event path. The repo contains entity keys such as `editor/entities/a00/exported/a00_vehicle_empty_buggy`.

This example narrows the unknowns but does not close Gate 0: it does not inspect the spawn return or result arrays, demonstrate a matching despawn, explicitly initialize resource-set IDs, or show error cleanup after a rejected/failed request. The C++ SDK macros establish candidate call sites and declarations, but this source alone does not prove the Rust ABI or that its behavior remains safe on this project's callback.

The Rust runtime now has a data-only `spawn_entity_intent` bundle with a session-local request ID, validated exported key, finite translation/quaternion, and explicit lifecycle component. It deliberately does not create a native spawn ID, call the SDK, or report that an asset exists; the request remains an ECS intent until the native lifecycle gate is closed.

These declarations make a spawn path plausible, but they do **not** establish:

- which entity-file identifier is valid for a particular world object;
- how required fields and resource sets must be initialized;
- whether the function's return value is a request ID, object ID, count, or completion signal;
- when an entity is actually present and safe to despawn;
- whether the calls are safe from this project's `UpdatePostSim` callback;
- whether world, camp, and instance contexts share the same lifecycle;
- what persistence and duplicate reconciliation rules are valid.

The handler's direct `Spawn` declaration uses an address without a distribution selector, and its global instance address is distribution-specific. The free spawn/despawn declarations provide GOG and Steam address pairs, but address availability alone is not proof that their ABI or lifecycle contract is correct for our supported executable. The Rust bridge should not call any of these until these gaps are resolved for the target build.

## Existing project boundary

The current project already has a host-driven lightweight Bevy ECS bridge, separate game/UI schedules, owned snapshots and typed intents, plus game callback instrumentation. It is a suitable destination for high-level spawn requests and results. ECS components/resources should carry owned data such as a validated asset key, transform, request state, and stable mod placement ID. Raw engine pointers, C++ strings/vectors, and native descriptor ownership belong in a narrow native adapter only.

The first operation should be compile-time/development-only and reversible. It should use an existing harmless asset that can be identified from game data, spawn once at a controlled transform, wait for positively observed completion, then despawn it and report the result. Do not add save-file persistence, arbitrary object selection, UI authoring, or a public extension ABI to that slice.

## Rust coverage started in this repository

The first Rust SDK tranche now lives under `src/api/game/abi/`. It is a partial, source-tracked port, not yet API parity:

| Pinned SDK source | Rust surface now present | Runtime state |
|---|---|---|
| `math3d.h` | Pointer-free `Vector2`, `Vector3`, `Vector4`, `Plane`, `AxisAlignedBox`, `Quaternion`, and `Matrix4x4`; vector/matrix value operations; candidate aliases for the SDK's direct matrix/quaternion helpers | Pure value operations tested; native math helper aliases are not called |
| `core/hash.h` | `HashId` and `HashStringFn` | Raw hash values usable; game hashing call not wired |
| `core/input.h` | Opaque action-map/device-manager handles and typed function signatures | Not called |
| `game/charactermanager.h`, `game/go/character.h` | Opaque character/manager types and typed signatures | Existing player and vehicle reads use these aliases; behavior unchanged |
| `game/cameramanager.h`, `game/go/go.h` | Opaque camera/game-object types, transform signatures, and `ObjectId` value | Existing transform/camera reads use typed aliases; setters not enabled here |
| `game/go/vehicle.h` | Opaque vehicle type, velocity signatures, and current candidate vtable indices | Existing reads/setter still use the previous candidate addresses/slots; no additional calls |
| `game/game.h` | `GameState` discriminants and `IsGuiOccludingMainDrawFn` signature | No new state reads |
| `game/spawnsystem.h` | Priority values, observed result names, opaque descriptor/queue/entity markers, and candidate native function signatures | No spawn/despawn calls; resource setup and lifecycle still block safe use |
| `game/go/pfxgo.h`, `physgo.h`, `damageable.h`, `vehicle.h`, `go.h` | Opaque hierarchy markers plus declaration-order catalog for virtual method names | Catalog ordinals are not numeric vtable slots; MSVC subobject offsets and full slot mapping remain unverified |
| `game/events.h` | Opaque event markers, handler signature, and declaration-order metadata | No subscription/dispatch API; SDK does not establish external callback lifetime |
| `core.h`, `std.h` | Not translated | Template containers, custom string/vector representations and Boost pointer layouts remain opaque |
| `core/graphics/graphicsengine.h` | Not translated | Existing `hudhook`/DX11 code supplies the project's rendering boundary; the SDK's broad graphics-engine data layout is not needed for core game APIs |
| `hookmgr.*`, `imgui/`, `minhook/` | Not copied | Existing plugin/hook/overlay stack supplies this infrastructure |

`src/api/game/abi/build.rs` records address pairs from the pinned SDK and the separately recorded local Steam executable fingerprint. The existing character-manager, player-character, character-vehicle, camera-manager, and camera-matrix reads now import their shared Steam constants from this profile, so the inventory and bridge do not carry separate copies of those addresses. The profile only resolves data for explicit SDK build labels; these absolute addresses remain candidates and the module does not call them. Several SDK entries contain `0` or suspicious duplicate addresses, which are preserved as declared evidence and must not be promoted to a callable binding without inspection.

This is a useful foundation but **not yet on-par with the community SDK**. The largest remaining gaps are exact MSVC multiple-inheritance/vtable mapping, several class-specific virtual signatures, core container/event/graphics interfaces, and a verified spawn descriptor lifecycle. SDK breadth is feasible; binary and lifecycle validation still determines which entries can become usable Rust APIs.

## Porting approach

1. **Inventory by capability:** record signature, supported executable/build, call phase, ownership, evidence, and verification status for each function.
2. **Port DTOs and declarations selectively:** represent copied vectors/matrices/IDs with Rust-owned values; keep uncertain native layouts opaque.
3. **Centralize address profiles:** one explicit build profile owns every address and signature check; unknown builds fail closed.
4. **Keep unsafe FFI narrow:** native constructor/destructor, string marshalling, and calls reside in one adapter module; no pointer crosses into ECS.
5. **Implement one vertical slice:** prove spawn, positive observation, cleanup, and repeated no-duplicate behavior before generalizing.
6. **Add persistence only after lifecycle proof:** use mod-owned per-playthrough data and reconcile placements on confirmed area readiness.

## Provenance and licensing

The pinned repository tree has no top-level `LICENSE` file. This audit uses linked public declarations as research evidence and does not copy SDK implementation code or vendor its dependencies. The missing license is not a reason to stop private, independently written wrappers around documented interfaces; it does mean reuse/redistribution terms should be resolved before copying code or publishing derived source. This is a provenance note, not legal advice.

## References

- [Pinned SDK tree](https://github.com/gigaHours/mm_sdk/tree/f2dc640769c88aae9c3965a2aca0aa138f0e62ab)
- [Spawn system declarations](https://github.com/gigaHours/mm_sdk/blob/f2dc640769c88aae9c3965a2aca0aa138f0e62ab/game/spawnsystem.h)
- [Game object declarations](https://github.com/gigaHours/mm_sdk/blob/f2dc640769c88aae9c3965a2aca0aa138f0e62ab/game/go/go.h)
- [Vehicle declarations](https://github.com/gigaHours/mm_sdk/blob/f2dc640769c88aae9c3965a2aca0aa138f0e62ab/game/go/vehicle.h)
- [SDK hook manager](https://github.com/gigaHours/mm_sdk/blob/f2dc640769c88aae9c3965a2aca0aa138f0e62ab/hookmgr.cpp)
- [SDK ImGui callback integration](https://github.com/gigaHours/mm_sdk/blob/f2dc640769c88aae9c3965a2aca0aa138f0e62ab/imgui/imgui_renderer.cpp)
