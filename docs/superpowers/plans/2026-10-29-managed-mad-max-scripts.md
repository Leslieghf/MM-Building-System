# Managed Mad Max Scripts and Runtime Console Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an IDE-authored, hot-loadable Rhai scripting runtime with managed callbacks in verified Mad Max phases and an in-game console for script lifecycle and diagnostics.

**Architecture:** Rust owns the Mad Max script API, phase contexts, callback registry, loader, and native adapters. A dispatcher in the existing Bevy game schedule invokes Rhai callbacks with copied, phase-scoped values; Bevy remains internal. The ImGui UI sends bounded console requests to the game callback and displays bounded results.

**Tech Stack:** Rust 2024, `bevy_app`/`bevy_ecs` 0.19.1, Hudhook ImGui 0.12, Rhai `=1.27.0` with finite limits, `std`, `sync`, and `no_module` enabled and `unchecked` disabled; Serde/TOML for package metadata.

**Spec:** `docs/superpowers/specs/2026-10-29-embedded-scripting-console-design.md`

## Global Constraints

- Mad Max game and UI callbacks keep separate thread-local Bevy worlds; only bounded owned DTOs cross between them.
- Scripts interact through stable Mad Max phase contexts and registered host capabilities, never Bevy `World`/`Entity`/`SystemParam` or native game pointers.
- Only verified host callback phases may be exposed; v1 game phase is `game.post_sim`, with UI render read-only and separate.
- Rhai uses finite operation, call-depth, expression-depth, variable, string, array, and map limits; do not enable `unchecked`.
- Script files are loaded only from one configured root; Rhai itself has no filesystem, network, process, or arbitrary FFI access.
- Callback work, package size, queue depth, active scripts, output, and scrollback are bounded; neither host callback waits for queue capacity.
- Mad Max stays open through source implementation and build; do not install or deploy the new ASI until the user closes the game.
- Do not add or run automated tests in this plan; use source review, formatting/build checks, and user-controlled in-game verification after deployment.

## Review Focus

- **Invalid package ID or path traversal:** loader rejects it and preserves active scripts. Manual check: request a name containing `..` and an absolute path outside the root; verify a visible rejection.
- **Broken reload:** a syntax or API-version error must leave the prior generation active. Manual check: load a valid script, break its file, reload, and confirm the old callback continues with the previous generation.
- **Missing/stale context values:** script sees an explicit unavailable result rather than fabricated data or a panic. Manual check: run at menu/loading and in a state with no current vehicle.
- **Runaway script:** Rhai's operation/depth/data limits stop execution and identify script/generation/callback. Manual check: execute an infinite loop and recursive/large-map examples; confirm the callback recovers and the game remains responsive.
- **Queue saturation or slow package preparation:** UI/game callbacks remain nonblocking and the console reports overload or pending status. Manual check: submit requests rapidly and prepare a deliberately large but within-limit source file.

---

### Task 1: Add the Rhai engine, Mad Max contexts, and game-phase dispatcher

**Files:**
- Create: `src/scripting/mod.rs`
- Create: `src/scripting/engine.rs`
- Create: `src/scripting/api.rs`
- Create: `src/scripting/manager.rs`
- Create: `src/scripting/dispatch.rs`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `src/lib.rs`
- Modify: `src/modding.rs`
- Modify: `src/runtime.rs`

**Interfaces:**
- `ScriptId(String)`, `ScriptGeneration(u64)`, `ScriptPhaseId`, `ScriptApiDescriptor`, `ScriptStatus`, and bounded `ScriptEvent` DTOs are owned types in `src/scripting/mod.rs`.
- `RhaiEngineFactory::build() -> rhai::Engine` registers only the v1 host API and sets all finite Rhai limits.
- `ScriptApiRegistry::v1() -> ScriptApiRegistry` describes host callback IDs and phase-specific exposed values/methods.
- `ScriptManager` is a Bevy `Resource` holding active AST/scope/state and dynamic per-phase callback registrations; it exposes `stage_prepared`, `stage_unload`, `stage_run`, `apply_pending_changes`, and `list` operations.
- `dispatch_game_post_sim(In(Res<CurrentGameFrame>), ResMut<ScriptManager>)` invokes recurring `on_post_sim(ctx)` callbacks and scheduled `on_run(ctx)` requests in stable script-ID order. `ctx` is built from copied values and an output sink valid for that invocation.

- [ ] Add `rhai = { version = "=1.27.0", default-features = false, features = ["std", "sync", "no_module"] }`; do not enable `unchecked`. Make the engine factory the sole place that constructs and configures an Engine.
- [ ] Define v1 semantic API IDs and the Rhai representation for `ctx.frame`, read-only copied `ctx.player`/`ctx.vehicle`, `ctx.log`, and script-owned `state` maps. Return explicit unavailable values.
- [ ] Configure finite operation, call-level, expression-depth, variable, string, array, and map limits with named constants in `engine.rs`.
- [ ] Add `ScriptManager` to the game Bevy app and register `dispatch_game_post_sim` in `MadMaxPostSimSet::ModSystems`; do not expose Bevy params to Rhai.
- [ ] Implement in-memory candidate installation and unload in the manager so the dispatcher can be integrated before the file loader exists.
- [ ] Route bounded script log/status events through an owned nonblocking channel; do not perform file I/O from the game callback.
- [ ] Run `cargo fmt --all -- --check` and `cargo build --release --locked`; review that the two app worlds remain thread-local.

**Manual verification:** The eventual example script must run in `game.post_sim`, preserve its map state across callbacks, write a bounded log result, and handle missing player/vehicle data. Deployment/observation waits until the user closes Mad Max.

### Task 2: Add package metadata, root-confined loading, and managed generations

**Files:**
- Create: `src/scripting/package.rs`
- Create: `src/scripting/loader.rs`
- Modify: `src/scripting/mod.rs`
- Modify: `src/scripting/engine.rs`
- Modify: `src/scripting/manager.rs`
- Modify: `src/runtime.rs`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`

**Interfaces:**
- `ScriptManifest { id: String, api_version: u32, callbacks: BTreeMap<String, String>, capabilities: Vec<String> }`, deserialized with unknown fields denied.
- Package layout: `<game-directory>/MadMaxModding/scripts/<script-id>/manifest.toml` and `main.rhai`.
- `LoadScriptRequest { request_id: u64, id: ScriptId, replace: bool }`, `PreparedScript { request_id: u64, manifest: ScriptManifest, ast: rhai::AST }`, and `ScriptLoadFailure { request_id: u64, id: ScriptId, message: String }` cross only bounded loader/game queues.
- `ScriptLoader::start(root: PathBuf, engine_factory: RhaiEngineFactory) -> ScriptLoader` owns a file-read/parse/compile worker; it never accesses Bevy or Mad Max.

- [ ] Add direct Serde/TOML dependencies and deserialize a strict manifest with only `id`, `api_version`, `callbacks`, and `capabilities`.
- [ ] Resolve the script root from the game executable directory, canonicalize the root and package paths, validate script IDs as safe path segments, and reject any path that escapes the root.
- [ ] Bound manifest and source bytes before parsing/compiling; validate API version, callback IDs, capability IDs, and callback function names against `ScriptApiRegistry`.
- [ ] Create a dedicated loader worker. It reads and compiles source off the game callback thread, returns a prepared AST or bounded error, and never evaluates scripts.
- [ ] Apply prepared candidates only before `MadMaxPostSim` runs. If read/parse/API validation fails, retain the active generation; successful reload replaces callback registration and scope/state, increments generation, and cancels the previous generation's pending intents.
- [ ] Implement unload and `on_run(ctx)` behavior: run once at the next `game.post_sim` boundary after snapshot import and before recurring callbacks; if absent, return a visible error.
- [ ] Cap package size, active scripts, callbacks per script, and loader request/result queue depth; make full queues return visible overload events.
- [ ] Run formatting and locked release build; inspect that no file I/O or worker wait occurs inside either host callback.

**Manual verification:** Load a valid package, reload a modified package, unload it, and run its `on_run`; confirm generation/status messages. Then make a syntax/API-version error and verify the previous generation keeps running. Check root traversal rejection.

### Task 3: Implement the console command API and bounded cross-thread transport

**Files:**
- Create: `src/scripting/console.rs`
- Modify: `src/scripting/mod.rs`
- Modify: `src/runtime.rs`
- Modify: `src/overlay.rs`

**Interfaces:**
- `ConsoleCommand`: `Help`, `ApiList { phase: Option<ScriptPhaseId> }`, `ScriptList`, `ScriptLoad(ScriptId)`, `ScriptReload(ScriptId)`, `ScriptRun(ScriptId)`, `ScriptUnload(ScriptId)`, and `ScriptStatus(ScriptId)`.
- `ConsoleRequest { request_id: u64, command: ConsoleCommand }` and `ConsoleEvent { request_id: Option<u64>, severity: ConsoleSeverity, script: Option<ScriptId>, generation: Option<ScriptGeneration>, phase: Option<ScriptPhaseId>, source_position: Option<SourcePosition>, text: String }` are owned bounded DTOs.
- Extend `GameEndpoint`/`UiEndpoint` with nonblocking request/event channels; no Rhai Engine, AST scope, or ECS entity crosses the channels.

- [ ] Parse console input into `ConsoleCommand`; reject unknown commands, extra arguments, malformed IDs, and unknown phases with clear console output.
- [ ] Send load/reload requests to the loader worker and prepared packages to the game callback; send run/unload/list/status commands to the game callback request channel.
- [ ] Implement `api list [phase]` from Rust-owned `ScriptApiRegistry` metadata, including signatures, availability, phase, read/write effect, and API version.
- [ ] Keep events, output messages, and command results bounded; use `try_send`/`try_recv`, display queue overloads, and associate asynchronous results by request ID.
- [ ] Do not expose capture start/stop or game mutations unless a real host adapter is present; list only implemented APIs.
- [ ] Run formatting and locked release build; review queue-full/disconnected behavior for both callback directions.

**Manual verification:** Submit every valid command, malformed command, and unknown ID; verify async package statuses map to the original request and scrollback remains capped.

### Task 4: Build the in-game ImGui console and input lifecycle

**Files:**
- Modify: `src/runtime.rs`
- Modify: `src/overlay.rs`
- Modify: `src/game_probe.rs` only if input capture requires a distinct console mode identifier.

**Interfaces:**
- Add `OverlayMode::ScriptConsole` with `owns_game_input() == true`.
- Add `ScriptConsoleUiState` with bounded input, command history cursor, scrollback view state, and visible script selection/status.
- ImGui uses only `ConsoleRequest`/`ConsoleEvent`; it does not read script files or call Mad Max APIs.

- [ ] Add a compact blue-grey runtime panel showing API/runtime status, loaded scripts and generations, command prompt, and scrollable output/errors; do not add an embedded code editor.
- [ ] Keep Ctrl+Shift+F6 for the debug-section picker; use Ctrl+Shift+F7 to open the script console unless inspection reveals a collision.
- [ ] While open, capture keyboard/mouse through existing `MessageFilter::InputAll` and show the cursor; Escape closes and returns input to Mad Max.
- [ ] Preserve current input/history while toggling; cap command history and visible scrollback according to named limits.
- [ ] Run formatting and locked release build; inspect the console/picker state transitions and focus behavior.

**Manual verification:** Open the console during gameplay, submit commands, use history, scroll events, close with Escape, and verify keyboard/mouse return to the game. Confirm Ctrl+Shift+F6 still opens the section picker.

### Task 5: Add an example package, user documentation, and release handoff

**Files:**
- Create: `examples/scripts/vehicle_watch/manifest.toml`
- Create: `examples/scripts/vehicle_watch/main.rhai`
- Modify: `README.md`
- Modify: `docs/bootstrap-install-record.md` only if deployment instructions change.

**Interfaces:**
- Example API v1 callbacks: recurring `on_post_sim(ctx)` and optional one-shot `on_run(ctx)`.
- Document exactly implemented console commands: `help`, `api list`, and `script list/load/reload/run/unload/status`.

- [ ] Add one example that reads copied player/vehicle values when available, maintains bounded script-owned map/array state, and logs through `ctx.log`.
- [ ] Document script root/package fields, callback lifetime, `on_run` ordering, state reset on reload, unload behavior, Rhai limits, and effects that unload cannot undo.
- [ ] Run `cargo fmt --all -- --check` and `cargo build --release --locked`; record artifact path/hash for later user-controlled installation.
- [ ] Review the final diff and ensure pre-existing unrelated changes are neither overwritten nor staged.
- [ ] Stop before installation if Mad Max is open; request a close only after the build and docs are reviewable.

**Manual verification:** After the user closes the game and chooses to install, load the example, confirm callback output and preserved state, reload and confirm generation/state reset, break source and confirm old generation remains, run `on_run` once, unload and confirm recurring output stops, then close the console and verify input returns to the game.

