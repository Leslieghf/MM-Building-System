//! Single source for script API metadata and its generated Markdown reference.
//!
//! [`API_CAPABILITIES`] and [`API_CALLBACKS`] describe the supported contract;
//! registration code and generated [`render_markdown`] output consume the same
//! catalog so the reference cannot silently drift from the runtime bindings.
//! Update catalog entries alongside any change to registered names, arguments,
//! return values, capability checks, or callback semantics.

/// Current script-visible contract version.
///
/// Packages declare this value as `api_version` in their manifest. A package
/// targeting a different version is rejected during validation.
pub const API_VERSION: u32 = 1;

/// Capability identifiers accepted by API v1 manifests.
pub const API_CAPABILITIES: &[&str] = &[
    "debug.draw",
    "game.read",
    "game.raycast",
    "game.spawn",
    "game.camera.control",
    "game.vehicle.write",
    "input.read",
    "log",
    "ui.panels",
    "ui.workbench",
];

/// Metadata used to validate scripts and generate the [`API_CALLBACKS`] reference.
pub struct ApiCallbackDoc {
    /// Required script function name.
    pub callback: &'static str,
    /// Manifest phase identifier.
    pub phase: &'static str,
    /// Context values visible to the callback.
    pub values: &'static [&'static str],
    /// Context methods visible to the callback.
    pub methods: &'static [&'static str],
    /// Availability semantics shared with the generated guide.
    pub unavailable: &'static str,
    /// Callback scheduling and effect semantics.
    pub effect: &'static str,
}

const API_VALUES: &[&str] = &[
    "ctx.frame.phase: string = game.post_sim [read; universal]",
    "ctx.frame.sequence: integer [read; universal]",
    "ctx.frame.sample_sequence: integer [read; universal; increments when native game telemetry is refreshed]",
    "ctx.frame.delta_seconds: float (seconds) [read; universal]",
    "ctx.frame.elapsed_seconds: float (seconds) [read; universal]",
    "ctx.frame.sample_age_seconds: float | () (seconds) [read; universal]",
    "ctx.format: MadMaxFormatter [pure formatting helpers; universal]",
    "ctx.math: MadMaxScriptMath [pure vector helpers; universal; no capability required]",
    "ctx.gizmos: MadMaxDebugDraw [debug.draw; experimental native CDebugRenderer submission]",
    "ctx.ui: MadMaxScriptUi [ui.panels; bounded script-owned panels and copied widget events]",
    "ctx.dependencies: MadMaxScriptDependencies [declared package and host API requirement availability]",
    "ctx.camera.available: bool [read; game.read; true when copied camera position and direction are both present]",
    "ctx.camera.position: vector | () [read; game.read; copied camera matrix translation]",
    "ctx.camera.direction: normalized 3D vector | () [read; game.read; row 2 of the copied camera matrix; in-game observation shows this sign points toward aim, while negating it points backward]",
    "ctx.camera_control: MadMaxCameraControl [game.camera.control; bounded, expiring override of the matrix returned by the native camera getter; does not detach the camera]",
    "ctx.input: mod-owned keyboard/mouse event snapshot [input.read]; available only while F1 overlay input capture is active; down/just_pressed/just_released only describe overlay-observed input, never native game actions or XInput state",
    "ctx.entities: MadMaxEntitySpawner [game.spawn; catalog-gated entity spawn requests; no native pointers or arbitrary entity paths]",
    "ctx.player.available: bool [read; game.read]",
    "ctx.player.position: vector | () [read; game.read]",
    "ctx.player.transform: array[16] | () [read; game.read; copied row-major game transform; translation at elements 12–14]",
    "ctx.player.forward: normalized horizontal vector | () [read; game.read; negative horizontal projection of row 2 of the observed character transform; actor facing only, does not include camera pitch]",
    "ctx.player.rotation: vector | () [read; game.read]",
    "ctx.player.velocity: vector | () [read; game.read]",
    "ctx.player.acceleration: vector | () [read; game.read]",
    "ctx.vehicle.available: bool [read; game.read]",
    "ctx.vehicle.position: vector | () [read; game.read]",
    "ctx.vehicle.transform: array[16] | () [read; game.read; copied row-major game transform; translation at elements 12–14]",
    "ctx.vehicle.forward: normalized horizontal vector | () [read; game.read; negative horizontal projection of row 2 of the observed vehicle transform]",
    "ctx.vehicle.rotation: vector | () [read; game.read]",
    "ctx.vehicle.velocity: vector | () [read; game.read]",
    "ctx.vehicle.acceleration: vector | () [read; game.read]",
    "ctx.vehicle.angular_velocity: vector | () [read; game.read]",
    "state: persistent map [script-owned read/write; reset per generation]",
];
const API_METHODS: &[&str] = &[
    "ctx.math.vec3(x: float, y: float, z: float) -> vector [pure; no capability required]",
    "ctx.math.add(a: vector, b: vector) -> vector [pure; component-wise addition]",
    "ctx.math.subtract(a: vector, b: vector) -> vector [pure; component-wise a - b]",
    "ctx.math.scale(v: vector, scalar: float) -> vector [pure; scalar multiplication]",
    "ctx.math.dot(a: vector, b: vector) -> float [pure; dot product]",
    "ctx.math.cross(a: vector, b: vector) -> vector [pure; right-handed cross product; right x up = +Z backward]",
    "ctx.math.length(v: vector) -> float [pure; Euclidean magnitude]",
    "ctx.math.distance(a: vector, b: vector) -> float [pure; Euclidean distance]",
    "ctx.math.normalize(v: vector) -> vector | () [pure; unit vector; () when magnitude <= 1e-12]",
    "ctx.math.direction(from: vector, to: vector) -> vector | () [pure; normalized to - from; () for coincident points or distance <= 1e-12]",
    "ctx.math.lerp(a: vector, b: vector, t: float) -> vector [pure; component-wise interpolation; finite t in [0,1]]",
    "ctx.input.down(key: string) -> bool [input.read]",
    "ctx.input.available() -> bool [input.read; false means the overlay sampler is inactive; false key states are not evidence of release in that case]",
    "ctx.input.just_pressed(key: string) -> bool [input.read]",
    "ctx.input.just_released(key: string) -> bool [input.read]",
    "ctx.dependencies.available(id: string) -> bool [true only for a declared requirement whose version is currently available]",
    "ctx.log.info(text: string) -> () [log; bounded console output]",
    "ctx.format.fixed(value: number) -> string [truncate to three decimal places; universal]",
    "ctx.format.vector(value: vector | ()) -> string [x, y, z at three decimal places; unavailable = --; universal]",
    "ctx.gizmos.axes(position: vector, length: number) -> () [debug.draw; world-space RGB axes; length in world units; length 0.1..1000]",
    "ctx.gizmos.transformed_axes(transform: array[16], length: number) -> () [debug.draw; native RGB basis axes from the supplied row-major transform; length 0.1..1000]",
    "ctx.gizmos.line(start: vector, end: vector, color: RGBA) -> () [debug.draw; one-frame world-space line]",
    "ctx.gizmos.arrow(start: vector, end: vector, color: RGBA) -> () [debug.draw; one-frame world-space auto-width arrow]",
    "ctx.gizmos.wire_sphere(center: vector, radius: number, color: RGBA) -> () [debug.draw; radial wire sphere; native slot 26]",
    "ctx.gizmos.xray_sphere(center: vector, radius: number, color: RGBA) -> () [debug.draw; radial wire sphere observed through walls; native slot 27]",
    "ctx.gizmos.surface_wire_sphere(center: vector, radius: number, color: RGBA) -> () [debug.draw; surface-following wire rings; native slot 28; geometry budget cost 60]",
    "ctx.gizmos.wire_aabb(minimum: vector, maximum: vector, color: RGBA) -> () [debug.draw; world-axis-aligned wire box; native slot 31]",
    "ctx.gizmos.wire_box(transform: MadMaxMatrix4x4, minimum: vector, maximum: vector, color: RGBA) -> () [debug.draw; transformed wire box; native slot 34]",
    "ctx.gizmos.xray_wire_box(transform: MadMaxMatrix4x4, minimum: vector, maximum: vector, color: RGBA) -> () [debug.draw; transformed wire box observed through walls; native slot 35]",
    "ctx.gizmos.solid_cube(transform: MadMaxMatrix4x4, minimum: vector, maximum: vector, color: RGBA) -> () [debug.draw; solid cube; observed with native Havok sphere-shape slot 36]",
    "ctx.gizmos.solid_arrow(transform: MadMaxMatrix4x4, minimum: vector, maximum: vector, color: RGBA) -> () [debug.draw; solid 3D arrow; observed with native Havok capsule-shape slot 38]",
    "ctx.vehicle.set_velocity(#{x: number, y: number, z: number}) -> () [game.read + game.vehicle.write; finite components in [-250,250] units/s; one request per invocation; game.post_sim native adapter]",
    "ctx.raycast.request(origin: vector, direction: vector, max_distance: number, filter: string) -> integer [game.raycast; data-only deferred query; `static_only` hits static geometry only; `player_aim` excludes Max's player character while hitting vehicles and static geometry in current in-game validation; one request per callback; identical requests in one accepted game frame share one native query; independent of debug.draw]",
    "ctx.raycast.camera(max_distance: number, filter: string) -> integer [game.raycast; deferred query using the copied camera world origin and view direction; scripts may submit one each callback for continuous tracking; identical requests in one accepted game frame share one native query and fan out ticket results; the frame cache expires after the callback]",
    "ctx.entities.request(catalog_id: string, transform: array[16]) -> integer [game.spawn; accepts green verified entries and yellow attempts with a usable entity asset path; red entries have no known spawn route; native submission is reported separately from observed spawn completion]",
    "ctx.camera_control.set_offset(world_offset: vector, euler_degrees: vector) -> () [game.camera.control; world-space translation offset plus world-axis yaw/pitch/roll; finite offset components within ±1000 and angles within ±180 degrees; refresh each callback; expires two game callbacks after the last successful request]",
    "ctx.camera_control.clear() -> () [game.camera.control; clear the override]",
    "ctx.camera_control.available() -> bool [game.camera.control; whether the GetCameraMatrix override hook installed for the supported executable]",
    "ctx.camera_control.external_calls() -> integer [game.camera.control; calls to the hooked getter outside this mod's own telemetry sampling]",
    "ctx.camera_control.external_override_calls() -> integer [game.camera.control; external hooked-getter calls where this mod's override was applied; does not prove the renderer consumed the returned matrix]",
    "ctx.entities.ready() -> bool [game.spawn; indicates whether this callback can enqueue its one request without exceeding the bounded spawn queue]",
    "ctx.raycast.result(ticket: integer) -> map [game.raycast; data-only result; state is pending, complete, unavailable, expired, or unknown; complete results include has_hit, hit_position, fraction, state_present; scripts may separately use debug.draw]",
    "ctx.ui.events(panel_id: string) -> map [ui.panels; copied widget changes delivered on a later game.post_sim callback; buttons are one-callback true values]",
    "ctx.ui.panel(id: string, title: string) -> MadMaxPanelBuilder [ui.panels; bounded declarative model; one to four stable panels per script]",
    "panel.heading(widget_id: string, text: string) -> MadMaxPanelBuilder [ui.panels; fixed text heading; chainable]",
    "panel.label(widget_id: string, text: string) -> MadMaxPanelBuilder [ui.panels; fixed text label; chainable]",
    "panel.status(widget_id: string, text: string, level: string) -> MadMaxPanelBuilder [ui.panels; stable=green, experimental=yellow (attempt; may work or may not), problematic=red; text is always shown; chainable]",
    "panel.checkbox(widget_id: string, label: string, value: bool) -> MadMaxPanelBuilder [ui.panels; widget change appears in ctx.ui.events on a later callback]",
    "panel.slider(widget_id: string, label: string, value: number, min: number, max: number) -> MadMaxPanelBuilder [ui.panels; finite bounded scalar; ±100000 limit]",
    "panel.number(widget_id: string, label: string, value: number, min: number, max: number, step: number) -> MadMaxPanelBuilder [ui.panels; finite bounded scalar; positive step]",
    "panel.select(widget_id: string, label: string, value: string, options: array) -> MadMaxPanelBuilder [ui.panels; 1 to 16 fixed string options]",
    "panel.button(widget_id: string, label: string) -> MadMaxPanelBuilder [ui.panels; one-callback true event]",
    "ctx.ui.publish(panel: MadMaxPanelBuilder) -> () [ui.panels; replaces the package's full panel snapshot for this callback; at most 4 panels and 48 widgets per panel]",
];
const API_ERRORS: &str = "Undeclared capabilities, malformed values, non-finite numbers, unknown raycast filters, duplicate panel/widget IDs, invalid widget bounds, and values outside documented bounds raise script errors. A callback error disables that script generation; a failed reload leaves the active generation running. Yellow spawn entries are unverified attempts and may fail or destabilize the game.";
const API_AVAILABILITY: &str = "Unavailable values are `()`. `available` means copied object data exists; individual fields can still be unavailable. `sample_age_seconds` reports the age of the copied sample. `ctx.input` is mod-runtime keyboard/mouse input observed by the overlay, not Mad Max's native keymap or controller state; check `ctx.input.available()` before interpreting false key states. Sampling is active only while F1 overlay input capture owns the UI and no text field is focused. F1 filters the overlay's keyboard/mouse messages from the game, but this does not establish XInput/gamepad suppression or native keybind-menu integration. No per-key native action suppression API is exposed.";
const API_COORDINATES: &str = "Vectors use +X right, +Y up, and -Z forward. Transforms are 16-number row-major arrays in Mad Max matrix order, with translation at indices 12–14. Vector maps use numeric `x`, `y`, and `z`; RGBA maps use `r`, `g`, `b`, and `a`. Math inputs are not mutated; returned vectors are fresh maps.";
const API_LIMITS: &str = "Math vector components and results are finite and within [-100000, 100000]. Scalar math arguments/results must be finite; `lerp` requires `t` in [0, 1]. `normalize` and `direction` return `()` for magnitude/distance <= 1e-12. Script panels allow 4 panels per package, 48 widgets per panel, 160 UTF-8 bytes per text value, and 16 selector options; widget numbers are finite, bounded, and within ±100000. Debug coordinates and transforms are finite and within ±100000, colors are in [0, 1], and radii are in [0.01, 10000]. Debug drawing is limited to 32 primitives and 256 estimated geometry segments per invocation, and 128 primitives and 512 segments per game callback. Raycasts accept coordinates within ±100000, distance in (0, 500], one request per script callback, and up to eight requests per game callback; results are copied and available on a later callback. Script spawning accepts a 16-number finite transform within ±100000, one catalog-approved request per callback and one per game callback. Vehicle velocity writes require copied velocity and are published only after successful callbacks.";
const PACKAGE_CONTRACT: &str = "Packages may declare `version` (default `0.0.0`), ordered confined `sources` (default `[\"main.rhai\"]`), typed `requires` records (`kind = \"package\" | \"host_api\"`, `id`, semantic `version`, optional flag), package `conflicts`, and optional `workbench_tab = { id, title, order }`. Required local packages are prepared dependency-first and activated together at a callback boundary; cycles, missing/incompatible requirements, and conflicts reject the graph. `ctx.dependencies.available(id)` lets a package conditionally expose features for its declared optional requirements. Package dependencies do not grant effect capabilities. Tab packages require host API `workbench.tabs` version `^1.0.0` and both `ui.workbench` and `ui.panels` capabilities. Rhai source files remain confined to the package; use the manifest source list instead of filesystem imports.";

/// Callback context, capability, and effect contracts for the current script API.
///
/// Package validation, runtime registration, and the generated API guide all
/// consume this catalog.
pub const API_CALLBACKS: &[ApiCallbackDoc] = &[
    ApiCallbackDoc {
        callback: "on_post_sim",
        phase: "game.post_sim",
        values: API_VALUES,
        methods: API_METHODS,
        unavailable: API_AVAILABILITY,
        effect: "Runs after each game simulation frame; reads copied data and may queue bounded vehicle, raycast, debug-draw, or panel effects",
    },
    ApiCallbackDoc {
        callback: "on_run",
        phase: "game.post_sim",
        values: API_VALUES,
        methods: API_METHODS,
        unavailable: API_AVAILABILITY,
        effect: "Runs only on request with the latest copied frame; may queue bounded vehicle, raycast, debug-draw, or panel effects",
    },
];

// The build script compiles this catalog separately to generate the checked-in API guide.
#[allow(dead_code)]
pub fn render_markdown() -> String {
    let mut output = format!(
        "# MadMaxModding Script API\n\n\
         This file is generated from the runtime API catalog during the Rust build.\n\n\
         API version: **{API_VERSION}**\n\n\
         ## Package contract\n\n\
         {PACKAGE_CONTRACT}\n\n\
         ## Capabilities\n\n",
    );
    for capability in API_CAPABILITIES {
        output.push_str(&format!("- `{capability}`\n"));
    }
    output.push_str("\n## Callback entry points\n\n");
    for descriptor in API_CALLBACKS {
        output.push_str(&format!(
            "- `{}` in `{}`: `{}(ctx: MadMaxPostSimContext)`; return value ignored. {}.\n",
            descriptor.callback, descriptor.phase, descriptor.callback, descriptor.effect
        ));
    }
    let api = &API_CALLBACKS[0];
    output.push_str("\n## Shared callback context\n\nAPI version: **1**. Both callbacks receive the same context and have the same capability rules.\n\n### Values\n\n");
    for value in api.values {
        output.push_str(&format!("- `{value}`\n"));
    }
    render_methods(&mut output, api.methods);
    output.push_str(&format!(
        "\n### Errors\n\n{API_ERRORS}\n\n### Availability\n\n{API_AVAILABILITY}\n\n### Coordinates and values\n\n{API_COORDINATES}\n\n### Limits and native behavior\n\n{API_LIMITS}\n"
    ));
    output
}

fn render_methods(output: &mut String, methods: &[&str]) {
    let groups: [(&str, &[&str]); 9] = [
        ("Math", &["ctx.math."]),
        ("Input", &["ctx.input."]),
        ("Logging and formatting", &["ctx.log.", "ctx.format."]),
        ("Camera controls", &["ctx.camera_control."]),
        ("Vehicle effects", &["ctx.vehicle."]),
        ("Raycasting", &["ctx.raycast."]),
        ("World entity spawning", &["ctx.entities."]),
        ("Script panels", &["ctx.ui.", "panel."]),
        ("Package dependencies", &["ctx.dependencies."]),
    ];
    for (title, prefixes) in groups {
        let matching = methods
            .iter()
            .filter(|method| prefixes.iter().any(|prefix| method.starts_with(prefix)));
        let rows: Vec<_> = matching.collect();
        if rows.is_empty() {
            continue;
        }
        output.push_str(&format!("\n### {title}\n\n"));
        for method in rows {
            output.push_str(&format!("- `{method}`\n"));
        }
    }
    render_debug_methods(output, methods);
}

fn render_debug_methods(output: &mut String, methods: &[&str]) {
    let groups: [(&str, fn(&str) -> bool); 4] = [
        ("Lines and orientation", |m| {
            [
                "ctx.gizmos.axes(",
                "ctx.gizmos.transformed_axes(",
                "ctx.gizmos.line(",
                "ctx.gizmos.arrow(",
            ]
            .iter()
            .any(|prefix| m.starts_with(prefix))
        }),
        ("Wire geometry", |m| {
            [
                "ctx.gizmos.wire_sphere(",
                "ctx.gizmos.surface_wire_sphere(",
                "ctx.gizmos.wire_aabb(",
                "ctx.gizmos.wire_box(",
            ]
            .iter()
            .any(|prefix| m.starts_with(prefix))
        }),
        ("Through-wall wire geometry", |m| {
            m.starts_with("ctx.gizmos.xray_")
        }),
        ("Solid debug shapes", |m| m.starts_with("ctx.gizmos.solid_")),
    ];
    for (title, matches) in groups {
        let rows: Vec<_> = methods.iter().filter(|method| matches(method)).collect();
        if rows.is_empty() {
            continue;
        }
        output.push_str(&format!("\n### {title}\n\n"));
        for method in rows {
            output.push_str(&format!("- `{method}`\n"));
        }
    }
}
