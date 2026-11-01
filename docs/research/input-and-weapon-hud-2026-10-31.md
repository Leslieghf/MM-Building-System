# Input ownership and weapon HUD reconnaissance — 2026-10-31

## Current mod input boundary

- Rust-owned keyboard/mouse state is represented by `InputFrame` and sampled at the overlay. Script `ctx.input` reads this mod-owned snapshot; it does not query Mad Max's native input system.
- General runtime input sampling was disabled (`ENABLE_RUNTIME_INPUT_SAMPLING = false`) after earlier crash investigations. The movement build enables the existing sampler only while F1 overlay input capture is active and no text field is focused; outside that state it publishes an empty unavailable frame so held keys cannot stick. This samples ImGui-observed keyboard/mouse state; it is not XInput/controller state. The first candidate crash was a missing `PendingUiInput` resource in the separate UI Bevy world, not evidence against the sampler; that resource is now initialized. Input behavior itself still needs in-game validation.
- Hudhook 0.9.3 queues window messages for asynchronous processing by the ImGui render pipeline. Its `BeforeWndProc::Break` skips Hudhook's ImGui message interpretation for that queued message. The subclass procedure has already queued the message and calls the original game's WndProc unless the previous frame's coarse `MessageFilter` blocks that message class. `MessageFilter::InputAll` is category-wide, not a per-key mask.
- Therefore, the current `before_wnd_proc` key branch is useful for overlay state/ImGui routing, but it is not evidence that the native game never sees that key. Per-key blocking cannot be promised by adding a Rhai method to the current message hook.

## Executable input leads

Ghidra `/MadMax.exe` currently shows:

- The import table lists `DirectInput8Create` from `DINPUT8.DLL`, `GetKeyState` from `USER32.DLL`, and `XInputGetState`/`XInputSetState` from `XINPUT9_1_0.DLL`.
- The import table also includes `GetProcAddress` and several `LoadLibrary*` APIs. The static import list contains no `GetAsyncKeyState` or Raw Input API names, but that does not exclude the game dynamically resolving additional input APIs at runtime.
- Dynamic/API name strings `XInputGetState`, `XInputSetState`, and `GetKeyState` near `0x1432C92FC`, `0x1432C92EA`, and `0x1432C8C4C`.
- RTTI names `CKeyboardRouter` and `IKeyboardRouter@Base` near `0x1417BA0C0` and `0x1417B9E50`.
- Additional RTTI/classes: `CInputActionOverride`, `CInputDevice`, `CInputDeviceManager`, `CKeyboard`, `CGameGui`, `CGameGuiCallbacks`, `CGuiMessageQueue`, `IGuiMessageModifier`, and `CFullScreenGuiController`. Ghidra's class-member inventory currently associates no named methods with `CInputActionOverride`, `CKeyboardRouter`, `CWeaponHolder`, `CWeaponSequencer`, `CGameGui`, or `CFullScreenGuiController`.
- Named engine/script entry strings include `GetButtonHashFromInputAction`, `GetButtonInput`, `GetRawInput`, `GetRawMoveInput`, `GetFilteredMoveInput`, `InitInput`, `DebugInputHistory`, and `GUIXInputControl`. These strings identify registered interfaces or functions, but are not decompiled function bodies.
- These establish multiple investigation leads, not which API currently receives the player's keyboard bindings, how the game combines keyboard/controller state, or where a safe suppression point lives. The currently mapped Ghidra class has no associated named member functions, so there is no proven `CKeyboardRouter` call surface yet.

## Weapon and HUD registry leads

Ghidra's engine-facing string table contains `CInputActionOverride` as a game-object type, `CStateContainerDataWeapon`, `StateContainerDataWeaponInputFlagsCleared`, `...SetFireInputs`, `...SetAimInputs`, `...SetThrowInputs`, `...SetQuickFireInputs`, and `...SetMiscInputs`. These names strongly suggest Mad Max has an action/state layer where weapon inputs are represented separately from physical devices. They do **not** establish a safe setter or demonstrate that a mod can consume those actions. Exact argument layouts and lifecycle are still unknown.

Other GUI/HUD names include `GUIXSetHUDElementState`, `activate_hud`, `deactivate_hud`, `Hud.HudGeneralNotifications`, `Hud.HudMiniMap`, `Hud.HudHealthBars`, `Hud.HudLootIcons`, `Hud.HudCombatPrompts`, `Display.HUDOpacity`, `gui_hidden_%s`, and `gui_shown_%s`. `CGameGui`, `CGameGuiCallbacks`, `CGuiStreamer`, `CGuiMessage`, and `CGuiMessageQueue` provide likely ownership layers for notifications and GUI streaming. The weapon wheel/slot renderer still has no exact name in the current string/class search. `CWeaponSelectedCondition` and `CHasWeaponInSelectedSlotCondition` are state conditions, not evidence of a wheel renderer.

`GUIXSetHUDElementState` and the `Hud.*` setting strings are promising for a later HUD probe, but there is no current script binding or confirmed signature. Do not call them through guessed ABI arguments. A safe next step is to identify their registration metadata and observe the effect of the game-owned HUD state functions before considering a Rust wrapper.

## Weapon UI leads and limits

String/RTTI reconnaissance finds `CWeaponHolder`, `CWeaponSequencer`, `CWeaponSelectedCondition`, and `StateContainerDataWeaponSelectBestScreenSpaceTarget`/`...ZoomedScreenSpaceTarget`. Searches for plain `WeaponWheel`, `WeaponSelect`, `WeaponSlot`, `RadialMenu`, and obvious HUD names did not reveal a clear weapon-wheel renderer or registration path. This does not mean the UI has no owner; symbols may be absent or named through GUI state/layout resources.

No claim is made that overlaying the weapon wheel is “easy.” A convincing overlay needs either a mapped native HUD suppression/ownership point or a deliberate separate mod UI. Replacing the wheel, intercepting input, and adding a true native weapon slot are distinct capabilities.

## Recommended implementation boundary

Keep the concepts separate in the script API:

1. **Mod input:** `ctx.input` reports keys observed by the mod runtime, with documented sampling source and edge lifetime.
2. **Native input policy:** a different capability lets a script request that specific native actions be consumed. It is unavailable until a synchronous/native input interception point is identified and manually validated. A key being visible to Rhai does not imply that Mad Max's corresponding action is suppressed.
3. **UI focus:** UI pointer capture is separate from keyboard action suppression. F1's existing coarse overlay capture filters keyboard/mouse message classes while the movement workbenches consume their mod-owned input frame. This does not prove suppression of polling-based DirectInput/XInput or support per-action blocking.

The public script catalog now explicitly documents that `ctx.input` is mod-runtime input and that `ctx.native_input` does not exist yet. The script spawn API has a separate `game.spawn` capability and currently accepts only the verified stockpile catalog item. This is the first bounded script-owned world effect; it does not solve native key suppression or script-panel mouse focus.

The next research pass should trace DirectInput/XInput acquisition and `CKeyboardRouter` callers, inspect message/raw-input routing, then locate the weapon selector HUD/control state. Only after that should we expose a native-action block request or replace the native weapon display. Meanwhile the script-owned spawn API and asset catalog can proceed independently.
