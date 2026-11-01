# GMod-Style Spawn Browser and Copyable Console Design

## Intent

Replace the cramped single-panel spawn menu with a compact GMod-inspired workspace, and make spawn attempts diagnosable from a copyable console transcript. The browser should use carefully tuned spacing, small descriptive icons, and dense tiles by default; larger tiles remain user-configurable.

## Current findings

- The current catalog has 466 unique records: 1 categorized as `Props`, 40 `Vehicles`, 422 `World Compositions`, and 3 `Discovered`. The sole Props row is the user-observed scrap stockpile. Most other entries came from community `all_entity`/`empty_vehicle` arrays and are candidate entity compositions, not known standalone props.
- The yellow state means a safe catalog key can be attempted; it does not establish that the resource exists or that Mad Max will instantiate it. Only the stockpile has been visually confirmed to spawn.
- Native submission is reported as `NativeCallSubmitted`, while later `spawned` and `failed` observations update mod-owned lifecycle state. Those later observations are not currently reported to the console, so a submitted-but-failed object is hard to diagnose.
- The current spawn menu is one 440x510 window with top category buttons, a grid, and placement/removal controls below it. It has no folder navigation or tool/settings side pane.
- The current console groups output under command/script category headers and indents responses, but selection is one row at a time. The existing ImGui binding exposes a read-only multiline text editor with native range selection and clipboard behavior.

## Goals

1. Arrange one resizable spawn workspace with main tabs at the top, a compact folder tree on the left, a scrollable thumbnail grid in the center, and a tabbed Tools/Settings side pane on the right.
2. Keep dense GMod-like spacing, restrained borders, small descriptive thumbnails, and compact row heights as the default. Provide a setting to change tile size.
3. Preserve the established green/yellow/red semantics and catalog allowlist. Do not imply yellow items are verified.
4. Send distinct, useful spawn lifecycle records to the console: request queued, native submission/rejection, observed spawn success, and game-reported spawn failure. Include the catalog ID/key and request/native ID when available.
5. Keep the console's command/script grouping and output indentation, while rendering the retained transcript as one read-only multiline field so users can select ranges across multiple entries and copy with normal text-editing shortcuts. Keep the existing command prompt/history and omit separate copy controls.
6. Retain bounded output/history and avoid a render-time cost proportional to the whole catalog for folder filtering or tile drawing.

## Non-goals

- Claiming community keys are valid or creating new standalone prop assets.
- Changing native spawn ABI or expanding asset discovery in this UI pass.
- Adding NPC or weapon catalog entries without known source records.
- Replacing the console with an external terminal or changing command/script semantics.

## Approaches considered

1. **Single GMod-inspired three-pane workspace (chosen).** Main catalog tabs, folder navigation, center grid, and right Tools/Settings tabs live in one window. This matches the requested interaction and allows precise density tuning without multiple floating windows.
2. **Polish the current single panel.** Less code, but keeps the cramped geometry and cannot provide the requested folder and tools panes.
3. **Keep browser and tools in separate windows.** Makes the code changes smaller but breaks the requested one-workspace flow and wastes screen space.

## Layout and behavior

- The window adapts to the game viewport and has sensible minimum/maximum dimensions. Margins, item spacing, row height, and icon bounds are deliberately small and consistent.
- Main tabs filter the catalog by its existing top-level category (`Props`, `Vehicles`, `World`, `Other`). The left tree filters by nested category folders; search applies to the selected tab/tree. Counts reflect the active filters.
- The center grid uses compact default tiles with small, category-appropriate iconography and status color. A Settings tab on the right offers compact/medium/large tile sizing. The exact asset key is available in a compact details view/tool panel rather than printed on every tile.
- The right panel has `Tools` and `Settings` tabs. Tools holds the placement-ray controls, hit marker option, recent owned spawns and removal actions. Settings holds tile size and existing browser options. The panel is separated from the center grid and does not replace the debug menu or other script panels.
- A safe-relative key continues to be attemptable when yellow. The Props tree only contains records categorized as Props; it will not relabel unknown encounter compositions as props to make the tab appear fuller.
- When placement rays are enabled, the spawn workbench refreshes its aim hit each accepted game callback and places at the most recently completed hit. `reticle_ray` also remains continuous while enabled. Identical ray inputs requested by multiple scripts in the same game callback are served from the shared per-callback result and issue only one native query.
- The console transcript is flattened into a bounded display buffer with explicit `Cmd:`, `Rhai:`, and `Rust: <module>` headers, indented response lines, severity markers, and readable line breaks. One compact read-only multiline text field provides drag selection across entries and native clipboard shortcuts. There is no separate copy button. Prompt and history remain below it.
- The spawn console line uses a distinct `SPAWN:` category and reports state transitions. Only outcomes observed by the native spawn handler are described as success/failure; absence of an observation remains pending/unknown.

## Constraints and risks

- Dear ImGui read-only multiline editing provides native text selection but uses one text color. The transcript will preserve structure through explicit headers, indentation, and severity prefixes instead of per-span ANSI colors inside that single field.
- Game-reported failure observations are bounded and per-frame; records must be correlated to the correct mod-owned native ID before reporting.
- The browser should continue using its generated catalog and should not accept arbitrary file paths from scripts.
- Existing focus, mouse capture, hold-Q behavior, transparency, and other overlay modes must continue to work.

## Acceptance criteria

- The spawn menu visibly presents the requested top tabs / left folders / center grid / right tabbed side pane with compact defaults and configurable tile size.
- The Props count reflects catalog metadata accurately; the stockpile remains its only entry until additional standalone props are actually curated.
- A click that reaches the native adapter yields a console record, followed by an observed-spawn or observed-failure record when Mad Max reports one. No report falsely treats `NativeCallSubmitted` as spawned.
- The console preserves command and script grouping and allows selection/copy across multiple rows in one text field.
- Build and formatting checks pass; no native ABI or catalog allowlist bypass is introduced.
