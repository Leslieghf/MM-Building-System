# Mad Max Building System

An in-process Rust modding runtime and construction-system research project for Mad Max.

## Current focus

The project has a working DX11 ASI overlay prototype. A direct player-transform probe was attempted and caused Mad Max to crash during startup, before any position sample was logged. The known-good installed ASI is backed up. The callback-only `UpdatePostSim` diagnostic built as x64 and was confirmed in Mad Max. The user observed its counter advancing in the main menu and in-game, and stopping during loading screens.

See `docs/research/` for evidence and `docs/superpowers/` for the design and plans. Treat game addresses, callback ABI, vtable layout, object lifetime, and callback thread as unverified until supported by evidence for the exact game build.

## Bevy runtime direction

The Bevy bridge uses two lightweight, headless Bevy apps. Mad Max's `UpdatePostSim` callback owns the game world and manually runs `MadMaxPostSim`; the DX11 render callback owns the UI world and manually runs `MadMaxRender`. Each world stays on its callback thread. The bridge does not call `App::update()`, create a Bevy renderer, or run physics/2D/3D plugins.

The ECS model is deliberately split by meaning:

- Resources hold current-frame facts and runtime/UI policy (`CurrentGameFrame`, diagnostics, preferences).
- `CurrentGameFrame` includes host callback delta/elapsed time plus a separate native sample sequence and age. The hook runs each callback, while native reads are throttled to about 50 ms; Bevy's `Time` plugin is intentionally absent because the host owns schedule timing.
- Player and vehicle proxy entities carry copied frame components for normal ECS queries. They are not engine handles and do not promise persistent identity.
- Effects are intent entities. For example, a compiled-in system can use `Commands` to spawn `player_vehicle_velocity_intent([x, y, z])`. The native adapter consumes at most one per game callback, re-resolves the player's current vehicle, validates the payload, invokes the candidate SDK binding, then records its outcome.
- The UI and game worlds have separate entity IDs. A small bounded nonblocking transport copies owned intent data between them; it never transfers Bevy entities or game pointers.

First-party Rust systems are registered in `src/modding.rs`. A game system belongs in `MadMaxPostSimSet::ModSystems`, for example:

```rust,ignore
fn example_system(mut commands: Commands, frame: Res<CurrentGameFrame>) {
    if let Some(velocity) = frame.0.vehicle_velocity {
        commands.spawn(player_vehicle_velocity_intent(velocity));
    }
}

app.add_systems(
    MadMaxPostSim,
    example_system.in_set(MadMaxPostSimSet::ModSystems),
);
```

This is an in-crate authoring seam, not yet a public Rust ABI for separately compiled mods. Bevy's Rust types do not define a stable cross-version binary interface. The initial setter uses an SDK-derived candidate vtable slot and has not been validated against the live game binary; its one-shot identity echo still needs user-controlled in-game observation. Do not launch Mad Max as part of build verification.

## Build target

The Rust library is a Windows x64 `cdylib` intended to be loaded as an ASI through the existing DirectX 11 overlay path. Generated files under `target/` and binaries are excluded from Git.


## Managed Rhai scripts and console

The source includes an API v1 Rhai runtime and an ImGui Dev Tools window. Broad mod-owned keyboard/mouse sampling remains rolled back after startup crashes; the narrower bind path, which checks only keys with configured binds, has been verified by the user. Building the DLL does not install it or copy example scripts.

Use **F1** to toggle keyboard, mouse, and raw-input capture; **F2** toggles the UI configurator; **F3** toggles Dev Tools. The single resizable Dev Tools window has **Console** and **Scripts** tabs. Opening the window captures input and displays the cursor. **Escape** closes it and returns input to the game. While capture is released, overlay windows do not accept input. The configurator controls background opacity (0.25–1.0; default 0.96). Native hover suppression uses the existing game-focus bridge and input-message filter; its behavior still needs in-game verification. Float metrics display three decimals, truncated toward zero and padded. Console keeps its prompt, Up/Down command history, and scrollable output. Scripts scans package folders on first open and on Rescan; it displays manifest metadata, validation issues, and current loaded generation/status. Discovery parses bounded manifests only: it does not compile or execute `main.rhai`. Select a valid package and press **Load** to opt in. Reload, Run, and Unload are explicit actions. Source editing happens in your normal IDE.

Create this directory beside `MadMax.exe` and copy the example package into it:

```text
MadMaxModding/
  scripts/
    vehicle_watch/
      manifest.toml
      main.rhai
```

The source example is in [`examples/scripts/vehicle_watch`](examples/scripts/vehicle_watch). The Scripts tab's first-open scan and **Rescan** button discover direct child package directories under `MadMaxModding/scripts` through the loader directly; they do not submit or print a console command. `script list` remains available as the console equivalent. The scan has bounded directory/package/manifest limits, validates manifest metadata on the loader worker, and confines package files to the configured root. It can show malformed folders/manifests as invalid rows without hiding valid neighbors. A missing root is shown as empty and can be rescanned after creating it. Package IDs must contain 1–64 lowercase ASCII letters, digits, underscores, or hyphens; Windows device names such as `con`, `nul`, `com1`, and `lpt1` are rejected. The directory, requested ID, and manifest ID must match.

The complete v1 manifest schema is:

```toml
id = "vehicle_watch"
api_version = 1
capabilities = ["game.read", "log"]

[callbacks]
"game.post_sim" = "on_post_sim"
on_run = "on_run"
```

Unknown fields, capabilities, phases, and callback function names are rejected. Declare one or both callbacks, each as a one-argument function in `main.rhai`. `game.read` permits access to copied player/vehicle data; `log` permits `ctx.log.info(text)`; `game.vehicle.write` permits the bounded vehicle setter described below (also requiring `game.read`). Undeclared capability access raises an error. `ctx.frame` is always available. Scripts have no arbitrary FFI, filesystem, process, network, or module-import API.

The console accepts exactly these commands (replace `<id>` with a package ID):

```text
help
clear
show_keys
show_binds
bind <key> "script run <id>"
unbind <key>
script list
script load <id>
script reload <id>
script run <id>
script unload <id>
script status <id>
```

For example, run `script load vehicle_watch`, wait for its active-generation message, then use `script run vehicle_watch`. It logs copied player position and vehicle velocity/acceleration about once per callback-clock second, represents missing fields as `unavailable`, and retains up to eight samples in script-owned maps/arrays. `script run` logs the retained count and object-data availability once.

`clear` clears console scrollback locally. `show_keys` lists bindable keys; `show_binds` lists bindings. `bind <key> "<console command>"` dispatches the quoted command through the same parser as typed console input when that bound key is pressed. For example, `bind g "script run vehicle_jump"` requests a loaded script's `on_run` callback on the next eligible game callback. `unbind <key>` removes the binding. F1, F2, and F3 are reserved for the mod UI. The script API reference is generated from the runtime catalog on each Rust build as `API.md`; deployment places it at `MadMaxModding\API.md`, beside the `scripts` directory. Console output renders ANSI SGR reset (`0`), bold (`1`), foreground colors (`30`–`37`), and bright foreground colors (`90`–`97`). Console input is limited to 256 UTF-8 bytes, history to 64 commands, and retained output to 256 entries / 128 KiB. Queues are bounded and nonblocking; overloads are reported, but individual events/results may be missing. Use `script list` (console) or Scripts → Rescan (direct UI discovery) to refresh discovered packages. When global input capture is off, Mad Max may also process a bound key; use F1 to toggle global capture when you want the mod UI to own keyboard and mouse input.

In the Console tab, click a `CMD:`/`SCRIPT:` category header or an output row to select it; **Ctrl+C** copies that whole row without ANSI escape codes. The command prompt supports normal **Ctrl+C**, **Ctrl+X**, and **Ctrl+V** text editing through the Windows Unicode clipboard.

Console lines wrap by default. Turn off **Wrap long lines** to use horizontal scrolling for wide output.

### Callback and reload semantics

`on_post_sim(ctx)` runs at the verified `game.post_sim` boundary after the host imports the current copied snapshot. `on_run(ctx)` requires `script run <id>` and runs once at the next eligible boundary, before all recurring callbacks. Callbacks are ordered by script ID; multiple accepted run requests each invoke one callback. Commands and prepared packages are processed at game callbacks, so loading/paused host callbacks can delay execution and status replies.

`ctx.frame` exposes `phase`, `sequence`, `delta_seconds`, `elapsed_seconds`, and `sample_age_seconds`. The player and vehicle expose `available`, `position`, `rotation`, `velocity`, and `acceleration`; vehicle also exposes `angular_velocity`. Vectors are maps `#{x: float, y: float, z: float}`. Missing values are Rhai unit `()`. `available` means some copied object data exists; check each field independently. Snapshots can be older than the callback: inspect `sample_age_seconds` for freshness rather than assuming reads were sampled this frame.

Scripts declaring the `input.read` capability can query the mod runtime's sampled keyboard/mouse-button state with `ctx.input.down("space")`, `ctx.input.just_pressed("space")`, and `ctx.input.just_released("space")`. Input is sampled from the existing Hudhook/ImGui event stream and copied into the game callback; this is not a Mad Max keymap or controller-input API.

The host supplies an initially empty global `state` map for each script generation. It persists across that generation's callbacks and may contain bounded maps/arrays, strings, numeric/boolean values, characters, or `()`. Do initialization inside a callback: top-level script statements are compiled but never executed. Context/log handles cannot be retained in `state`; copied field values can. Setter handles also expire when their invocation ends. Callback return values are ignored.

`load` rejects an already loaded ID. `reload` explicitly reads and compiles the package on a worker; saving a file alone does not reload it. A failed read, compile, or manifest validation preserves the old generation. Successful activation replaces callbacks, resets scope/state, assigns a new generation, and cancels pending one-shots from the replaced generation. A callback runtime error disables that generation and clears its state; a valid reload is needed to resume it. Errors include script/generation/callback and source position when available.

`unload` removes future callbacks, releases the generation's state, and cancels its pending preparations/one-shots. It cannot interrupt a callback already executing or undo effects already applied. Unloading, replacing, or disabling a generation cancels its pending vehicle effects before native dispatch; velocity already applied is not reversed.

### One-shot vehicle jump

The [`vehicle_jump`](examples/scripts/vehicle_jump) package has only `on_run`; it adds **+18.000 units/s to Y** (world up) while preserving the copied X/Z velocity. It runs in the game callback through the existing `CVehicle::SetVelocity` adapter. Enter a vehicle, then use:

```text
script load vehicle_jump
script run vehicle_jump
```

Wait for the active-generation message before running. Each `run` requests one jump; there is no Space binding yet. Without copied current-vehicle velocity, it logs an unavailable message and requests no write. `vehicle_watch` is the observational example: it logs copied position/velocity/acceleration about once per second and retains eight samples; it never jumps or changes the game.

`ctx.vehicle.set_velocity(#{x: number, y: number, z: number})` requires `game.read` and `game.vehicle.write`. Every component must be finite and within **[-250.000, 250.000] units/s**. Exactly one successful request is allowed per invocation, with at most 32 accepted effects per game callback and 32 outstanding script velocity intents. Malformed, unavailable, excess, or unauthorized requests raise script errors; uncaught errors disable that generation. Effects are published only if the callback and retained-state validation both succeed. The console reports queueing and the later native result (`Applied`, `NoCurrentVehicle`, `SetterUnavailable`, or another adapter rejection). `on_run completed` means the callback completed, not that native application succeeded.

The native adapter resolves whichever vehicle Max occupies when the queued intent is consumed; it does not retain an object pointer. Multiple queued writes are absolute velocities calculated from the copied snapshot, not cumulative impulses, and one native intent is consumed per callback. Loading, callback availability, or earlier intents may delay application. Unload/reload cancels effects that have not yet been consumed. Runtime jump behavior remains to be verified in-game.

### Runtime limits

Rhai is pinned to 1.27.0 with `std`, `sync`, and `no_module`; `unchecked` is disabled. Current finite limits are defined in `src/scripting/engine.rs`, `manager.rs`, and `package.rs`:

| Limit | Value |
| --- | --- |
| Operations per callback | 20,000 |
| Call levels | 16 |
| Expression depth (global / function) | 32 / 32 |
| Variables / functions | 128 / 64 |
| String bytes / array items / map items | 4,096 / 256 / 128 |
| Retained state values / nesting depth | 1,024 / 16 |
| Source / manifest bytes | 64 KiB / 8 KiB |
| Active scripts / callbacks per package | 16 / 2 |
| Queued one-shots per phase | 16 |
| Velocity writes per invocation / callback / outstanding | 1 / 32 / 32 |
| Velocity component absolute limit | 250.000 units/s |
| Log calls per invocation / event text bytes | 8 / 1,024 |

The runtime reports execution/size-limit errors through the console; excess log calls are suppressed. These limits bound script work and data, not a hard wall-clock execution deadline. In-game verification remains pending: load/run/reload/unload the example, check a broken reload preserves its prior generation, and verify console input returns to the game when closed.
