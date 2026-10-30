# Mod-owned Input and Console Bindings

**Updated:** 2026-10-29  
**Status:** Runtime-input design selected; implementation remains open

## User direction

The Rust mod runtime owns its input API. Read keyboard input from the existing Hudhook/ImGui window-input stream, publish an owned input frame to Bevy, and expose it to Rhai scripts. Do not reverse-engineer Mad Max's internal keymap, DirectInput device state, XInput, `GetKeyState` polling, or gameplay action classes to populate this API.

F1, F2, and F3 are hardcoded and already implemented: F1 toggles global UI/game-input capture, F2 opens the UI configurator, and F3 opens the script console. Escape closes a mod window while capture remains active until Escape key-up. The user tested the installed candidate and confirmed this fixes Mad Max's pause-menu leak.

## Existing input route

Hudhook 0.9.3 hooks the game's window procedure and queues messages for ImGui. Its synchronous filter is category-wide: it either blocks all keyboard/mouse/Raw Input categories or allows them through to the saved game procedure. ImGui interprets queued messages during the render pass. This is already the mod's Rust-side input stream; the game is not the source for script input.

The separate `CInputDeviceManager::m_IsInFocus` bridge globally gates Mad Max input while a mod panel owns focus. Keep this as the all-input suppression behavior. Do not add a Mad Max-specific or per-key hook for the current scope.

Ghidra lists `DirectInput8Create`, `GetKeyState`, `XInputGetState`, and `XInputSetState` imports. Import presence alone does not establish which path drives gameplay input. XInput is for controllers, not the physical keyboard. These are historical leads only; they are not prerequisites for the Rust input API.

## Implementation target

- Add an owned, bounded Bevy `InputFrame` with current-down, just-pressed, and just-released key state.
- Carry input from the UI/render side to the game callback side without blocking either host callback or sharing game pointers.
- Expose key queries to Rhai through an `input.read` capability, with stable Rust-defined key IDs.
- Add `bind`, `unbind`, `show_keys`, and `show_binds` using the same Rust key vocabulary. Bind only existing mod commands, beginning with `script run <id>`; never evaluate arbitrary shell text or raw Rhai from a key event. Reserve F1/F2/F3.
- Keep the console prompt and configured key names bounded. Treat repeated keydown messages as one press edge, and clear held state on focus loss so keys cannot stick.

Global UI capture suppresses all Mad Max input while the mod owns focus. With global capture off, Rust may still observe a key that Mad Max also processes; the current user direction does not require per-key native suppression. If that requirement changes, first try consuming the key at the Rust input boundary, without reverse-engineering Mad Max's keymap.

## References

- Hudhook 0.9.3 source: `renderer/pipeline.rs` (queued messages and synchronous category filter), `renderer/input.rs` (keyboard/Raw Input translation), and `renderer/msg_filter.rs` (input categories).
- Microsoft Learn: [XInput APIs](https://learn.microsoft.com/en-us/windows/win32/xinput/) describe controller input; [Using Raw Input](https://learn.microsoft.com/en-us/windows/win32/inputdev/using-raw-input) describes `WM_INPUT` device events.
- Runtime integration points: `src/overlay.rs`, `src/runtime.rs`, `src/modding.rs`, `src/scripting/api.rs`, and `src/scripting/console.rs`.
