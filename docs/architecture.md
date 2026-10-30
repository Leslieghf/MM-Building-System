# Architecture

This crate is an ASI-hosted modloader and code-library foundation. Its data path is:

```text
MadMax.exe
  -> build-specific game adapter
  -> owned GameSnapshot
  -> Bevy ECS schedules and mod contracts
  -> Rhai callbacks and typed effects
  -> validated game adapter operations
  -> owned outcomes and developer-tool presentation
```

The adapter copies data out of the game before it crosses callbacks or enters ECS. Scripts and ordinary mod systems receive owned values and submit bounded requests; native pointers do not cross that boundary.

## Module map

| Module | Responsibility |
| --- | --- |
| `api::game::abi` | Build fingerprint, executable addresses, ABI declarations, and native value layouts. |
| `api::game::adapter` | Guarded memory access, hook lifecycle, sampling, owned telemetry, and validated effect adapters for the supported build. |
| `api::ecs` | Owned snapshots, Bevy schedules, input/effect contracts, bounded callback channels, and script dispatch. |
| `api::rhai` | Rhai context, capability enforcement, value conversion, debug drawing, and the generated API catalog. |
| `api::scripts` | Script packages, validation, engine and manager lifecycle, console protocol, and reload tracking. |
| `kernel` | `InitializeASI`, startup, endpoint composition, and operational file logging. |
| `devtools` | DX11/ImGui overlay, script and console presentation, clipboard integration, and UI-specific state. |

The paths below follow ownership of the work, including within those domains:

```text
api/
  game/
    abi/                    version-specific declarations and layouts
    adapter/
      hooks/                hook installation and callback entry
      sampling/             native reads copied into owned telemetry
  ecs/
    channels.rs             bounded cross-callback transport
    frame.rs                owned game snapshot
    intents.rs              typed mod requests and outcomes
    world/                  game-side Bevy runtime orchestration
      commands.rs           request import and dispatch
      effects.rs            queued effects emitted from ECS
  rhai/                     script-facing API, context, and conversions
    catalog.rs              versioned API descriptions
    math.rs                 pure script math helpers
    bindings/debug_draw.rs  bounded invocation-local debug geometry
  scripts/                  packages, compilation, lifecycle, console protocol
    console.rs              bounded command/event transport and output history
devtools/
  input.rs                  ImGui key translation into stable input keys
  model/
    console.rs              prompt history, discovery, and script rows
    overlay.rs              input ownership and appearance preferences
    presentation.rs         UI snapshot and display-row resources
  overlay/                  DX11 hook and ImGui panels
  presentation.rs           UI-side Bevy schedule and intent flush
kernel/                     loader entry and startup composition
```

UI state belongs to the developer tools even when it reflects script state; the script API and console transport remain in `api::scripts`. Native declarations remain under the game ABI, while only copied, validated behavior lives in the adapter. This keeps version-specific memory details out of mod-facing contracts and the UI.

`src/lib.rs` exposes only `api`, `kernel`, and `devtools` as top-level domains, plus the loader entrypoint and selected spawn contracts. The `runtime`, `native`, `ui`, `sdk`, and `scripting` top-level layouts have been removed.

## Callback and ownership model

The kernel creates bounded endpoints and installs the developer overlay and game callback. The game adapter validates the executable fingerprint and callback state, samples at its configured interval, and copies results into `GameSnapshot`. The game-side Bevy world projects that snapshot into queryable proxy components, runs registered systems and script dispatch, then emits typed effects. The adapter re-resolves the relevant game object when consuming an effect and reports a copied outcome.

The UI and game Bevy worlds remain separate and are driven by their host callbacks; neither uses `App::update()` or Bevy rendering. A bounded nonblocking transport moves owned requests and outcomes between them. UI state does not transfer entities or game object references.

## Safety and support limits

Addresses, offsets, vtable candidates, and callback details apply to the supported executable build only. The adapter fails closed when its known-build checks fail. A raw pointer is never a stable object handle: reads are guarded, useful values are copied, queues and per-callback work are bounded, and panics are contained at foreign callbacks.

The velocity setter and debug-renderer paths still depend on game-specific candidates and need user-controlled in-game verification. A successful compile does not prove those candidates are correct. Build verification does not install or launch the game.
