# Tenfold Ammo Ray Capture Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Capture up to 320 detailed ammo world-query samples in one run while preserving existing categories and a bounded fixed buffer.

**Architecture:** Keep the current allocation-free hook and static sample array. Increase the ammo ordinal limit from 32 to 320 and the array from 144 to 432 entries, which covers all existing reservations plus the 288 added ammo records and preserves 12 spare slots.

**Tech Stack:** Rust, existing atomic static sample storage.

**Spec:** `docs/superpowers/specs/2026-10-28-world-query-caller-activity-design.md` (retain the bounded, allocation-free capture constraints; this change intentionally updates its old sample-budget statement for the next candidate only).

## Global Constraints

- Keep capture storage fixed-size and allocation-free on the hook path.
- Preserve the existing 8 height, 16 GraphScript, 30 repeat-caller, 6 spaced-occlusion, and 40 first-per-caller sample reservations.
- Do not interact with the live game process.

## Review Focus

- The ammo sample ordinal must permit exactly 320 calls; verify the source comparison.
- The sample array must fit all planned reservations; verify 432 = 320 + 8 + 16 + 30 + 6 + 40 + 12.
- Existing sample indices and return-capture bounds must continue referencing the same capacity constant.
- Storage remains statically allocated and bounded.
- No live game or installed ASI changes occur in this task.

---

### Task 1: Expand the bounded ammo detail budget

**Files:**
- Modify: `src/game_probe.rs`

**Interfaces:**
- Retain `WORLD_QUERY_SAMPLE_CAPACITY` as the shared fixed sample-array bound.
- Retain `WORLD_QUERY_AMMO_SAMPLE_CALLS` as the per-run ammo ordinal counter.

- [x] Update the capacity comment and `WORLD_QUERY_SAMPLE_CAPACITY` from 144 to 432.
- [x] Update the ammo capture comparison from 32 to 320.
- [x] Check arithmetic and compile the release candidate without installing it into the open game.
