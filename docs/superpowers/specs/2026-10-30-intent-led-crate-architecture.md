# Constituent-first architecture review

## Intent

The project is an ASI mod host for Mad Max. Its eventual API should turn build-specific executable access into safe, owned, useful modding capabilities, including ECS-facing data/effects and a Rhai surface. A small kernel should load the ASI, connect host callbacks, and drive the runtime. Higher-level module names should be derived from the responsibilities and dependencies of the concrete pieces, not chosen first and used to force-fit them.

## Current end-to-end flow

```text
MadMax.exe
  -> SDK declarations + Steam build metadata
  -> native hooks, guarded reads, and sampling
  -> copied GameSnapshot telemetry
  -> bounded endpoint into the Bevy game schedule
  -> ECS projections and script dispatch
  -> Rhai GamePostSimContext built from copied data
  -> bounded typed effects / debug draw requests
  -> ECS intent handling and native adapter calls
  -> copied outcomes to Bevy and developer overlay
```

The overlay has its own render callback and Bevy UI world. It exchanges owned snapshots, console events, discovery results, and UI intents through bounded channels. The ASI entry point currently lives in the overlay module, so startup, runtime composition, game hooking, and presentation are tangled at the root boundary.

## Concrete constituent inventory

| Constituent | What it does today | Boundary/data it owns |
|---|---|---|
| `sdk/build.rs`, `sdk/types.rs`, `sdk/game/*`, `sdk/core/*` | Reconstructed ABI/type declarations and candidate addresses for the inspected build. | Describes native layouts/calls; declarations alone do not validate a call. |
| `native/game_probe/config.rs`, `memory.rs` | Build RVAs, offsets, sampling budgets, executable checks, guarded memory reads. | Build assumptions and raw-memory safety. |
| `native/game_probe/sampling_hooks.rs`, `world_query_hooks.rs`, `world_query_capture.rs` | Hook installation and callback-time observations, including bounded query capture. | Native pointers and callback-local state. |
| `native/game_probe/sampling.rs`, `telemetry.rs` | Copy validated observations into typed values and publish them across threads. | Owned telemetry; no retained game pointers. |
| `native/game_probe/runtime_loop.rs` | Callback gate, schedule handoff, and dispatch of approved game effects. | Game callback timing and exclusivity. |
| `runtime/model.rs`, `endpoints.rs`, `game.rs`, `game_loop.rs` | Snapshot/intent data, bounded channels, Bevy game schedules, and effect processing. | ECS resources, copied frames, and validated requests. The current model file also mixes UI and game DTOs. |
| `scripting/api.rs`, `api_catalog.rs`, `vector.rs` | Register Rhai methods and describe the versioned script surface; build copied contexts, validate script values/capabilities, collect bounded effects. | Rhai-facing contract. Today it consumes snapshots; raw game access is elsewhere. |
| `scripting/dispatch.rs`, `manager.rs`, `engine.rs` | Run script callbacks from ECS, manage active scripts, construct the Rhai engine. | Script lifecycle and invocation. |
| `scripting/loader.rs`, `package.rs`, `auto_reload.rs` | Confined package discovery, validation/compilation, worker delivery, and reload fingerprints. | Filesystem/package lifecycle and bounded worker messages. |
| `scripting/console.rs`, `ui/overlay/*`, `runtime/presentation.rs` | Typed console messages and commands, ImGui panels/input, UI-side world projection. | Developer-tool presentation and command transport. These are currently split across three roots. |
| `lib.rs`, `ui/overlay/mod.rs`, `runtime/modding.rs`, `native/game_probe/mod.rs` | ASI export, startup, endpoint wiring, schedule registration, and hook orchestration. | Kernel/composition responsibilities currently scattered among feature modules. |
| `native/logging.rs`, `bin/trace_xref_probe.rs` | Append-only diagnostics and a standalone Windows process scanner. | Operational diagnostics and a separate investigation utility. |

## Candidate grouping, not final paths

The constituents suggest three different kinds of work that should remain distinguishable:

1. **Executable access and API adaptation:** build metadata, ABI declarations, guarded reads, hooks, typed sampling, ECS projection, capability-checked Rhai bindings, and typed effects. This follows the path from raw executable state to the public modding API. The user has proposed treating this as a possible top-level `api` domain.
2. **Kernel/composition:** the ASI entry point, startup, callback lifecycle, schedule registration, endpoint wiring, and shutdown/failure handling. This is small compared with feature modules and should be the composition root, not another name for game access. The user has proposed a possible top-level kernel module.
3. **Developer tools and package lifecycle:** script package loading/lifecycle plus console/editor/diagnostic presentation. The final grouping depends on whether those are one mod-authoring product area or separate script and host-tool areas.

A bottom-up pass should next inspect coupling within each candidate group, split mixed leaf modules (notably game/UI DTOs, Rhai API adapters, and overlay startup), and draw dependency arrows. Only after that should the top-level directory names be selected. Repeat the same exercise inside each chosen area until leaf ownership is clear.

## Constraints for the eventual design

- Raw pointers remain inside native callback/adapter code; cross-thread and script-facing values are owned, bounded types.
- Keep build-specific addresses in one authoritative place.
- Separate raw ABI declarations, safe game adapters, ECS mod contracts, and Rhai presentation even if they live under a shared `api` namespace.
- Keep kernel orchestration narrow and explicit; feature modules should not become the application entry point.
- Preserve public script API behavior, hook order, queue limits, and failure behavior during structural cleanup.
- No hot reload redesign, gameplay feature work, or game installation/launch.

## Dependency-derived proposal for review

This tree follows the accepted constituent inventory. It treats `api` as the path from executable access to user-facing modding capabilities, keeps `kernel` as a narrow composition root, and gives the substantial overlay its own `devtools` area rather than hiding it inside the kernel.

```text
src/
  lib.rs                         public exports
  api/
    game/
      abi/                       SDK declarations and build metadata
      memory.rs                  guarded native reads
      hooks/                     callback installation and hook entrypoints
      sampling/                  raw observations -> owned game telemetry
      adapter.rs                 typed intents -> validated native calls
    ecs/
      model.rs                   owned frame, entity projections, intents/outcomes
      channels.rs                bounded game/UI/script endpoints
      schedules.rs               game and mod-system schedule registration
      scripting.rs               ECS systems dispatching script callbacks/effects
    rhai/
      context.rs                 copied values available to script invocations
      bindings.rs                register user-facing Rhai functions/methods
      capabilities.rs            capability checks and effect gating
      vector.rs                  pure script math helpers
      catalog.rs                 API version and generated reference metadata
    scripts/
      engine.rs                  Rhai engine configuration
      manager.rs                 active script lifecycle and invocation
      loader.rs                  confined package discovery/worker
      package.rs                 manifest validation and compilation inputs
      auto_reload.rs             package content fingerprints/reload tracking
      console.rs                 owned requests/events shared by UI and scripts
  kernel/
    mod.rs                       composition root and callback coordination
    entry.rs                     exported ASI initialization and startup
    game_loop.rs                 game callback gate and bounded ECS/effect loop
  devtools/
    overlay/
      driver.rs                  DX11 render callback and UI-world synchronization
      panels.rs                  ImGui console, diagnostics, and input panels
      clipboard.rs               Win32 clipboard backend
      config.rs                  overlay input/opacity policy
    presentation.rs              runtime diagnostics projection/readout preparation
    logging.rs                   append-only operational diagnostics
  bin/
    trace_xref_probe.rs          standalone process-inspection utility
```

### Dependency arrows

```text
api::game::abi -> api::game::hooks -> api::game::sampling
api::game::sampling -> api::ecs::channels / api::ecs::model
api::ecs -> api::rhai / api::scripts
api::rhai -> api::ecs (owned invocation data and typed effects)
api::scripts -> api::rhai + api::ecs
kernel -> api + devtools (wires callbacks and endpoints)
devtools -> api::ecs + api::scripts::console (reads copied state, sends typed commands)
```

`api` must not depend on `kernel` or `devtools`; `kernel` may compose both. `devtools` consumes owned API data and sends typed requests; it never calls raw game functions. Raw SDK declarations are not safe adapters. The adapter layer validates and copies observations before publishing them, and validates effects before invoking supported native functions.

### Questions to resolve before implementation

- Should `api::scripts` stay under `api` because it implements the public Rhai modding surface, or should script package lifecycle be a sibling capability?
- Does `api::ecs` correctly name the Bevy-facing owned contract, or should it be `api::runtime` with ECS as one implementation detail?
- Is `devtools::presentation` the right home for diagnostics readouts, with shared DTOs remaining in `api::ecs`?

Once these are settled, the tree is the structural design to approve before file moves. Any additional mixed leaf modules discovered during implementation should trigger another small inventory-and-review pass before being split.
