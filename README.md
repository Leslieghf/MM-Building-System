# Mad Max modloader and scripting API

This repository builds a Windows x64 ASI modloader foundation for Mad Max. It supplies a DX11 developer overlay, a constrained Rhai scripting API, and Rust-side integration points for building code-library mods. It is not a content mod: the examples demonstrate API use and do not add permanent world content.

## Try the script examples

1. Build the ASI as described in [Development](docs/development.md), then load it with a compatible ASI loader for the supported game executable.
2. Beside `MadMax.exe`, create `MadMaxModding/scripts/`.
3. Copy a package such as [`vehicle_watch`](examples/scripts/vehicle_watch) into that directory.
4. Start Mad Max and open Dev Tools with **F3**. In **Scripts**, select the package and choose **Load**. In **Console**, `script run vehicle_watch` invokes its `on_run` callback.

Package layout, capabilities, callback behavior, and limits are in [Using Rhai scripts](docs/scripting.md). The generated, complete method reference is [Rhai API v1](docs/rhai-api.md).

## What the library provides

- A build-checked adapter for one known Mad Max executable build.
- Owned game snapshots and bounded, typed effects for Rust systems and Rhai callbacks.
- Headless Bevy schedules that run on host callbacks without creating a renderer or taking ownership of game entities.
- A constrained Rhai v1 API and managed package lifecycle.
- A DX11 developer overlay for script management, console output, and diagnostics.

The current Rust system-registration seam is in-process only; it is not a stable ABI for separately compiled Rust mods. Rhai packages are the supported user-authored extension format. This project does not expose arbitrary native calls, filesystem, process, or network access to scripts.

## Project guide

- [Architecture](docs/architecture.md): runtime flow and module responsibilities.
- [Using Rhai scripts](docs/scripting.md): install location, package format, callbacks, and common commands.
- [Development](docs/development.md): build, verification, and contribution boundaries.
- [Research notes](docs/research/README.md): dated evidence and investigative history, not an onboarding guide.

Game addresses and object layouts are build-specific; several native capabilities remain experimental or await in-game verification. See [Architecture](docs/architecture.md) for the safety boundary.
