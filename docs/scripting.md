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

## Package manifest

```toml
id = "vehicle_watch"
api_version = 1
capabilities = ["game.read", "log"]

[callbacks]
"game.post_sim" = "on_post_sim"
on_run = "on_run"
```

The package directory, requested ID, and manifest ID must match. IDs contain lowercase ASCII letters, digits, `_`, or `-`. Callback functions accept one context argument. Top-level statements do not run. `state` persists for one loaded generation and resets after a successful reload.

## Callback behavior

`game.post_sim` runs after the game simulation callback. `on_run` is invoked by `script run <id>` through the same callback context. Both receive one `MadMaxPostSimContext`; return values are ignored. A callback error disables that script generation. A failed reload leaves the current generation active.

## Console commands

```text
help
script list
script load vehicle_watch
script run vehicle_watch
script reload vehicle_watch
script unload vehicle_watch
```

`script list` discovers manifests without compiling or executing scripts. Load and reload compile explicitly.

## Capabilities

API v1 capabilities are `game.read`, `log`, `input.read`, `game.vehicle.write`, and `debug.draw`. Frame metadata and pure `ctx.math` helpers need no capability.

## Errors and recovery

Undeclared capability access and invalid API arguments raise script errors. Invalid manifests remain visible as invalid packages. Fix the manifest or callback and reload; a failed reload keeps the last active generation running.

## Safety and limits

Scripts receive copied snapshots. They cannot access arbitrary FFI, files, processes, networks, or modules. Vehicle writes and debug drawing are validated, bounded requests handled by the game callback. Native vehicle-write and renderer behavior remains experimental pending in-game verification. See [Rhai API v1](rhai-api.md) for signatures, values, coordinate conventions, errors, and limits.
