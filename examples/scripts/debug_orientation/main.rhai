// Raw transform basis calibration.
// Native CDebugRenderer axes colors: red = matrix row 0, green = row 1,
// blue = row 2. In-game observation indicates actor transforms use row 0 =
// right, row 1 = up, row 2 = backward. Arrows compare the corrected public
// forward vector (green) with its reverse (magenta).
fn point(base, x, y, z) {
    #{x: base.x + x, y: base.y + y, z: base.z + z}
}

fn draw_forward_candidates(ctx, position, forward, height, length) {
    if position == () || forward == () {
        return;
    }

    let origin = point(position, 0.0, height, 0.0);
    let positive = point(origin, forward.x * length, 0.0, forward.z * length);
    let negative = point(origin, -forward.x * length, 0.0, -forward.z * length);
    let positive_color = #{r: 0.15, g: 1.0, b: 0.15, a: 1.0};
    let negative_color = #{r: 1.0, g: 0.1, b: 1.0, a: 1.0};

    ctx.gizmos.arrow(origin, positive, positive_color);
    ctx.gizmos.arrow(origin, negative, negative_color);
}

// Player axes are short; vehicle axes and forward candidates are longer.
fn on_post_sim(ctx) {
    if ctx.player.transform != () {
        ctx.gizmos.transformed_axes(ctx.player.transform, 4.0);
        draw_forward_candidates(ctx, ctx.player.position, ctx.player.forward, 1.0, 6.0);
    }
    if ctx.vehicle.transform != () {
        ctx.gizmos.transformed_axes(ctx.vehicle.transform, 8.0);
        draw_forward_candidates(ctx, ctx.vehicle.position, ctx.vehicle.forward, 2.0, 12.0);
    }
}
