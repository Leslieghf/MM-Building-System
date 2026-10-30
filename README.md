# Mad Max modloader and scripting API

This project is an in-process Windows x64 modloader foundation for one supported Mad Max executable build. It provides a managed Rhai scripting surface, an owned Rust/ECS integration layer, guarded access to game state, and a DX11 developer overlay. It is a code library and modding host: examples exercise APIs and do not add persistent game content.

## Start here

1. Build the ASI using [Development](docs/development.md).
2. Place the resulting library beside a compatible ASI loader and the supported `MadMax.exe` build. Installation is a separate manual step; builds do not modify the game directory.
3. Create `MadMaxModding/scripts/` beside `MadMax.exe`, then copy one directory from [`examples/scripts`](examples/scripts) into it.
4. Start the game, open Dev Tools with **F3**, open **Scripts**, rescan if needed, select the package, and load it. Use **Console** to run `script run <id>` for packages with `on_run`.

The package format, callback lifecycle, capabilities, console commands, and recovery behavior are in [Using Rhai scripts](docs/scripting.md). The complete generated callback reference is [Rhai API v1](docs/rhai-api.md).

## How the host is arranged

```text
MadMax.exe callbacks
    -> build-checked native adapter
    -> owned game snapshot
    -> Rust systems and Rhai callbacks
    -> bounded typed effects
    -> native adapter

Hudhook / ImGui -> input snapshot and developer-tools presentation
```

Native addresses, layouts, and calling conventions belong to `api::game::abi`; guarded pointer access and game calls belong to `api::game::adapter`. The rest of the runtime exchanges copied values and typed requests across that boundary. See [Architecture](docs/architecture.md) for ownership and phase ordering.

## Extension surfaces

- **Rhai packages** are the supported user-authored extension format. Scripts receive a versioned, capability-checked context and can request only documented bounded effects.
- **Rust/ECS systems** express in-process host behavior using owned frame data, Bevy resources/components, and named callback sets. They are source integration points, not a stable ABI or a separately loadable Rust plugin format.
- **The native adapter** is tied to a recorded executable fingerprint. A successful build does not validate a candidate address or native behavior in game.

Scripts have no general FFI, file, process, network, or dynamic module access. Some native vehicle-write and debug-render paths remain experimental. The repository’s [research notes](docs/research/README.md) are dated evidence, not supported usage instructions.

## Documentation

- [Architecture](docs/architecture.md) — runtime phases, module ownership, data flow, and native safety boundary.
- [Using Rhai scripts](docs/scripting.md) — install a package, define callbacks, request capabilities, and diagnose failures.
- [Rhai API v1](docs/rhai-api.md) — generated signatures, values, errors, and limits.
- [Rust integration](docs/rust-api.md) — ECS data contracts and in-process integration constraints.
- [Development](docs/development.md) — build, verify, and change the implementation safely.
