fn vector(x, y, z) {
    #{x: x, y: y, z: z}
}

fn axis(input, positive, negative) {
    (if input.down(positive) { 1.0 } else { 0.0 }) - (if input.down(negative) { 1.0 } else { 0.0 })
}

fn clamp(value, minimum, maximum) {
    if value < minimum { minimum } else if value > maximum { maximum } else { value }
}

fn on_post_sim(ctx) {
    if !state.contains("initialized") {
        state.initialized = true;
        state.enabled = false;
        state.position = vector(0.0, 0.0, 0.0);
        state.applied_offset = vector(0.0, 0.0, 0.0);
        state.rotation = vector(0.0, 0.0, 0.0);
        state.speed = 12.0;
        state.turn_rate = 75.0;
        state.status = "Disabled";
        state.override_requested = false;
    }

    let events = ctx.ui.events("freecam_workbench");
    if events.contains("speed") { state.speed = events.speed; }
    if events.contains("turn_rate") { state.turn_rate = events.turn_rate; }
    if events.contains("position_x") { state.position.x = events.position_x; }
    if events.contains("position_y") { state.position.y = events.position_y; }
    if events.contains("position_z") { state.position.z = events.position_z; }
    if events.contains("yaw") { state.rotation.x = events.yaw; }
    if events.contains("pitch") { state.rotation.y = events.pitch; }
    if events.contains("roll") { state.rotation.z = events.roll; }

    if events.contains("enabled") {
        state.enabled = events.enabled;
        if state.enabled && ctx.camera.available {
            state.position = ctx.camera.position;
            state.applied_offset = vector(0.0, 0.0, 0.0);
            state.rotation = vector(0.0, 0.0, 0.0);
            state.status = "Detached pose initialized from current camera";
        } else if !state.enabled {
            state.status = "Disabled; native camera restored";
        }
    }
    if events.contains("reset_pose") && ctx.camera.available {
        let base = ctx.math.subtract(ctx.camera.position, state.applied_offset);
        state.position = base;
        state.rotation = vector(0.0, 0.0, 0.0);
        state.status = "Pose reset to current game camera";
    }

    let input_available = ctx.input.available();
    let hook_available = ctx.camera_control.available();
    if state.enabled && ctx.camera.available && hook_available {
        let delta = clamp(ctx.frame.delta_seconds, 0.0, 0.05);
        if input_available {
            let forward = ctx.math.normalize(ctx.camera.direction);
            let right = if forward == () {
                vector(1.0, 0.0, 0.0)
            } else {
                let candidate = ctx.math.normalize(ctx.math.cross(forward, vector(0.0, 1.0, 0.0)));
                if candidate == () { vector(1.0, 0.0, 0.0) } else { candidate }
            };
            let move_forward = axis(ctx.input, "w", "s");
            let move_right = axis(ctx.input, "d", "a");
            let move_up = axis(ctx.input, "space", "lctrl");
            let movement = ctx.math.add(
                ctx.math.add(ctx.math.scale(if forward == () { vector(0.0, 0.0, -1.0) } else { forward }, move_forward), ctx.math.scale(right, move_right)),
                vector(0.0, move_up, 0.0)
            );
            let move_direction = ctx.math.normalize(movement);
            if move_direction != () {
                state.position = ctx.math.add(state.position, ctx.math.scale(move_direction, state.speed * delta));
            }
            state.rotation.x = clamp(state.rotation.x + axis(ctx.input, "right", "left") * state.turn_rate * delta, -180.0, 180.0);
            state.rotation.y = clamp(state.rotation.y + axis(ctx.input, "up", "down") * state.turn_rate * delta, -85.0, 85.0);
            state.status = "Active · keyboard pose control";
        } else {
            state.status = "Active · F1 capture required for keyboard movement";
        }

        let base_position = ctx.math.subtract(ctx.camera.position, state.applied_offset);
        let offset = ctx.math.subtract(state.position, base_position);
        ctx.camera_control.set_offset(offset, state.rotation);
        state.applied_offset = offset;
        state.override_requested = true;
    } else if state.override_requested {
        ctx.camera_control.clear();
        state.applied_offset = vector(0.0, 0.0, 0.0);
        state.override_requested = false;
        if !ctx.camera.available { state.status = "Camera sample unavailable; override cleared"; }
    }

    if state.enabled && !hook_available { state.status = "Camera override hook unavailable"; }
    let panel = ctx.ui.panel("freecam_workbench", "Free Camera");
    panel.status("hook_status", if hook_available { "EXPERIMENTAL · native camera getter override" } else { "PROBLEMATIC · camera override hook unavailable" }, if hook_available { "experimental" } else { "problematic" });
    panel.checkbox("enabled", "Enable detached camera pose", state.enabled);
    panel.slider("speed", "Move speed", state.speed, 1.0, 100.0);
    panel.slider("turn_rate", "Turn rate", state.turn_rate, 10.0, 180.0);
    panel.heading("world_position_heading", "WORLD POSITION");
    panel.number("position_x", "X", state.position.x, -100000.0, 100000.0, 0.25);
    panel.number("position_y", "Y", state.position.y, -100000.0, 100000.0, 0.25);
    panel.number("position_z", "Z", state.position.z, -100000.0, 100000.0, 0.25);
    panel.heading("rotation_heading", "ADDITIVE CAMERA ROTATION · DEGREES");
    panel.slider("yaw", "Yaw", state.rotation.x, -180.0, 180.0);
    panel.slider("pitch", "Pitch", state.rotation.y, -85.0, 85.0);
    panel.slider("roll", "Roll", state.rotation.z, -180.0, 180.0);
    panel.button("reset_pose", "Reset to current game camera");
    panel.label("position_value", "Target  " + ctx.format.vector(state.position));
    panel.label("camera_value", "Game camera  " + ctx.format.vector(ctx.camera.position));
    panel.label("getter_calls", "Game getter calls  " + ctx.format.fixed(ctx.camera_control.external_calls()));
    panel.label("override_calls", "Override applied  " + ctx.format.fixed(ctx.camera_control.external_override_calls()));
    panel.status("input_status", if input_available { "STABLE · F1-captured mod keyboard input" } else { "PROBLEMATIC · press F1 to capture mod input" }, if input_available { "stable" } else { "problematic" });
    panel.label("movement_keys", "WASD move · Space / Left Ctrl vertical · arrows turn");
    panel.label("camera_limit", "Position detaches from Max; view still uses the native camera getter and additive rotation.");
    panel.label("native_input_limit", "F1 captures overlay keyboard/mouse messages; gamepad and native keybinds are separate WIP.");
    panel.label("status", state.status);
    ctx.ui.publish(panel);
}
