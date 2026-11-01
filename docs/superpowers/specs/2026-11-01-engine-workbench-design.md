# Engine Workbench Design

## Intent

Give the user one compact, hot-reloadable in-game control surface for piloting and probing the game bridge. The near-term win is testing several known controls and telemetry values in one gameplay session, while making unavailable engine controls visible as research gaps instead of pretending they work.

## Approaches considered

1. **Add all controls to the spawn menu's Tools side panel.** This has the tightest spatial use, but ties camera, motion, and future weapon/player experiments to spawn-menu lifecycle and navigation.
2. **Publish one script-owned Engine Workbench window with a module selector.** This uses the existing bounded script-panel API, stays hot-reloadable, and keeps one compact surface. It is the selected first step.
3. **Build a Rust-native general inspector/editor.** This could provide richer docking and reusable typed inspectors, but requires a new native UI framework before the game APIs are understood.

## Goals

- One panel, one compact module selector, and dense values/controls; no pile of floating experiment windows.
- Camera page exposes the existing bounded camera pose override and availability state.
- Vehicle page shows copied position, forward, velocity, acceleration, and provides bounded velocity edits plus a one-shot upward impulse.
- Player page shows currently copied transform/position/forward/velocity/acceleration when present.
- Vehicle-tuning, weapon/damage, and Max/upgrades pages show concrete compiled-script leads found in the indexed content while stating that the Rust bridge has no corresponding live calls yet; no decorative controls that do nothing.
- Keep each operation script-owned, hot-reloadable, and guarded by declared capabilities.

## Non-goals

- Claim detached freecam or native camera-input ownership. Existing camera control only adjusts the game camera getter result.
- Change weapon damage, vehicle specifications, or Max upgrades until a real getter/setter route is mapped and validated.
- Add raw engine pointers, arbitrary memory writes, or changes to the spawn menu.
- Add new native backend APIs in this slice.

## Constraints and behavior

- Use the existing script panel API: one stable panel ID, at most 48 widgets, and copied UI events delivered to a later `game.post_sim` callback.
- Camera pose requests refresh while enabled and clear when disabled or on script unload.
- Vehicle changes use `ctx.vehicle.set_velocity`; no write is issued when the copied vehicle sample is unavailable.
- All camera and velocity bounds remain enforced by the native/script adapters.
- Render the panel from the `engine_workbench` package only; do not duplicate it in another UI surface.
- Experimental values and unmapped pages use the project's green/yellow/red semantics accurately.

## Success criteria

- The script package discovers, loads, and hot-reloads using the existing script manager.
- One workbench panel switches among Camera, Vehicle, Player, Weapons, and Max pages without creating more windows.
- Camera controls preserve the current override behavior; vehicle velocity controls use the existing bounded write path.
- Unsupported weapon/upgrades controls are plainly unavailable, while live copied data remains useful.
- The widget count remains under 48 and the panel's output is compact enough for normal gameplay.
