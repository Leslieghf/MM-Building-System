# Source-style key binding design

## Intent

Let a modder bind a key to a quoted console command, for example `bind g "script run vehicle_jump"`, and have the command follow the same console handling path as typed input. This replaces the current script-only bind syntax.

## Goals

- Parse and retain the quoted command text on the UI side.
- On a bound key press, submit that text through the existing console command handler so registered local and game commands behave consistently.
- Detect press edges only for keys that have bindings, using the ImGui press event path already used by F1/F2/F3.
- Preserve reserved F1/F2/F3 controls and the existing key-name list.

## Non-goals

- Re-enable full per-frame input sampling, the InputFrame mailbox, or script `ctx.input` reads. Those remain disabled while their crash cause is isolated.
- Add command chaining, shell expansion, or a new scripting expression syntax.
- Change global mouse/input capture behavior.

## Approaches

- **Store quoted console text and route it through the existing handler (chosen):** supports any command the console currently recognizes and naturally expands as commands are added. Unknown commands produce the same error as typed input.
- **Parse and store a typed command at bind time:** rejects unknown commands early, but requires the bind parser to know every command shape and makes later console additions less flexible.
- **Keep script-only binds:** smallest change, but does not match the requested Source-style behavior.

## Behavior and constraints

- Syntax: `bind <key> "<console command>"`; empty commands and command strings over the existing 256-byte limit are rejected. `unbind <key>` and `show_binds` remain.
- F1, F2, and F3 stay reserved.
- At render time, only currently bound keys are checked for a press edge. Mouse bindings use the corresponding ImGui mouse-click event; keyboard bindings use the ImGui key-press event.
- Key binds remain UI-owned and submit through the existing bounded console request path. No game callback or native input hook is added.
- Full input sampling remains disabled until the crash path is independently understood.

## Acceptance

- `bind g "script run vehicle_jump"` appears as `g -> script run vehicle_jump` in `show_binds` and requests the same run as typing `script run vehicle_jump` when G is pressed.
- A quoted command for another supported console command is dispatched through the same handler.
- Empty or over-limit commands and reserved keys are rejected; unknown commands report the normal console parse error when triggered.
- The new binary launches successfully with the main per-frame sampler still disabled.
