fn identity_at(position) {
    [1.0, 0.0, 0.0, 0.0,
     0.0, 1.0, 0.0, 0.0,
     0.0, 0.0, 1.0, 0.0,
     position.x, position.y, position.z, 1.0]
}

fn offset(position, x, y, z) {
    #{x: position.x + x, y: position.y + y, z: position.z + z}
}

fn batch_position(state) {
    let index = state.batch_index;
    let spacing = state.spacing;
    if state.mode == "line_x" {
        return offset(state.anchor, (index - (state.count - 1) / 2.0) * spacing, 0.15, 0.0);
    }
    if state.mode == "line_z" {
        return offset(state.anchor, 0.0, 0.15, (index - (state.count - 1) / 2.0) * spacing);
    }
    if state.mode == "stack_y" {
        return offset(state.anchor, 0.0, 0.15 + index * spacing, 0.0);
    }
    offset(state.anchor, 0.0, 0.15, 0.0)
}

fn on_post_sim(ctx) {
    if !state.contains("initialized") {
        state.initialized = true;
        state.enabled = false;
        state.distance = 120.0;
        state.filter = "player_aim";
        state.ticket = ();
        state.hit = ();
        state.mode = "";
        state.anchor = ();
        state.count = 4.0;
        state.spacing = 2.5;
        state.batch_index = 0.0;
        state.last_status = "idle";
    }

    let events = ctx.ui.events("placement_lab");
    if events.contains("enabled") { state.enabled = events.enabled; }
    if events.contains("distance") { state.distance = events.distance; }
    if events.contains("filter") { state.filter = events.filter; }
    if events.contains("count") { state.count = events.count; }
    if events.contains("spacing") { state.spacing = events.spacing; }
    if events.contains("cancel") && events.cancel {
        state.mode = "";
        state.last_status = "batch cancelled; queued requests may still complete";
    }

    if state.ticket != () {
        let result = ctx.raycast.result(state.ticket);
        if result.state == "complete" {
            if result.has_hit { state.hit = result.hit_position; }
            else { state.hit = (); }
        } else if result.state == "unavailable" || result.state == "expired" {
            state.hit = ();
        }
    }
    if state.enabled && ctx.camera.position != () && ctx.camera.direction != () {
        state.ticket = ctx.raycast.request(
            ctx.camera.position,
            ctx.math.normalize(ctx.camera.direction),
            state.distance,
            state.filter
        );
    } else {
        state.ticket = ();
        state.hit = ();
    }

    if events.contains("line_x") && events.line_x && state.hit != () {
        state.mode = "line_x";
        state.anchor = state.hit;
        state.batch_index = 0.0;
        state.last_status = "queueing X-axis line";
    } else if events.contains("line_z") && events.line_z && state.hit != () {
        state.mode = "line_z";
        state.anchor = state.hit;
        state.batch_index = 0.0;
        state.last_status = "queueing Z-axis line";
    } else if events.contains("stack") && events.stack && state.hit != () {
        state.mode = "stack_y";
        state.anchor = state.hit;
        state.batch_index = 0.0;
        state.last_status = "queueing vertical stack";
    }

    if state.mode != "" && state.batch_index < state.count && ctx.entities.ready() {
        let position = batch_position(state);
        let _request_id = ctx.entities.request(
            "props.scrap_stockpile_small",
            identity_at(position)
        );
        state.batch_index += 1.0;
        state.last_status = "queued " + ctx.format.fixed(state.batch_index)
            + "/" + ctx.format.fixed(state.count);
        if state.batch_index >= state.count {
            state.mode = "";
        }
    } else if state.mode != "" {
        state.last_status = "waiting for bounded spawn queue";
    }

    let panel = ctx.ui.panel("placement_lab", "Spawn Placement Lab");
    panel.heading("ray", "PLACEMENT RAY");
    panel.checkbox("enabled", "Enable aim ray", state.enabled);
    panel.select("filter", "Query filter", state.filter, ["player_aim", "static_only"]);
    panel.slider("distance", "Range", state.distance, 1.0, 500.0);
    panel.label("hit", "Hit position  " + ctx.format.vector(state.hit));
    panel.heading("layout", "STOCKPILE LAYOUTS");
    panel.status("lab_status", "EXPERIMENTAL · batch placement is not yet validated in-game", "experimental");
    panel.slider("count", "Items per batch", state.count, 1.0, 8.0);
    panel.slider("spacing", "Spacing", state.spacing, 1.0, 8.0);
    panel.button("line_x", "Queue line along X");
    panel.button("line_z", "Queue line along Z");
    panel.button("stack", "Queue vertical stack");
    panel.button("cancel", "Cancel remaining batch");
    panel.label("status", "Queue  " + state.last_status);
    panel.label("scope", "Verified asset  Scrap Stockpile only");
    ctx.ui.publish(panel);
}
