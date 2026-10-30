# Using Rhai scripts

Rhai packages are the supported user-authored extension format. A package is a directory under `MadMaxModding/scripts` beside `MadMax.exe`:

```text
MadMaxModding/
  scripts/
    vehicle_watch/
      manifest.toml
      main.rhai
```

The repository includes [`vehicle_watch`](../examples/scripts/vehicle_watch) and [`vehicle_jump`](../examples/scripts/vehicle_jump). Copy a package directory into the game-side scripts folder; building the ASI does not install scripts.

## Manifest and callbacks

```toml
id = "vehicle_watch"
api_version = 1
capabilities = ["game.read", "log"]

[callbacks]
"game.post_sim" = "on_post_sim"
on_run = "on_run"
```

The directory, requested ID, and manifest ID must match. IDs use lowercase ASCII letters, digits, `_`, and `-`. `main.rhai` defines one-argument callback functions; package top-level statements are not run. Each script generation has its own persistent `state` map.

In Mad Max, press **F3** to open Dev Tools and use **Scripts → Rescan** if the package was copied while the game was open. Select a valid package and choose **Load**. Use the Console for commands such as:

```text
help
script list
script load vehicle_watch
script run vehicle_watch
script reload vehicle_watch
script unload vehicle_watch
```

Loading, running, reloading, and unloading are explicit actions. Discovery validates bounded package metadata; it does not compile or execute a script until Load or Reload is requested. A failed reload preserves the active generation. A callback error disables that generation; a successful reload creates a fresh generation and state.

## Capabilities and safety

API v1 capabilities are `game.read`, `log`, `input.read`, `game.vehicle.write`, and `debug.draw`. A package must declare the capabilities it uses. `ctx.frame` and pure `ctx.math` helpers are always available. Game data comes from copied snapshots, not live pointers. Vehicle writes and debug drawing are validated, bounded effects processed by the game callback. Scripts have no arbitrary FFI, filesystem, process, network, or module-import access.

For exact values, signatures, coordinate conventions, limits, and experimental methods, see the generated [Rhai API v1 reference](rhai-api.md). The `vehicle_watch` example only logs observations; `vehicle_jump` demonstrates a one-shot velocity request and should be used only with awareness that native behavior awaits further in-game verification.
