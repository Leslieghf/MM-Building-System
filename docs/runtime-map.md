# Runtime architecture

The crate is an ASI bootstrap for a single supported Mad Max executable build. Its runtime reads selected game state, exposes owned snapshots to Rust mod systems and Rhai scripts, then presents diagnostics and controls in a DX11 overlay. Native access is intentionally narrow: pointers and hooks stay in `native`; schedules, queues, commands, and copied data stay in `runtime`; drawing and input stay in `ui`.

## Crate layout

| Module | Responsibility | Boundary
|---|---|---|
| `src/native/` | Build-specific hooks, guarded memory reads, telemetry publication, and diagnostic logging. | Raw game addresses and pointers do not escape this area.
| `src/runtime/` | Bevy schedules, owned frame snapshots, bounded intent queues, mod-system registration, and game/UI coordination. | Runtime systems exchange copied Rust data and validated intents.
| `src/ui/` | Formatting helpers and the DX11/ImGui overlay, including clipboard handling and UI configuration. | UI consumes runtime endpoints and never dereferences game memory.
| `src/scripting/` | Rhai API, package loading, lifecycle, console messages, and reload tracking. | Scripts call registered APIs; they do not receive native pointers.
| `src/sdk/` | Reconstructed Mad Max declarations and build-specific metadata. | Declarations describe candidates; adapters decide what is safe to call.

## Native probe

`native::game_probe` coordinates installation and sampling. `config` owns addresses and budgets for the supported build; `memory` performs guarded reads; `sampling_hooks` observes player, vehicle, camera, and weapon state; `world_query_hooks` owns hook lifecycle; `world_query_capture` copies bounded query results; `sampling` converts validated observations into telemetry; `telemetry` publishes owned values across threads; `runtime_loop` gates callback execution and hands snapshots to Bevy. Keep hook callbacks short and avoid retaining native pointers past their callback.

## Runtime and presentation

`runtime` is the ownership boundary for Bevy resources and cross-thread channels. Game callbacks publish `GameSnapshot`; game-side systems consume bounded intents; UI-side systems render `UiPresentation` and return UI input as intents. The overlay render driver owns a thread-local UI app, synchronizes queued snapshots and outcomes, then renders the app. Clipboard access is isolated behind the ImGui backend.

`runtime::modding` registers mod systems into the game and UI schedules. Script APIs enqueue intent or diagnostic data through the same owned boundary; they do not access native hook state directly.

## Safety and build assumptions

Addresses, offsets, vtable slots, and timing limits are executable-build-specific. Update them only with evidence for the supported executable and keep one authoritative definition. A raw pointer is an observation, not a safe object handle. Check memory regions before reads, copy values before crossing threads, bound queues and per-frame work, and contain panics at foreign callbacks.
