# Vehicle Dynamics Readouts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add native vehicle angular-velocity and previous-frame speed magnitudes to the existing debug panel.

**Architecture:** Sample both vectors from the existing `UpdatePostSim` game callback. Resolve the SDK-derived primary-vtable slots 57 and 59, validate executable code and the returned vector pointer, copy finite values into the existing vehicle snapshot, and display magnitudes in the Vehicle section.

**Tech Stack:** Rust 2024, Windows MSVC release target, existing ImGui overlay.

**Spec:** `docs/superpowers/specs/2026-10-27-vehicle-dynamics-design.md`

## Global Constraints

- Keep callback sampling interval, UI layout, and player readouts unchanged.
- Do not guess physical units; display scalar magnitudes.
- Do not launch Mad Max; confirm it is closed before deployment.
- Do not add or run tests unless requested.

## Review Focus

- Vtable slot mismatch or wrong ABI can crash in-process; slots are SDK-source-derived, not runtime-verified.
- `GetVelocityLastFrame` may return null or unreadable storage; validate before copying.
- Reject non-finite vectors and keep overlay snapshots independent of game-owned memory.

---

### Task 1: Sample and display vehicle dynamics

**Files:**
- Modify: `src/game_probe.rs`
- Modify: `src/overlay.rs`
- Include: `docs/superpowers/specs/2026-10-27-vehicle-dynamics-design.md`

**Interfaces:**
- Extend the vehicle snapshot with copied angular-velocity and previous-frame-velocity vectors.
- Expose the values through `vehicle_sample()` for overlay formatting.

- [ ] Add slot constants 57 and 59 and an SDK-compatible function type for the two getters.
- [ ] Read angular velocity via an output vector; call previous-frame velocity getter, validate its returned pointer, then copy three finite components.
- [ ] Publish copied values with the existing vehicle snapshot and render only magnitudes with neutral labels.
- [ ] Format Rust and build using `cargo build --release --target x86_64-pc-windows-msvc`; inspect `git diff --check`.
- [ ] Confirm Mad Max is closed, back up the installed ASI, deploy the build, verify hashes, commit, and push.

**Completion check:** Release build succeeds, deployed ASI hash matches the build, and the branch is clean after push.
