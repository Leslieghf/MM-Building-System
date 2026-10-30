# Intent-led Rust Architecture Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reorganize the Mad Max ASI crate from raw executable access through owned ECS contracts to Rhai modding APIs, with a narrow kernel and separate developer tools.

**Architecture:** Derive paths from the accepted constituent inventory. `api` owns build-aware game adaptation, ECS contracts, Rhai bindings, and script lifecycle; `kernel` owns ASI entry and callback composition; `devtools` owns the overlay and diagnostics presentation; operational log writing belongs to `kernel::diagnostics`. Preserve the direction `raw game -> owned telemetry -> ECS -> Rhai -> typed effects -> validated native adapter`.

**Tech Stack:** Rust 2024, Bevy 0.19.1, Rhai 1.27.0, Hudhook 0.9.3, Windows MSVC target.

**Spec:** `docs/superpowers/specs/2026-10-30-intent-led-crate-architecture.md`

## Global Constraints

- Raw pointers stay within native callback/adapter code; cross-thread and script-facing values are owned, bounded types.
- Keep build-specific addresses in one authoritative place.
- Separate raw ABI declarations, safe game adapters, ECS mod contracts, and Rhai presentation.
- Keep kernel orchestration narrow and explicit; feature modules should not become the application entry point.
- Preserve public script API behavior, hook order, queue limits, and failure behavior during structural cleanup.
- No hot reload redesign, gameplay feature work, or game installation/launch.
- Do not add or run tests; run formatting, Windows-target check, and build as requested.

## Review Focus

- Unknown executable/build fingerprint: verify fail-closed checks remain in the game adapter and build metadata stays single-source; inspect changed resolver paths and run Windows-target `cargo check`.
- Callback overlap/thread mismatch: verify callback gate and panic containment stay together in the kernel game-loop path; inspect the moved function and run Windows-target `cargo check`.
- Full/disconnected bounded channels: verify send/receive outcomes and configured capacities are unchanged; inspect channel constructors and call sites, then build.
- Invalid script capability/value or expired invocation: verify Rhai validation and bounded invocation ownership stay in `api::rhai`; inspect moved API registration and run Windows-target `cargo check`.
- UI-thread/native-memory boundary: verify devtools receives only copied snapshots and emits typed requests; inspect `devtools` imports and run Windows-target `cargo check`.

---

## File Map

- `src/lib.rs`: minimal module declarations and stable crate exports.
- `src/api/game/abi/`: reconstructed SDK declarations, build fingerprint, address metadata, and common native types.
- `src/api/game/`: guarded memory operations, hook lifecycle, sampling, copied telemetry, and validated effect adapters.
- `src/api/ecs/`: owned frame/intent/input models, bounded channels, Bevy game/UI schedules, mod-system registration, and script dispatch systems.
- `src/api/format.rs`: pure shared display formatting used by Rhai and diagnostics.
- `src/api/rhai/`: invocation context, Rhai bindings, capabilities, math helpers, and API catalog/reference generation.
- `src/api/scripts/`: engine, active script manager, package loader/validation, reload fingerprinting, and UI-neutral console protocol.
- `src/kernel/`: `InitializeASI`, startup, endpoint wiring, callback gate/schedule loop, composition, and `diagnostics.rs`.
- `src/devtools/`: DX11/ImGui driver, panels, clipboard, overlay policy, and UI presentation.
- `src/kernel/diagnostics.rs`: append-only operational logging used by startup and callbacks.
- `src/bin/trace_xref_probe.rs`: stays a separate investigation utility.
- `docs/runtime-map.md`: final responsibility/data-flow map; preserve historical research docs.

## Task 1: Establish the game-access/API backend

**Files:** move `src/sdk/**` to `src/api/game/abi/` except ImGui-backed mod input state, which moves to `src/api/ecs/input.rs` and devtools key mapping; move `src/native/game_probe/**` into `src/api/game/`; keep build metadata with `api::game::abi` and move append-only logging to `kernel::diagnostics.rs`. Add explicit module docs and avoid wildcard imports at newly created module boundaries.

**Interfaces:** consumes the existing SDK declarations, endpoint types, and typed intent/outcome types during migration; produces a game adapter that publishes owned telemetry and accepts only validated typed effects. Keep the `STEAM_IMAGE_BASE` definition in the ABI build metadata and re-export it internally where required.

- [ ] Move ABI declarations and build metadata under `api::game::abi`, updating all imports without changing addresses or signatures.
- [ ] Group guarded memory helpers, passive hooks, world-query hooks/capture, sampling, and telemetry by the responsibilities in the spec.
- [ ] Keep raw callback state private; expose only install/start operations, copied sample DTOs, and adapter operations needed by the kernel.
- [ ] Confirm the moved module graph has no dependency on `devtools` and no new dependency cycle; run Windows-target `cargo check`.

## Task 2: Separate ECS contract and public Rhai surface

**Files:** split `src/runtime/model.rs`, `endpoints.rs`, `game.rs`, `game_loop.rs`, and `presentation.rs` across `src/api/ecs/`; move pure shared formatters from `src/ui/format.rs` to `src/api/format.rs`; move `src/scripting/dispatch.rs` to `src/api/ecs/script_dispatch.rs`; move `src/scripting/api.rs`, `api_catalog.rs`, and `vector.rs` to `src/api/rhai/`.

**Interfaces:** `api::ecs` owns copied telemetry, bounded endpoints, schedule labels, typed intents/outcomes, and mod-system registration. `api::rhai` consumes owned invocation data and returns bounded typed effects; it never receives native pointers. Script engine/lifecycle interfaces stay stable under `api::scripts`.

- [ ] Split mixed runtime DTOs into game telemetry, intent/effect, and developer-tool presentation contracts; keep queue capacities and public types unchanged.
- [ ] Move Bevy schedule setup, script ECS dispatch, and intent application into focused `api::ecs` modules.
- [ ] Move Rhai context construction, registration, capability checks, vector functions, and reference catalog into focused `api::rhai` modules.
- [ ] Move engine, manager, loader, package validation, reload tracking, and console transport into `api::scripts`; isolate the existing ImGui key-name mapping behind a devtools adapter so the console protocol is UI-framework-neutral.
- [ ] Update all imports and verify the dependency direction `ecs -> rhai/scripts`, with no reverse dependency on `kernel` or `devtools`; run Windows-target `cargo check`.

## Task 3: Make the kernel the actual entry/composition root

**Files:** create `src/kernel/mod.rs`, `entry.rs`, and `game_loop.rs`; move `InitializeASI`, startup, endpoint wiring, game callback coordination, and runtime construction from the overlay/native modules.

**Interfaces:** `kernel::entry::InitializeASI` remains the exported `extern "system"` entry point. Kernel initialization composes `api` and `devtools`; game hooks and UI driver receive their endpoint through explicit one-time setup functions.

- [ ] Move ASI export and startup/error containment into `kernel::entry` and retain its exported symbol.
- [ ] Move callback stability gate, overlap protection, schedule invocation, and typed effect drain into `kernel::game_loop` without changing callback order or fail-closed behavior.
- [ ] Make `kernel::mod` wire API and devtools endpoints; keep domain modules from calling back into kernel internals.
- [ ] Verify exported entry and all module boundaries through `cargo check` and Windows-target build.

## Task 4: Put overlay and operational presentation under devtools

**Files:** move `src/ui/overlay/{mod.rs,panel.rs,clipboard.rs,config.rs}` and UI-facing readout projection to `src/devtools/`; move Bevy UI schedule setup and intent flushing to `src/api/ecs/ui.rs`; split the overlay driver from the panel and Win32 clipboard adapter.

**Interfaces:** `devtools` consumes copied API snapshots, diagnostics, and script console protocol types. It sends typed UI/script requests through bounded endpoints; it does not call raw game functions or depend on native hook state.

- [ ] Move the ImGui driver, panels, clipboard backend, and overlay policy into named devtools modules.
- [ ] Move diagnostics/readout projection to devtools while keeping shared DTOs in `api::ecs`.
- [ ] Rename stale role names (including `BootstrapNotice` to `OverlayRenderDriver`) and add short comments only for responsibilities, safety constraints, and ownership handoffs.
- [ ] Inspect imports to confirm no raw SDK/native pointer dependency in devtools; run Windows-target `cargo check`.

## Task 5: Finalize docs, names, and branch state

**Files:** update `src/lib.rs`, `docs/runtime-map.md`, and the accepted architecture spec; no historical research-plan rewrites.

- [ ] Confirm crate root exposes only the chosen top-level `api`, `kernel`, and `devtools` domains plus the stable ASI/spawn exports.
- [ ] Search for stale module paths and obsolete names; update active architecture docs to the final tree and explain raw/owned/ECS/Rhai/kernel boundaries.
- [ ] Run `cargo fmt --all`, Windows-target `cargo check`, `cargo build --target x86_64-pc-windows-msvc`, and `git diff --check`; report pre-existing warnings accurately.
- [ ] Commit the structural cleanup to `main` and push it. Confirm every retained feature branch is merged. Change GitHub's default branch to `main` before deleting the former default branch; delete only obsolete, fully merged feature branches, then verify local and remote refs.

## Execution Notes

This is one architecture migration with dependency-ordered checkpoints, not independent parallel subsystems: the raw API, ECS contracts, Rhai surface, kernel, and devtools all exchange interfaces that must be reviewed together. Preserve compatibility at each move. Do not add tests or launch/install the game under the current task constraints.
