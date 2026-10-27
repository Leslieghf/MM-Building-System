# Rust-Only Player Transform Probe Implementation Plan

> **For agentic workers:** Use the previously requested subagent-driven workflow for independent review, but keep the implementation steps sequential because each depends on the user's Steam-session observation.

**Goal:** Add a staged, Rust-only diagnostic that first inspects the candidate player `CGameObject` vtable path without calling it, then—only if the user reviews and accepts that evidence—samples Max's world position through `GetTransform`.

**Architecture:** Keep the existing Rust ASI, callback, and overlay. On every 120th `UpdatePostSim` callback, fetch the player afresh, validate the candidate subobject and vtable path, and report status. The first build does not invoke `GetTransform`; a later separately reviewed build may call zero-based vtable slot 22 with `this = character + 0x8` and a caller-owned 16-float matrix.

**Tech Stack:** Rust 2024, `hudhook` 0.9.3, Windows x64 APIs already used by the ASI. No C++, new runtime, or new dependency is planned.

**Spec:** [`docs/superpowers/specs/2026-10-27-player-transform-probe-design.md`](../specs/2026-10-27-player-transform-probe-design.md)

## Global Constraints

- Keep all code in Rust, including the inheritance adjustment, virtual dispatch, checks, scheduling, status handling, and overlay rendering.
- Sample at most once per 120 `UpdatePostSim` callbacks.
- Fetch the character for each scheduled sample; do not retain game-owned manager, character, matrix, or other pointers between callbacks.
- Keep the probe read-only.
- Do not call `GetTransform` in the first candidate; observe its candidate vtable path first.
- Do not add or run automated tests unless the user asks.
- Show every changed or added Rust source line before building or installing it.
- The user starts the game through Steam. Do not launch the game on the user's behalf.

## Review Focus

- **Unsupported executable or image base:** keep existing fail-closed identity checks; inspect the code path and build output before candidate install.
- **Loading screen or absent player:** status remains waiting/unavailable and no pointer is reused; user observes menu/gameplay transitions via Steam.
- **Unreadable player/subobject/vtable memory:** report preflight failure and do not read beyond a validated readable span.
- **Unexpected vtable slot target:** require slot 22 to point into committed executable memory; the first candidate never invokes it.
- **Failed/non-finite transform result in the later candidate:** show an explicit unavailable state and do not display stale matrix contents.

---

### Task 1: Add bounded Rust ABI and memory-read primitives

**Files:**
- Modify: `src/game_probe.rs`

**Interfaces:**
- Produces `const CHARACTER_TO_GAME_OBJECT_OFFSET: usize = 0x8` and `const GET_TRANSFORM_VTABLE_SLOT: usize = 22`, documented as SDK-derived candidates rather than runtime-proven facts.
- Produces `#[repr(C)] struct Matrix4x4 { values: [[f32; 4]; 4] }` for the later transform call.
- Produces `fn readable_span(address: usize, length: usize) -> bool`, which rejects overflow, uncommitted/guard/no-access memory, and a span not fully covered by the queried readable region.

- [ ] **Step 1: Implement `readable_span` using the existing `VirtualQuery` wrapper.** Keep executable-page validation separate; readable and executable are distinct predicates.
- [ ] **Step 2: Add the candidate offset, slot, and matrix representation with comments pointing to the pinned SDK declarations.** Do not add any virtual call in this task.
- [ ] **Step 3: Review the focused diff and run `git diff --check`.** Expected: no whitespace errors; no test files or dependencies changed.

### Task 2: Build the non-calling player-vtable preflight

**Files:**
- Modify: `src/game_probe.rs`
- Modify: `src/overlay.rs`

**Interfaces:**
- Consumes Task 1's `readable_span`, candidate offset, and candidate slot.
- Produces a `player_layout_status() -> Option<&'static str>` view over atomic status, with statuses for waiting, no player, unreadable object/subobject/vtable, non-executable slot target, and structurally readable candidate.
- The scheduled probe obtains a fresh manager and player pointer each time, computes `character + 0x8`, reads its vtable pointer and slot 22 only after bounded-span checks, and never invokes the slot.

- [ ] **Step 1: Replace the latched getter-only success state with per-sample acquisition for the preflight.** Never store the manager or character pointer in static state.
- [ ] **Step 2: Add the bounded vtable/slot inspection and publish only a status, never a raw pointer.** Keep all work after the original callback and at most once per 120 callback invocations.
- [ ] **Step 3: Update the overlay to show callback count and layout status.** Do not show coordinates or imply that an executable slot is semantically proven to be `GetTransform`.
- [ ] **Step 4: Show the user every changed Rust line, wait for approval, and run `git diff --check`.** The reviewed source must contain no `transmute` or call through the candidate vtable slot in this preflight candidate.
- [ ] **Step 5: Build the approved preflight candidate.** Run `cargo build --release --target x86_64-pc-windows-msvc`; expected: exit code 0 and a DLL artifact. Do not launch Mad Max.
- [ ] **Step 6: Ask the user to close Mad Max, preserve the current getter-only ASI, then install and hash-verify the preflight candidate.** The user launches through Steam and reports the overlay status. If the candidate is unreadable, non-executable, or otherwise ambiguous, stop before Task 3.

### Task 3: Add the Rust-only `GetTransform` call, conditional on preflight review

**Files:**
- Modify: `src/game_probe.rs`
- Modify: `src/overlay.rs`

**Interfaces:**
- Consumes the accepted Task 2 preflight and the same fresh character acquisition path.
- Produces a copied `Option<[f32; 3]>`-style position sample; no manager, character, vtable, or matrix pointer escapes the callback.
- The call signature is `unsafe extern "system" fn(*mut c_void, *mut Matrix4x4) -> bool`, with `this` adjusted to the candidate `CGameObject` subobject and the target loaded from slot 22.

- [ ] **Step 1: Reconcile the observed preflight with the SDK's inheritance/vtable declaration.** Proceed only if the user agrees the evidence supports the candidate slot; otherwise stop and report what remains unknown.
- [ ] **Step 2: Add one transform call per scheduled sample.** Initialize a local 16-float matrix, call slot 22 on the callback path, and read `[3][0..=2]` only when the returned boolean is true.
- [ ] **Step 3: Reject non-finite coordinates and clear stale output on every failed sample.** Overlay statuses must distinguish no player, transform failure, invalid coordinates, and a valid position.
- [ ] **Step 4: Show the user every changed Rust line, wait for approval, and run `git diff --check`.** Confirm the previous no-call preflight remains available for rollback.
- [ ] **Step 5: Build the approved candidate.** Run `cargo build --release --target x86_64-pc-windows-msvc`; expected: exit code 0. Do not launch Mad Max.
- [ ] **Step 6: Ask the user to close Mad Max, preserve the preflight candidate, install and hash-verify the position candidate, and wait for the user to launch it through Steam.** The user checks that coordinates change while Max moves and remain stable while stationary.

### Task 4: Record the result and checkpoint the verified state

**Files:**
- Modify: `docs/research/rust-transform-feasibility.md`
- Create: `docs/research/player-transform-probe-2026-10-27.md`

- [ ] **Step 1: Record the user-observed Steam-session result, candidate hashes, and rollback path without claiming more than the evidence supports.**
- [ ] **Step 2: Commit the reviewed source and research record.** Keep each checkpoint separate for preflight and position-read stages.

## Manual Verification Sequence

- Build the preflight candidate with the exact target command above; do not run automated tests.
- User launches through Steam and checks the status in menu and gameplay; the loading screen may show waiting because callbacks pause.
- Only after reviewing the preflight result, build and install the transform candidate.
- User moves Max and confirms coordinates move; user stops and confirms they stabilize. Any crash, stale values, non-finite values, or ambiguous preflight stops progression and triggers rollback.
