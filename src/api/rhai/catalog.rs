//! Single source for the script API catalog and generated [`render_markdown`] reference.

/// Current script-visible contract version.
///
/// Packages declare this value as `api_version` in their manifest. A package
/// targeting a different version is rejected during validation.
pub const API_VERSION: u32 = 1;

/// Capability identifiers accepted by API v1 manifests.
pub const API_CAPABILITIES: &[&str] = &[
    "debug.draw",
    "game.read",
    "game.vehicle.write",
    "input.read",
    "log",
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
    "ctx.frame.delta_seconds: float (seconds) [read; universal]",
    "ctx.frame.elapsed_seconds: float (seconds) [read; universal]",
    "ctx.frame.sample_age_seconds: float | () (seconds) [read; universal]",
    "ctx.format: MadMaxFormatter [pure formatting helpers; universal]",
    "ctx.math: MadMaxScriptMath [pure vector helpers; universal; no capability required]",
    "ctx.gizmos: MadMaxDebugDraw [debug.draw; experimental native CDebugRenderer submission]",
    "ctx.input: mod-owned keyboard/mouse event snapshot [input.read]; ctx.input.down(key: string), just_pressed(key: string), just_released(key: string) -> bool",
    "ctx.player.available: bool [read; game.read]",
    "ctx.player.position: vector | () [read; game.read]",
    "ctx.player.transform: array[16] | () [read; game.read; copied row-major game transform; translation at elements 12–14]",
    "ctx.player.forward: normalized horizontal vector | () [read; game.read; negative horizontal row 2 of observed character transform]",
    "ctx.player.rotation: vector | () [read; game.read]",
    "ctx.player.velocity: vector | () [read; game.read]",
    "ctx.player.acceleration: vector | () [read; game.read]",
    "ctx.vehicle.available: bool [read; game.read]",
    "ctx.vehicle.position: vector | () [read; game.read]",
    "ctx.vehicle.transform: array[16] | () [read; game.read; copied row-major game transform; translation at elements 12–14]",
    "ctx.vehicle.forward: normalized horizontal vector | () [read; game.read; negative horizontal row 2 of observed vehicle transform]",
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
    "ctx.input.just_pressed(key: string) -> bool [input.read]",
    "ctx.input.just_released(key: string) -> bool [input.read]",
    "ctx.log.info(text: string) -> () [log; bounded console output]",
    "ctx.format.fixed(value: number) -> string [truncate to three decimal places; universal]",
    "ctx.format.vector(value: vector | ()) -> string [x, y, z at three decimal places; unavailable = --; universal]",
    "ctx.gizmos.axes(position: vector, length: number) -> () [debug.draw; world-space RGB axes; length in world units; length 0.1..1000]",
    "ctx.gizmos.transformed_axes(transform: array[16], length: number) -> () [debug.draw; native RGB basis axes from the supplied row-major transform; length 0.1..1000]",
    "ctx.gizmos.line(start: vector, end: vector, color: RGBA) -> () [debug.draw; one-frame world-space line]",
    "ctx.gizmos.arrow(start: vector, end: vector, color: RGBA) -> () [debug.draw; one-frame world-space auto-width arrow]",
    "ctx.gizmos.sphere(center: vector, radius: number, color: RGBA) -> () [debug.draw; slot 26; wire sphere]",
    "ctx.gizmos.sparse_sphere(center: vector, radius: number, color: RGBA) -> () [debug.draw; slot 27; sparse queued wire sphere]",
    "ctx.gizmos.tessellated_sphere(center: vector, radius: number, color: RGBA) -> () [debug.draw; slot 28; dense wire sphere; geometry budget cost 60]",
    "ctx.gizmos.aabb(minimum: vector, maximum: vector, color: RGBA) -> () [debug.draw; slot 31; world-axis-aligned box]",
    "ctx.gizmos.transformed_aabb(transform: MadMaxMatrix4x4, minimum: vector, maximum: vector, color: RGBA) -> () [debug.draw; slot 34; translation at elements 12–14]",
    "ctx.gizmos.queued_transformed_aabb(transform: MadMaxMatrix4x4, minimum: vector, maximum: vector, color: RGBA) -> () [debug.draw; slot 35; alternate queued transformed box]",
    "ctx.gizmos.bounds_sphere(transform: MadMaxMatrix4x4, minimum: vector, maximum: vector, color: RGBA) -> () [debug.draw; experimental slot 36; transform argument inferred as a matrix pointer]",
    "ctx.gizmos.bounds_cylinder(transform: MadMaxMatrix4x4, minimum: vector, maximum: vector, color: RGBA) -> () [debug.draw; experimental slot 37; transform argument inferred as a matrix pointer]",
    "ctx.gizmos.bounds_capsule(transform: MadMaxMatrix4x4, minimum: vector, maximum: vector, color: RGBA) -> () [debug.draw; experimental slot 38; transform argument inferred as a matrix pointer]",
    "ctx.gizmos.bounds_convex_vertices(transform: MadMaxMatrix4x4, minimum: vector, maximum: vector, color: RGBA) -> () [debug.draw; experimental slot 39; this accepts bounds, not a custom vertex buffer]",
    "ctx.vehicle.set_velocity(#{x: number, y: number, z: number}) -> () [game.read + game.vehicle.write; finite components in [-250,250] units/s; one request per invocation; game.post_sim native adapter]",
];
const API_ERRORS: &str = "Undeclared capabilities, malformed values, non-finite numbers, and values outside documented bounds raise script errors. A callback error disables that script generation; a failed reload leaves the active generation running.";
const API_AVAILABILITY: &str = "Unavailable values are `()`. `available` means copied object data exists; individual fields can still be unavailable. `sample_age_seconds` reports the age of the copied sample. Input comes from the mod's Hudhook/ImGui event stream, not the game's native keymap or controller polling API.";
const API_COORDINATES: &str = "Vectors use +X right, +Y up, and -Z forward. Transforms are 16-number row-major arrays in Mad Max matrix order, with translation at indices 12–14. Vector maps use numeric `x`, `y`, and `z`; RGBA maps use `r`, `g`, `b`, and `a`. Math inputs are not mutated; returned vectors are fresh maps.";
const API_LIMITS: &str = "Math vector components and results are finite and within [-100000, 100000]. Scalar math arguments/results must be finite; `lerp` requires `t` in [0, 1]. `normalize` and `direction` return `()` for magnitude/distance <= 1e-12. Debug coordinates and transforms are finite and within ±100000, colors are in [0, 1], and radii are in [0.01, 10000]. Debug drawing is limited to 32 primitives and 256 estimated geometry segments per invocation, and 128 primitives and 512 segments per game callback. Vehicle velocity writes require copied velocity and are published only after successful callbacks. Bounds-shape methods remain experimental; their matrix-pointer argument and render visibility/lifetime depend on native behavior.";

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
        effect: "Runs after each game simulation frame; reads copied data and may queue bounded vehicle or debug-draw effects",
    },
    ApiCallbackDoc {
        callback: "on_run",
        phase: "game.post_sim",
        values: API_VALUES,
        methods: API_METHODS,
        unavailable: API_AVAILABILITY,
        effect: "Runs only on request with the latest copied frame; may queue bounded vehicle or debug-draw effects",
    },
];

// The build script compiles this catalog separately to generate the checked-in API guide.
#[allow(dead_code)]
pub fn render_markdown() -> String {
    let mut output = format!(
        "# MadMaxModding Script API\n\n\
         This file is generated from the runtime API catalog during the Rust build.\n\n\
         API version: **{API_VERSION}**\n\n\
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
    let groups: [(&str, &[&str]); 5] = [
        ("Math", &["ctx.math."]),
        ("Input", &["ctx.input."]),
        ("Logging and formatting", &["ctx.log.", "ctx.format."]),
        ("Debug drawing", &["ctx.gizmos."]),
        ("Vehicle effects", &["ctx.vehicle."]),
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
}
