# Per-Script Automatic Reload Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let each loaded Rhai package automatically reload after stable edits to its `manifest.toml` or `main.rhai`, controlled by an inline toggle in the Scripts tab.

**Architecture:** Keep all filesystem work on the existing script-loader worker. The worker polls only enabled loaded package IDs every 500 ms, compares bounded content fingerprints, and emits one automatic reload request after two identical observations. `ScriptManager` owns per-loaded-script toggle state and continues using its existing prepare-then-activate flow; a failed prepare leaves the active generation running. The UI sends a typed toggle request and receives the accepted state and any reload error through the existing bounded console-event transport.

**Tech Stack:** Rust 2024, Bevy ECS resources, std bounded `mpsc` channels, Rhai, Dear ImGui, existing package loader and test helpers. No new dependency.

**Spec:** `docs/superpowers/specs/2026-10-30-per-script-auto-reload-design.md`

## Global Constraints

- Watch only loaded scripts whose session toggle is On; default all newly loaded scripts to Off.
- Poll at 500 ms on the loader worker; never read or hash files from render/game callbacks.
- Watch `manifest.toml` and `main.rhai` only, within existing package root and byte limits.
- Require the same fingerprint in two consecutive polls before automatic reload; detect same-size edits by hashing contents.
- Keep only one automatic preparation per script pending; coalesce later edits while it is pending.
- An unchanged failed fingerprint is reported once and is not retried until contents change.
- A failed reload leaves the prior active generation running; successful reload preserves Auto Reload state.
- A manual reload supersedes an automatic reload for the same script when both are pending.
- Missing watched files report once and remain watched so restoring the package resumes detection.
- Keep worker/UI channels bounded; show a single error in the script row and one console error event.
- Preserve the user's 43 existing dirty/untracked paths. Do not commit, stage, push, install, or launch Mad Max as part of this plan.

## Review Focus

- Atomic editor save or same-size content change: must become one reload after the stable debounce; test content fingerprints and two-poll debounce.
- Invalid syntax or manifest: old generation stays active and the same broken fingerprint does not spam retries; test manager and worker result handling.
- Manual reload during automatic preparation: manual request is authoritative and stale automatic results cannot activate; test request ordering and stale-result rejection.
- Toggle Off or unload while a watch notification is pending: no later automatic replacement is accepted; test manager/watch-state coordination.
- Package disappearance and restoration: report a stable missing state once and recover when files return; test missing-to-present transition.

---

### Task 1: Add package fingerprints and debounce state

**Files:**
- Create: `src/scripting/auto_reload.rs`
- Modify: `src/scripting/mod.rs`
- Test: `src/scripting/auto_reload.rs`

**Interfaces:**
- Produces `PackageFingerprint`, a value covering presence and content of `manifest.toml` and `main.rhai`.
- Produces `AutoReloadTracker::observe(id: &ScriptId, fingerprint: PackageFingerprint) -> bool`, which returns true only after a new fingerprint is observed identically in two consecutive polls and has not already been issued.
- Produces `AutoReloadTracker::set_enabled(id: ScriptId, enabled: bool)` and `remove(id: &ScriptId)`.
- File reads use the existing root-confinement and size-bounded helpers in `loader.rs`; factor those helpers into `pub(crate)` functions if needed rather than duplicating their policy.

- [ ] **Step 1: Write failing unit tests** for stable two-poll debounce, one notification per fingerprint, new-fingerprint re-arm, same-length edits, manifest-only edits, missing files, Off removal, and re-enable baseline.
- [ ] **Step 2: Run `cargo test --lib scripting::auto_reload::tests`** and confirm the new module/interface is missing.
- [ ] **Step 3: Implement fingerprint and tracker state** in `auto_reload.rs`, keeping path reads bounded and confined through existing loader helpers.
- [ ] **Step 4: Run `cargo test --lib scripting::auto_reload::tests`** and confirm all watcher-state tests pass.

### Task 2: Integrate polling with the loader worker

**Files:**
- Modify: `src/scripting/loader.rs`
- Modify: `src/scripting/auto_reload.rs`
- Test: `src/scripting/loader.rs`

**Interfaces:**
- `ScriptLoader::set_auto_reload(id: ScriptId, enabled: bool) -> Result<(), &'static str>` sends a bounded watch configuration update.
- `ScriptLoader::try_auto_reload() -> Option<LoadScriptRequest>` drains at most one automatic request per call; generated requests have `replace: true` and a unique request ID from the shared request-ID allocator.
- Add a typed `LoaderRequest::SetAutoReload { id, enabled }` and a bounded watcher notification channel.
- The loader worker uses `recv_timeout(Duration::from_millis(500))`; on timeout it fingerprints only enabled package IDs and returns to request handling promptly.

- [ ] **Step 1: Write failing worker tests** for enabling/disabling a watch, two-poll stable change notification, coalescing a second edit while one notice is pending, and full notification-queue recovery without losing the changed fingerprint.
- [ ] **Step 2: Run `cargo test --lib scripting::loader::tests`** and confirm the watcher control/notification API is absent.
- [ ] **Step 3: Add worker polling and bounded notifications**; the worker must continue serving normal load/discovery requests and must never sleep in game or UI callbacks.
- [ ] **Step 4: Add one shared monotonically increasing request-ID allocator** used by UI console submissions and automatic reload requests, preserving the manager's stale-request ordering contract.
- [ ] **Step 5: Run `cargo test --lib scripting::loader::tests`** and confirm worker tests pass.

### Task 3: Track toggle state and enforce reload precedence

**Files:**
- Modify: `src/scripting/manager.rs`
- Modify: `src/scripting/console.rs`
- Modify: `src/runtime.rs`
- Test: `src/scripting/manager.rs`
- Test: `src/runtime.rs`

**Interfaces:**
- Add UI-only `ConsoleCommand::ScriptAutoReload { id: ScriptId, enabled: bool }`; it is submitted as a typed request and is not accepted from parsed console text.
- Extend active script listings and script status events with `auto_reload: bool` and optional bounded `reload_error` text.
- `ScriptManager::set_auto_reload(id: &ScriptId, enabled: bool, loader: &ScriptLoader) -> Result<(), &'static str>` updates state only if the worker accepts the corresponding watch update.
- `ScriptManager::request_auto_reload(loader: &ScriptLoader, id: ScriptId)` routes worker notices through `request_load` with `replace: true`.
- A successful replacement preserves the prior script's toggle state. A new load starts Off. Unload removes state and watch configuration.
- The runtime drains UI lifecycle/toggle requests before processing automatic notifications and loader results; request IDs and current per-script pending IDs reject stale replacements.

- [ ] **Step 1: Write failing manager tests** for default Off, enabling/disabling, preservation across successful replacement, cleanup on unload, automatic error once per content fingerprint, and manual request superseding stale automatic preparation.
- [ ] **Step 2: Run `cargo test --lib scripting::manager::tests`** and confirm the toggle and auto-reload path is missing.
- [ ] **Step 3: Implement manager state and result handling** without changing callback scheduling, capabilities, or script state reset behavior.
- [ ] **Step 4: Write a runtime test** showing watcher notices are handled after user requests and accepted automatic reloads enter the same worker/activation pipeline as manual reloads.
- [ ] **Step 5: Run the manager and runtime test filters** and confirm the new lifecycle cases pass.

### Task 4: Add the inline Scripts-tab toggle and error status

**Files:**
- Modify: `src/overlay.rs`
- Modify: `src/runtime.rs`
- Modify: `src/scripting/console.rs`
- Test: `src/overlay.rs`

**Interfaces:**
- Extend `ScriptConsoleStatusRow` with `auto_reload: bool` and `reload_error: Option<String>`.
- Each loaded runtime row shows the script ID/status and an inline `Auto Reload: On/Off` button. The button submits the typed request directly, without adding a fake `CMD:` row to the transcript.
- Accepted changes update the row from a script status event; errors appear once in the row and once in the console transcript.

- [ ] **Step 1: Add UI-state tests** for applying Auto Reload status/error events to the correct row and clearing an old error when a new reload succeeds.
- [ ] **Step 2: Run `cargo test --lib overlay::`** and confirm the new row state behavior is missing.
- [ ] **Step 3: Implement the inline toggle** in the existing loaded-runtime list and submit through `ConsoleRequestSender::submit`.
- [ ] **Step 4: Ensure the typed toggle is not parsed as a text command and does not emit a command prompt transcript row.**
- [ ] **Step 5: Run `cargo test --lib overlay:: scripting::console::`** and confirm toggle event/UI tests pass.

### Task 5: Full verification and integration review

**Files:**
- Review: `src/scripting/auto_reload.rs`, `src/scripting/loader.rs`, `src/scripting/manager.rs`, `src/scripting/console.rs`, `src/runtime.rs`, `src/overlay.rs`
- Test: repository library test suite

- [ ] Run `cargo fmt --check`.
- [ ] Run `cargo test --lib` and confirm all tests pass.
- [ ] Run `cargo build --release` and confirm release build succeeds.
- [ ] Run `git diff --check`.
- [ ] Have a fresh reviewer check watcher lifecycle, stale request suppression, bounded channels, reload failure behavior, and UI/event consistency.
- [ ] Do not stage, commit, push, install, or launch Mad Max. The user will review the concrete build before any game install or live check.

## Completion

- The design is a session-only worker-polled toggle; no third-party watcher dependency is added.
- Local repository branch is `main`; remote branch cleanup is a separate repository operation and remains pending GitHub connectivity.
- Leave all existing unrelated working-tree files intact.
