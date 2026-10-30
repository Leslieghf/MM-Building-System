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
    let red = #{r: 1.0, g: 0.0, b: 0.0, a: 1.0};
    let green = #{r: 0.0, g: 1.0, b: 0.0, a: 1.0};
    let blue = #{r: 0.0, g: 0.35, b: 1.0, a: 1.0};
    let yellow = #{r: 1.0, g: 0.85, b: 0.0, a: 1.0};
    let cyan = #{r: 0.0, g: 1.0, b: 1.0, a: 1.0};
    let magenta = #{r: 1.0, g: 0.0, b: 1.0, a: 1.0};
    let orange = #{r: 1.0, g: 0.35, b: 0.0, a: 1.0};
    let white = #{r: 1.0, g: 1.0, b: 1.0, a: 1.0};

    ctx.gizmos.axes(center, 40.0);
    ctx.gizmos.line(point(center, -20.0, 0.0, 0.0), point(center, 20.0, 0.0, 0.0), red);
    ctx.gizmos.line(point(center, 0.0, -20.0, 0.0), point(center, 0.0, 20.0, 0.0), green);
    ctx.gizmos.line(point(center, 0.0, 0.0, -20.0), point(center, 0.0, 0.0, 20.0), blue);
    ctx.gizmos.arrow(center, point(center, 0.0, 2.5, 0.0), yellow);

    // Facade slots 26–28: sparse, queued sparse, and tessellated wire spheres.
    ctx.gizmos.sphere(point(center, -2.0, 2.0, 0.0), 0.75, cyan);
    ctx.gizmos.sparse_sphere(point(center, 0.0, 2.0, 0.0), 0.75, magenta);
    ctx.gizmos.tessellated_sphere(point(center, 2.0, 2.0, 0.0), 0.75, orange);

    // Slots 31, 34, and 35: direct and transformed AABBs.
    ctx.gizmos.aabb(point(center, -3.0, 4.0, -0.75), point(center, -1.0, 6.0, 0.75), white);
    let transformed_min = #{x: -0.75, y: -0.75, z: -0.75};
    let transformed_max = #{x: 0.75, y: 0.75, z: 0.75};
    ctx.gizmos.transformed_aabb(transform_at(point(center, 0.0, 4.5, 0.0)), transformed_min, transformed_max, cyan);
    ctx.gizmos.queued_transformed_aabb(transform_at(point(center, 3.0, 4.5, 0.0)), transformed_min, transformed_max, magenta);

    // Slots 36–39 are experimental: Ghidra shows a 64-bit transform argument,
    // interpreted here as a pointer to this 4x4 matrix.
    let bounds_min = #{x: -0.75, y: -0.75, z: -0.75};
    let bounds_max = #{x: 0.75, y: 0.75, z: 0.75};
    ctx.gizmos.bounds_sphere(transform_at(point(center, -4.5, 7.0, 0.0)), bounds_min, bounds_max, yellow);
    ctx.gizmos.bounds_cylinder(transform_at(point(center, -1.5, 7.0, 0.0)), bounds_min, bounds_max, cyan);
    ctx.gizmos.bounds_capsule(transform_at(point(center, 1.5, 7.0, 0.0)), bounds_min, bounds_max, magenta);
    ctx.gizmos.bounds_convex_vertices(transform_at(point(center, 4.5, 7.0, 0.0)), bounds_min, bounds_max, orange);
}
