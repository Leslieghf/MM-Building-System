# Source-style Key Binds Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `bind g "command args"` execute the quoted console command on G without re-enabling the crash-prone all-key sampler.

**Architecture:** Store each binding as a bounded command string in the UI-owned console sender. At render time, check press events only for currently bound keys and pass triggered strings through the same console handler as typed commands. Leave the full `InputFrame` sampler and mailbox disabled.

**Tech Stack:** Rust, `imgui-rs` via Hudhook, existing bounded console channels.

**Spec:** `docs/superpowers/specs/2026-10-29-source-style-key-binds-design.md`

## Global Constraints

- Keep F1, F2, and F3 reserved for the existing mod controls.
- Keep command input bounded to 256 UTF-8 bytes.
- Do not add native input hooks or restore all-key per-frame polling.
- Keep binds local to the render-side console and route game commands through the existing bounded request channel.
- Do not add or run automated tests unless the user asks; run format/build checks and perform the requested one-launch validation.

## Review Focus

- Outer quotes and whitespace: only the quoted command text is stored; empty or over-limit values are rejected.
- Command-string dispatch: the same typed and local command handlers run, and unknown commands report the normal parser error.
- Key press edge: held keys do not repeat; keyboard and mouse use their respective ImGui press APIs.
- Reserved controls: F1/F2/F3 cannot be rebound.
- Runtime input boundary: general script input snapshots remain disabled and no input mailbox publication is reintroduced.

---

### Task 1: Store and dispatch source-style command bindings

**Files:**
- Modify: `src/scripting/console.rs` — bind command representation, parser, sender map, help text.
- Modify: `src/overlay.rs` — local bind UI, listing, per-bound-key edge detection, shared command dispatch.
- Modify: `docs/bootstrap-install-record.md` — record the candidate hash, install, and user launch result.

**Interfaces:**
- `ConsoleCommand::Bind { key: u32, command: String }` replaces the script-only payload.
- `ConsoleRequestSender::bindings() -> &BTreeMap<u32, String>` supplies the current key-to-command map.
- The render loop checks only those keys and calls the existing `submit_console_input` path for each command press.

- [x] Parse `bind <key> "<console command>"`; retain the exact inner command, reject empty or over-limit commands, and keep F1/F2/F3 reserved.
- [x] Change stored bindings and `show_binds` output from `ScriptId` to the command string.
- [x] Detect press edges only for bound keys; use ImGui's keyboard press API for keyboard keys and mouse-click API for mouse buttons.
- [x] Dispatch triggered text through the same local/queued console handler as typed commands; unknown commands produce the existing parse-error output.
- [x] Run `cargo fmt --check`, `cargo build --release --locked`, and `git diff --check`; do not run automated tests.
- [x] When the game is closed, install the candidate while preserving the current ASI; ask the user for one launch and a `bind g "script run vehicle_jump"` check. Restore the known-good ASI if startup fails.
