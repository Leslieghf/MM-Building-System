fn on_post_sim(ctx) {
    if ctx.camera.position != () && ctx.camera.direction != () {
        let camera_origin = ctx.camera.position;
        let direction = ctx.camera.direction;
        let current_tip = ctx.math.add(camera_origin, ctx.math.scale(direction, 18.0));
        let reverse_tip = ctx.math.subtract(camera_origin, ctx.math.scale(direction, 18.0));
        let origin_color = #{r: 1.0, g: 0.9, b: 0.0, a: 1.0};
        let current_color = #{r: 0.1, g: 1.0, b: 0.2, a: 1.0};
        let reverse_color = #{r: 1.0, g: 0.1, b: 1.0, a: 1.0};
        ctx.gizmos.wire_sphere(camera_origin, 0.45, origin_color);
        ctx.gizmos.arrow(camera_origin, current_tip, current_color);
        ctx.gizmos.arrow(camera_origin, reverse_tip, reverse_color);
    }

    if state.ticket != () {
        let result = ctx.raycast.result(state.ticket);
        if result.state == "complete" && result.has_hit && result.hit_position != () {
            let hit = result.hit_position;
            let red = #{r: 1.0, g: 0.05, b: 0.05, a: 1.0};
            let cyan = #{r: 0.0, g: 1.0, b: 1.0, a: 1.0};

            ctx.gizmos.surface_wire_sphere(hit, 1.0, red);
            ctx.gizmos.xray_sphere(hit, 1.5, cyan);
        }
    }

    if ctx.camera.position == () || ctx.camera.direction == () {
        return;
    }
    if state.callback_sequence != ctx.frame.sequence {
        state.ticket = ctx.raycast.request(
            ctx.camera.position,
            ctx.camera.direction,
            500.0,
            "static_only"
        );
        state.callback_sequence = ctx.frame.sequence;
    }
}
