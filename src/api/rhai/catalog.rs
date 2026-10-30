//! Single source of truth for the user-facing script API reference and runtime registry.

pub const API_VERSION: u32 = 1;

pub const API_CAPABILITIES: &[&str] = &[
    "debug.draw",
    "game.read",
    "game.vehicle.write",
    "input.read",
    "log",
];

pub struct ApiCallbackDoc {
    pub callback: &'static str,
    pub phase: &'static str,
    pub values: &'static [&'static str],
    pub methods: &'static [&'static str],
    pub unavailable: &'static str,
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
const API_UNAVAILABLE: &str = "Input is sampled from the mod's Hudhook/ImGui event stream and forwarded as a bounded snapshot; it is not Mad Max's native keymap or controller polling API. Unavailable vectors/scalars are (); vector = #{x: float, y: float, z: float}; RGBA = #{r: float, g: float, b: float, a: float}, each component in [0,1]. available means some copied object data exists, not that every field exists. sample_age_seconds reports freshness. Undeclared capability access errors; frame metadata and pure ctx.math helpers are universal. Script positions, vectors, transforms, gizmos, and vehicle velocity writes use +X right, +Y up, -Z forward with no Z reflection. Transforms remain 16-number row-major arrays, with translation at indices 12–14. Math vector inputs must contain exactly numeric x, y, z components (integer or float), each finite and within [-100000,100000]. Math vector results are fresh maps with finite components in [-100000,100000]; arguments are never mutated. Math scale factors and dot/length/distance scalar results must be finite but have no spatial component bound. Malformed, non-finite, out-of-range inputs or results raise script errors. All ctx.math methods require no capability. normalize and direction return () for magnitude/distance <= 1e-12; other invalid input raises an error. lerp requires finite t in [0,1]. Vehicle writes require available copied velocity; only successful callbacks publish effects. Debug drawing submits copied world-space primitives through CDebugRenderer during game.post_sim; each invocation handle expires after the callback. Coordinates must be finite and within ±100000, colors are finite RGBA in [0,1], radii are [0.01,10000], and transforms are 16 finite values in Mad Max matrix order (translation at elements 12–14), within ±100000. Calls are bounded to 32 primitives and 256 estimated geometry segments per invocation, plus 128 primitives and 512 geometry segments per game callback. Bounds-shape slots 36–39 use a matrix-pointer argument inferred from the wrapper and remain experimental. Native visibility and lifetime depend on the game's render queue.";
const API_EFFECT: &str = "reads copied game data; script state/output writes; bounded current-player-vehicle velocity intent; experimental bounded world-space CDebugRenderer submissions through game.post_sim adapter";

pub const API_CALLBACKS: &[ApiCallbackDoc] = &[
    ApiCallbackDoc {
        callback: "on_post_sim",
        phase: "game.post_sim",
        values: API_VALUES,
        methods: API_METHODS,
        unavailable: API_UNAVAILABLE,
        effect: API_EFFECT,
    },
    ApiCallbackDoc {
        callback: "on_run",
        phase: "game.post_sim",
        values: API_VALUES,
        methods: API_METHODS,
        unavailable: API_UNAVAILABLE,
        effect: API_EFFECT,
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
    for descriptor in API_CALLBACKS {
        output.push_str(&format!(
            "\n## `{}` — `{}`\n\nAPI version: **{}**\n\nCallback signature: `{}(ctx: MadMaxPostSimContext)`; return value ignored.\n\nEffect: {}.\n\n### Context values\n\n",
            descriptor.callback, descriptor.phase, API_VERSION, descriptor.callback, descriptor.effect
        ));
        for value in descriptor.values {
            output.push_str(&format!("- `{value}`\n"));
        }
        output.push_str("\n### Methods\n\n");
        for method in descriptor.methods {
            output.push_str(&format!("- `{method}`\n"));
        }
        output.push_str(&format!(
            "\n### Availability and limits\n\n{}\n",
            descriptor.unavailable
        ));
    }
    output
}
