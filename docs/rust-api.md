# Rust integration

## Support status

The Rust layer is an in-process integration API used to compose host systems. It is not a separately loadable mod ABI: this crate is built as a `cdylib`, Rust type layout and dependency versions are not stabilized for external plugins, and the host does not discover Rust libraries at runtime. User-authored packages should use [Rhai](scripting.md).

## Runtime phases

The host copies data from the native adapter into [`api::ecs::GameSnapshot`](../src/api/ecs/frame.rs). `GameRuntime` projects that snapshot into ECS resources and runs the `MadMaxPostSim` schedule on the game callback thread. Sets execute in order: `Import`, `ModSystems`, then `Export`. Import makes the current frame and accepted requests available; systems read owned state and may queue typed work; export validates, bounds, and sends accepted effects back to the adapter. The UI uses its separate render schedule and may not perform native game access.

`GameSnapshot` is the boundary value, not a live view. Optional fields mean the adapter could not provide a valid sample for that callback. Sequence, age, callback-gate, and drop counters describe freshness and transport health; they do not make unavailable game data safe to infer.

## Data and effects

Use the ECS projection for read access:

- `CurrentGameFrame` contains the latest copied snapshot.
- `PlayerCharacterProxy` and `CurrentPlayerVehicleProxy` identify projected entities.
- `PlayerFrameData` and `VehicleFrameData` contain optional sampled transforms, velocities, and other supported values.
- `RuntimeDiagnostics` reports callback, queue, and effect outcomes.

Effects are requests, not direct ownership of game objects. `SetPlayerVehicleVelocity` and spawn intents pass through bounded queues and validation. The native adapter remains the only layer allowed to dereference game pointers or invoke game functions. Native writes are experimental and should be treated as unverified until tested against the supported executable in game.

## Registering systems

First-party systems are registered in `api::ecs::modding`; game systems belong in `MadMaxPostSimSet::ModSystems` and UI preparation systems in `MadMaxRenderSet::ModSystems`. Systems should consume ECS data, use typed intents for effects, and avoid waiting on cross-callback channels. The game schedule uses a single-threaded executor because it runs on a host callback with thread-affinity constraints.

There is currently no public third-party registration or runtime loading mechanism for separately compiled Rust code. Adding source-level systems requires building them into the host and verifying that their access does not bypass the adapter boundary.

## API map

| Rust module | Contract |
| --- | --- |
| `api::ecs` | Owned snapshots, ECS projection, schedules, diagnostics, and bounded intents. |
| `api::scripts` | Script packages, lifecycle requests, console transport, and events. |
| `api::rhai` | Versioned Rhai context registration and API catalog. |
| `api::game::abi` | Build identity and reverse-engineered native ABI declarations. Use only through reviewed adapter code. |
| `api::game::adapter` | Private native hook, memory, sampling, and effect implementation. |

## Safety

Do not retain native addresses as object handles, use reverse-engineered function aliases as general APIs, or infer runtime correctness from compilation. Address metadata is specific to the build fingerprint. Queue limits and effect validation are part of the runtime contract; callers must handle full queues and unavailable samples.
