# GMod-Style Spawn Menu Design

## Intent

Move the verified scrap-stockpile spawn action out of the UI Configurator into a dedicated, compact, hold-Q spawn menu. The menu is a reusable front door for later props, vehicles, NPCs, and other game-owned spawn types.

## Goals

- Holding Q opens a focused panel and captures game input; releasing Q closes it and returns input to Mad Max.
- Show one populated `Props` tab with a thumbnail-like Scrap Stockpile tile in a scrollable grid.
- Show `Vehicles` and `NPCs` as visibly inactive category placeholders.
- A tile click submits one spawn request at the latest available reticle hit, with a small world-up placement offset and the existing player orientation convention.
- Repeated clicks submit repeated requests; no one-shot test guard remains.

## Non-goals

- Native despawn, deletion of map objects, persistent object IDs, or broader asset discovery.
- Additional categories or spawnable assets before their game-side spawn paths are understood.
- A script-authored UI API; this is a native baseline menu.

## Approaches

- Keep the test control in the configurator: smallest code change, but keeps the feature in the wrong place and does not establish a spawn-menu pattern.
- Build a compact native spawn menu with the verified prop as the first tile: matches the already-approved GMod-style mockup and creates a stable home for later categories. **Selected.**
- Build a script-owned menu: more flexible later, but depends on UI scripting plumbing and adds unnecessary indirection for the first native game spawn action.

## Interaction and appearance

The menu is a compact blue-grey panel with a title bar, a tab strip, and a scrollable thumbnail grid. `Props` is active. `Vehicles` and `NPCs` are disabled placeholders. The item has a simple custom drawn thumbnail and the short label `Scrap Stockpile`; there is no description text. The tile is disabled until a reticle hit and player transform are available. The window hook observes Q-down/up, then enables the existing global game-input capture and message filter while the menu is held. Focus loss releases the held state to avoid a stuck menu.

## Data and behavior

The menu submits the existing typed spawn intent using the fixed verified stockpile asset key. Placement uses the latest copied `raycast_hit_position`, adds 0.5 world-up, and preserves the current player transform orientation. Each click submits an independent request. Existing request-ID and native object identity behavior is not expanded in this change; repeated-spawn safety remains a manual verification point.

## Constraints and calls

- Keep the existing F1/F2/F3 behavior: input capture toggle, UI Configurator, and Dev Tools.
- Opening the spawn menu temporarily owns input regardless of the F1 override; closing it restores normal HUD-mode capture preferences.
- No new crate or asset dependency; draw the placeholder thumbnail with ImGui primitives.
- The existing approved one-tab GMod-style mockup is the visual baseline.
