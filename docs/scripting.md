# Using Rhai scripts

Rhai packages are the supported user-authored extension format. A package runs inside the modloader’s constrained API v1 context; it is not a general-purpose embedded Rhai environment.

## Install a package

Build the ASI using [Development](development.md), load it with a compatible ASI loader, and create this directory beside `MadMax.exe`:

```text
MadMaxModding/scripts/<package-id>/
  manifest.toml
  main.rhai
```

Copy an example from [`examples/scripts`](../examples/scripts). Start the game, open Dev Tools with **F3**, open **Scripts**, choose **Rescan** if the package was added after startup, select the package, and choose **Load**. Package discovery validates manifests without compiling or executing source; loading and reloading perform compilation on a worker and activate the result at a game callback boundary.

## Declare the package

```toml
id = "vehicle_watch"
api_version = 1
capabilities = ["game.read", "log"]

[callbacks]
"game.post_sim" = "on_post_sim"
on_run = "on_run"
```

`id` must match both the package directory and manifest location. It uses 1–64 lowercase ASCII letters, digits, `_`, or `-`; Windows device names are rejected. `api_version` must equal the host API version. A package can request at most 16 capabilities and declare at most two callbacks. Source and package sizes are bounded by the runtime; see the constants in [`api::scripts::package`](../src/api/scripts/package.rs).

## Implement callbacks

Each callback takes exactly one context argument. Top-level statements are not executed, so initialize persistent data inside a callback.

```rhai
fn on_post_sim(ctx) {
    if !state.contains("last_log") {
        state.last_log = -2.0;
    }
    if ctx.frame.elapsed_seconds - state.last_log < 1.0 {
        return;
    }
    state.last_log = ctx.frame.elapsed_seconds;
    ctx.log.info("Vehicle: " + ctx.format.vector(ctx.vehicle.position));
}

fn on_run(ctx) {
    ctx.log.info(`Frame ${ctx.frame.sequence}`);
}
```

`game.post_sim` runs after the sampled game callback. `on_run` runs only after `script run <id>` is requested; it receives the latest copied frame and may run before the package’s first `game.post_sim` callback. Return values are ignored. The `state` map persists for the loaded generation, including between `on_run` and `game.post_sim`; a successful reload starts a new generation with empty state.

## Request capabilities

API v1 capabilities are:

| Capability | Grants |
| --- | --- |
| `game.read` | Access to available copied player and vehicle values. |
| `input.read` | Read mod-owned keyboard/mouse events collected by Hudhook/ImGui; this is not native game or controller input polling. |
| `log` | Bounded informational console output. |
| `game.vehicle.write` | Request a validated vehicle velocity effect when copied velocity is available. |
| `debug.draw` | Request bounded one-frame debug primitives through the native renderer adapter. |
| `game.raycast` | Submit a deferred world query and read its copied result on a later callback. |
| `ui.panels` | Publish bounded, script-owned ImGui windows and receive copied widget changes on a later callback. |

Frame metadata, `ctx.math`, and formatting helpers do not require a capability. `available` indicates that an object sample exists; individual properties can still be unavailable and are represented by `()`.

## Script-owned panels

Packages that request `ui.panels` can publish up to four independent windows from a callback. A panel is a declarative snapshot: call `ctx.ui.panel(id, title)`, add headings, labels, checkboxes, sliders, numeric inputs, selectors, or buttons, then call `ctx.ui.publish(panel)`. The latest snapshot replaces that package's previous windows. The render thread draws copied values only; it never executes Rhai. Widget changes are delivered through `ctx.ui.events(panel_id)` on a later `game.post_sim` callback. Button values are one-callback `true` edges.

Panels remain independent of the compact HUD and can be open together. The UI Configurator's **Script panels** toggle hides or shows every script panel without unloading scripts or changing each panel's open state. The Scripts dashboard can reopen a panel that was closed with its title-bar button. F1 controls input capture: panels display read-only while capture is off, and their controls become interactive while capture is on.

Panel IDs and widget IDs use lowercase letters, digits, `_`, and `-`. Each panel supports up to 48 widgets; text is limited to 160 UTF-8 bytes and selectors to 16 fixed string options. Numeric values must be finite and within ±100,000; selector values must match one of their options. The first example is [`reticle_ray`](../examples/scripts/reticle_ray/main.rhai), which provides the Ray Workbench and keeps ray visualization in the separate `debug.draw` capability.

Current in-game raycast validation distinguishes the two supported filters: `static_only` hits static geometry only; `player_aim` does not hit Max's player character, but does hit Max's vehicle, other vehicles, and static geometry. These observations validate the query path and filter behavior for those cases; they do not by themselves validate every copied camera or actor transform as a ray source.

Ray queries are demand-driven: merely loading a script or opening a panel does not execute one. Use `ctx.raycast.camera(max_distance, filter)` when the copied camera origin and view direction are the intended ray, or `ctx.raycast.request(origin, direction, max_distance, filter)` for a custom ray. Identical requests from multiple consumers in the same game frame are served by one native query and receive separate result tickets. The shared frame result is discarded afterward; consumers that need a fresh camera hit must request again.

## Use the console

Open **F3 → Console** and use:

```text
help
script list
script load vehicle_watch
script run vehicle_watch
script reload vehicle_watch
script unload vehicle_watch
```

`script list` rescans package manifests but does not compile source. `load` compiles and activates a generation; `reload` compiles a replacement; `run` queues one `on_run` callback; `unload` removes the active generation. Requests use bounded queues and may be rejected when capacity is exhausted.

## Errors and recovery

Invalid manifests remain visible in the Scripts view with a validation issue. Compile errors include source position when available. Unknown callbacks, undeclared capabilities, malformed values, non-finite numbers, and out-of-range values become script errors. A callback error disables that generation. A failed reload keeps the last active generation running; correct the package and retry reload.

## Execution limits and safety

The Rhai engine bounds operations, recursion, variables, functions, strings, arrays, maps, and source size. Per-invocation and per-frame budgets also bound callbacks, logs, vehicle effects, debug primitives, and geometry work. See [Rhai API v1](rhai-api.md) for exact method signatures, value shapes, coordinate conventions, and limits.

## Package files and dependencies

Packages may list multiple Rhai files in manifest order. The host compiles them into one package without enabling Rhai filesystem imports. Every path is relative to that package folder and subject to path and size limits.

```toml
id = "event_workbench"
api_version = 1
version = "0.1.0"
sources = ["main.rhai", "events.rhai"]
capabilities = ["ui.panels", "ui.workbench"]
requires = [
    { kind = "host_api", id = "workbench.tabs", version = "^1.0.0" },
]

[workbench_tab]
id = "events"
title = "Events"
order = 100
```

`requires` and `conflicts` express package/API compatibility, while `capabilities` grant specific effects. A tab package also publishes its copied panels from a callback; the Rust shell renders those values and returns generation-checked UI events. See the generated [API reference](rhai-api.md) for limits and lifecycle semantics. The [Event Workbench example](../examples/scripts/event_workbench/README.md) shows a multi-file package and read-only tab.

The Scripts dashboard marks required dependency graphs as ready or blocked, and calls out missing optional dependencies without preventing load. A blocked package cannot be loaded until its required local packages and host API versions are available. Inside a callback, `ctx.dependencies.available("package-or-api-id")` is true only for an ID declared in `requires` whose version is currently available; undeclared IDs and unavailable optional requirements return false.

Scripts cannot access arbitrary native calls, FFI, files, processes, networks, or Rhai modules. Vehicle writes and debug rendering are experimental native paths. Their requests can compile and pass API validation without producing the expected in-game result.
