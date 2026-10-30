# Using Rhai scripts

Rhai is the supported user-authored extension format. Packages live beside `MadMax.exe` under `MadMaxModding/scripts/`.

## Quick start

Build the ASI using [Development](development.md), then copy a package such as [`vehicle_watch`](../examples/scripts/vehicle_watch) into the scripts folder:

```text
MadMaxModding/scripts/vehicle_watch/
  manifest.toml
  main.rhai
```

Open Dev Tools with **F3**, choose **Scripts → Rescan** if the package was added after startup, then select it and choose **Load**.

## Package contract

```toml
id = "vehicle_watch"
api_version = 1
capabilities = ["game.read", "log"]

[callbacks]
"game.post_sim" = "on_post_sim"
on_run = "on_run"
```

The package directory, requested ID, and manifest ID must match. IDs contain lowercase ASCII letters, digits, `_`, or `-`. Callback functions accept one context argument. Top-level statements do not run. `state` persists for one loaded generation and resets after a successful reload.

## Console commands

```text
help
script list
script load vehicle_watch
script run vehicle_watch
script reload vehicle_watch
script unload vehicle_watch
```

`script list` discovers manifests without compiling or executing scripts. Load and reload compile explicitly. A failed reload leaves the active generation running; a callback error disables that generation.

## Capabilities

API v1 capabilities are `game.read`, `log`, `input.read`, `game.vehicle.write`, and `debug.draw`. Frame metadata and pure `ctx.math` helpers need no capability.

## Errors

Undeclared capability access and invalid API arguments raise script errors. Invalid manifests are shown as invalid packages. A callback error disables its active generation; a failed reload leaves the current generation running.

## Safety and limits

Scripts receive copied snapshots. They cannot access arbitrary FFI, files, processes, networks, or modules. Vehicle writes and debug drawing are validated, bounded requests handled by the game callback. Native vehicle-write and renderer behavior remains experimental pending in-game verification. Exact signatures, values, coordinate conventions, and limits are in the generated [Rhai API v1 reference](rhai-api.md).
