# MadMaxModding Script API

This file is generated from the runtime API catalog during the Rust build.

API version: **1**

## Host-only Rust data types

`api::assets` defines owned asset descriptors and procedural geometry for host-side tools. These Rust types are not registered in Rhai, are not available to scripts, and do not load game assets or create native objects.

## Package contract

Packages may declare `version` (default `0.0.0`), ordered confined `sources` (default `["main.rhai"]`), typed `requires` records (`kind = "package" | "host_api"`, `id`, semantic `version`, optional flag), package `conflicts`, and optional `workbench_tab = { id, title, order, surface }`. Each tab package is independently loadable and may select `panels`, `events`, `scripts`, `console`, `catalog`, or `debug` as its native surface; package code and capabilities remain separate. The `debug` package controls the Rust-rendered live HUD through its load/unload lifecycle. Required local packages are prepared dependency-first and activated together at a callback boundary; cycles, missing/incompatible requirements, and conflicts reject the graph. `ctx.dependencies.available(id)` lets a package conditionally expose features for its declared optional requirements. Package dependencies do not grant effect capabilities. Tab packages require host API `workbench.tabs` version `^1.0.0` and both `ui.workbench` and `ui.panels` capabilities. Rhai source files remain confined to the package; use the manifest source list instead of filesystem imports.

## Capabilities

- `debug.draw`
- `game.read`
- `game.raycast`
- `game.spawn`
- `game.events.dispatch`
- `game.camera.control`
- `game.vehicle.write`
- `input.read`
- `log`
- `ui.panels`
- `ui.workbench`

## Callback entry points

- `on_post_sim` in `game.post_sim`: `on_post_sim(ctx: MadMaxPostSimContext)`; return value ignored. Runs after each game simulation frame; reads copied data and may queue bounded vehicle, raycast, debug-draw, or panel effects.
- `on_run` in `game.post_sim`: `on_run(ctx: MadMaxPostSimContext)`; return value ignored. Runs only on request with the latest copied frame; may queue bounded vehicle, raycast, debug-draw, or panel effects.

## Shared callback context

API version: **1**. Both callbacks receive the same context and have the same capability rules.

### Values

- `ctx.frame.phase: string = game.post_sim [read; universal]`
- `ctx.frame.sequence: integer [read; universal]`
- `ctx.frame.sample_sequence: integer [read; universal; increments when native game telemetry is refreshed]`
- `ctx.frame.delta_seconds: float (seconds) [read; universal]`
- `ctx.frame.elapsed_seconds: float (seconds) [read; universal]`
- `ctx.frame.sample_age_seconds: float | () (seconds) [read; universal]`
- `ctx.format: MadMaxFormatter [pure formatting helpers; universal]`
- `ctx.math: MadMaxScriptMath [pure vector helpers; universal; no capability required]`
- `ctx.gizmos: MadMaxDebugDraw [debug.draw; experimental native CDebugRenderer submission]`
- `ctx.ui: MadMaxScriptUi [ui.panels; bounded script-owned panels and copied widget events]`
- `ctx.dependencies: MadMaxScriptDependencies [declared package and host API requirement availability]`
- `ctx.camera.available: bool [read; game.read; true when copied camera position and direction are both present]`
- `ctx.camera.position: vector | () [read; game.read; copied camera matrix translation]`
- `ctx.camera.direction: normalized 3D vector | () [read; game.read; row 2 of the copied camera matrix; in-game observation shows this sign points toward aim, while negating it points backward]`
- `ctx.camera_control: MadMaxCameraControl [game.camera.control; bounded, expiring override of the matrix returned by the native camera getter; does not detach the camera]`
- `ctx.input: mod-owned keyboard/mouse event snapshot [input.read]; available only while F1 overlay input capture is active; down/just_pressed/just_released only describe overlay-observed input, never native game actions or XInput state`
- `ctx.entities: MadMaxEntitySpawner [game.spawn; catalog-gated entity spawn requests; no native pointers or arbitrary entity paths]`
- `ctx.events: MadMaxGameEvents [game.events.dispatch; catalogued name-only action dispatch]`
- `ctx.player.available: bool [read; game.read]`
- `ctx.player.position: vector | () [read; game.read]`
- `ctx.player.transform: array[16] | () [read; game.read; copied row-major game transform; translation at elements 12–14]`
- `ctx.player.forward: normalized horizontal vector | () [read; game.read; negative horizontal projection of row 2 of the observed character transform; actor facing only, does not include camera pitch]`
- `ctx.player.rotation: vector | () [read; game.read]`
- `ctx.player.velocity: vector | () [read; game.read]`
- `ctx.player.acceleration: vector | () [read; game.read]`
- `ctx.vehicle.available: bool [read; game.read]`
- `ctx.vehicle.position: vector | () [read; game.read]`
- `ctx.vehicle.transform: array[16] | () [read; game.read; copied row-major game transform; translation at elements 12–14]`
- `ctx.vehicle.forward: normalized horizontal vector | () [read; game.read; negative horizontal projection of row 2 of the observed vehicle transform]`
- `ctx.vehicle.rotation: vector | () [read; game.read]`
- `ctx.vehicle.velocity: vector | () [read; game.read]`
- `ctx.vehicle.acceleration: vector | () [read; game.read]`
- `ctx.vehicle.angular_velocity: vector | () [read; game.read]`
- `state: persistent map [script-owned read/write; reset per generation]`

### Math

- `ctx.math.vec3(x: float, y: float, z: float) -> vector [pure; no capability required]`
- `ctx.math.add(a: vector, b: vector) -> vector [pure; component-wise addition]`
- `ctx.math.subtract(a: vector, b: vector) -> vector [pure; component-wise a - b]`
- `ctx.math.scale(v: vector, scalar: float) -> vector [pure; scalar multiplication]`
- `ctx.math.dot(a: vector, b: vector) -> float [pure; dot product]`
- `ctx.math.cross(a: vector, b: vector) -> vector [pure; right-handed cross product; right x up = +Z backward]`
- `ctx.math.length(v: vector) -> float [pure; Euclidean magnitude]`
- `ctx.math.distance(a: vector, b: vector) -> float [pure; Euclidean distance]`
- `ctx.math.normalize(v: vector) -> vector | () [pure; unit vector; () when magnitude <= 1e-12]`
- `ctx.math.direction(from: vector, to: vector) -> vector | () [pure; normalized to - from; () for coincident points or distance <= 1e-12]`
- `ctx.math.lerp(a: vector, b: vector, t: float) -> vector [pure; component-wise interpolation; finite t in [0,1]]`

### Input

- `ctx.input.down(key: string) -> bool [input.read]`
- `ctx.input.available() -> bool [input.read; false means the overlay sampler is inactive; false key states are not evidence of release in that case]`
- `ctx.input.just_pressed(key: string) -> bool [input.read]`
- `ctx.input.just_released(key: string) -> bool [input.read]`

### Logging and formatting

- `ctx.log.info(text: string) -> () [log; bounded console output]`
- `ctx.format.fixed(value: number) -> string [truncate to three decimal places; universal]`
- `ctx.format.vector(value: vector | ()) -> string [x, y, z at three decimal places; unavailable = --; universal]`

### Camera controls

- `ctx.camera_control.set_offset(world_offset: vector, euler_degrees: vector) -> () [game.camera.control; world-space translation offset plus world-axis yaw/pitch/roll; finite offset components within ±1000 and angles within ±180 degrees; refresh each callback; expires two game callbacks after the last successful request]`
- `ctx.camera_control.clear() -> () [game.camera.control; clear the override]`
- `ctx.camera_control.available() -> bool [game.camera.control; whether the GetCameraMatrix override hook installed for the supported executable]`
- `ctx.camera_control.external_calls() -> integer [game.camera.control; calls to the hooked getter outside this mod's own telemetry sampling]`
- `ctx.camera_control.external_override_calls() -> integer [game.camera.control; external hooked-getter calls where this mod's override was applied; does not prove the renderer consumed the returned matrix]`

### Vehicle effects

- `ctx.vehicle.set_velocity(#{x: number, y: number, z: number}) -> () [game.read + game.vehicle.write; finite components in [-250,250] units/s; one request per invocation; game.post_sim native adapter]`

### Raycasting

- `ctx.raycast.request(origin: vector, direction: vector, max_distance: number, filter: string) -> integer [game.raycast; data-only deferred query; `static_only` hits static geometry only; `player_aim` excludes Max's player character while hitting vehicles and static geometry in current in-game validation; one request per callback; identical requests in one accepted game frame share one native query; independent of debug.draw]`
- `ctx.raycast.camera(max_distance: number, filter: string) -> integer [game.raycast; deferred query using the copied camera world origin and view direction; scripts may submit one each callback for continuous tracking; identical requests in one accepted game frame share one native query and fan out ticket results; the frame cache expires after the callback]`
- `ctx.raycast.result(ticket: integer) -> map [game.raycast; data-only result; state is pending, complete, unavailable, expired, or unknown; complete results include has_hit, hit_position, fraction, state_present; scripts may separately use debug.draw]`

### World entity spawning

- `ctx.entities.request(catalog_id: string, transform: array[16]) -> integer [game.spawn; accepts green verified entries and yellow attempts with a usable entity asset path; red entries have no known spawn route; native submission is reported separately from observed spawn completion]`
- `ctx.entities.ready() -> bool [game.spawn; indicates whether this callback can enqueue its one request without exceeding the bounded spawn queue]`

### Game actions

- `ctx.events.dispatch_action(name: string) -> () [game.events.dispatch; one catalogued name-only special-camera action per invocation; queued for the accepted game callback; native submission does not confirm resulting behavior]`

### Script panels

- `ctx.ui.events(panel_id: string) -> map [ui.panels; copied widget changes delivered on a later game.post_sim callback; buttons are one-callback true values]`
- `ctx.ui.panel(id: string, title: string) -> MadMaxPanelBuilder [ui.panels; bounded declarative model; one to four stable panels per script]`
- `panel.heading(widget_id: string, text: string) -> MadMaxPanelBuilder [ui.panels; fixed text heading; chainable]`
- `panel.label(widget_id: string, text: string) -> MadMaxPanelBuilder [ui.panels; fixed text label; chainable]`
- `panel.status(widget_id: string, text: string, level: string) -> MadMaxPanelBuilder [ui.panels; stable=green, experimental=yellow (attempt; may work or may not), problematic=red; text is always shown; chainable]`
- `panel.checkbox(widget_id: string, label: string, value: bool) -> MadMaxPanelBuilder [ui.panels; widget change appears in ctx.ui.events on a later callback]`
- `panel.slider(widget_id: string, label: string, value: number, min: number, max: number) -> MadMaxPanelBuilder [ui.panels; finite bounded scalar; ±100000 limit]`
- `panel.number(widget_id: string, label: string, value: number, min: number, max: number, step: number) -> MadMaxPanelBuilder [ui.panels; finite bounded scalar; positive step]`
- `panel.select(widget_id: string, label: string, value: string, options: array) -> MadMaxPanelBuilder [ui.panels; 1 to 16 fixed string options]`
- `panel.button(widget_id: string, label: string) -> MadMaxPanelBuilder [ui.panels; one-callback true event]`
- `panel.text_field(widget_id: string, label: string, value: string) -> MadMaxPanelBuilder [ui.panels; single-line UTF-8 input; 160-byte value limit]`
- `panel.code_editor(widget_id: string, label: string, value: string, editable: bool) -> MadMaxPanelBuilder [ui.panels; bounded multiline buffer; 8192-byte limit; editable changes are copied events and do not write files]`
- `panel.hierarchy_item(widget_id: string, label: string, value: string, depth: integer, selected: bool) -> MadMaxPanelBuilder [ui.panels; flattened selectable tree row; depth 0..16; selected value returned as a copied event]`
- `ctx.ui.publish(panel: MadMaxPanelBuilder) -> () [ui.panels; replaces the package's full panel snapshot for this callback; at most 4 panels and 48 widgets per panel]`

### Package dependencies

- `ctx.dependencies.available(id: string) -> bool [true only for a declared requirement whose version is currently available]`

### Lines and orientation

- `ctx.gizmos.axes(position: vector, length: number) -> () [debug.draw; world-space RGB axes; length in world units; length 0.1..1000]`
- `ctx.gizmos.transformed_axes(transform: array[16], length: number) -> () [debug.draw; native RGB basis axes from the supplied row-major transform; length 0.1..1000]`
- `ctx.gizmos.line(start: vector, end: vector, color: RGBA) -> () [debug.draw; one-frame world-space line]`
- `ctx.gizmos.arrow(start: vector, end: vector, color: RGBA) -> () [debug.draw; one-frame world-space auto-width arrow]`

### Wire geometry

- `ctx.gizmos.wire_sphere(center: vector, radius: number, color: RGBA) -> () [debug.draw; radial wire sphere; native slot 26]`
- `ctx.gizmos.surface_wire_sphere(center: vector, radius: number, color: RGBA) -> () [debug.draw; surface-following wire rings; native slot 28; geometry budget cost 60]`
- `ctx.gizmos.wire_aabb(minimum: vector, maximum: vector, color: RGBA) -> () [debug.draw; world-axis-aligned wire box; native slot 31]`
- `ctx.gizmos.wire_box(transform: MadMaxMatrix4x4, minimum: vector, maximum: vector, color: RGBA) -> () [debug.draw; transformed wire box; native slot 34]`

### Through-wall wire geometry

- `ctx.gizmos.xray_sphere(center: vector, radius: number, color: RGBA) -> () [debug.draw; radial wire sphere observed through walls; native slot 27]`
- `ctx.gizmos.xray_wire_box(transform: MadMaxMatrix4x4, minimum: vector, maximum: vector, color: RGBA) -> () [debug.draw; transformed wire box observed through walls; native slot 35]`

### Solid debug shapes

- `ctx.gizmos.solid_cube(transform: MadMaxMatrix4x4, minimum: vector, maximum: vector, color: RGBA) -> () [debug.draw; solid cube; observed with native Havok sphere-shape slot 36]`
- `ctx.gizmos.solid_arrow(transform: MadMaxMatrix4x4, minimum: vector, maximum: vector, color: RGBA) -> () [debug.draw; solid 3D arrow; observed with native Havok capsule-shape slot 38]`

### Errors

Undeclared capabilities, malformed values, non-finite numbers, unknown raycast filters, duplicate panel/widget IDs, invalid widget bounds, and values outside documented bounds raise script errors. A callback error disables that script generation; a failed reload leaves the active generation running. Yellow spawn entries are unverified attempts and may fail or destabilize the game.

### Availability

Unavailable values are `()`. `available` means copied object data exists; individual fields can still be unavailable. `sample_age_seconds` reports the age of the copied sample. `ctx.input` is mod-runtime keyboard/mouse input observed by the overlay, not Mad Max's native keymap or controller state; check `ctx.input.available()` before interpreting false key states. Sampling is active only while F1 overlay input capture owns the UI and no text field is focused. F1 filters the overlay's keyboard/mouse messages from the game, but this does not establish XInput/gamepad suppression or native keybind-menu integration. No per-key native action suppression API is exposed.

### Coordinates and values

Vectors use +X right, +Y up, and -Z forward. Transforms are 16-number row-major arrays in Mad Max matrix order, with translation at indices 12–14. Vector maps use numeric `x`, `y`, and `z`; RGBA maps use `r`, `g`, `b`, and `a`. Math inputs are not mutated; returned vectors are fresh maps.

### Limits and native behavior

Math vector components and results are finite and within [-100000, 100000]. Scalar math arguments/results must be finite; `lerp` requires `t` in [0, 1]. `normalize` and `direction` return `()` for magnitude/distance <= 1e-12. Script panels allow 4 panels per package, 48 widgets per panel, 160 UTF-8 bytes per ordinary text value, an 8192-byte multiline editor buffer, hierarchy depth 0..16, and 16 selector options; widget numbers are finite, bounded, and within ±100000. The code editor only edits the copied UI buffer; it does not write package files. Rust owns native widget rendering, text limits, scroll regions, and event validation; Rhai publishes bounded data descriptions and consumes copied change events. Debug coordinates and transforms are finite and within ±100000, colors are in [0, 1], and radii are in [0.01, 10000]. Debug drawing is limited to 32 primitives and 256 estimated geometry segments per invocation, and 128 primitives and 512 segments per game callback. Raycasts accept coordinates within ±100000, distance in (0, 500], one request per script callback, and up to eight requests per game callback; results are copied and available on a later callback. Script spawning accepts a 16-number finite transform within ±100000, one catalog-approved request per callback and one per game callback. Vehicle velocity writes require copied velocity and are published only after successful callbacks.
