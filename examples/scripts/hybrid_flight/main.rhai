fn vector(x, y, z) {
    #{x: x, y: y, z: z}
}

fn axis(input, positive, negative) {
    (if input.down(positive) { 1.0 } else { 0.0 }) - (if input.down(negative) { 1.0 } else { 0.0 })
}

fn on_post_sim(ctx) {
    if !state.contains("initialized") {
        state.initialized = true;
        state.enabled = false;
        state.speed = 35.0;
        state.status = "Disabled";
        state.last_write = "No velocity write sent";
    }

    let events = ctx.ui.events("hybrid_flight");
    if events.contains("speed") { state.speed = events.speed; }
    if events.contains("enabled") {
        state.enabled = events.enabled;
        if state.enabled && ctx.vehicle.available && ctx.vehicle.velocity != () {
            state.status = "Vehicle velocity control ready";
        } else if state.enabled {
            state.enabled = false;
            state.status = "No current vehicle; on-foot control is not mapped yet";
        } else {
            state.status = "Disabled · no further velocity writes";
        }
    }

    let input_available = ctx.input.available();
    if state.enabled && ctx.vehicle.available && ctx.vehicle.velocity != () {
        if input_available && ctx.camera.available {
            let forward = ctx.math.normalize(ctx.camera.direction);
            let forward = if forward == () { vector(0.0, 0.0, -1.0) } else { forward };
            let right = ctx.math.normalize(ctx.math.cross(forward, vector(0.0, 1.0, 0.0)));
            let right = if right == () { vector(1.0, 0.0, 0.0) } else { right };
            let movement = ctx.math.add(
                ctx.math.add(ctx.math.scale(forward, axis(ctx.input, "w", "s")), ctx.math.scale(right, axis(ctx.input, "d", "a"))),
                vector(0.0, axis(ctx.input, "space", "lctrl"), 0.0)
            );
            let direction = ctx.math.normalize(movement);
            let requested_velocity = if direction == () { vector(0.0, 0.0, 0.0) } else { ctx.math.scale(direction, state.speed) };
            ctx.vehicle.set_velocity(requested_velocity);
            state.last_write = "Direct velocity submitted";
            state.status = if direction == () { "Hover · zero velocity held" } else { "Flying · camera-relative velocity" };
        } else if !input_available {
            state.status = "Waiting for F1 input capture; velocity is unchanged";
            state.last_write = "No write sent";
        } else {
            state.status = "Camera sample unavailable; velocity is unchanged";
            state.last_write = "No write sent";
        }
    } else if state.enabled {
        state.enabled = false;
        state.status = "Vehicle velocity unavailable; flight disabled without another write";
    }

    let panel = ctx.ui.panel("hybrid_flight", "Hybrid Flight");
    panel.status("flight_status", if ctx.vehicle.available { "EXPERIMENTAL · direct vehicle velocity" } else { "PROBLEMATIC · no current vehicle" }, if ctx.vehicle.available { "experimental" } else { "problematic" });
    panel.checkbox("enabled", "Enable vehicle flight", state.enabled);
    panel.slider("speed", "Flight speed", state.speed, 1.0, 120.0);
    panel.label("velocity", "Velocity  " + ctx.format.vector(ctx.vehicle.velocity));
    panel.label("controls", "F1 captures input · W/S forward/back · A/D strafe · Space/Ctrl vertical");
    panel.label("behavior", "Camera-relative direct velocity; no movement input holds zero velocity");
    panel.label("state", state.status + " · " + state.last_write);
    panel.status("physics_boundary", "PROBLEMATIC · game physics and collisions remain active", "problematic");
    panel.label("max_wip", "On-foot Max control is pending a verified player movement adapter.");
    panel.label("input_wip", "F1 captures overlay keyboard/mouse messages; XInput and native keybind-menu routing are unverified.");
    ctx.ui.publish(panel);
}
