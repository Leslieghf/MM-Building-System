# Mad Max Dev Tools Dashboard Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Split the current mixed script console into Console and Scripts tabs, and expose safe package discovery with explicit user-initiated loading.

**Architecture:** Keep the existing single ImGui window and switch its body between two tabs. Extend the existing script-loader worker with a bounded discovery request/result path; it scans direct child packages, validates manifest metadata only, and never compiles or executes Rhai code during discovery. UI actions reuse existing lifecycle command requests and worker ownership rules.

**Tech Stack:** Rust, Bevy ECS resources, Dear ImGui via hudhook, bounded `std::sync::mpsc`, TOML/Serde, existing Rhai script manager.

**Spec:** `docs/superpowers/specs/2026-10-29-dev-tools-dashboard-design.md`

## Global Constraints

- No filesystem traversal or Rhai compilation on the render or game callback thread.
- Discovery never compiles or runs scripts.
- Package paths remain canonicalized and confined to the configured scripts root.
- F1/F2/F3 behavior and global input capture remain unchanged.
- Preserve existing console history, output, command behavior, script lifecycle, and bounded queues.
- Do not modify or install the rolled-back input candidate as part of dashboard work.

## Review Focus

- Missing scripts directory at initial scan: show an empty expected path and allow rescan after creation.
- Malformed or mismatched manifest: show that package as invalid without suppressing valid neighbors.
- Junction/symlink escape: reject without reading outside the canonical root.
- Full/disconnected discovery channel: return bounded visible error and keep UI responsive.
- Source package deleted while a generation remains active: retain runtime status until explicit unload.

---

### Task 1: Worker-owned package discovery

**Files:**
- Modify: `src/scripting/loader.rs`
- Modify: `src/scripting/package.rs`

**Interfaces:**
- Add `DiscoverScriptsRequest { request_id: u64 }`.
- Add bounded DTO `DiscoveredScript { package_name: String, id: Option<ScriptId>, manifest: Option<ScriptManifestSummary>, issue: Option<String> }` so malformed directory names and invalid manifests remain visible as rows.
- Add `ScriptManifestSummary { id: String, api_version: u32, callbacks: Vec<String>, capabilities: Vec<String> }`.
- Add `ScriptDiscoveryResult { request_id: u64, root: String, packages: Vec<DiscoveredScript>, error: Option<String> }`.
- Add `ScriptLoader::try_discover(request: DiscoverScriptsRequest)` and `try_discovery_result()`.

- [x] Add a request enum for load and discovery while preserving current load API behavior.
- [x] Implement direct-child scan with caps, bounded manifest reads, manifest parsing/metadata validation, and per-directory errors.
- [x] Do not open `main.rhai`, compile Rhai, or execute scripts during discovery.
- [x] Keep canonical root confinement and verify file handle paths using existing helpers.
- [x] Return each package's bad manifest as a row-level issue; a scan-level failure must be distinct.
- [x] Keep bounded worker queues; report discovery overload separately from script-load failures.

### Task 2: UI state and discovery transport

**Files:**
- Modify: `src/runtime.rs`
- Modify: `src/scripting/console.rs`

**Interfaces:**
- Extend `ScriptConsoleUiState` with active tab, discovered inventory, selected ID, discovery request/status, and a one-time first-open scan flag.
- Add a local `script list` action that requests discovery and reports eventual results; it must not imply that packages are active.
- Merge worker discovery results with current manager status by stable `ScriptId`.

- [x] Add bounded UI-state fields and ensure defaults do not auto-load or auto-run scripts.
- [x] Route initial Scripts-tab entry and Rescan through the loader worker.
- [x] Route `script list` through discovery, replacing its active-only listing semantics.
- [x] Keep lifecycle request paths (load/reload/run/unload) unchanged and feed resulting states back into the dashboard.
- [x] Ensure queue-full and worker-unavailable states become concise console/dashboard messages.

### Task 3: Dev Tools tabs and Scripts dashboard

**Files:**
- Modify: `src/overlay.rs`
- Modify: `src/runtime.rs`

- [x] Rename the window from `Mad Max Script Console` to `Mad Max Dev Tools`.
- [x] Add Console and Scripts tabs; Console contains prompt, history, output, and no script-status block.
- [x] Add a scrollable discovered-package inventory with ID, validity, callback/capability metadata, generation, and active/disabled/unloaded state.
- [x] Add Rescan and explicit Load/Reload/Run/Unload actions, including unload for loaded packages whose files disappeared.
- [x] Show concise scan/lifecycle progress and errors in Scripts, preserving console output for command history and general logs.
- [x] Preserve tab selection and scroll state; preserve F1/F2/F3/Escape semantics.

### Task 4: Docs and verification

**Files:**
- Modify: `README.md`
- Modify: `docs/bootstrap-install-record.md`

- [x] Document package discovery location, manifest-only scan, explicit load, and Scripts tab use.
- [x] Document that broad mod-owned runtime input sampling remains rolled back pending crash isolation.
- [x] Run `cargo fmt --check`, `cargo build --release --locked`, and `git diff --check`.
- [x] Review changes for accidental ASI installation or game-process interaction; none are part of this plan.
