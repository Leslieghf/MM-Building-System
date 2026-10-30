# Console and UI Comfort Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development.

**Goal:** Polish console output and command entry, then align UI capture/configuration controls with the requested hotkeys.

**Architecture:** Keep console state in the existing UI Bevy world. Local presentation commands such as `clear` execute in the render callback; game-affecting commands continue across the existing bounded request channel. Keep Hudhook's category-wide input filter and defer real per-key binds until selective Raw Input interception is established. The user requires bound keys to suppress their native Mad Max actions.

**Tech Stack:** Rust 2024, Bevy ECS 0.19.1, Hudhook 0.9.3 / ImGui 0.12.

**Spec:** `docs/superpowers/specs/2026-10-29-console-and-ui-comfort-design.md`

## Global Constraints

- Keep the split UI/game worlds and their bounded owned messages unchanged.
- Do not add or run automated tests; use formatting, source review, and a locked release build.
- Do not deploy until the game is closed; do not launch Mad Max for verification.
- Preserve integer counters/IDs as integers; fixed precision applies to floating-point metrics.
- Do not claim Source-style key bindings until bound keys are proven exclusive across normal and raw input.

## Review Focus

- Invalid/overlong command submission: prompt still clears and the error remains visible.
- ANSI input: only the supported SGR subset changes text appearance; malformed/unsupported controls cannot affect console state.
- UI opacity at lower/upper slider limits: text remains readable and every panel responds consistently.
- Escape while capture is active: key and raw-input categories stay filtered until Escape key-up; only then does Mad Max regain input focus.
- F1/F2/F3 state transitions: hint text, cursor state, panel visibility, and capture state agree.

---

### Task 1: Console behavior and rendering

**Files:** `src/scripting/console.rs`, `src/overlay.rs`, `src/runtime.rs`

- [x] Add `clear` as a local console command/output action and display command parse/transport failures in severity colors.
- [x] Clear submitted prompt contents whether parsing/enqueue succeeds or fails; keep bounded history behavior and visibly reject overlong input rather than dispatching its truncated prefix.
- [x] Render bounded ANSI SGR foreground/bold/reset spans in console output.
- [x] Remove API List, Help, and per-row Status buttons; retain Refresh scripts.
- [x] Audit floating telemetry so every float uses fixed three-decimal truncation; preserve natural integral forms.
- [x] Run formatting check and locked release build; inspect diff for queue/thread-boundary changes.

### Task 2: UI controls and opacity

**Files:** `src/runtime.rs`, `src/overlay.rs`

- [x] Add a bounded opacity preference and apply it to HUD, configurator, and console backgrounds.
- [x] Add the configurator slider.
- [x] Bind F1 to capture toggle, F2 to configurator, and F3 to console; update visible hints and close behavior.
- [x] Ensure Escape remains handled by the overlay while capture is active, including Hudhook's keyboard/Raw Input filter.
- [x] Keep game input captured until the Escape key-up is observed, then restore Mad Max input focus; candidate build completed, in-game validation pending.
- [x] Run formatting check and locked release build; inspect final diff.

### Follow-up: mod-owned input and bindings

- [x] Keep F1/F2/F3 hardcoded as capture, configurator, and console controls.
- [x] Sample keyboard and mouse buttons from the existing Hudhook/ImGui input stream into a bounded Rust `InputFrame` resource with down/just-pressed/just-released state.
- [x] Forward snapshots to the game callback through a bounded, nonblocking mailbox and expose `ctx.input` to scripts only with `input.read`.
- [x] Implement local `bind <key> script run <id>`, `unbind`, `show_keys`, and `show_binds`; reserve F1/F2/F3.
- [ ] In-game verification of input edge state and one bound `on_run` callback.

These bindings are mod-owned and do not suppress selected Mad Max actions. With global input capture off, a binding can coincide with a native action. Per-key suppression and game-specific input polling remain out of scope; global UI capture continues to suppress all game input while the mod owns focus.
