// The host supplies an empty persistent `state` map for each generation.
// Top-level statements are not executed; initialize inside a callback.
fn on_post_sim(ctx) {
    let frame = ctx.frame;
    if !state.contains("recent") {
        state.recent = [];
        state.last_log = -2.0;
    }

    // At most one sample/log per second, rather than per host callback.
    if frame.elapsed_seconds - state.last_log < 1.0 {
        return;
    }
    state.last_log = frame.elapsed_seconds;

    let vehicle = ctx.vehicle;
    let sample = #{
        sequence: frame.sequence,
        vehicle_position: vehicle.position,
        vehicle_velocity: vehicle.velocity,
        vehicle_acceleration: vehicle.acceleration
    };
    // Keep eight samples. Each map contains only script-owned copied values.
    if state.recent.len() >= 8 {
        state.recent.remove(0);
    }
    state.recent.push(sample);

    let output = "Vehicle Position: " + ctx.format.vector(vehicle.position)
        + "\nVehicle Velocity: " + ctx.format.vector(vehicle.velocity)
        + "\nVehicle Acceleration: " + ctx.format.vector(vehicle.acceleration);
    ctx.log.info(output);
}

fn on_run(ctx) {
    // A queued on_run precedes on_post_sim, including its first initialization.
    let count = 0;
    if state.contains("recent") {
        count = state.recent.len();
    }
    let vehicle = ctx.vehicle;
    ctx.log.info(`${count} retained samples | vehicle data: ${vehicle.available}`);
}
