fn point(base, x, y, z) {
    #{x: base.x + x, y: base.y + y, z: base.z + z}
}

// Flat Mad Max 4x4 layout: translation occupies elements 12, 13, and 14.
fn transform_at(position) {
    [1.0, 0.0, 0.0, 0.0,
     0.0, 1.0, 0.0, 0.0,
     0.0, 0.0, 1.0, 0.0,
     position.x, position.y, position.z, 1.0]
}

fn on_post_sim(ctx) {
    let base = ctx.player.position;
    let forward = ctx.player.forward;
    if ctx.vehicle.position != () && ctx.vehicle.forward != () {
        base = ctx.vehicle.position;
        forward = ctx.vehicle.forward;
    }
    if base == () || forward == () {
        return;
    }

    // Put the test cluster 20 units ahead of the active vehicle, or Max on foot.
    // Forward is projected onto the ground plane so the rows stay level.
    let ahead = point(base, forward.x * 20.0, 0.0, forward.z * 20.0);
    let center = point(ahead, 0.0, 1.5, 0.0);
    let actor_transform = ctx.player.transform;
    if ctx.vehicle.transform != () {
        actor_transform = ctx.vehicle.transform;
    }
    let red = #{r: 1.0, g: 0.0, b: 0.0, a: 1.0};
    let yellow = #{r: 1.0, g: 0.85, b: 0.0, a: 1.0};
    let cyan = #{r: 0.0, g: 1.0, b: 1.0, a: 1.0};
    let magenta = #{r: 1.0, g: 0.0, b: 1.0, a: 1.0};
    let orange = #{r: 1.0, g: 0.35, b: 0.0, a: 1.0};
    let white = #{r: 1.0, g: 1.0, b: 1.0, a: 1.0};

    ctx.gizmos.axes(center, 40.0);
    ctx.gizmos.transformed_axes(transform_at(center), 40.0);
    ctx.gizmos.line(point(center, -1.5, 0.5, 0.0), point(center, 1.5, 0.5, 0.0), red);
    ctx.gizmos.arrow(center, point(center, 0.0, 2.5, 0.0), yellow);

    // Radial wire sphere, its through-wall variant, and surface-following rings.
    ctx.gizmos.wire_sphere(point(center, -2.0, 2.0, 0.0), 0.75, cyan);
    ctx.gizmos.xray_sphere(point(center, 0.0, 2.0, 0.0), 0.75, magenta);
    ctx.gizmos.surface_wire_sphere(point(center, 2.0, 2.0, 0.0), 0.75, orange);

    // Axis-aligned wire box, transformed wire box, and its through-wall variant.
    ctx.gizmos.wire_aabb(point(center, -3.0, 4.0, -0.75), point(center, -1.0, 6.0, 0.75), white);
    let transformed_min = #{x: -0.75, y: -0.75, z: -0.75};
    let transformed_max = #{x: 0.75, y: 0.75, z: 0.75};
    ctx.gizmos.wire_box(transform_at(point(center, 0.0, 4.5, 0.0)), transformed_min, transformed_max, cyan);
    ctx.gizmos.xray_wire_box(transform_at(point(center, 3.0, 4.5, 0.0)), transformed_min, transformed_max, magenta);

    // The two distinct solid forms retained from the native bounds-shape experiment.
    let solid_min = #{x: -0.75, y: -0.75, z: -0.75};
    let solid_max = #{x: 0.75, y: 0.75, z: 0.75};
    ctx.gizmos.solid_cube(transform_at(point(center, -1.5, 7.0, 0.0)), solid_min, solid_max, yellow);
    ctx.gizmos.solid_arrow(transform_at(point(center, 1.5, 7.0, 0.0)), solid_min, solid_max, orange);

    // Show the active actor's oriented axes separately from the world-aligned marker above.
    if actor_transform != () {
        ctx.gizmos.transformed_axes(actor_transform, 5.0);
    }
}
