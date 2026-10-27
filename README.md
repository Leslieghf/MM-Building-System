# Mad Max Building System

An in-process Rust modding runtime and construction-system research project for Mad Max.

## Current focus

The project has a working DX11 ASI overlay prototype. A direct player-transform probe was attempted and caused Mad Max to crash during startup, before any position sample was logged. The known-good installed ASI is backed up. The callback-only `UpdatePostSim` diagnostic built as x64 and was confirmed in Mad Max. The user observed its counter advancing in the main menu and in-game, and stopping during loading screens.

See `docs/research/` for evidence and `docs/superpowers/` for the design and plans. Treat game addresses, callback ABI, vtable layout, object lifetime, and callback thread as unverified until supported by evidence for the exact game build.

## Build target

The Rust library is a Windows x64 `cdylib` intended to be loaded as an ASI through the existing DirectX 11 overlay path. Generated files under `target/` and binaries are excluded from Git.

