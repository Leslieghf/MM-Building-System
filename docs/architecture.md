# Architecture

The project is a Windows x64 ASI modloader and code library for one supported Mad Max executable build. Its public extension format is Rhai; the Rust system seam is in-process and is not a stable external ABI.

## Runtime flow

```text
MadMax.exe → game adapter → owned snapshot → ECS systems and Rhai callbacks
           ← validated effects ← typed intents ←
```

The adapter confines native pointers and ABI details. Mod systems, scripts, and developer tools exchange copied values through bounded channels. Unsupported executable fingerprints fail closed.

## Code ownership

| Path | Responsibility |
| --- | --- |
| `api::game::abi` | Supported-build identity, native addresses, layouts, and ABI declarations. |
| `api::game::adapter` | Hook lifecycle, guarded native access, telemetry sampling, and effect application. |
| `api::ecs` | Owned frame data, schedules, mod contracts, intents, outcomes, and callback transport. |
| `api::rhai` | Versioned script context, capability checks, conversions, helpers, and API catalog. |
| `api::scripts` | Package discovery, validation, compilation, lifecycle, console protocol, and reload tracking. |
| `kernel` | ASI entrypoint, startup composition, and operational logging. |
| `devtools` | DX11 overlay, input translation, console and script presentation. |

Each module owns the data and policy named by its responsibility. Game-specific addresses stay in the ABI layer; user-facing script semantics are defined by the Rhai catalog and generated API reference.

## Support boundary

Addresses and layouts apply only to the recorded executable build. Successful compilation does not validate native behavior. Velocity writes and debug-renderer calls remain experimental until verified in game. Builds do not install or launch the game.
