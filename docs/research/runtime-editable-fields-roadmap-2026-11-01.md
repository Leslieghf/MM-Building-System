# Runtime editable-fields roadmap

## What the content inventory can answer

The local content workbench is useful for finding candidate files and links, but it does not by itself reveal live runtime values or safely editable game-object fields. Useful names already surfaced include:

- **Camera:** `camera.xvmc`, `camera_conditions.smodc`, `camera_fsm.smodc`, and `camera_fsm_dfov.smodc`.
- **Weapon/damage:** `weapon.smodc`, `debug_do_damage.xvmc`, `char_damage_vehicle.xvmc`, `veh_damage_base.xvmc`, `weapons_common.fsb`, and per-weapon audio banks.
- **Vehicle configuration:** `.vengine` / `.venginec` engine resources, `.vpgeneral` / `.vpgeneralc` general-physics resources, `vehicle.xvmc`, and `vehicle.smodc`.
- **Max upgrades:** `mmax_head_upgrades.xvmc`, `player_upgrades.smod_eventsc`, `upgradeablevaluesmanager.xvmc`, `visibleplayerupgrades.xvmc`, and `gui_maxupgrades.fsb`.
- **Transform/offset leads:** `align_offset.xvmc` and `scripts/animationstatetasks/char_add_bb_offset_rotation.xvmc`.

These are path/name discoveries from the local master path list and compiled bundle-member index. The `.xvmc`, `.smodc`, `.venginec`, and `.vpgeneralc` suffixes indicate compiled resources; their existence does not mean they are editable source or direct runtime setters. Audio banks are not weapon-stat data. Follow the exact asset/package reference graph and inspect caller behavior before assigning semantics to any field.

## Focused compiled-script probes

Selected read-only previews now show more than filenames:

| Compiled resource | Strings/symbols observed | What it supports | What it does not establish |
| --- | --- | --- | --- |
| `scripts/upgrades/acceleration.xvmc` | `GetEngineWeightAdjustment`, `GetEnginePowerPeakRpm`, `GetEngineTorquePeakRpm`, `GetUpgradeEngineTorqueScale`, `GetUpgradeEngineResistanceScale`, `ChangeMass`, `ChangeAccelerationScale`, `SetRevStartupTime`, `ActivateEnhacementLevel`, `DeactivateEnhacementLevel` | There is a game-script vehicle-upgrade path with named engine stat operations. This is the strongest current next target for vehicle tuning. | No arguments, receiver layout, timing, safety bounds, or direct Rust-callable ABI have been recovered from printable strings alone. |
| `scripts/gameobjects/vehicle/damagesystem/body_health_state.xvmc` | `GetBodyHealth`, `GetMaxHealth`, `SetHealth`, `UpdateHealth`, `DropAllArmor` | The vehicle body-health system routes health operations through script/game-object libraries. | This is object health, not weapon damage tuning; no Rust binding or general GameObject target handle exists yet. |
| `scripts/gameobjects/vehicle/damagesystem/bullet_damage_handler.xvmc` | `HitByBullet`, `WeakspotOnHitByBullet`, `SetHitByPlayer`, plus `Damage`, `Impulse`, `AmmunitionType`, `ShapeKey`, `PartIndex` | Bullet impacts pass structured damage and impulse information through a named handler. This is a concrete route for learning damage flow. | The strings do not show where damage is authored/scaled or prove that any field can be safely changed. |
| `scripts/animationstatetasks/char_add_bb_offset_rotation.xvmc` | `GetTransform`, `BlackBoardVector`, `RotationOffset`, `RotationMatrix`, `AngleBetween`, `MaximumAdjustment`, `RotateInstantly` | This is an animation-state/bounding-box offset-and-rotation task, broadly included in packages. | Despite the word “offset,” it is not evidence of a general world-transform or freecam setter. |
| `vehicles/engines/cam_gen2.venginec` | `EngineDesc`, `RevSoundEvent`, `RpmFilter`, `DamageScale`, `DamageThreshold`, `RpmWeight`, `VelocityWeight`, and sound/load filter names | This particular vehicle-engine resource is visibly tied to engine audio behavior and its response parameters. | These strings should not be mistaken for the car's torque, mass, gearing, or handling values. |

The health and bullet-handler samples above were read from the compiled member data inside the Rockstar ATV `.ee` package; each member path appears in 353 bundle owners in the generated index. That indicates shared package use, not a count of live objects. The `acceleration.xvmc` path is a standalone compiled script and its preview is under the ignored `target/content-index/previews/` folder. The selected `.venginec`/`.vpgeneralc` family counts in the local index are 21/22 unique paths with 1,728/401 bundle occurrences; those are resource counts, not editable live properties.

## Current live bridge boundary

The current script API can read copied camera, player, and vehicle pose/velocity/acceleration values. It can request a bounded offset/rotation on the existing game camera getter and submit a bounded current-vehicle velocity write. It does not expose a detached camera, player transform setter, weapon damage/rate setter, vehicle engine/handling tuning, or Max upgrade/equipment setters.

The `engine_workbench` package is now a 16-page, one-panel Sandbox Suite. Its live probes cover frame/sample telemetry, copied player/vehicle/camera state, camera pose override, continuous deferred camera ray and visualization, one-shot catalog spawn at a fresh hit, supported debug-draw variants, coordinate comparisons, panel events, and bounded vehicle velocity writes. Side effects are page/toggle gated. This supports a broad in-game pass without claiming unsupported native calls.

The remaining pages are WIP system cards for vehicle tuning, weapons/damage, Max upgrades, GameObjects/Havok, NPC/animation, and effects/assets. They list the concrete compiled-resource names from the focused previews and the exact boundary of the current bridge. Compiled script symbols remain evidence for where to investigate, not Rust-callable controls.

## Efficient investigation order

1. Use the new panel to recheck camera pose adjustment and vehicle velocity writes alongside live telemetry.
2. Inspect the compiled camera/weapon/upgrade candidates for referenced object/class names and connected `.ee` records; keep each result in the asset workbench's provenance/dependency model.
3. Trace a single existing UI-driven weapon or upgrade action from its compiled script/resource references to the native receiver and state field.
4. Only after a getter/setter candidate is identified, add a bounded experimental control to the corresponding workbench page and validate several values in one gameplay session.
5. Promote a control from experimental only after its effect is observed and repeatable; preserve a reset path for every persistent or high-impact value.

## Current workbench surface

The hot-reloadable `engine_workbench` package now publishes a 16-page **Mad Max Sandbox** suite. Live pages cover overview, frame/sample telemetry, camera pose adjustment, deferred camera raycasts, player and vehicle readouts, catalog spawning at a fresh ray hit, supported gizmo variants, coordinate comparisons, and mod UI/input-sampler availability. Vehicle velocity and one-shot jump controls remain bounded and require a current vehicle sample. Raycasts, gizmos, and camera writes stop when their page/toggle is inactive. A spawn operation freezes its selected catalog ID when requested, waits for a fresh successful hit, and submits at most one catalog request.

Vehicle tuning, weapon/damage, Max upgrades, GameObjects/Havok, NPC/animation, and effects/assets are visible as WIP system pages. They identify research leads and missing adapters without suggesting the game-script symbols are callable APIs. No native API or generated API reference changed for this suite. Static validation found 16 selector choices, 135 unique widget IDs, and no duplicate widget IDs. The deployed Rhai source hash is recorded in `docs/research/install-history.md`; final compilation and behavior are still checked by loading it in-game.
