# Bounded Ray Candidate Capture Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bound deep metadata reads in the world-query candidate callback while preserving early and sparse late candidate samples plus the minimum callback fraction.

**Architecture:** Keep the existing signature-checked callback tap and thread-local sampled-query association. Select fixed callback ordinals `0, 1, 2, 4, 8, 16, 32` for full before/after metadata capture. For every callback, increment the total and update a scalar-only minimum finite fraction in `[0,1]`; never chase candidate pointers for unscheduled callbacks. Log total callback count, minimum fraction, and the selected ordinal records independently.

**Tech Stack:** Rust 2024, atomics, existing MinHook callback tap.

**Spec:** `docs/research/2026-10-28-world-query-output-capture.md`, sections “Candidate callback tap hot-path audit” and “Lifetime-safe fallback policy”.

## Current Execution Status (2026-10-28)

The implemented candidate uses detailed callback ordinals `0, 1, 2, 4, 8, 16, 32`; all same-thread callbacks update a scalar count/minimum without deep pointer reads on unscheduled ordinals. Selected records capture the per-ray record before and after the original callback and five raw qwords from the collector-local ray block after it returns, while the collector frame is still alive. The original callback remains called once and its exact float is forwarded. The helper output capture is ten qwords (`0x50` bytes). A mistaken address calculation briefly suggested writes beyond `+0x4C`; rechecking the relative calls showed they target the point-in-bounds predicate at RVA `0x25267E0`, not `0x25277E0`, so no output-span expansion is justified.

The corrected candidate adds a signature-checked observer for the helper's point-bounds predicate. It forwards the original byte result and stores up to two point/radius/result tuples only when a sampled helper query is active on the same thread; it performs the point read only after confirming that TLS association. `cargo fmt --check`, `cargo build --release --locked`, and `git diff --check` passed; no tests were run. Candidate DLL: 2,484,224 bytes, SHA-256 `52576B685E02F8A07A15561B7F925596F7A7629490782088697D2405E136D604`. It remains uninstalled; Mad Max PID `20880` is still running the older ASI, and its bootstrap log has not changed. The complete Rust diff must still be reviewed before a later installation. Runtime callback coverage, stability, fraction/output correlation, and hit semantics remain unverified.

#### Latest bounded recapture adjustment (2026-10-28 07:28 UTC)

The live watcher showed both `wind_occlusion` (return RVA `0x1FC0E0E`) and `vehicle_sound_occlusion` (return RVA `0x21DC95A`) active, while static caller analysis showed each branches on output `+0x14` at a `1.0f` threshold. The ten existing repeat callers sampled their first three adjacent queries; these two high-rate callers would otherwise receive only one first-use sample. The source now gives each a separate sparse schedule at its per-caller call ordinals `0`, `256`, and `2048`. This is a bounded source/build change only; per-caller rate is unknown, and no runtime result is claimed.

Expected allocation: 32 ammo + 8 height + 16 script + 30 existing repeated + 6 sparse occlusion + 40 remaining first-use callers = 132 / 144 samples, leaving 12 spare. `cargo fmt --check`, `cargo build --release --locked`, and `git diff --check` passed. Candidate DLL: 2,484,736 bytes, SHA-256 `561CF45849E8C7A211B01AEFF45EB5FF6AC6341B91EDEF7DBC1A1A16D7F7CE53`. It remains uninstalled; the full Rust diff still requires line-by-line review before a future closed-game install.

## Global Constraints

- Preserve the original callback's exact `f32` return value and call it exactly once for every invocation.
- Do not install or deploy the candidate while Mad Max PID `20880` is running.
- Keep selected-candidate before/after reads inside the callback detour; never defer candidate handles, caster state, or per-ray records until after the helper returns, because the collector is helper-stack-local.
- Keep all capture storage fixed-size and allocation-free in the callback path.
- Keep the callback tap signature check and existing helper/caller sample caps intact.
- Show the complete Rust diff for the user's line-by-line review before any later install.

## Review Focus

- Unscheduled callbacks still increment total count and contribute valid finite fractions to the minimum; test unscheduled ordinals.
- NaN, infinity, and values outside `[0,1]` never replace the minimum; test all invalid classes.
- Negative zero is normalized to positive zero for bitwise atomic ordering, or an equivalent float-aware update proves `-0.0` is retained as numeric zero.
- A smaller fraction arriving after a larger one replaces it; test out-of-order updates.
- Selected ordinal mapping has exactly seven slots and no aliasing; test each selected and neighboring ordinal.
- Nested sampled → unsampled helper calls mask the outer sample, then restore the outer association when the nested frame returns; test stack push/pop with an explicit no-sample frame.
- The detour invokes the original once and returns its value unchanged; inspect the trampoline path and build output.

---

### Task 1: Fixed-schedule candidate sampling and minimum-fraction summary

**Files:**
- Modify: `src/game_probe.rs`
- Modify: `docs/research/2026-10-28-world-query-output-capture.md`
- Test: unit tests in `src/game_probe.rs`

**Interfaces:**
- Add pure selector `world_query_candidate_capture_slot(callback_ordinal: u32) -> Option<usize>` mapping only ordinals `0, 1, 2, 4, 8, 16, 32` to distinct slots `0..6`.
- Add a minimum-fraction update helper over an `AtomicU32` initialized to positive infinity; accept only finite values in `[0,1]` and retain the numerical minimum.
- Extend each sampled query record with a scalar minimum fraction; preserve the existing total candidate callback counter.

- [ ] **Step 1: Write failing policy tests**

Test that the selector maps exactly the seven configured ordinals and returns `None` for adjacent unscheduled ordinals. Test that minimum tracking accepts `1.0`, accepts `0.0`, replaces larger values with smaller values regardless of arrival order, and ignores NaN, positive/negative infinity, and values below zero or above one.

- [ ] **Step 2: Run the policy tests and confirm the expected failures**

Run: `cargo test --lib world_query_candidate_capture_policy`

Expected: compilation/test failure because the selector and minimum-tracking policy do not yet exist.

- [ ] **Step 3: Implement fixed-slot selection and scalar-only fraction tracking**

In `src/game_probe.rs`, increment the existing callback count for every associated callback. Perform pointer, transform, vtable, and before/after record reads only when the callback ordinal maps to one of the seven fixed slots. For every callback, call the original exactly once, update the scalar minimum only for finite in-range fractions, then finish the detailed record only when a slot was selected. Remove rolling-slot overwrites and the best-candidate deep-copy slot. Add the total and minimum scalar to the sample log.

- [ ] **Step 4: Run the policy tests and formatting checks**

Run: `cargo test --lib world_query_candidate_capture_policy`

Expected: all selector and minimum-fraction policy tests pass, including the signed-zero boundary case.

Run: `cargo fmt --check`

Expected: clean formatting.

- [ ] **Step 5: Build and inspect the final diff**

Run: `cargo build --release` and `git diff --check`.

Expected: release build succeeds, diff check is clean, retained candidate detail is capped at seven records per query, callback count/minimum include unscheduled callback returns, and the detour still forwards the original return unchanged. Do not install or deploy this build in the current session.
