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

Frame metadata, `ctx.math`, and formatting helpers do not require a capability. `available` indicates that an object sample exists; individual properties can still be unavailable and are represented by `()`.

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

Scripts cannot access arbitrary native calls, FFI, files, processes, networks, or Rhai modules. Vehicle writes and debug rendering are experimental native paths. Their requests can compile and pass API validation without producing the expected in-game result.
