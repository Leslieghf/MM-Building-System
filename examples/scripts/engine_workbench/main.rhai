fn vec3(x, y, z) {
    #{x: x, y: y, z: z}
}

fn point(origin, x, y, z) {
    #{x: origin.x + x, y: origin.y + y, z: origin.z + z}
}

fn identity_at(position) {
    [1.0, 0.0, 0.0, 0.0,
     0.0, 1.0, 0.0, 0.0,
     0.0, 0.0, 1.0, 0.0,
     position.x, position.y, position.z, 1.0]
}

fn local_bounds(radius) {
    #{x: -radius, y: -radius, z: -radius}
}

fn local_bounds_max(radius) {
    #{x: radius, y: radius, z: radius}
}

fn optional_number(ctx, value) {
    if value == () { "--" } else { ctx.format.fixed(value) }
}

fn on_post_sim(ctx) {
    if !state.contains("initialized") {
        state.initialized = true;
        state.page = "Overview";
        state.camera_enabled = false;
        state.camera_requested = false;
        state.camera_offset = vec3(0.0, 0.0, 0.0);
        state.camera_rotation = vec3(0.0, 0.0, 0.0);
        state.vehicle_velocity = vec3(0.0, 0.0, 0.0);
        state.vehicle_velocity_touched = false;
        state.vehicle_action = "Idle";
        state.ray_enabled = false;
        state.ray_filter = "player_aim";
        state.ray_range = 100.0;
        state.ray_show_line = true;
        state.ray_show_hit = true;
        state.ray_ticket = ();
        state.ray_hit = ();
        state.ray_fraction = ();
        state.ray_status = "No result";
        state.ray_query_status = "Disabled";
        state.spawn_id = "props.scrap_stockpile_small";
        state.spawn_queued_id = "";
        state.spawn_range = 100.0;
        state.spawn_phase = "";
        state.spawn_ticket = ();
        state.spawn_hit = ();
        state.spawn_status = "Ready";
        state.spawn_request = ();
        state.gizmo_enabled = false;
        state.gizmo_scale = 1.0;
        state.input_event = "No UI event yet";
    }

    let events = ctx.ui.events("sandbox_suite");
    if events.contains("page") { state.page = events.page; }

    if events.contains("camera_enabled") { state.camera_enabled = events.camera_enabled; }
    if events.contains("camera_x") { state.camera_offset.x = events.camera_x; }
    if events.contains("camera_y") { state.camera_offset.y = events.camera_y; }
    if events.contains("camera_z") { state.camera_offset.z = events.camera_z; }
    if events.contains("camera_yaw") { state.camera_rotation.x = events.camera_yaw; }
    if events.contains("camera_pitch") { state.camera_rotation.y = events.camera_pitch; }
    if events.contains("camera_roll") { state.camera_rotation.z = events.camera_roll; }
    if events.contains("camera_reset") {
        state.camera_enabled = false;
        state.camera_offset = vec3(0.0, 0.0, 0.0);
        state.camera_rotation = vec3(0.0, 0.0, 0.0);
    }

    if !state.vehicle_velocity_touched && ctx.vehicle.velocity != () {
        state.vehicle_velocity = ctx.vehicle.velocity;
    }
    if events.contains("vehicle_copy") && state.page == "Vehicle" && ctx.vehicle.velocity != () {
        state.vehicle_velocity = ctx.vehicle.velocity;
        state.vehicle_velocity_touched = true;
        state.vehicle_action = "Target copied from current vehicle";
    }
    if events.contains("vehicle_x") { state.vehicle_velocity.x = events.vehicle_x; state.vehicle_velocity_touched = true; }
    if events.contains("vehicle_y") { state.vehicle_velocity.y = events.vehicle_y; state.vehicle_velocity_touched = true; }
    if events.contains("vehicle_z") { state.vehicle_velocity.z = events.vehicle_z; state.vehicle_velocity_touched = true; }
    if events.contains("apply_velocity") && state.page == "Vehicle" {
        if ctx.vehicle.available && ctx.vehicle.velocity != () {
            ctx.vehicle.set_velocity(state.vehicle_velocity);
            state.vehicle_action = "Velocity request submitted";
            ctx.log.info("Sandbox Suite: vehicle velocity request submitted");
        } else {
            state.vehicle_action = "No current vehicle sample; no write sent";
        }
    }
    if events.contains("jump") && state.page == "Vehicle" {
        if ctx.vehicle.available && ctx.vehicle.velocity != () {
            let velocity = ctx.vehicle.velocity;
            let jump_y = if velocity.y + 18.0 > 250.0 { 250.0 } else { velocity.y + 18.0 };
            ctx.vehicle.set_velocity(vec3(velocity.x, jump_y, velocity.z));
            state.vehicle_action = "One-shot +18.000 Y velocity request submitted";
            ctx.log.info("Sandbox Suite: one-shot vehicle jump request submitted");
        } else {
            state.vehicle_action = "No current vehicle sample; no write sent";
        }
    }

    if events.contains("ray_enabled") && state.page == "Raycast" { state.ray_enabled = events.ray_enabled; }
    if events.contains("ray_filter") && state.page == "Raycast" { state.ray_filter = events.ray_filter; state.ray_ticket = (); state.ray_hit = (); state.ray_fraction = (); state.ray_status = "Configuration changed"; }
    if events.contains("ray_range") && state.page == "Raycast" { state.ray_range = events.ray_range; state.ray_ticket = (); state.ray_hit = (); state.ray_fraction = (); state.ray_status = "Configuration changed"; }
    if events.contains("ray_show_line") && state.page == "Raycast" { state.ray_show_line = events.ray_show_line; }
    if events.contains("ray_show_hit") && state.page == "Raycast" { state.ray_show_hit = events.ray_show_hit; }

    if state.page == "Raycast" && state.ray_enabled {
        if state.ray_ticket != () {
            let result = ctx.raycast.result(state.ray_ticket);
            if result.state == "complete" {
                state.ray_ticket = ();
                if result.has_hit && result.hit_position != () {
                    state.ray_hit = result.hit_position;
                    state.ray_fraction = result.fraction;
                    state.ray_status = "Hit";
                } else {
                    state.ray_hit = ();
                    state.ray_fraction = result.fraction;
                    state.ray_status = "Miss";
                }
            } else if result.state == "unavailable" || result.state == "expired" || result.state == "unknown" {
                state.ray_ticket = ();
                state.ray_hit = ();
                state.ray_fraction = ();
                state.ray_status = result.state;
            } else {
                state.ray_query_status = "Pending";
            }
        }
        if state.ray_ticket == () && ctx.camera.available {
            state.ray_ticket = ctx.raycast.camera(state.ray_range, state.ray_filter);
            state.ray_query_status = "Updating";
        } else if !ctx.camera.available {
            state.ray_status = "Camera source unavailable";
            state.ray_hit = ();
            state.ray_fraction = ();
            state.ray_query_status = "Unavailable";
        }
    } else {
        state.ray_ticket = ();
        state.ray_hit = ();
        state.ray_fraction = ();
        state.ray_query_status = "Paused";
        if state.page == "Raycast" { state.ray_status = "Disabled"; state.ray_query_status = "Disabled"; }
    }

    if state.page == "Raycast" && state.ray_enabled && ctx.camera.available {
        let direction = ctx.math.normalize(ctx.camera.direction);
        if direction != () {
            if state.ray_show_line {
                let endpoint = ctx.math.add(ctx.camera.position, ctx.math.scale(direction, state.ray_range));
                ctx.gizmos.line(ctx.camera.position, endpoint, #{r: 1.0, g: 0.85, b: 0.0, a: 1.0});
            }
            if state.ray_show_hit && state.ray_hit != () {
                ctx.gizmos.surface_wire_sphere(state.ray_hit, 0.65, #{r: 1.0, g: 0.1, b: 0.1, a: 1.0});
                ctx.gizmos.xray_sphere(state.ray_hit, 1.0, #{r: 0.0, g: 0.9, b: 1.0, a: 1.0});
            }
        }
    }

    if events.contains("spawn_id") && state.page == "Spawn probe" { state.spawn_id = events.spawn_id; }
    if events.contains("spawn_range") && state.page == "Spawn probe" { state.spawn_range = events.spawn_range; }
    if events.contains("spawn_cancel") && state.page == "Spawn probe" {
        state.spawn_phase = "";
        state.spawn_ticket = ();
        state.spawn_hit = ();
        state.spawn_queued_id = "";
        state.spawn_status = "Cancelled; no spawn submitted";
    }
    if state.spawn_phase != "" && state.page != "Spawn probe" {
        state.spawn_phase = "";
        state.spawn_ticket = ();
        state.spawn_hit = ();
        state.spawn_queued_id = "";
        state.spawn_status = "Cancelled when leaving page; nothing submitted";
    }
    if state.page == "Spawn probe" && state.spawn_phase == "ray_pending" && state.spawn_ticket != () {
        let result = ctx.raycast.result(state.spawn_ticket);
        if result.state == "complete" {
            state.spawn_ticket = ();
            if result.has_hit && result.hit_position != () {
                state.spawn_hit = result.hit_position;
                state.spawn_phase = "spawn_ready";
                state.spawn_status = "Fresh hit; waiting for spawn queue";
            } else {
                state.spawn_phase = "";
                state.spawn_hit = ();
                state.spawn_status = "Ray missed; no spawn submitted";
            }
        } else if result.state == "unavailable" || result.state == "expired" || result.state == "unknown" {
            state.spawn_phase = "";
            state.spawn_ticket = ();
            state.spawn_hit = ();
            state.spawn_status = "Ray query " + result.state + "; no spawn submitted";
        }
    }
    if state.page == "Spawn probe" && state.spawn_phase == "spawn_ready" && ctx.entities.ready() {
        let position = point(state.spawn_hit, 0.0, 0.15, 0.0);
        state.spawn_request = ctx.entities.request(state.spawn_queued_id, identity_at(position));
        state.spawn_status = "Catalog spawn request submitted; see console lifecycle report";
        state.spawn_phase = "";
        ctx.log.info("Sandbox Suite: catalog spawn request submitted for " + state.spawn_queued_id);
        state.spawn_queued_id = "";
    } else if state.page == "Spawn probe" && state.spawn_phase == "spawn_ready" {
        state.spawn_status = "Fresh hit; waiting for bounded spawn queue";
    }
    if events.contains("spawn_selected") && state.page == "Spawn probe" {
        if state.spawn_phase == "" && ctx.camera.available {
            state.spawn_queued_id = state.spawn_id;
            state.spawn_ticket = ctx.raycast.camera(state.spawn_range, "player_aim");
            state.spawn_phase = "ray_pending";
            state.spawn_status = "Waiting for one camera ray result";
            state.spawn_hit = ();
            ctx.log.info("Sandbox Suite: one-shot spawn placement query requested");
        } else if state.spawn_phase != "" {
            state.spawn_status = "Previous spawn operation still pending";
        } else {
            state.spawn_status = "Camera unavailable; no query submitted";
        }
    }

    if events.contains("gizmo_enabled") && state.page == "Gizmo probe" { state.gizmo_enabled = events.gizmo_enabled; }
    if events.contains("gizmo_scale") && state.page == "Gizmo probe" { state.gizmo_scale = events.gizmo_scale; }
    if events.contains("ui_event_probe") && state.page == "Input / UI" {
        state.input_event = "Button event received on frame " + ctx.format.fixed(ctx.frame.sequence);
        ctx.log.info("Sandbox Suite: script panel button event received");
    }

    let camera_hook = ctx.camera_control.available();
    if state.page == "Camera" && state.camera_enabled && camera_hook {
        ctx.camera_control.set_offset(state.camera_offset, state.camera_rotation);
        state.camera_requested = true;
    } else if state.camera_requested {
        ctx.camera_control.clear();
        state.camera_requested = false;
    }

    if state.page == "Gizmo probe" && state.gizmo_enabled {
        let anchor = ctx.player.position;
        let forward = ctx.player.forward;
        if ctx.vehicle.available && ctx.vehicle.position != () && ctx.vehicle.forward != () {
            anchor = ctx.vehicle.position;
            forward = ctx.vehicle.forward;
        }
        if anchor != () && forward != () {
            let scale = state.gizmo_scale;
            let center = point(anchor, forward.x * 14.0, 2.5, forward.z * 14.0);
            let red = #{r: 1.0, g: 0.1, b: 0.1, a: 1.0};
            let green = #{r: 0.1, g: 1.0, b: 0.1, a: 1.0};
            let blue = #{r: 0.1, g: 0.35, b: 1.0, a: 1.0};
            let cyan = #{r: 0.0, g: 0.9, b: 1.0, a: 1.0};
            let yellow = #{r: 1.0, g: 0.85, b: 0.0, a: 1.0};
            let magenta = #{r: 1.0, g: 0.0, b: 0.9, a: 1.0};
            let orange = #{r: 1.0, g: 0.35, b: 0.0, a: 1.0};
            let white = #{r: 1.0, g: 1.0, b: 1.0, a: 1.0};
            let min = local_bounds(scale * 0.6);
            let max = local_bounds_max(scale * 0.6);
            ctx.gizmos.axes(center, scale * 1.5);
            ctx.gizmos.line(point(center, -4.0 * scale, 0.0, 0.0), point(center, 4.0 * scale, 0.0, 0.0), red);
            ctx.gizmos.arrow(center, point(center, 0.0, 2.0 * scale, 0.0), yellow);
            ctx.gizmos.wire_sphere(point(center, -4.0 * scale, 0.0, 0.0), 0.7 * scale, cyan);
            ctx.gizmos.surface_wire_sphere(point(center, -2.0 * scale, 0.0, 0.0), 0.7 * scale, magenta);
            ctx.gizmos.xray_sphere(point(center, 0.0, 0.0, 0.0), 0.7 * scale, orange);
            ctx.gizmos.wire_aabb(point(center, 2.0 * scale, -0.7 * scale, -0.7 * scale), point(center, 3.4 * scale, 0.7 * scale, 0.7 * scale), white);
            ctx.gizmos.wire_box(identity_at(point(center, -3.0 * scale, 2.0 * scale, 0.0)), min, max, cyan);
            ctx.gizmos.xray_wire_box(identity_at(point(center, -1.0 * scale, 2.0 * scale, 0.0)), min, max, magenta);
            ctx.gizmos.solid_cube(identity_at(point(center, 1.0 * scale, 2.0 * scale, 0.0)), min, max, green);
            ctx.gizmos.solid_arrow(identity_at(point(center, 3.0 * scale, 2.0 * scale, 0.0)), min, max, blue);
        }
    }

    let panel = ctx.ui.panel("sandbox_suite", "Mad Max Sandbox");
    panel.select("page", "MODULE", state.page, ["Overview", "Telemetry", "Camera", "Raycast", "Vehicle", "Player", "Spawn probe", "Gizmo probe", "Math / orientation", "Input / UI", "Vehicle tuning", "Weapons / damage", "Max upgrades", "GameObjects / Havok", "NPC / animation", "Effects / assets"]);

    if state.page == "Overview" {
        panel.status("overview_live", "STABLE · copied telemetry and script panels", "stable");
        panel.label("overview_frame", "Frame  " + ctx.format.fixed(ctx.frame.sequence) + "  Δt  " + ctx.format.fixed(ctx.frame.delta_seconds));
        panel.label("overview_actor", "Player  " + (if ctx.player.available { "available" } else { "--" }) + "   Vehicle  " + (if ctx.vehicle.available { "available" } else { "--" }));
        panel.label("overview_camera", "Camera  " + (if ctx.camera.available { "available" } else { "--" }) + "   Input sampler  " + (if ctx.input.available() { "active" } else { "inactive" }));
        panel.status("overview_experimental", "EXPERIMENTAL · camera, ray, spawn, vehicle write, and gizmos", "experimental");
        panel.label("overview_wip", "WIP pages map damage, tuning, upgrades, objects, Havok, NPCs, effects, and assets.");
        panel.label("overview_note", "Inactive pages stop their raycasts, draw calls, and camera writes.");
    }

    if state.page == "Telemetry" {
        panel.heading("telemetry_frame_heading", "RUNTIME / FRAME");
        panel.label("telemetry_phase", "Phase  " + ctx.frame.phase);
        panel.label("telemetry_sequence", "Frame  " + ctx.format.fixed(ctx.frame.sequence) + "   Sample  " + ctx.format.fixed(ctx.frame.sample_sequence));
        panel.label("telemetry_timing", "Delta  " + ctx.format.fixed(ctx.frame.delta_seconds) + "   Elapsed  " + ctx.format.fixed(ctx.frame.elapsed_seconds));
        panel.label("telemetry_sample_age", "Sample age  " + optional_number(ctx, ctx.frame.sample_age_seconds));
        panel.heading("telemetry_camera_heading", "CAMERA");
        panel.label("telemetry_camera_position", "Position  " + ctx.format.vector(ctx.camera.position));
        panel.label("telemetry_camera_direction", "View  " + ctx.format.vector(ctx.camera.direction));
        panel.heading("telemetry_player_heading", "PLAYER / VEHICLE");
        panel.label("telemetry_player", "Player pos  " + ctx.format.vector(ctx.player.position) + "   fwd  " + ctx.format.vector(ctx.player.forward));
        panel.label("telemetry_vehicle", "Vehicle pos  " + ctx.format.vector(ctx.vehicle.position) + "   fwd  " + ctx.format.vector(ctx.vehicle.forward));
        panel.label("telemetry_motion", "Vehicle vel  " + ctx.format.vector(ctx.vehicle.velocity) + "   accel  " + ctx.format.vector(ctx.vehicle.acceleration));
    }

    if state.page == "Camera" {
        panel.status("camera_status", if camera_hook { "EXPERIMENTAL · native camera getter override" } else { "PROBLEMATIC · camera hook unavailable" }, if camera_hook { "experimental" } else { "problematic" });
        panel.checkbox("camera_enabled", "Enable camera pose adjustment", state.camera_enabled);
        panel.heading("camera_offset_heading", "WORLD OFFSET");
        panel.slider("camera_x", "X", state.camera_offset.x, -50.0, 50.0);
        panel.slider("camera_y", "Y", state.camera_offset.y, -50.0, 50.0);
        panel.slider("camera_z", "Z", state.camera_offset.z, -50.0, 50.0);
        panel.heading("camera_rotation_heading", "WORLD ROTATION · DEGREES");
        panel.slider("camera_yaw", "Yaw", state.camera_rotation.x, -90.0, 90.0);
        panel.slider("camera_pitch", "Pitch", state.camera_rotation.y, -90.0, 90.0);
        panel.slider("camera_roll", "Roll", state.camera_rotation.z, -90.0, 90.0);
        panel.button("camera_reset", "Reset camera adjustment");
        panel.label("camera_current", "Position  " + ctx.format.vector(ctx.camera.position));
        panel.label("camera_view", "Direction  " + ctx.format.vector(ctx.camera.direction));
        panel.label("camera_detach", "WIP · native-follow camera only; no detached freecam or GameObject transform write.");
    }

    if state.page == "Raycast" {
        panel.status("ray_status", "EXPERIMENTAL · one deferred camera query while enabled", "experimental");
        panel.checkbox("ray_enabled", "Run continuous camera ray", state.ray_enabled);
        panel.select("ray_filter", "Filter", state.ray_filter, ["player_aim", "static_only"]);
        panel.slider("ray_range", "Range", state.ray_range, 1.0, 500.0);
        panel.checkbox("ray_show_line", "Draw ray line", state.ray_show_line);
        panel.checkbox("ray_show_hit", "Draw hit marker", state.ray_show_hit);
        panel.label("ray_state", "Result  " + state.ray_status + "   Query  " + state.ray_query_status);
        panel.label("ray_origin", "Origin  " + ctx.format.vector(ctx.camera.position));
        panel.label("ray_direction", "Direction  " + ctx.format.vector(ctx.camera.direction));
        panel.label("ray_hit", "Hit  " + ctx.format.vector(state.ray_hit));
        panel.label("ray_fraction", "Fraction  " + optional_number(ctx, state.ray_fraction));
        panel.label("ray_gate", "Off-page or unchecked = no query or debug draw.");
    }

    if state.page == "Vehicle" {
        panel.status("vehicle_status", if ctx.vehicle.available { "EXPERIMENTAL · current vehicle velocity write" } else { "PROBLEMATIC · no vehicle sample" }, if ctx.vehicle.available { "experimental" } else { "problematic" });
        panel.label("vehicle_position", "Position  " + ctx.format.vector(ctx.vehicle.position));
        panel.label("vehicle_forward", "Forward  " + ctx.format.vector(ctx.vehicle.forward));
        panel.label("vehicle_velocity", "Velocity  " + ctx.format.vector(ctx.vehicle.velocity));
        panel.label("vehicle_acceleration", "Acceleration  " + ctx.format.vector(ctx.vehicle.acceleration));
        panel.label("vehicle_rotation", "Rotation  " + ctx.format.vector(ctx.vehicle.rotation));
        panel.label("vehicle_angular", "Angular velocity  " + ctx.format.vector(ctx.vehicle.angular_velocity));
        panel.heading("vehicle_write_heading", "BOUNDED VELOCITY WRITE · UNITS/S");
        panel.slider("vehicle_x", "Target X", state.vehicle_velocity.x, -250.0, 250.0);
        panel.slider("vehicle_y", "Target Y", state.vehicle_velocity.y, -250.0, 250.0);
        panel.slider("vehicle_z", "Target Z", state.vehicle_velocity.z, -250.0, 250.0);
        panel.button("vehicle_copy", "Copy current velocity");
        panel.button("apply_velocity", "Apply velocity");
        panel.button("jump", "One-shot +18 Y");
        panel.label("vehicle_action", state.vehicle_action);
        panel.label("vehicle_write_note", "Write only occurs on a button edge with copied velocity available.");
    }

    if state.page == "Player" {
        panel.status("player_status", if ctx.player.available { "STABLE · copied player sample" } else { "PROBLEMATIC · player sample unavailable" }, if ctx.player.available { "stable" } else { "problematic" });
        panel.label("player_position", "Position  " + ctx.format.vector(ctx.player.position));
        panel.label("player_forward", "Actor forward  " + ctx.format.vector(ctx.player.forward));
        panel.label("player_rotation", "Rotation  " + ctx.format.vector(ctx.player.rotation));
        panel.label("player_velocity", "Velocity  " + ctx.format.vector(ctx.player.velocity));
        panel.label("player_acceleration", "Acceleration  " + ctx.format.vector(ctx.player.acceleration));
        panel.label("player_transform", "Copied transform  " + (if ctx.player.transform == () { "--" } else { "available" }));
        panel.status("player_edit_gap", "PROBLEMATIC · no player transform/health/upgrades setter", "problematic");
    }

    if state.page == "Spawn probe" {
        panel.status("spawn_probe_status", "EXPERIMENTAL · one fresh camera ray then one catalog request", "experimental");
        panel.select("spawn_id", "Entity", state.spawn_id, ["props.scrap_stockpile_small", "props.loot.roadkill_lootbox"]);
        panel.slider("spawn_range", "Range", state.spawn_range, 1.0, 500.0);
        panel.button("spawn_selected", "Place selected at aim");
        panel.button("spawn_cancel", "Cancel pending placement");
        panel.label("spawn_operation", "Operation  " + state.spawn_status);
        panel.label("spawn_hit", "Last hit  " + ctx.format.vector(state.spawn_hit));
        panel.label("spawn_queue", "Queue  " + (if ctx.entities.ready() { "ready" } else { "full / unavailable" }));
        panel.label("spawn_id_value", "Last request  " + optional_number(ctx, state.spawn_request));
        panel.label("spawn_note", "Submission is not proof of successful spawn; check the console lifecycle report.");
    }

    if state.page == "Gizmo probe" {
        panel.status("gizmo_status", "EXPERIMENTAL · bounded native CDebugRenderer shapes", "experimental");
        panel.checkbox("gizmo_enabled", "Draw all mapped variants", state.gizmo_enabled);
        panel.slider("gizmo_scale", "Scale", state.gizmo_scale, 0.25, 4.0);
        panel.label("gizmo_variants", "Axes · line · arrow · wire/xray spheres · AABB · wire/xray box");
        panel.label("gizmo_solids", "Solid cube · solid 3D arrow · surfaces follow player/vehicle forward.");
        panel.label("gizmo_budget", "Draw stops when this page or toggle is off; 11 primitives per active frame.");
    }

    if state.page == "Math / orientation" {
        panel.heading("math_basis_heading", "COORDINATE BASIS");
        panel.label("math_basis", "+X right   +Y up   -Z forward");
        panel.label("math_player_forward", "Player  " + ctx.format.vector(ctx.player.forward));
        panel.label("math_vehicle_forward", "Vehicle  " + ctx.format.vector(ctx.vehicle.forward));
        panel.label("math_camera_direction", "Camera view  " + ctx.format.vector(ctx.camera.direction));
        let alignment = ();
        if ctx.player.forward != () && ctx.vehicle.forward != () {
            alignment = ctx.math.dot(ctx.player.forward, ctx.vehicle.forward);
        }
        panel.label("math_actor_alignment", "Player·vehicle dot  " + optional_number(ctx, alignment));
        panel.label("math_camera_player_dot", "Camera pitch differs from actor-forward plane; compare vectors above.");
        panel.label("math_matrix", "Transforms use row-major game matrix; translation indices 12–14.");
    }

    if state.page == "Input / UI" {
        let input_active = ctx.input.available();
        panel.status("input_status", if input_active { "EXPERIMENTAL · F1-captured mod input active" } else { "PROBLEMATIC · press F1 to capture mod input" }, if input_active { "experimental" } else { "problematic" });
        panel.label("input_space", "Space down  " + (if input_active { if ctx.input.down("space") { "yes" } else { "no" } } else { "--" }));
        panel.label("input_mouse", "Mouse1 down  " + (if input_active { if ctx.input.down("mouse1") { "yes" } else { "no" } } else { "--" }));
        panel.button("ui_event_probe", "Test script-panel event");
        panel.label("input_event", state.input_event);
        panel.label("input_boundary", "UI button events are not native game/controller input.");
        panel.label("input_wip", "WIP · activate safe sampler; prove native suppression before custom weapon binds.");
    }

    if state.page == "Vehicle tuning" {
        panel.heading("tuning_heading", "COMPILED VEHICLE UPGRADE ROUTE");
        panel.status("tuning_status", "PROBLEMATIC · script symbols found; no Rust tuning API", "problematic");
        panel.label("tuning_mutators", "acceleration.xvmc: ChangeMass, ChangeAccelerationScale, SetRevStartupTime");
        panel.label("tuning_getters", "Engine torque/power RPM and upgrade scale getters are named there too.");
        panel.label("tuning_configs", "21 engine + 22 physics config resources; decoding and live routing remain WIP.");
        panel.label("tuning_next", "TODO · receiver, argument contract, effect timing, and reversible reset.");
    }

    if state.page == "Weapons / damage" {
        panel.heading("damage_heading", "WEAPON DAMAGE FLOW");
        panel.status("damage_status", "PROBLEMATIC · no active weapon/damage setter in the bridge", "problematic");
        panel.label("damage_event", "bullet_damage_handler: HitByBullet, Damage, Impulse, AmmunitionType");
        panel.label("damage_object", "body_health_state: GetBodyHealth, GetMaxHealth, SetHealth (object health)");
        panel.label("damage_next", "TODO · trace damage source/scaling and identify a targetable weapon/object handle.");
        panel.label("damage_ray", "CWeapon::GetRayWorldPosition remains mapped for ray-origin research only.");
    }

    if state.page == "Max upgrades" {
        panel.heading("upgrade_heading", "PLAYER / VEHICLE UPGRADES");
        panel.status("upgrade_status", "PROBLEMATIC · upgrade state and equipment setters unmapped", "problematic");
        panel.label("upgrade_methods", "acceleration.xvmc: ActivateEnhacementLevel / DeactivateEnhacementLevel");
        panel.label("upgrade_assets", "mmax_head_upgrades, player_upgrades, upgradeablevaluesmanager, visibleplayerupgrades");
        panel.label("upgrade_next", "TODO · connect UI/config records to the live player and vehicle objects.");
    }

    if state.page == "GameObjects / Havok" {
        panel.heading("object_heading", "WORLD OBJECTS / PHYSICS");
        panel.status("object_status", "PROBLEMATIC · per-instance enumeration/edit/removal API unmapped", "problematic");
        panel.label("object_known", "Known: catalog spawn can create selected CRigidObject packages with collision assets.");
        panel.label("object_query", "Known: deferred world ray query works with static_only and player_aim filters.");
        panel.label("object_next", "TODO · live object handles, transforms, Havok bodies, material and collision controls.");
        panel.label("object_removal", "TODO · selection identity and safe removal lifecycle for map/spawned objects.");
    }

    if state.page == "NPC / animation" {
        panel.heading("npc_heading", "CHARACTERS / ANIMATION");
        panel.status("npc_status", "PROBLEMATIC · individual NPC and animation runtime APIs unmapped", "problematic");
        panel.label("npc_assets", "Entity index includes CCharacter records and 12,919 animation paths.");
        panel.label("npc_offset", "char_add_bb_offset_rotation is an animation task, not a general world transform setter.");
        panel.label("npc_next", "TODO · isolate single-character spawn, skeleton/rig, state and animation controls.");
    }

    if state.page == "Effects / assets" {
        panel.heading("effects_heading", "MEDIA / RESOURCE TOOLS");
        panel.status("effects_status", "PROBLEMATIC · no in-game effect/audio spawn or preview API", "problematic");
        panel.label("effects_inventory", "Index: effects, textures, scripts/shaders, animation and sound-bank resources.");
        panel.label("effects_gallery", "Offline searchable inventory: target/content-index/asset-workbench.html");
        panel.label("effects_preview", "Local previews cover selected DDS textures, compiled strings, and model samples.");
        panel.label("effects_next", "TODO · runtime effect/audio dispatch; model, texture, sound and animation viewers.");
    }

    ctx.ui.publish(panel);
}
