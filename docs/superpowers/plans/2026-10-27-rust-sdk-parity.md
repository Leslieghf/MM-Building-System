# Rust Mad Max SDK Parity Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans inline. Steps use checkboxes and TDD for executable behavior.

**Status:** First implementation tranche added; whole-SDK parity and native spawn remain incomplete.

**Goal:** Build an original Rust-facing SDK surface incrementally toward the Mad Max-specific declarations in the pinned community SDK, then extend it with verified capabilities for construction.

**Architecture:** Keep the ASI and Bevy ECS host. Add a focused `sdk` module split into ABI value types, build/address metadata, and game API declarations. Raw pointers are typed as opaque handles; `unsafe extern` aliases are explicit candidate declarations, not safe callable wrappers. The community SDK is a pinned research reference, not copied source.

**Tech Stack:** Rust 2024, Windows x64, current `hudhook` and lightweight Bevy ECS dependencies; no C++ dependency and no copied ImGui/MinHook.

**Spec:** `docs/research/2026-10-27-mm-sdk-rust-port-audit.md`; `docs/superpowers/specs/2026-10-27-bevy-runtime-bridge-design.md`

## Global Constraints

- Keep all currently uncommitted bridge work intact.
- Use pinned SDK revision `f2dc640769c88aae9c3965a2aca0aa138f0e62ab` as API evidence and cite it in module docs.
- Separate declaration coverage from runtime-verified coverage; never label an address or call verified solely because it appears in the SDK.
- Keep engine pointers and ABI-specific objects out of ECS; ECS sees owned values and typed intents/results.
- No C++ source, bundled SDK dependencies, UI backend, or hook backend copied into this repository.
- No game launch, install, deploy, or process mutation as part of this pass.
- User wants to inspect/review each Rust line; keep the changes cohesive and report new API and unsafe assumptions precisely.

## Review Focus

- SDK revision changes or distribution mismatch: all address values must carry build/distribution provenance and resolve to unavailable when unsupported.
- C++ layout drift: `repr(C)` value types must have tested size/alignment; opaque classes must not pretend to be complete Rust structs.
- Unverified call ABI: declaration metadata must not accidentally expose an invocable native function.
- Owned ECS request values must never contain native pointers, C++ container layouts, or borrowed game strings.
- Spawn return values and queue declarations must not be represented as success/completion until game-side semantics are independently established.

## File Map

- `src/sdk/mod.rs`: public safe SDK facade and coverage model.
- `src/sdk/types.rs`: exact small ABI value types (`Vector3`, `Matrix4x4`, `HashId`) and layout tests.
- `src/sdk/build.rs`: explicit distribution/build identity and immutable address-pair metadata.
- `src/sdk/game/`: declarations grouped by SDK headers; opaque types and candidate function signatures only until individually verified.
- `docs/research/2026-10-27-mm-sdk-rust-port-audit.md`: coverage table updated as modules are ported.

## Tasks

### Task 1: Add portable ABI value types

**Files:** Create `src/sdk/mod.rs`, `src/sdk/types.rs`; modify `src/lib.rs`.

- [x] Write unit tests for `HashId` sentinel semantics, math operations, and ABI value layouts.
- [x] Run the focused tests and observe expected unresolved-type failures before implementation.
- [x] Implement pointer-free math values from `math3d.h`, including `Vector2/3/4`, plane, AABB, quaternion, matrix, and Rust implementations of the SDK's pure vector/matrix operations.
- [x] Re-run tests, `cargo fmt --check`, and Windows x64 `cargo check`.

### Task 2: Add explicit build and address metadata

**Files:** Create `src/sdk/build.rs`; update SDK audit.

- [x] Test distribution address selection, missing addresses, and unsupported-build behavior.
- [x] Record SDK Steam/GOG address pairs from direct headers/source definitions and retain per-symbol source paths; do not wire new calls.
- [x] Add the recorded local Steam executable fingerprint separately from the SDK's distribution labels; keep address pairs classified as candidates.
- [x] Verify tests and compile.

### Task 3: Port safe/read-only declaration modules

**Files:** Create `src/sdk/game/{character,vehicle,camera,object,state,hierarchy,events}.rs`; create `src/sdk/core/{hash,input}.rs` and `src/sdk/math.rs`.

- [x] Define opaque object types and explicit Windows function-pointer aliases for the player, vehicle, camera, object-transform, input, event-handler, and math declarations used or tracked in this tranche.
- [x] Move the existing bridge's function-pointer aliases and candidate vehicle slots into `sdk` modules; do not change the candidate addresses or add native operations.
- [x] Preserve owned snapshots in the existing runtime; keep new native signatures as declaration-only aliases.
- [x] Compile the declaration modules and test all ABI values used by the slice.
- [x] Add per-class method-order metadata for the SDK's `CGameObject`, `IPfxGameObject`, `CPhysicsGameObject`, `CDamageable`, `CVehicle`, and `CEventHandler` declarations.
- [ ] Validate numeric MSVC vtable indices and base-subobject offsets against the target executable before treating any ordinal as a callable slot.

### Task 4: Port construction declarations without unsafe invocation

**Files:** Create `src/sdk/game/spawn.rs`; update construction plan and audit.

- [x] Represent the SDK's spawn priorities and last-frame observation labels as Rust enums.
- [x] Keep `SSpawnInstanceInfo`, native strings/vectors, handler, queue item, and spawned entity opaque.
- [x] Add candidate typed signatures and dual-distribution addresses for spawn ID, spawn, despawn, and entity-file setter declarations.
- [x] Document descriptor/resource/completion gaps that block invocation.
- [x] Run format, unit tests, and Windows x64 build.

### Task 5: Coverage review and handoff

- [x] Compare every tracked `game/` and `core/` SDK header against the current Rust module coverage; intentionally omitted layouts/backends are documented in the audit.
- [x] Record the distinction between declarations, address candidates, existing bridge observations, and unverified spawn behavior in the audit.
- [x] Run `cargo fmt --check`, Windows x64 build, 20 library tests, and `git diff --check`.
- [x] Report the partial surface and the remaining method-order/runtime gaps; no spawn vertical slice is claimed.

## Ruling: Worktree

The checkout is a normal Git checkout but already sits on a `codex/` feature branch with substantial user-approved, uncommitted Bevy bridge work. A fresh linked worktree would omit those changes and make the SDK layer harder to integrate/review. Continue in this existing feature branch, preserving all unrelated working-tree edits. Cost if wrong: SDK changes share the current branch and require careful diff selection.

## Follow-up needed for actual parity

This plan delivers ABI values, address provenance, a declaration-order catalog, and the first set of type-safe declarations. It does not claim full parity. The remaining SDK work is exact virtual ABI mapping for `CGameObject`, `IPfxGameObject`, `CPhysicsGameObject`, `CDamageable`, and `CVehicle`; selected `core.h`/`std.h` utility types; and deciding which graphics-engine declarations should remain excluded because this repository already uses `hudhook`. Native object spawning is a separate proof task and remains gated by the construction-first plan.
