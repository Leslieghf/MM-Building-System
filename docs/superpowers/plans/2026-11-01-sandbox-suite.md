# Mad Max Sandbox Suite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

**Goal:** Expand the existing Rhai Engine Workbench into one compact selector-driven in-game sandbox for exercising all currently mapped bridge mechanisms and tracking unmapped systems.

**Architecture:** Preserve the existing `engine_workbench` package and its bounded native adapters. Add 16 pages to one script panel; each page is a focused micro-playground, while WIP-only pages document exact content leads and missing bridge operations. Keep all side effects disabled by default and gated by the active page plus explicit controls.

**Tech Stack:** Existing Rust host, Rhai API v1, TOML script manifest, local compiled-asset preview tooling.

**Spec:** `docs/superpowers/specs/2026-11-01-sandbox-suite-design.md`

## Global Constraints

- One panel ID; no more than 16 page choices and 48 widgets on any selected page.
- Do not add new native APIs or arbitrary memory/FFI calls.
- Camera control remains an expiring native-camera getter override, not detached freecam.
- Raycasts use the supported deferred API and stop when the page/toggle is inactive.
- Spawning uses only catalog IDs and at most one native spawn request per callback.
- Vehicle writes use the bounded `ctx.vehicle.set_velocity` adapter and explicit button edges.
- Input sampling may be unavailable; UI button events are not native game-input evidence.
- No automated tests are added or run; no game launch, ASI build, or native install is required.

## Review Focus

- Stale/missing copied actor or camera values: format as unavailable; do not use them for writes.
- Deferred ray result states: handle pending, complete hit, complete miss, unavailable, expired, and unknown without reusing an old hit as a new result.
- A spawn button must never submit without a fresh successful hit and available bounded spawn queue.
- Camera override must clear when disabled or unavailable; other pages must not continue ray/gizmo work.
- Page changes and delayed widget events must not repeat a prior one-shot action.

## Task 1: Expand the existing Rhai suite

**Files:** Modify `examples/scripts/engine_workbench/main.rhai`, `manifest.toml`, and `README.md`.

- [x] Declare only the existing capabilities actually used by the expanded package.
- [x] Add the 16 pages from the spec to one `page` selector.
- [x] Add overview/runtime telemetry, copied player/vehicle/camera values, and coordinate/math comparisons.
- [x] Preserve and bound the camera offset/rotation and vehicle velocity/jump controls.
- [x] Add an opt-in deferred camera-ray page with result status, filter, range, hit readout, and optional hit gizmo; stop requests when disabled or off-page.
- [x] Add a click-triggered spawn probe that freezes the selected catalog ID, waits for a fresh ray hit, then submits one allowed catalog ID.
- [x] Add an opt-in, actor-relative diagnostic page for supported gizmo variants, respecting the per-invocation primitive/geometry budget.
- [x] Add an input/UI page that reports `ctx.input.available()` accurately and records a UI button event only.
- [x] Add concise WIP pages for tuning/damage/upgrades/GameObjects/Havok/NPCs/animation/effects/assets with evidence-backed names and no fake controls.
- [x] Document how to load/reload the expanded package and how page gating affects side effects.

## Task 2: Refresh editable-field and suite research

**Files:** Modify `docs/research/runtime-editable-fields-roadmap-2026-11-01.md`; add an entry to `docs/research/install-history.md`.

- [x] Record the exact live vs WIP boundary for every page.
- [x] Capture the local asset-gallery preview count and the existing candidate symbols without claiming callable native signatures.
- [x] Record deployed script hashes; note that no ASI or generated API reference changed.

## Task 3: Static validation and deployment

**Files:** Existing package deployed to `MadMaxModding/scripts/engine_workbench`; local asset workbench output remains under ignored `target/content-index/`.

- [x] Parse the manifest and inspect all page/widget IDs, bounds, and capability use.
- [x] Review `git diff --check` for source whitespace errors.
- [x] Copy the package source to the installed scripts directory and compare hashes.
- [x] Do not launch the game. User will Rescan/Load in Dev Tools; Rhai compilation, runtime behavior, and visual fit remain pending.

## Review Focus Notes

The script manager performs Rhai compilation when the user loads/reloads a package. Since the game may be open and the script folder is hot-reloadable, deployment can be verified by hashes without replacing the ASI or restarting the process. The user-facing next step is one multi-page in-game pass, not one test per feature.
