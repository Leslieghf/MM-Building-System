# Mad Max Sandbox Suite

This expands the existing `engine_workbench` package into one compact, hot-reloadable suite. In Dev Tools → Scripts, rescan if needed, then load or reload `engine_workbench`. It publishes one selector-driven **Mad Max Sandbox** panel.

## Live pages

- **Overview / Telemetry:** frame timing, native sample age, and copied camera/player/vehicle values.
- **Camera:** bounded world offset and yaw/pitch/roll on the existing camera getter. This follows the game's camera and is not detached freecam or a GameObject transform setter.
- **Raycast:** continuous deferred camera ray while enabled on this page, with filter/range, hit/fraction readout, and optional line/hit visualization. Turning it off or leaving the page stops new requests and draw calls. Disable other continuous ray test scripts during an isolated performance check.
- **Vehicle:** copied motion values, bounded velocity sliders, copy-current-velocity, and a one-shot +18 Y write. Writes occur only on button events and with a current vehicle velocity sample.
- **Player:** copied actor values; transform/health/upgrade writes remain unmapped.
- **Spawn probe:** one fresh camera query on button press, then one catalog-approved spawn request only after a hit. Available choices are the verified stockpile and the experimental roadkill lootbox. A request-submitted message is not proof the game completed the spawn; check the console lifecycle report. A pending operation can be cancelled before submission.
- **Gizmo probe:** draws the mapped axes, line, arrow, wire/x-ray spheres, AABB, wire/x-ray boxes, solid cube, and solid 3D arrow 14 units in front of the current vehicle or Max. It is disabled by default and runs only on this page while checked.
- **Math / orientation:** compares camera view, actor facing, and the documented game coordinate basis.
- **Input / UI:** reports the mod-owned input sampler's availability and includes a UI-button event probe. Keyboard/mouse sampling is active only while F1 capture owns input and no text field is focused; it is not native game/controller input.

## WIP pages

Vehicle tuning, weapons/damage, Max upgrades, GameObjects/Havok, NPC/animation, and effects/assets show concrete compiled-resource leads and the next missing bridge operation. Compiled names such as `ChangeMass` or `SetHealth` are game-script symbols, not Rust-callable ABI guarantees. These pages intentionally contain no fake sliders.

The system status colors retain the project's meaning: green for stable, yellow for a real experimental mechanism, red for a missing/unmapped runtime route.

## Cost and side effects

The suite uses one panel and creates only the selected page's widgets. Raycasts, gizmos, and camera override requests are gated by their page and explicit toggle; vehicle writes and spawn submissions are button-driven. Ray state is deferred and ticketed, spawn requires a fresh successful hit, and the spawn queue remains bounded by the host. The suite does not add native code or change the generated API reference.

The final Rhai compile check occurs when the game script manager loads/reloads the package. No in-game runtime validation is claimed until the user loads it.
