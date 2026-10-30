# Mad Max Bevy Runtime Bridge Design

**Date:** 2026-10-27  
**Status:** Approved for implementation  
**Scope:** First in-process runtime slice, including one native vehicle setter

## Intent

Turn the existing Rust ASI into a small, headless Bevy runtime that Mad Max drives through named game and render phase schedules. Both game behavior and overlay state should use Bevy resources, entities, components, and systems, so later construction, inventory, and scripted systems can build on one ECS boundary rather than calling SDK-shaped addresses everywhere.

The first slice keeps the current ASI, DX11 overlay, and `UpdatePostSim` hook. It uses only lightweight Bevy app/ECS crates: no Bevy renderer, UI renderer, window, physics, 2D/3D stack, or independent update loop. It defines a game-side `MadMaxPostSim` schedule and a render-side `MadMaxRender` schedule, each with its own Bevy `App`/`World` owned by its respective host callback thread. The game schedule consumes an owned vehicle snapshot and includes a one-shot `SetVehicleVelocity` request that echoes the velocity copied from the same callback. This exercises the setter path without intentionally changing speed. The one-shot request runs in the first observation build after callback ownership is stable and a current vehicle snapshot is available.

The overlay's sections, visibility, readout values, and presentation state are Bevy resources/entities/components updated by UI systems. Existing ImGui remains the immediate-mode draw and input backend; it reads the UI world's prepared presentation data and sends UI actions back as typed ECS intent entities. ImGui interactions create or update components/resources in the UI world; an internal bounded transport DTO carries those intents into the game world, where systems operate on ECS entities. It does not become a second ECS or access game objects.

## Bevy concept mapping

- **Resources are singleton/current-frame facts and host-wide policy.** Start with `CurrentGameFrame`, `RuntimeDiagnostics`, and `UiPreferences` resources. The copied game snapshot includes callback delta/elapsed time and separately reports the native sample sequence and age (reads are throttled to about 50 ms). Bevy's `Time` plugin is not installed because the host owns scheduling. Do not keep raw game pointers, SDK objects, or references in resources.
- **Entities/components represent things with identity or independent lifetime.** The player and currently occupied vehicle begin as proxy entities tagged with role components and copied snapshot components. Engine addresses never become Bevy `Entity` IDs. Until stable game object identity is established, these proxy entities are explicitly current-frame/session-local representations, not durable engine identity. Construction objects later become mod-owned persistent entities with components for transform, recipe, placement context, and save key.
- **Intent entities represent requested effects.** A mod system expresses a setter request by spawning an entity with `SetPlayerVehicleVelocity { velocity }` and a target policy component such as `TargetCurrentPlayerVehicle`. The entity remains inspectable in ECS until the native adapter consumes it and attaches/publishes an outcome. This is the ECS-facing API, not an imperative native command call.
- **The native adapter is the only translation boundary.** Before the schedule it turns current engine state into owned resources/components and materializes bounded incoming intent envelopes as ECS entities. After the schedule it selects eligible intent entities, validates data and the current engine object, invokes at most one native mutation, then records a copied outcome. ECS systems never call SDK addresses.
- **Schedules and system sets encode host phases.** Each callback runs one named schedule manually. The post-simulation schedule has explicit import/logic/export ordering; the render schedule prepares presentation state. Both use a single-threaded executor on their callback owner. `App::update()` and Bevy's default application loop are intentionally not used.
- **The two apps stay callback-thread-local.** Bevy `App`/`World` is not `Send + Sync` in this configuration, and the DX11 render-loop object must satisfy `Send + Sync`. Neither app is stored inside the hook/render-loop object or moved between threads: each is initialized lazily in thread-local storage on its owning callback thread. Only private bounded DTO channels are shared between callback domains.
- **Use messages and observers only inside a world when their semantics fit.** Bevy 0.19 messages are buffered for schedule-driven readers and normally depend on an update/cleanup system; observers react immediately to triggers. Neither is the cross-thread transport. Durable or reviewable requests stay as ECS intent entities; short-lived internal notifications may use messages only after their lifecycle is explicitly scheduled.
- **Separate callback-owned worlds stay separate.** The game and UI worlds have different Bevy `Entity` spaces. A bounded private transport DTO copies only owned data between them. UI interactions update UI ECS state or spawn UI intent entities; the bridge recreates the corresponding intent component in the game world.
## Why

The project now reaches player and vehicle state from a Mad Max callback, and native vehicle velocity/acceleration getters work as candidate bindings. The community SDK uses the same `UpdatePostSim` hook to run game events and `Game()` after the original callback. That is a useful phase-dispatch precedent. A Bevy schedule can organize Rust behavior, while a narrow adapter retains responsibility for game pointers and calls.

This is a bridge into the game, not a second simulation that replaces the host. Bevy does not make game calls thread-safe by itself. The callback thread, reentrancy, and object lifetime still need runtime diagnostics and strict boundaries.

## Goals

- Keep the runtime in Rust and loaded by the existing ASI.
- Use only lightweight Bevy app/ECS functionality, without Bevy rendering, UI, windowing, physics, or 2D/3D crates.
- Add explicit `MadMaxPostSim` and `MadMaxRender` schedules, manually invoked by their existing host callbacks.
- Represent overlay sections, visibility, values, and presentation state with Bevy resources and entities/components; retain ImGui only for drawing and input capture.
- Give systems owned, pointer-free snapshots and typed runtime events; never expose game pointers or raw addresses to ECS systems or the overlay.
- Move snapshots, events, and ECS intent payloads between separate game/UI worlds through bounded, nonblocking handoff queues.
- Route ECS intent entities through a bounded, per-callback native adapter step that resolves the current game object afresh.
- Include `SetPlayerVehicleVelocity([f32; 3])` as the first mutation request and a one-shot identity echo of the velocity read in that callback.
- Reject non-finite intent values; cap the number of native mutations applied per callback; report rejected, dropped, and applied intents.
- Keep the DX11 callback as the UI world's owner and a consumer of copied snapshots, never as a place for game calls.
- Record callback thread identity, overlap/reentrancy, schedule runs, and intent outcomes to expose lifecycle assumptions.

## Non-goals

- Full translation or source reuse of `mm_sdk`; it remains technical reference only.
- A separate C++ DLL, Bevy renderer, second process, or competing game loop.
- Dynamic loading of arbitrary Rust DLL plugins. Bevy's Rust traits do not define a stable cross-version binary ABI.
- Calling undocumented game event subscription APIs; the SDK declarations do not establish their registration lifecycle.
- Exposing raw game pointers, raw addresses, unrestricted closures, direct struct writes, or arbitrary memory patches to systems.
- Vehicle setters beyond `SetVelocity` in this slice.
- Persistence, construction, save modification, or public third-party mod ABI.

## Approaches

1. **Call `App::update()` from the hook.** Fewest lines, but couples all future Bevy schedules and app lifecycle to one opaque host callback and risks running unrelated systems at the wrong phase.
2. **Headless Bevy apps/worlds with named host-driven schedules (selected).** Game and render callbacks each own an ECS world and invoke only their named schedule. Typed snapshots/events and internal intent payloads cross bounded queues; the render hook only reads copied presentation data. This matches the user's desire to make the UI ECS-driven while avoiding cross-thread access to one mutable Bevy world.
3. **Run Bevy in a sidecar process.** Adds process isolation, but requires IPC, synchronization, lifecycle management, and still needs the in-process callback for game operations. It is premature for the current capability set.

## Runtime flow

```text
Mad Max UpdatePostSim
    -> call original game function
    -> verify this callback is serialized and owned by the expected thread
    -> adapter resolves current character/vehicle and copies GameSnapshot
    -> publish snapshot/events to bounded game-to-UI handoff
    -> update GameWorld resources and run MadMaxPostSim
    -> collect at most one eligible native intent from ECS
    -> adapter re-resolves current vehicle and applies validated SetVelocity
    -> publish copied intent outcome to UI handoff

DX11 render callback
    -> drain newest snapshot and intent outcome events
    -> update UiWorld resources/entities/components
    -> run MadMaxRender
    -> ImGui draws the ECS-prepared presentation and captures input
    -> UI input updates ECS state or spawns an intent entity; the internal transport forwards it for a later UpdatePostSim
```

The callback records its first observed thread ID and counts consecutive calls on that thread, as well as overlapping entries. The runtime waits for a stable serial streak before sampling for Bevy and enabling the one-shot setter. If a callback overlaps or arrives on a different thread, the original callback still runs, but this mod skips all adapter reads, ECS schedules, and setter calls for that entry; it publishes diagnostics without blocking or trying to serialize the game callback with a mutex. Rust panics must not unwind across the native hook boundary.

## Data and safety boundary

- `GameSnapshot` contains copied player/vehicle fields and a monotonic local sample sequence; it has no pointer or reference to game-owned memory.
- Game-side and UI-side Bevy `App`/`World` instances never run concurrently against shared mutable ECS state. `MadMaxPostSim` runs only from the serialized game callback; `MadMaxRender` runs only from the DX11 callback.
- `GameEvent` contains typed state changes and intent outcomes. The ECS-facing API is component/resource based: for example, systems create a `SetPlayerVehicleVelocity` intent entity. A private, owned transport DTO moves that intent between worlds; it is not a public command API. Cross-world queues are bounded and nonblocking; overflow increments a visible drop counter rather than blocking either host callback.
- UI sections and readout rows are Bevy-owned entities/components or resources. UI systems build a small presentation model; the existing ImGui adapter draws that model and converts captured interactions into typed input/events.
- A `SetPlayerVehicleVelocity { velocity }` ECS intent targets whichever vehicle the player currently occupies when the native adapter consumes it. The intent contains no persistent object pointer or engine handle.
- The adapter checks that values are finite, re-resolves the current vehicle immediately before dispatch, validates the vtable region and executable target, and applies no more than one mutating intent per callback.
- The identity echo is issued once after the first valid vehicle snapshot, using the exact velocity copied in that same callback. It is not a repeated correction loop.
- `SetVelocity` is declared in [`vehicle.h`](https://github.com/gigaHours/mm_sdk/blob/f2dc640769c88aae9c3965a2aca0aa138f0e62ab/game/go/vehicle.h). Candidate primary-vtable slot 38 is counted from inherited declarations and earlier `CVehicle` virtuals. This is source-derived, not live-binary-verified; the setter remains a crash risk until user-controlled in-game observation confirms the ABI.
- Current native getters and the setter must run only within the approved host callback. ECS systems receive copied values and create typed requests; they do not make native calls.
- The game callback owns synchronous game schedule execution initially. Background ECS workers and async systems are deferred until callback identity/lifetime evidence supports them. Cross-thread queues carry only owned DTOs for intents, never engine pointers.

## SDK evidence and limits

At pinned `gigaHours/mm_sdk` revision `f2dc640769c88aae9c3965a2aca0aa138f0e62ab`, [`imgui_renderer.cpp`](https://github.com/gigaHours/mm_sdk/blob/f2dc640769c88aae9c3965a2aca0aa138f0e62ab/imgui/imgui_renderer.cpp) calls the original `UpdatePostSim`, then `ProcessGame()`. That dispatches queued game events and each renderer's `Game()` method; the render callback separately dispatches render events and `Render()`.

[`imgui.h`](https://github.com/gigaHours/mm_sdk/blob/f2dc640769c88aae9c3965a2aca0aa138f0e62ab/imgui/imgui.h) queues C++ closures behind a mutex. This is precedent for phase-specific work, not an implementation to copy: the queue has no backpressure/lifetime contract for our needs and its processing semantics should not define the Rust API. [`events.h`](https://github.com/gigaHours/mm_sdk/blob/f2dc640769c88aae9c3965a2aca0aa138f0e62ab/game/events.h) declares event types and handlers but does not demonstrate a safe external subscribe/unsubscribe lifecycle. The SDK does not document callback thread identity, reentrancy, or object-lifetime guarantees.

## Extension model

The first systems are Rust systems compiled into this ASI and registered with its Bevy app. The current `src/modding.rs` seam makes that concrete for this repository, and `player_vehicle_velocity_intent` returns a bundle that systems can spawn with `Commands`. This is convenient in-crate ECS authoring, not a public Rust API for another DLL: the project will not claim that separately compiled Rust modules can implement Bevy traits safely across compiler or dependency versions. If independently distributed mod extensions become a goal, define a deliberate, versioned boundary later, such as a narrow C ABI or a sandboxed scripting format.

## Verification and delivery

- Format and build the Windows release; do not add or run tests unless the user asks.
- Inspect source and diff before build; explain the setter slot/ABI uncertainty.
- Before deployment, confirm Mad Max is closed, back up the installed ASI, deploy the matching build, and verify hashes. Do not launch the game.
- The first in-game observation is user-controlled. Report callback ownership, schedule run count, setter outcome, and whether the current-velocity echo caused any visible behavior.

## Success criteria

- The release builds and retains the current overlay and game-state readouts.
- `MadMaxPostSim` and `MadMaxRender` are manually invoked by their respective Mad Max host callbacks and consume/update only their owning ECS world.
- Overlay state and readout presentation are Bevy ECS data; ImGui draws that data without calling game APIs.
- The one-shot typed identity request is applied through the candidate `SetVelocity` adapter only after the stable-serial callback gate and pointer/value preconditions pass; otherwise the runtime reports why it remains pending or was skipped.
- The overlay reports schedule/callback/intent diagnostics without exposing engine pointers.
- No Bevy rendering, window, physics, 2D/3D stack, or independent update loop is introduced.

## Decisions left to the builder

- Select compatible lightweight Bevy app/ECS crates and features for this Rust toolchain.
- Choose the smallest explicit `ScheduleLabel`/resource/component setup that permits game and UI systems to be registered cleanly.
- Choose bounded nonblocking queue mechanics with clear full-queue behavior.
- Keep callback instrumentation low-cost and avoid allocations/locks where practical; document any unavoidable callback work.


