# Scripted Spawn Workbench Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the native spawn browser into a script-extensible workbench, backed by versioned multi-file packages with explicit dependencies and an initial read-only Events tab.

**Architecture:** Keep ImGui rendering and package discovery in the Rust host. Packages declare confined source files, versions, dependencies, conflicts, and optional workbench tab metadata; Rhai callbacks publish bounded copied tab models and receive copied input events. The loader resolves and compiles a complete local dependency graph before the manager changes active generations at a callback boundary.

**Tech Stack:** Rust, Bevy ECS-only runtime, Rhai 1.27, TOML, `semver` 1.0.28, ImGui.

**Spec:** `docs/superpowers/specs/2026-11-01-scripted-spawn-workbench-design.md`

## Global Constraints

- Preserve current packages: missing `version` means `0.0.0`; missing `sources` means `main.rhai`; missing `requires` and `conflicts` means empty lists.
- Keep package dependencies separate from effect permissions; each package still needs the matching capability for game or UI effects.
- Load only installed packages; never enable Rhai filesystem modules or arbitrary `import`/file access.
- Confine every declared source path to its package directory; enforce UTF-8, per-file, combined-source, manifest, and model limits.
- Keep compilation/discovery off the render callback and activate, replace, or unload packages only at callback boundaries.
- A rejected dependency graph leaves all currently active package generations unchanged.
- Keep the base spawn tab usable when optional tabs are missing, invalid, or unloaded; do not execute Rhai or scan files while rendering.
- Do not expose arbitrary event dispatch or claim that unresolved RTTI classes are dispatchable events.
- Keep detailed contracts in Rustdoc and generated `API.md`; keep `docs/scripting.md` concise and link to the generated reference.
- Do not add or run tests unless the user asks; use focused code review and a build/format check only when implementation is authorized.

## Review Focus

- Missing, duplicate, malformed, oversized, or path-escaping source entries must fail package preparation without reading outside the package.
- Cycles, missing/incompatible required dependencies, conflicts, and unloads with active dependents must produce actionable errors and no partial activation.
- Replacing a package must not lose its last active generation if any source fails to read or compile.
- Stale UI events from an unloaded/reloaded package generation must not reach the new package generation.
- Tab failure or absence must not hide or break the base spawn browser, and UI drawing must not run scripts or perform filesystem work.

---

### Task 1: Extend the package manifest with versions and typed relations

**Files:**
- Modify: `Cargo.toml` (add direct `semver = "=1.0.28"` dependency, already present transitively in `Cargo.lock`)
- Modify: `src/api/scripts/package.rs`
- Modify: `src/api/scripts/loader/discovery.rs`
- Modify: `src/api/scripts/mod.rs`
- Modify: `docs/scripting.md`
- Modify: `docs/rhai-api.md` only if package authoring is included in its generated section; otherwise keep package semantics in `docs/scripting.md`
- Modify: existing package manifest examples under `examples/scripts/**/manifest.toml` only where a new tab or dependency is declared

**Interfaces:**
- Add `PackageVersion(semver::Version)`, `PackageRequirementKind::{Package, HostApi}`, `PackageRequirement { kind, id, version: semver::VersionReq, optional }`, and `PackageConflict { id, version: semver::VersionReq }` in `package.rs`.
- Extend `ScriptManifest` with serde-defaulted `version`, `sources`, `requires`, `conflicts`, and optional `workbench_tab` metadata. Keep the current `id`, `api_version`, `callbacks`, and `capabilities` fields unchanged.
- Extend `ScriptManifestSummary` with bounded, display-safe version/relation/tab summaries for discovery UI; do not expose raw unbounded manifest strings.

- [x] Add direct semver dependency and manifest value types with strict serde parsing and bounded relation counts.
- [x] Validate package IDs, duplicate/conflicting declarations, semantic version strings, reserved host API IDs, and declared tab metadata; preserve legacy defaults above.
- [x] Update package Rustdoc and concise authoring docs with one complete manifest example and explain the difference between `requires` and `capabilities`.
- [x] Review every existing manifest for compatibility with the defaults and ensure no existing package requires an edit just to continue loading.

### Task 2: Load and hot-reload explicit multi-file packages safely

**Files:**
- Modify: `src/api/scripts/package.rs`
- Modify: `src/api/scripts/loader.rs`
- Modify: `src/api/scripts/loader/discovery.rs`
- Modify: `src/api/scripts/auto_reload.rs`
- Modify: `src/api/scripts/loader.rs` package snapshot helpers and bounded file-reading helpers

**Interfaces:**
- Add an owned `PackageSource { path: String, contents: String }` transfer value; paths are validated manifest-relative paths, not filesystem handles.
- Change internal package compilation to accept `&[PackageSource]`, concatenate in manifest order, and retain a source-line map so Rhai compile diagnostics can report package file and line.
- Change `PackageFingerprint` and `AutoReloadSnapshot` to cover manifest plus every declared source path/content. Keep snapshot bytes paired with the fingerprint so a later edit cannot change what is compiled.

- [x] Implement source-list normalization: `main.rhai` is required, paths are unique, relative, UTF-8, and reject root/prefix/parent traversal and unsupported path forms.
- [x] Read each file through the existing canonical/opened-path confinement checks; enforce per-file and combined package-size limits before compilation.
- [x] Compile the ordered source set as one AST and translate compile positions back to the correct source path and local line.
- [x] Update discovery to validate every declared source without loading Rhai modules or executing code.
- [x] Update auto-reload snapshots and debounce fingerprints to observe every declared source; a read/compile failure keeps the currently active generation.
- [x] Preserve the existing single-file preparation path through the default `sources = ["main.rhai"]` behavior.

### Task 3: Resolve dependencies and activate package graphs transactionally

**Files:**
- Create: `src/api/scripts/package_graph.rs`
- Modify: `src/api/scripts/mod.rs`
- Modify: `src/api/scripts/loader.rs`
- Modify: `src/api/scripts/loader/discovery.rs`
- Modify: `src/api/scripts/manager.rs`
- Modify: `src/api/scripts/manager/invocation.rs` only if active package metadata must accompany callbacks
- Modify: `src/api/scripts/console/command.rs`, `src/api/scripts/console/output.rs`, and `src/devtools/overlay/panel.rs` for dependency status and actionable lifecycle feedback

**Interfaces:**
- Add `PreparedPackageGraph { packages: Vec<PreparedScript>, requested_id: ScriptId }`, ordered dependency-first and produced only after every required node validates and compiles.
- Add `resolve_package_graph(root, requested_id, installed_manifests, active_versions) -> Result<PackageGraphPlan, PackageGraphError>` in `package_graph.rs`; use stable lexical ID ordering among otherwise-ready nodes.
- Add `ScriptManager::stage_prepared_graph(graph) -> Result<(), String>` and a single pending graph-activation change. Existing `stage_prepared` remains for direct/internal one-package callers and routes through the same validation.
- Discovery rows gain a bounded dependency state: ready, optional unavailable, or blocked with a concise reason.

- [x] Resolve required installed package and host API requirements, including version constraints; optional requirements never block and are represented as unavailable to the package.
- [x] Detect cycles, missing/incompatible requirements, duplicate IDs, package conflicts, active conflicts, and active-count overflow before staging any activation.
- [x] Recheck graph compatibility at the callback boundary, then install all new/replacement generations as one transaction; retain prior active generations if validation fails.
- [x] Reject unload of a package with active required dependents and name those dependents; do not cascade unload implicitly.
- [x] Ensure replacement and auto-reload cannot leave a dependent active against an incompatible version; reject the update atomically with a useful failure.
- [x] Surface dependency state in script discovery and lifecycle output without adding verbose per-frame console noise.

### Task 4: Add a versioned script-owned workbench tab contract

**Files:**
- Create: `src/api/scripts/workbench.rs`
- Modify: `src/api/scripts/mod.rs`
- Modify: `src/api/rhai/bindings.rs`
- Modify: `src/api/rhai/catalog.rs`
- Modify: `src/api/scripts/manager.rs`
- Modify: `src/api/scripts/manager/invocation.rs`
- Modify: `src/devtools/state/presentation.rs`
- Modify: `src/devtools/overlay/spawn_menu.rs`
- Modify: `src/devtools/overlay/mod.rs`
- Modify: `build.rs`-generated `docs/rhai-api.md` through the normal catalog generation workflow

**Interfaces:**
- Define versioned host API ID `workbench.tabs` at version `1.0.0`; packages must declare it as a `host_api` requirement and separately request the `ui.workbench` permission capability to publish interactive tab content.
- Add `WorkbenchTabPublication { package_id, generation, tab_id, title, order, sections }`, bounded copied `WorkbenchSection`/`WorkbenchRow` values, and `WorkbenchTabEvent { package_id, generation, tab_id, section_id, row_id, value }`.
- Add a Rhai builder reachable from `ctx.workbench`; it publishes the package's complete tab snapshot during a game callback. The renderer consumes only the latest copied snapshot and queues copied events for the next callback.
- Extend the native shell's tab registry to combine the always-present Spawn tab with currently active valid package tabs. Stable tab ordering is `(order, package_id, tab_id)`.

- [x] Add the `ui.workbench` permission to the API capability registry and runtime enforcement; do not make a host API dependency grant permission.
- [x] Add bounded Rhai builders for tab metadata/content and validate IDs, duplicate rows, text lengths, row counts, and allowed value kinds before publication.
- [x] Add generation-checked event routing and remove tab publications/events when a package unloads or is replaced.
- [x] Refactor `spawn_menu.rs` so tab selection is data-driven while retaining the current native Spawn content and tools/settings side panel.
- [x] Keep all workbench render paths free of Rhai calls, loader access, content scans, or rebuilding the spawn asset catalog.
- [x] Add concise API catalog entries and regenerate the script API reference; document a small tab package example in `docs/scripting.md`.

### Task 5: Deliver the read-only Events workbench package

**Files:**
- Create: `examples/scripts/event_workbench/manifest.toml`
- Create: `examples/scripts/event_workbench/main.rhai`
- Create: `examples/scripts/event_workbench/events.rhai`
- Create: `examples/scripts/event_workbench/README.md`
- Create: `docs/research/event-workbench-inventory.md`
- Modify: `docs/scripting.md` for package installation/load instructions if needed

**Interfaces:**
- The package declares `version`, `sources`, a `workbench.tabs` host API requirement, and `ui.workbench` capability; its tab metadata is `Events`, with stable order after Spawn.
- Inventory records use explicit `kind`, `name`, `owner`, `payload_status`, `observation_status`, `dispatch_status`, and `source` fields; unknown values remain unknown.

- [x] Add only currently mapped action registry/publisher facts and RTTI leads from the approved design note; label related but unlinked RTTI as leads, not confirmed registry members.
- [x] Publish a searchable/listable read-only inventory with clear mapped, unknown, and unavailable states; do not add live observation or dispatch controls.
- [x] Keep inventory data separate from presentation logic across the package's declared source files.
- [x] Document that event observation and dispatch are later capabilities requiring their own mapped timing, payload, and ownership contracts.

### Task 6: Documentation and integration review

**Files:**
- Modify: `docs/scripting.md`
- Modify: generated `docs/rhai-api.md`
- Modify: `docs/architecture.md` only for the final package/workbench boundary summary
- Modify: `docs/research/event-workbench-inventory.md`
- Modify: `examples/scripts/event_workbench/README.md`

- [x] Ensure documentation contains one authoritative package manifest example, one short multi-file include example, and one tab publication example; link to generated `API.md` rather than duplicating signatures.
- [x] Review the design acceptance criteria against the implementation, especially rollback behavior, legacy package compatibility, event inventory labels, and render-thread boundaries.
- [x] Perform a focused code review for path confinement, stale-generation events, dependency ordering, bounded copied UI values, and callback-boundary transaction semantics.
- [x] If the user authorizes verification, run formatting and a Windows-target build; do not install/deploy or commit/push as part of this plan unless separately requested.


Implementation status: all listed implementation and source-review items were completed. Automated tests were neither added nor run under the repository instruction; validation was limited to targeted rustfmt, diff whitespace checks, and cargo check --lib. No in-game package load/reload test was performed in this pass.
