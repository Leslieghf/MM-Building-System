# Mad Max Sandbox Suite

## Intent

Expand the existing `engine_workbench` into a compact in-game sandbox suite for validating many bridge mechanisms in one gameplay session. The user wants a GMod-like ability to pilot and inspect the game, and has explicitly asked for breadth: working experiments where APIs exist and clearly marked TODO/WIP coverage where they do not.

## Approaches

1. **One selector-driven script panel.** Keep one window and switch among a bounded set of domains. This fits the current one-to-four-panel script API, avoids floating-window clutter, and lets one package own shared camera/raycast/spawn state.
2. **Many separately loaded micro-playground packages.** This maximizes modularity and allows hot-loading only selected tools, but spreads controls across many windows and duplicates state and query work.
3. **Add native Rust adapters for every desired system before building UI.** This offers a stronger long-term typed API, but delays the user's request to try many things and encourages unvalidated engine calls.

The selected first implementation is **one selector-driven panel in the existing `engine_workbench` package**. It preserves the package identity and existing real controls while adding pages for all currently supported mechanisms and concise WIP cards for unmapped systems. No new native API is added in this slice.

## Goals

- Keep one compact panel with at most 16 selector pages, below the API's 48-widget per-panel cap.
- Make already-mapped mechanisms easy to exercise: copied telemetry, camera pose override, continuous camera ray, one-shot catalog spawn, bounded vehicle velocity write, and the supported debug-draw variants.
- Display frame/sample diagnostics and math/orientation comparisons that help validate more than one subsystem during a session.
- Mark input sampling as unavailable when the runtime says it is inactive; demonstrate script-panel events without implying that game input was sampled.
- Provide WIP pages for vehicle tuning, weapons/damage, Max upgrades, GameObjects, Havok, NPC/animation, effects/audio, and asset inspection. Each WIP page must cite concrete content leads or an exact missing API, and must not show nonfunctional controls as if they work.
- Keep side effects gated by explicit controls and the relevant selected page. Camera override clears when disabled; continuous raycasts and debug geometry stop outside their selected/active page; spawning remains a one-shot request.
- Reuse the existing script package and hot-reload path; no game restart or ASI build should be needed.

## Non-goals

- No arbitrary memory access, new FFI calls, or unverified native setters.
- No claim of detached freecam, general GameObject transform editing/removal, weapon damage control, car tuning, Max upgrade setters, Havok body operations, NPC animation control, or effect/audio spawning.
- No replacement of the GMod-style spawn browser or asset gallery.
- No always-on per-frame raycast or gizmo work when the matching page/checkbox is inactive.

## Page contract

The fixed page list (maximum 16 selector choices) is:

1. Overview
2. Telemetry
3. Camera
4. Raycast
5. Vehicle
6. Player
7. Spawn probe
8. Gizmo probe
9. Math / orientation
10. Input / UI
11. Vehicle tuning
12. Weapons / damage
13. Max upgrades
14. GameObjects / Havok
15. NPC / animation
16. Effects / assets

The suite publishes only one stable panel ID. Each page adds widgets only while selected. The ray page uses one camera query from `game.post_sim` while enabled and polls the previous deferred ticket. Spawn is a two-stage user action: one camera query on button click, followed by one catalog-approved spawn submission only after a hit. Gizmo drawing is bounded and only active while its page and toggle are active. The vehicle page uses the already bounded velocity write and only sends one write per button edge.

## Evidence and status vocabulary

- **Stable/green:** already observed working or a read-only copied value with a documented contract.
- **Experimental/yellow:** a real bounded adapter or query exists, but effect/edge behavior is still being validated.
- **Problematic/red:** the requested runtime mechanism is not mapped, even if candidate game assets or script symbols exist.

Compiled script symbols such as `ChangeMass`, `SetHealth`, or `HitByBullet` are research leads. They do not imply that the Rhai bridge can call those methods.

## Success criteria

- The existing package can reload and presents all 16 pages in one window.
- Existing camera and vehicle controls retain their bounded behavior.
- A user can turn on a continuous camera ray and inspect hit state/visualization, then disable it and observe work stop.
- A user can submit a one-shot stockpile or experimental lootbox spawn request using the selected ray hit.
- The gizmo page can render the supported shape variants from a useful player/vehicle-relative location while remaining disabled by default.
- Every unmapped system is visible as a WIP page with accurate evidence and no pretend control.
- Package source and manifest are copied to the installed scripts directory; the game script loader is the final Rhai compilation/runtime check.

## Open questions for later slices

- Which game-script invocation route can safely drive `SetHealth` or vehicle-upgrade methods against a selected live object?
- Can the input sampler be activated safely, and can a script receive input without stealing native game controls?
- What native `CGameObject` handles and lifecycle APIs support per-instance selection, transform editing, and removal?
- Which compiled vehicle configuration fields represent actual handling/performance rather than engine audio response?
