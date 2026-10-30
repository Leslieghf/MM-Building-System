# Runtime source map

| Area | Owner |
|---|---|
| Supported game build, addresses, and sampling budgets | src/game_probe/config.rs |
| Game hook, owned telemetry, and read-only sampling | src/game_probe/mod.rs |
| DX11 overlay and UI behavior | src/overlay/mod.rs |
| UI and clipboard configuration | src/overlay/config.rs |
| Append-only diagnostics | src/logging.rs |
| Game type and address candidates | src/sdk/ |

The game probe publishes copied Rust telemetry to the runtime/UI boundary; raw game pointers remain in the native probe. Offsets and virtual slots are build-specific candidates and must be reviewed against the supported executable.
