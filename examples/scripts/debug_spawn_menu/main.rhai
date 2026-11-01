fn identity_at(position) {
    [1.0, 0.0, 0.0, 0.0,
     0.0, 1.0, 0.0, 0.0,
     0.0, 0.0, 1.0, 0.0,
     position.x, position.y, position.z, 1.0]
}

fn on_post_sim(ctx) {
    if !state.contains("initialized") {
        state.initialized = true;
        state.max_distance = 120.0;
        state.filter = "player_aim";
        state.show_marker = false;
        state.ticket = ();
        state.pending_spawn = false;
        state.spawn_catalog_id = "props.scrap_stockpile_small";
        state.hit = ();
        state.status = "ready";
        state.last_spawn = "none";
    }

    let events = ctx.ui.events("spawn_workbench");
    if events.contains("distance") { state.max_distance = events.distance; }
    if events.contains("filter") { state.filter = events.filter; }
    if events.contains("show_marker") { state.show_marker = events.show_marker; }

    let completed_this_callback = false;
    if state.ticket != () {
        let result = ctx.raycast.result(state.ticket);
        if result.state == "complete" {
            state.ticket = ();
            completed_this_callback = true;
            if result.has_hit {
                state.hit = result.hit_position;
                state.status = "hit";
                if state.pending_spawn {
                    let placement = #{
                        x: state.hit.x,
                        y: state.hit.y + 0.15,
                        z: state.hit.z
                    };
                    if ctx.entities.ready() {
                        ctx.entities.request(state.spawn_catalog_id, identity_at(placement));
                        state.last_spawn = "request queued; awaiting native callback";
                    } else {
                        state.last_spawn = "spawn queue busy; click again";
                    }
                    state.pending_spawn = false;
                }
            } else {
                state.hit = ();
                state.status = "no hit";
                if state.pending_spawn { state.last_spawn = "aim ray has no hit"; }
                state.pending_spawn = false;
            }
        } else if result.state == "unavailable" || result.state == "expired" {
            state.ticket = ();
            completed_this_callback = true;
            state.hit = ();
            state.status = result.state;
            if state.pending_spawn { state.last_spawn = "raycast unavailable"; }
            state.pending_spawn = false;
        }
    }

    if events.contains("catalog_selection") {
        state.spawn_catalog_id = events.catalog_selection;
        state.pending_spawn = true;
        state.status = "waiting for placement query";
    }

    if state.pending_spawn && state.ticket == () && !completed_this_callback {
        if ctx.camera.position == () || ctx.camera.direction == () {
            state.status = "camera unavailable";
            state.last_spawn = "camera unavailable";
            state.pending_spawn = false;
        } else {
            state.ticket = ctx.raycast.camera(state.max_distance, state.filter);
            if state.ticket != () {
                state.status = "query pending";
            } else {
                state.status = "raycast unavailable";
                state.last_spawn = "raycast unavailable";
                state.pending_spawn = false;
            }
        }
    }

    if state.show_marker && state.hit != () {
        ctx.gizmos.surface_wire_sphere(
            state.hit,
            0.7,
            #{r: 0.1, g: 0.9, b: 1.0, a: 1.0}
        );
        ctx.gizmos.xray_sphere(
            state.hit,
            1.0,
            #{r: 1.0, g: 0.15, b: 0.25, a: 1.0}
        );
    }

    let panel = ctx.ui.panel("spawn_workbench", "PLACEMENT TOOLS");
    panel.heading("placement", "PLACEMENT");
    panel.select("filter", "Query filter", state.filter, ["player_aim", "static_only"]);
    panel.slider("distance", "Range", state.max_distance, 1.0, 500.0);
    panel.checkbox("show_marker", "Show last hit marker", state.show_marker);
    panel.label("ray_state", "Placement query  " + state.status);
    panel.label("hit_position", "Last hit  " + ctx.format.vector(state.hit));
    panel.label("spawn_state", "Last spawn  " + state.last_spawn);
    ctx.ui.publish(panel);
}
