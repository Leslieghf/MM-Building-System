# Catalog Stabilization and Freecam Design

## Goal

Make the existing spawn catalog more useful without pretending unverified entities are safe, establish a controlled in-game test for Roadkill Lootbox, and build a separate path toward a usable camera workbench/freecam.

## Current facts

- The spawn adapter accepts only `props.scrap_stockpile_small`; the browser is generated from curated and community candidate data.
- Roadkill Lootbox is an inspected `.ee` gameplay composition with rigid-body, interaction, trigger, sound, loot, and graph-script components. Its current readiness is blocked because it is not a neutral prop.
- Most other catalog items are only names or archived bundle entries. Enabling all of them at once would turn unknown composition behavior into a game-wide crash/softlock risk and make failures difficult to attribute.
- The Rust bridge has a verified `CCameraControlManager::GetCameraMatrix` path. It has no verified camera setter or detached-camera implementation.
- The game is currently open. Candidate files may be prepared, but replacing the loaded ASI must wait for a user-confirmed game shutdown.

## Decisions

1. Keep readiness/evidence separate from spawn permission. Green means verified in-game. Yellow means a deliberate attempt that may work, fail, or destabilize the game; every catalog entry with a safe-relative asset path is yellow and spawnable. Roadkill Lootbox uses the same generic experimental status and attempt route.
2. Keep the native/script spawn boundary catalog-based rather than accepting arbitrary strings. Generate the allowed ID-to-asset-path table from catalog files. Red is reserved for entries with no usable asset path, so the game spawn function cannot be called for them.
3. Make the spawn menu dispatch the selected test item through the existing raycast and script-owned spawn path. The workbench records a pending catalog ID, performs one on-demand raycast, then submits exactly that ID at the hit position.
4. Keep catalog status evidence-based. Stable means observed and verified; experimental means a spawn attempt has a usable asset path but its runtime outcome is not established; red means there is no known spawn route. “All entries” means audit and classify all current entries, while allowing the user-approved yellow attempts.
5. Color language is strict and shared across the spawn browser: green means stable/verified; yellow means an attempt whose behavior may or may not work (experimental); red means no known spawn path. Yellow must not be used to say an item is known problematic.
6. Build freecam in stages. First add a separate, script-owned camera workbench that requests an additive pose override (world translation plus relative yaw/pitch/roll) on the existing game camera matrix. The native hook must call the original getter first, validate finite bounded values, and only then adjust its copied output. Disabling/unloading the script removes the override. This does not claim a detached camera: it is a camera-control experiment to see whether consumers honor the getter output.
7. Do not write directly into undocumented camera-manager memory or claim native-input suppression for the freecam. Continuous keyboard/mouse flight and complete detachment follow only after the override and control/input lifecycle are demonstrated.
8. Do not touch or restart the currently running game. Prepare a candidate build and request a close only when ready to install.

## First acceptance criteria

- Roadkill Lootbox appears in the browser with the same yellow experimental label as other unverified entries, and can only be requested through the exact allowlisted ID.
- It uses the existing one-shot camera ray/hit placement and reports native submission/observation separately; it is not promoted to stable on submission alone.
- Every green or yellow row routes through the catalog-backed generic entity spawn path; red rows have no usable path and remain disabled.
- A separate `freecam_workbench` script can enable/disable and tune a bounded offset/rotation override. Missing camera-control support is reported plainly rather than silently displaying a fake preview.
- Camera modification occurs only through a validated getter-output hook, never by exposing a game pointer to Rhai.

## Follow-up boundary

After user verification: collect spawn lifecycle observations for the lootbox; inspect its interactive behavior; test camera override against visible framing; then decide whether to proceed to detached camera state, mod-owned flight input, collision mode, and building tools.
