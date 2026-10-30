# World Query Caller Activity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add bounded per-caller query frequency summaries to the next diagnostic build.

**Architecture:** Increment a fixed atomic counter at the existing query-call tap for each registered return address. The existing background caller logger computes five-second deltas and logs only non-empty windows, sorted by descending call count and limited to twelve sites.

**Tech Stack:** Rust atomics, existing background logger, Windows x64 ASI.

**Spec:** `docs/superpowers/specs/2026-10-28-world-query-caller-activity-design.md`

## Global Constraints

- Keep the call hook allocation-free and do no log I/O on the game thread.
- Preserve all existing sampling caps and the shared helper return handling.
- Do not launch, close, or modify the running Mad Max process.
- Do not label helper Boolean values as physical hit/miss results.

## Review Focus

- Unknown/out-of-image caller addresses: preserve the existing caller-table behavior and report non-game addresses without underflow.
- Counter increments racing with the logger snapshot: use monotonic relaxed atomics and carry any increment not included in a snapshot into the next window.
- Idle query periods: omit empty windows so the log stays quiet.
- Large caller sets: cap output at the top twelve while keeping counters for all 256 registered slots.
- Long sessions: use saturating delta arithmetic and avoid per-call output growth.

---

### Task 1: Add fixed caller counters and periodic activity summaries

**Files:**
- Modify: `src/game_probe.rs`
- Spec: `docs/superpowers/specs/2026-10-28-world-query-caller-activity-design.md`

**Interfaces:**
- The existing `record_world_query_caller(return_address) -> Option<usize>` assigns a stable slot in `WORLD_QUERY_CALLERS`.
- `record_world_query_call` increments that slot's counter once for each observed helper invocation.
- `world_query_caller_log_worker` reports counts from the existing background worker every five seconds.

- [ ] Add `WORLD_QUERY_CALLER_CALLS: [AtomicU64; WORLD_QUERY_CALLER_CAPACITY]` and increment the returned slot in the tap with `Ordering::Relaxed`.
- [ ] Add a background-only aggregation helper that compares current counters with a fixed previous-count array, sorts nonzero deltas, and logs one line with the top twelve caller RVAs and counts.
- [ ] Run `cargo fmt --check`, `git diff --check`, and `cargo build --release`; do not run tests for this instrumentation-only change.
- [ ] Review the diff to confirm formatting, sorting, and log writes occur only on the background worker and the game process was not touched.
