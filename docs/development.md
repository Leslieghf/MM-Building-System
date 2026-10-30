# Development

## Build and verify

The project produces a Windows x64 `cdylib` for an ASI loader. With the MSVC Rust target installed, run:

```powershell
cargo fmt --all -- --check
cargo check --target x86_64-pc-windows-msvc
cargo build --target x86_64-pc-windows-msvc --release
cargo doc --no-deps --target x86_64-pc-windows-msvc
```

The artifact is under `target/x86_64-pc-windows-msvc/release/`. Deployment and game installation are manual. These commands compile and document Rust code; they do not validate native addresses or behavior in a live game.

## Implementation map

| Path | Change here when |
| --- | --- |
| `src/kernel` | ASI initialization, startup composition, or operational logging changes. |
| `src/api/game/abi` | Supported build identity or native layout/call metadata changes; record evidence in research notes. |
| `src/api/game/adapter` | Guarded reads, callback hooks, telemetry capture, or native effect application changes. |
| `src/api/ecs` | Owned frame contracts, schedule ordering, typed intents, diagnostics, or queue bounds change. |
| `src/api/scripts` | Package loading, validation, console protocol, generation lifecycle, or script limits change. |
| `src/api/rhai` | Script-visible names, values, capabilities, argument validation, or callback behavior change. |
| `src/devtools` | Input translation or DX11/ImGui presentation changes. |
| `examples/scripts` | User-facing package examples change; keep each manifest and callback contract valid. |

## Change rules

- Keep native pointers and unsafe calls inside the adapter. Copy data into owned snapshots before crossing into ECS, scripts, or UI.
- Keep new cross-boundary work typed, validated, and bounded. Document queue-full and unavailable-data behavior.
- Treat any script-visible name, value, capability, or semantic change as an API contract change. Update `api::rhai::catalog` and regenerate `docs/rhai-api.md` by building the crate.
- Mark reverse-engineered claims as observed, inferred, or unverified. Do not turn research candidates into supported behavior without validation.
- Keep docs about outward behavior and integration contracts. Internal algorithm commentary belongs in code only when it explains a safety constraint a maintainer must preserve.

## Documentation map

The root [README](../README.md) routes readers to the [architecture](architecture.md), [Rhai package guide](scripting.md), generated [Rhai API reference](rhai-api.md), and [Rust integration guide](rust-api.md). Research notes under `docs/research/` preserve dated evidence and are not current setup instructions.
