# Script-Owned Micro-Playgrounds Design

## Intent

Give Rhai packages a bounded way to publish independent, interactive ImGui panels so a modder can build focused in-game workbenches instead of growing one always-on debug menu. The first panel is a live ray workbench for inspecting and changing one ray query while the game runs. It is intended to help distinguish camera-origin/basis problems from query filters and actual nearby collisions.

The compact debug HUD remains available beside these panels. The visual target is the approved mockup in the Superpowers brainstorming session: dark blue-gray ImGui panels, blue title bars, compact rows, fixed-precision values, a small Player/Vehicle HUD at top-left, and a separate Ray Workbench window.

## Goals

- Let a loaded Rhai package publish one or more separately positioned/openable windows that coexist with the debug HUD and other script panels.
- Keep Rhai execution out of the DX11/ImGui render callback. Scripts publish a bounded declarative panel model during the existing `game.post_sim` callback; native ImGui renders that model.
- Return bounded, copied widget changes to the owning script on a later accepted game callback. Keep package and generation ownership attached to panel state and events.
- Preserve the existing overlay palette, spacing, opacity, input-capture rules, and keyboard shortcuts instead of creating a parallel visual or input system.
- Add a UI Configurator toggle for script-authored panels, without making that global visibility switch unload scripts or overwrite each panel's own open/closed state.
- Move ray-specific diagnostics out of the general debug HUD and into a purpose-built script panel.
- Provide a first ray workbench script with live ray enable, origin/direction source, filter profile, range, visualization toggles, and a readable current query/result.
- Keep ray requests and gizmo rendering separate script capabilities; the UI API only describes controls and display values.
- Make scripts reloadable: a panel updates with its script generation; unloading removes its panel and discards queued old-generation events.

## Non-goals

- Implement a freecam or write the game's camera. The bridge currently has a camera getter only; a game-owned freecam needs separate camera-control reverse engineering.
- Expose raw ImGui objects, callbacks/closures into Rust, native pointers, or arbitrary game calls to Rhai.
- Add a new general keyboard polling mechanism for the first panel. Mouse interaction with the panel controls replaces console binds for ray settings. Freecam key subscriptions can be designed later.
- Allow scripts to create arbitrary widgets or unbounded content. The first API is a small, typed widget vocabulary with explicit resource limits.
- Add multiple simultaneous ray requests to this workbench. It edits and displays one ray; general-purpose raycast requests remain a separate API.
- Replace the existing debug HUD, Dev Tools, or UI Configurator.

## Current baseline and constraints

- The overlay owns one ImGui context and renders the debug HUD, UI Configurator, and Dev Tools. The HUD is currently compact, uses a dark blue-gray panel, and shares a global opacity setting.
- `OverlayMode` makes the Configurator and Dev Tools exclusive modes, while the HUD is separately drawn. Script panels must not set or replace `OverlayMode`; they are independent windows and remain visible alongside the HUD.
- The ImGui hook runs on the render thread. Script callbacks currently execute in the accepted game post-simulation phase. The UI ECS world is separate and intentionally does not access game resources.
- General input sampling is disabled. Existing global capture is toggled by F1 and controls whether ImGui windows accept input and whether Hudhook suppresses game input. F2 and F3 remain the Configurator and Dev Tools shortcuts.
- A general-purpose, ticketed Rhai raycast API and independent debug-draw capability already exist. Camera pose is copied into script context, but its relationship to the rendered aim is still unproven. The current query’s origin/direction arithmetic and hit-fraction reconstruction agree with the native helper; this design makes those values inspectable and adjustable rather than assuming the camera mapping is correct.
- Package discovery, load, reload, unload, generation ownership, and auto-reload already exist. They do not currently expose script-authored UI.

## Approaches

- **Script-built panel model with native ImGui rendering — selected.** Game-phase Rhai callbacks emit typed panel/widget descriptions. The overlay draws them and sends copied interaction events back through a bounded mailbox. It preserves the existing thread boundary and makes panel rendering independent of arbitrary script execution time, at the cost of applying a UI change on a later game callback.
- **Rhai `ui.render` callback.** Scripts would build or draw controls from the render hook. This is a natural immediate-mode API, but it moves script execution and failure/latency risk onto the rendering thread and crosses the current UI/game scheduling boundary.
- **Rust-only ray panel first.** This would reach an in-game workbench sooner, but it would not deliver reusable script-owned panels and could create a second UI implementation that must later be replaced.

The user selected the script-built panel model and approved the Ray Workbench mockup. The native renderer will own all Dear ImGui calls; Rhai will author only bounded data and consume input values on the next game callback.

## User-facing design

### Independent micro-playgrounds

Each loaded script may publish panels with stable package-scoped IDs. Each panel is a separate top-level ImGui window with its own title and visibility state. Panels coexist with the `Mad Max Debug` HUD and with panels from other packages. Creating, showing, or closing a script panel does not switch overlay modes or hide the HUD.

The title area follows the mockup: the human-readable panel title at left and the owning script ID at right. The body uses the current blue-gray palette and compact row style. Use the existing opacity preference and numeric formatter. Do not add lifecycle/status boilerplate to every panel.

The UI Configurator includes a `Script panels` checkbox, enabled by default. It globally shows or hides script-authored panels while leaving each panel's own open/closed state intact. This is a presentation preference only: turning it off does not pause, unload, or revoke capabilities from scripts. Individual panel close buttons still hide only that panel; the Scripts dashboard can reopen a closed panel when global visibility is enabled.

F1 remains the existing global mouse/game-input capture toggle. When capture is off, script panels are display-only and pass game input through, matching current overlay behavior. When capture is on, their controls accept mouse input and game input is suppressed by the existing path. The workbench requires no `bind` command or new function keys.

Panel close/open state is native UI state, separate from script state. Loading a script opens its panels by default. Closing a panel hides it without unloading the script; the Scripts dashboard can reopen a panel. Unloading removes its panels. A successful reload replaces the panel descriptions while retaining host visibility settings for matching stable IDs; a failed reload leaves the active generation and its panels intact.

### Compact debug HUD

The normal debug HUD continues showing the existing Player and Vehicle sections by default. Its Raycast section and any ray query output rows are removed. The Camera section may continue to show general view rotation, but query origin, direction, range, filter, state, fraction, hit distance, and hit position belong only in the Ray Workbench.

### First script: `reticle_ray`

The ray workbench maintains one active ray request and shows the state of its latest ticket. Its initial layout matches the approved mockup:

- **Raycast:** enabled checkbox and current state (live, pending, hit, miss, unavailable).
- **Ray source:** independent origin and direction sources, including camera view, player, vehicle, and user-entered values when those samples exist. This separation is deliberate so origin and direction can be varied independently.
- **Filter:** a dropdown of currently mapped profiles (`static_only` and `player_aim`); no raw masks.
- **Maximum distance:** a clamped slider within the raycast API's established limit.
- **Ray:** copied origin and normalized direction, formatted to fixed precision.
- **Result:** hit/miss state; when hit, position, segment fraction, and world distance from origin. Missing results show an explicit placeholder and never reuse a prior hit point.
- **Visualization:** independent toggles for ray segment, origin marker, and hit marker. The script composes raycast and gizmo APIs; the raycast API itself remains data-only.

The panel supports custom origin and direction components as bounded numeric controls so camera-derived values can be compared with manually chosen rays. Each query uses the exact ray and distance displayed for that ticket. The script must not display an old ticket's result as current after controls change: it marks the new query pending until its result arrives.

## Runtime and data flow

1. At each accepted `game.post_sim`, the script callback consumes UI values/events produced by the previous rendered frame.
2. Rhai builds the next panel model from script state and copied game data. If enabled, `reticle_ray` queues exactly one ray request and optional debug primitives using that same configuration.
3. The native adapter validates and bounds panel descriptions and transfers owned data to the UI world. No Rhai object, callback, native pointer, or borrowed game data crosses into the render hook.
4. The render schedule consumes the newest panel snapshots. The ImGui hook renders the HUD and every open script panel using the established theme.
5. ImGui widget changes become typed events keyed by package ID, generation, panel ID, and widget ID. A bounded mailbox returns them to the game callback.
6. The game callback discards events for unloaded/replaced generations, applies current values, and submits any subsequent ray request. Ray results return through the existing ticket API on a later callback.

Panel publication is replacement-based per script generation: a fresh model supersedes the preceding model, so removed widgets disappear and do not leave stale controls. A short render gap while a game callback is delayed retains the most recent model; stale game samples remain visibly unavailable according to existing snapshot freshness semantics.

## Initial API shape

The exact method spelling is an implementation-plan choice, but the API should be declarative and typed. It should provide a panel builder and a small set of widgets: heading/label, checkbox, bounded scalar slider, bounded numeric input, fixed-option selector, and button. Each widget has a stable script-supplied ID. Read-only labels support formatted vectors and status text.

Script-visible UI state must be copied values/maps, not borrowed widget references. Widgets produce events/values consumed by a later game callback. The script remains responsible for turning those values into ray requests and drawing choices. The native renderer does not infer ray behavior from labels or widget names.

Add a dedicated capability for publishing interactive panels. A script that only performs game callbacks or raycasts does not implicitly get window creation. `reticle_ray` declares the panel capability along with its existing raycast/read/debug-draw capabilities. Update generated `API.md`, the Rhai API reference, and example package manifest/source.

## Resource limits and failure handling

- Bound panels per script, total visible panels, widgets per panel/frame, text size, option counts, slider ranges, and queued UI events.
- Validate package/panel/widget identifiers and numeric values before publishing. Reject or omit malformed panel updates without affecting other scripts or the ImGui frame.
- Use latest-value coalescing for continuous controls such as sliders; preserve discrete button/checkbox edges so rapid input is not silently lost.
- Keep generation IDs on all UI events. Reload, unload, or script failure invalidates old events and query tickets.
- A UI model exceeding its budget is rejected as one package update and reported through the existing script/console error channel; native rendering continues.
- The render hook performs no filesystem access, Rhai evaluation, or raycast execution.

## Rollout

1. Add bounded panel and widget data types, transport, UI-owned state, and script capability. Keep native ImGui rendering as the only presentation backend.
2. Render one synthetic panel from Rust tests/fixtures to verify coexistence, theme reuse, input capture, close/reopen, and generation cleanup before exposing it to Rhai.
3. Add Rhai builder/value bindings and generated documentation; validate events cross back to the owning script callback.
4. Rewrite `reticle_ray` as the first script-owned workbench, move ray result display out of the HUD, and document the package.
5. Build and install only after game closure; validate visual parity, controls, ray source/filter/range changes, miss clearing, reload/unload behavior, and hot reload in game.

## Acceptance criteria

- Two independent script panels and the compact debug HUD can be visible at once; opening/closing one does not switch or replace the others.
- The real panel uses the existing dark blue-gray theme, title-bar color, opacity behavior, compact spacing, and fixed-precision values shown in the mockup.
- The UI Configurator's `Script panels` toggle hides/shows all script-owned panels without unloading scripts or changing per-panel visibility; its default is enabled.
- The HUD no longer displays ray query fields. Ray origin, direction, range, filter, state, fraction, distance, and optional hit point appear only in the Ray Workbench.
- A user can enable/disable the one ray, change origin and direction sources, edit custom vectors, change supported filter and range, and toggle visual primitives without console commands or new keyboard binds.
- The displayed result is associated with the displayed request configuration; changes show a pending state until a corresponding result arrives. Miss/unavailable states clear the old hit marker.
- UI edits reach only the owning active script generation; reload and unload do not apply stale queued events.
- Invalid or over-budget UI data cannot crash or stall the render callback and cannot prevent other windows from rendering.
- API documentation and the sample `reticle_ray` package describe the new panel contract and its delayed interaction/result timing.
- Actual freecam/game camera mutation remains out of scope until a camera setter/control path is mapped.
