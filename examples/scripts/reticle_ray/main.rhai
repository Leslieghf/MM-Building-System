fn vector_with(x, y, z) {
    #{x: x, y: y, z: z}
}

fn on_post_sim(ctx) {
    if !state.contains("initialized") {
        state.initialized = true;
        state.enabled = false;
        state.origin_source = "camera";
        state.direction_source = "camera";
        state.filter = "static_only";
        state.max_distance = 100.0;
        state.custom_origin = vector_with(0.0, 0.0, 0.0);
        state.custom_direction = vector_with(0.0, 0.0, -1.0);
        state.show_ray = true;
        state.show_origin = true;
        state.show_hit = true;
        state.ticket = ();
        state.status = "disabled";
        state.hit = ();
        state.fraction = ();
        state.hit_distance = ();
        state.ticket_origin = ();
    }

    let changes = ctx.ui.events("ray_workbench");
    let query_configuration_changed = false;
    if changes.contains("enabled") {
        state.enabled = changes.enabled;
        query_configuration_changed = true;
    }
    if changes.contains("origin_source") {
        state.origin_source = changes.origin_source;
        query_configuration_changed = true;
    }
    if changes.contains("direction_source") {
        state.direction_source = changes.direction_source;
        query_configuration_changed = true;
    }
    if changes.contains("filter") {
        state.filter = changes.filter;
        query_configuration_changed = true;
    }
    if changes.contains("max_distance") {
        state.max_distance = changes.max_distance;
        query_configuration_changed = true;
    }
    if changes.contains("origin_x") { state.custom_origin.x = changes.origin_x; query_configuration_changed = true; }
    if changes.contains("origin_y") { state.custom_origin.y = changes.origin_y; query_configuration_changed = true; }
    if changes.contains("origin_z") { state.custom_origin.z = changes.origin_z; query_configuration_changed = true; }
    if changes.contains("direction_x") { state.custom_direction.x = changes.direction_x; query_configuration_changed = true; }
    if changes.contains("direction_y") { state.custom_direction.y = changes.direction_y; query_configuration_changed = true; }
    if changes.contains("direction_z") { state.custom_direction.z = changes.direction_z; query_configuration_changed = true; }
    if changes.contains("show_ray") { state.show_ray = changes.show_ray; }
    if changes.contains("show_origin") { state.show_origin = changes.show_origin; }
    if changes.contains("show_hit") { state.show_hit = changes.show_hit; }
    if query_configuration_changed {
        state.ticket = ();
        state.status = "configuration changed; refreshing";
        state.hit = ();
        state.fraction = ();
        state.hit_distance = ();
    }

    if state.ticket != () {
        let result = ctx.raycast.result(state.ticket);
        if result.state == "complete" {
            state.ticket = ();
            if result.has_hit && result.hit_position != () {
                state.status = "hit";
                state.hit = result.hit_position;
                state.fraction = result.fraction;
                if state.ticket_origin != () {
                    state.hit_distance = ctx.math.distance(state.ticket_origin, state.hit);
                }
            } else {
                state.status = "miss";
                state.hit = ();
                state.fraction = result.fraction;
                state.hit_distance = ();
            }
        } else if result.state == "unavailable" || result.state == "expired" {
            state.ticket = ();
            state.status = result.state;
            state.hit = ();
            state.fraction = ();
            state.hit_distance = ();
        } else if state.hit == () {
            state.status = "pending";
        }
    }

    let origin = ();
    if state.origin_source == "camera" { origin = ctx.camera.position; }
    if state.origin_source == "player" { origin = ctx.player.position; }
    if state.origin_source == "vehicle" { origin = ctx.vehicle.position; }
    if state.origin_source == "custom" { origin = state.custom_origin; }

    let direction = ();
    if state.direction_source == "camera" { direction = ctx.camera.direction; }
    if state.direction_source == "player" { direction = ctx.player.forward; }
    if state.direction_source == "vehicle" { direction = ctx.vehicle.forward; }
    if state.direction_source == "custom" { direction = state.custom_direction; }
    if direction != () { direction = ctx.math.normalize(direction); }

    if !state.enabled {
        state.ticket = ();
        state.status = "disabled";
        state.hit = ();
        state.fraction = ();
        state.hit_distance = ();
    } else if origin == () || direction == () {
        state.ticket = ();
        state.status = "source unavailable";
        state.hit = ();
        state.fraction = ();
        state.hit_distance = ();
    } else if state.ticket == () {
        state.ticket_origin = origin;
        if state.origin_source == "camera" && state.direction_source == "camera" {
            state.ticket = ctx.raycast.camera(state.max_distance, state.filter);
        } else {
            state.ticket = ctx.raycast.request(origin, direction, state.max_distance, state.filter);
        }
        if state.ticket != () { state.status = "updating"; }
    }

    if state.enabled && origin != () && direction != () {
        let ray_tip = ctx.math.add(origin, ctx.math.scale(direction, state.max_distance));
        let yellow = #{r: 1.0, g: 0.85, b: 0.0, a: 1.0};
        let cyan = #{r: 0.0, g: 1.0, b: 1.0, a: 1.0};
        if state.show_ray { ctx.gizmos.line(origin, ray_tip, yellow); }
        if state.show_origin { ctx.gizmos.wire_sphere(origin, 0.35, cyan); }
        if state.show_hit && state.hit != () {
            let red = #{r: 1.0, g: 0.1, b: 0.1, a: 1.0};
            ctx.gizmos.surface_wire_sphere(state.hit, 0.8, red);
            ctx.gizmos.xray_sphere(state.hit, 1.2, cyan);
        }
    }

    let panel = ctx.ui.panel("ray_workbench", "Ray Workbench");
    panel.heading("query", "RAYCAST");
    panel.checkbox("enabled", "Enable ray", state.enabled);
    panel.select("origin_source", "Origin", state.origin_source, ["camera", "player", "vehicle", "custom"]);
    panel.select("direction_source", "Direction", state.direction_source, ["camera", "player", "vehicle", "custom"]);
    panel.select("filter", "Filter", state.filter, ["static_only", "player_aim"]);
    panel.slider("max_distance", "Maximum distance", state.max_distance, 1.0, 500.0);
    panel.label("query_mode", "Continuous query  every game frame");

    if state.origin_source == "custom" {
        panel.heading("custom_origin_heading", "CUSTOM ORIGIN");
        panel.number("origin_x", "X", state.custom_origin.x, -100000.0, 100000.0, 0.1);
        panel.number("origin_y", "Y", state.custom_origin.y, -100000.0, 100000.0, 0.1);
        panel.number("origin_z", "Z", state.custom_origin.z, -100000.0, 100000.0, 0.1);
    }
    if state.direction_source == "custom" {
        panel.heading("custom_direction_heading", "CUSTOM DIRECTION");
        panel.number("direction_x", "X", state.custom_direction.x, -1.0, 1.0, 0.01);
        panel.number("direction_y", "Y", state.custom_direction.y, -1.0, 1.0, 0.01);
        panel.number("direction_z", "Z", state.custom_direction.z, -1.0, 1.0, 0.01);
    }

    panel.heading("current_ray", "CURRENT RAY");
    panel.label("origin_readout", "Origin  " + ctx.format.vector(origin));
    panel.label("direction_readout", "Direction  " + ctx.format.vector(direction));
    panel.label("range_readout", "Range  " + ctx.format.fixed(state.max_distance));
    panel.label("filter_readout", "Filter  " + state.filter);
    panel.heading("result", "LATEST RESULT");
    panel.label("status_readout", "State  " + state.status);
    panel.label("hit_readout", "Hit position  " + ctx.format.vector(state.hit));
    let fraction_text = "--";
    if state.fraction != () { fraction_text = ctx.format.fixed(state.fraction); }
    let distance_text = "--";
    if state.hit_distance != () { distance_text = ctx.format.fixed(state.hit_distance); }
    panel.label("fraction_readout", "Fraction  " + fraction_text);
    panel.label("distance_readout", "Hit distance  " + distance_text);
    panel.heading("visualization", "VISUALIZATION");
    panel.checkbox("show_ray", "Ray line", state.show_ray);
    panel.checkbox("show_origin", "Origin marker", state.show_origin);
    panel.checkbox("show_hit", "Hit marker", state.show_hit);
    ctx.ui.publish(panel);
}
