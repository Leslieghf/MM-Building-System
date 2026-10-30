// Run manually: script load vehicle_jump; then script run vehicle_jump.
// No recurring callback: each accepted run requests exactly one jump.
fn on_run(ctx) {
    let velocity = ctx.vehicle.velocity;
    if velocity == () {
        ctx.log.info("vehicle_jump: current vehicle velocity unavailable; enter a vehicle first");
        return;
    }
    // Mad Max world height uses Y. Preserve horizontal momentum.
    ctx.vehicle.set_velocity(#{x: velocity.x, y: velocity.y + 18.0, z: velocity.z});
    ctx.log.info("vehicle_jump: requested +18.000 units/s upward velocity; native result follows");
}
