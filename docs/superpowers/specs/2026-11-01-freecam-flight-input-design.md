# Free Camera, Hybrid Flight, and Input Boundary

## Intent

Give the user a practical way to inspect and move around Mad Max during one gameplay session. Reuse the existing camera-matrix override and vehicle velocity adapter, and make mod-owned keys usable without sending those keyboard/mouse messages to the game while F1 capture is enabled.

## Approaches

1. **Script workbenches over current adapters (selected).** Enable input sampling only while the existing F1 overlay capture owns input; extend the freecam script with keyboard movement; add a vehicle target-position controller that writes bounded velocity each callback. This ships useful movement without guessing new game ABI.
2. **Add transform setters immediately.** Directly teleports Max and vehicles and can provide true collision-free noclip, but neither `CPlayer` nor a general `CGameObject` transform setter is mapped. Calling guessed slots risks crashes and invalid Havok state.
3. **Hook native input APIs and register Mad Max actions.** This could support native keybind-menu entries and gamepad handling, but Ghidra evidence currently identifies candidate systems (`CKeyboardRouter`, `CInputActionOverride`, DirectInput/XInput imports) without a proven interception/registration contract.

## Goals

- Provide a free-camera workbench with bounded speed, keyboard translation, keyboard yaw/pitch, an explicit enable/disable control, and a reset path.
- Enable the existing mod-owned keyboard/mouse snapshot only while F1 overlay capture is active. Keep unavailable state explicit outside capture.
- Provide a hybrid flight panel that automatically controls the current vehicle using a script-owned target position and bounded velocity correction; stop issuing writes when disabled or no current vehicle is available.
- Show Max flight, true collision bypass, native input suppression beyond current F1 keyboard/mouse capture, and Mad Max keybind-menu integration as concrete WIP items.
- Keep controls separate from the 16-page general Sandbox Suite and hot-reloadable.

## Constraints and non-goals

- The camera route overrides `CCameraControlManager::GetCameraMatrix` output; it is not an independent renderer camera. The script computes a world position offset from the copied camera pose and uses additive Euler rotation.
- F1 capture filters overlay keyboard/mouse messages. It does not prove XInput/gamepad suppression, native action interception, or integration with the game's Controls menu.
- The vehicle adapter exposes only bounded `CVehicle::SetVelocity`; the flight panel is a position-hold velocity controller, not a teleport or collision-free noclip mode. Physics and collisions remain active.
- No guessed native vtable calls, player/vehicle transform writes, new direct input hooks, or arbitrary key suppression are introduced.
- Mod input is sampled only while overlay input capture is active, reducing idle work and avoiding deliberate game/mod dual-control.

## Interaction

The user loads `freecam_workbench` or `hybrid_flight` from Dev Tools → Scripts. F1 captures overlay keyboard/mouse input; WASD moves, Space/Left Ctrl move vertically, and arrow keys adjust camera yaw/pitch or flight direction. Each script has a UI enable switch, speed controls, copied state, and explicit status. Disabling a workbench stops its writes; camera disable clears the override. Hybrid flight selects a current vehicle automatically; on-foot Max mode remains informational until a player movement adapter is mapped.

## Later investigation

Trace the native `CInputActionOverride`/`CKeyboardRouter` registration path for controls-menu bindings and per-action suppression. Separately map a safe player `SetTransform`/movement route and the Havok synchronization/teleport lifecycle before implementing Max flight or true noclip.
