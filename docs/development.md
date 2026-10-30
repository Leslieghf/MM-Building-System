# Development

## Build

This repository builds a Windows x64 `cdylib` for an ASI loader. With the MSVC Rust target installed:

```powershell
cargo fmt --all -- --check
cargo check --target x86_64-pc-windows-msvc
cargo build --target x86_64-pc-windows-msvc --release
```

The library artifact is under `target/x86_64-pc-windows-msvc/release/`. Copying or renaming it for an ASI loader is a separate deployment step; builds do not modify a game installation. Never use a build as evidence that a game hook or address candidate works in the live executable.

## Change boundaries

- Keep build addresses and executable fingerprints authoritative in `api::game::abi`.
- Keep raw pointers and guarded calls inside `api::game`; pass owned telemetry and typed effects across its boundary.
- Keep schedule, queue, and request limits explicit in `api::ecs` and `api::scripts`.
- Treat Rhai API changes as versioned contract changes; update the catalog in `api::rhai` and its generated reference together.
- Keep kernel composition narrow and UI presentation inside `devtools`.
- Do not add game content or expose arbitrary native access through the script API.

## Documentation map

`README.md` is the entry point; `architecture.md` explains the implementation, `scripting.md` is the package onboarding guide, and `rhai-api.md` is generated from the runtime API catalog. Dated reverse-engineering notes remain in `docs/research/` as evidence and historical context; they are not current usage instructions. The documentation refresh plan and decisions are in `docs/maintainers/`.
