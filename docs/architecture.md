# Architecture

The project hosts code extensions inside a Windows x64 ASI for one supported Mad Max executable build. Rhai is the user-authored package format. Rust and Bevy provide the in-process runtime structure; they are not a dynamic Rust plugin ABI.

## Runtime flow

```text
Game callback
  -> verify supported image and callback gate
  -> sample native state into an owned GameSnapshot
  -> import snapshot and bounded requests (Import)
  -> run Rust systems and Rhai callbacks (ModSystems)
  -> validate and export typed effects (Export)
  -> native boundary applies only supported native operations

Render callback
  -> collect mod-owned Hudhook/ImGui input
  -> run render schedule
  -> present the DX11 developer overlay
```

The game schedule is single-threaded and callback-driven. Script loading and package discovery run on a worker; prepared results are activated at the game callback boundary. Scripts observe copied frame data and submit bounded effects. They never receive native pointers.

## Module ownership

| Module | Owns |
| --- | --- |
| `api::game::abi` | Supported build identity, image-relative addresses, layouts, and native calling-convention declarations. |
| `api::game::native` | Callback hooks, guarded memory reads, sampling, thread/re-entry gates, and native effect application. |
| `api::ecs::components` | ECS components projected from copied player and vehicle samples. |
| `api::ecs::resources` | Owned frame snapshots and runtime diagnostics. |
| `api::ecs::systems` | Schedule labels, schedule construction, and first-party system registration/dispatch. |
| `api::ecs::transport` | Bounded callback and UI endpoint channels. |
| `api::ecs::runtime` | Game callback orchestration and intent import/export. |
| `api::ecs::{input,intents}` | Mod-owned input snapshots and typed effect requests. |
| `api::scripts` | Package validation, asynchronous discovery/compilation, generation lifecycle, bounded console requests and events. |
| `api::rhai` | API v1 context, capability enforcement, conversions, pure helpers, and generated reference catalog. |
| `kernel` | ASI entrypoint, startup composition, panic boundary, and operational logging. |
| `devtools` | ImGui input/presentation and script/console controls; it does not own native game state. |

The native game boundary is the only owner of raw executable interaction. The ECS projection and script context are owned-value interfaces. Research notes may describe candidate layouts or hooks that are not enabled or validated; they are not supported APIs.

## Module placement rules

- Use top-level modules for stable domains (`api`, `devtools`, `kernel`) and nested domain modules for game, ECS, scripting, and presentation concerns.
- Inside ECS, place Bevy data in `components` or `resources`, execution in `systems`, cross-thread endpoints in `transport`, and callback lifecycle orchestration in `runtime`.
- Use descriptive domain names for behavior (`input`, `intents`, `native`) and classification names only when every item shares that classification (`types`, `systems`, `components`, `resources`).
- Avoid generic catch-all modules such as `state`, `common`, or `misc`; split mixed modules by ownership and responsibility.

## Rhai callback contract

Each package declares API version 1, capabilities, and callback names in `manifest.toml`. `game.post_sim` runs after a sampled game callback. `on_run` is a manually requested invocation through that same context. A callback receives one context and ignores its return value. Each loaded generation owns its state and limits. A callback error disables that generation; a failed replacement leaves the prior generation active.

Callback execution validates requested capabilities and values, enforces Rhai engine limits, and bounds logs and effect budgets. The exact signatures and outward behavior are defined in [Rhai API v1](rhai-api.md), generated from the runtime catalog.

## Native boundary and guarantees

The native boundary checks the executable fingerprint and guards readable/executable address ranges before access. These checks reduce risk; they cannot establish that a reverse-engineered layout or call is correct. Snapshot fields may be unavailable, callback-gate diagnostics may delay or reject sampling, and bounded channels may drop requests. Consumers must inspect availability and outcomes.

Script vehicle writes and debug renderer calls are experimental. A successful `cargo check`, `cargo build`, or API documentation build verifies Rust compilation only; it does not validate in-game effects. Builds do not install or launch Mad Max.

## Extension boundaries

- Use Rhai packages for user-authored extensions. Capabilities are explicit and do not grant general filesystem, network, process, FFI, or module access.
- Use `api::ecs` source integration for systems compiled into the host. This is not a stable binary interface.
- Keep executable-specific logic inside `api::game::abi` and `api::game::native`.
- Keep presentation in `devtools`; game systems communicate through owned data and typed requests.
