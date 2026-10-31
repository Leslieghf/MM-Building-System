//! Read-only sampling that turns validated native observations into owned values.
//!
//! `actors`, `camera`, and `reticle` decode their respective observations;
//! `telemetry` defines the bounded publication records, and
//! `world_query_capture` owns experimental diagnostic capture. Sampling does
//! not retain game pointers or apply script effects.
mod actors;
mod camera;
mod reticle;
mod telemetry;
pub(super) use actors::*;
pub(super) use camera::*;
pub(super) use reticle::*;
mod world_query_capture;
pub(in crate::api::game::native) use telemetry::*;
pub(in crate::api::game::native) use world_query_capture::*;

use super::*;

pub(super) fn current_game_snapshot(thread_id: u32) -> GameSnapshot {
    let player = PLAYER_SAMPLE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let player_fresh = player
        .sampled_at
        .is_some_and(|sampled_at| sampled_at.elapsed() <= PLAYER_SAMPLE_MAX_AGE);
    let player_stale = player.status == 10 && !player_fresh;
    let camera = CAMERA_SAMPLE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let camera_fresh = camera
        .sampled_at
        .is_some_and(|sampled_at| sampled_at.elapsed() <= Duration::from_millis(100));
    let vehicle = VEHICLE_SAMPLE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let vehicle_fresh = vehicle
        .sampled_at
        .is_some_and(|sampled_at| sampled_at.elapsed() <= PLAYER_SAMPLE_MAX_AGE);
    let latest_sample_at = player.sampled_at.max(vehicle.sampled_at);
    let sample_age_seconds = latest_sample_at.map(|sampled_at| sampled_at.elapsed().as_secs_f32());

    let mut snapshot = GameSnapshot {
        callback_sequence: UPDATE_POST_SIM_CALLS.load(Ordering::Acquire),
        camera_raycast_hook_installed: CAMERA_RAYCAST_HOOK_INSTALLED.load(Ordering::Acquire),
        camera_raycast_calls: CAMERA_RAYCAST_CALLS.load(Ordering::Acquire),
        world_query_hook_installed: WORLD_QUERY_HOOK_INSTALLED.load(Ordering::Acquire),
        world_query_calls: WORLD_QUERY_CALLS.load(Ordering::Acquire),
        world_query_last_return_address: match WORLD_QUERY_LAST_RETURN_ADDRESS
            .load(Ordering::Acquire)
        {
            0 => None,
            address => Some(address),
        },
        sample_sequence: GAME_SAMPLE_SEQUENCE.load(Ordering::Acquire),
        sample_age_seconds,
        callback_thread_id: Some(thread_id),
        callback_stable_streak: CALLBACK_STABLE_STREAK.load(Ordering::Acquire),
        callback_overlap_count: CALLBACK_OVERLAPS.load(Ordering::Acquire),
        callback_thread_mismatch_count: CALLBACK_THREAD_MISMATCHES.load(Ordering::Acquire),
        callback_gate_open: CALLBACK_GATE_OPEN.load(Ordering::Acquire),
        callback_panics: CALLBACK_RUNTIME_PANICS.load(Ordering::Acquire),
        ..GameSnapshot::default()
    };
    copy_player_snapshot(&mut snapshot, &player, player_stale);
    copy_camera_snapshot(&mut snapshot, &camera, camera_fresh);
    copy_vehicle_snapshot(&mut snapshot, &vehicle, vehicle_fresh);
    snapshot
}

pub(super) fn sample_camera_for_callback() {
    let camera = sample_camera_matrix().and_then(|matrix| {
        let ray = WorldQueryRayInput::from_camera_matrix(&matrix)?;
        Some((
            [ray.origin.x, ray.origin.y, ray.origin.z],
            [ray.direction.x, ray.direction.y, ray.direction.z],
            rotation_xyz_degrees(&matrix),
        ))
    });
    let mut sample = CAMERA_SAMPLE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    match camera {
        Some((position, direction, view_rotation)) => {
            sample.position = Some(position);
            sample.direction = Some(direction);
            sample.view_rotation = view_rotation;
            sample.sampled_at = Some(Instant::now());
        }
        None => {
            sample.position = None;
            sample.direction = None;
            sample.view_rotation = None;
            sample.sampled_at = None;
        }
    }
}

fn copy_player_snapshot(snapshot: &mut GameSnapshot, player: &PlayerSampleState, stale: bool) {
    snapshot.player_position = (!stale).then_some(player.position).flatten();
    snapshot.player_forward = (!stale).then_some(player.forward).flatten();
    snapshot.player_transform = (!stale).then_some(player.transform).flatten();
    snapshot.player_rotation = (!stale).then_some(player.rotation).flatten();
    snapshot.player_velocity = (!stale).then_some(player.velocity).flatten();
    snapshot.player_acceleration = (!stale).then_some(player.acceleration).flatten();
}

fn copy_camera_snapshot(snapshot: &mut GameSnapshot, camera: &CameraSampleState, fresh: bool) {
    snapshot.camera_position = fresh.then_some(camera.position).flatten();
    snapshot.camera_direction = fresh.then_some(camera.direction).flatten();
    snapshot.view_rotation = fresh.then_some(camera.view_rotation).flatten();
}

#[cfg(test)]
mod camera_snapshot_tests {
    use super::*;

    #[test]
    fn camera_pose_is_independent_of_player_telemetry_staleness() {
        let camera = CameraSampleState {
            position: Some([1.0, 2.0, 3.0]),
            direction: Some([0.0, 0.0, -1.0]),
            view_rotation: Some([0.0, 0.0, 45.0]),
            sampled_at: Some(Instant::now()),
        };
        let mut snapshot = GameSnapshot::default();

        copy_camera_snapshot(&mut snapshot, &camera, true);

        assert_eq!(snapshot.camera_position, Some([1.0, 2.0, 3.0]));
        assert_eq!(snapshot.camera_direction, Some([0.0, 0.0, -1.0]));
        assert_eq!(snapshot.view_rotation, Some([0.0, 0.0, 45.0]));
    }

    #[test]
    fn stale_camera_pose_is_not_republished() {
        let camera = CameraSampleState {
            position: Some([1.0, 2.0, 3.0]),
            direction: Some([0.0, 0.0, -1.0]),
            view_rotation: Some([0.0, 0.0, 45.0]),
            sampled_at: Some(Instant::now()),
        };
        let mut snapshot = GameSnapshot::default();

        copy_camera_snapshot(&mut snapshot, &camera, false);

        assert_eq!(snapshot.camera_position, None);
        assert_eq!(snapshot.camera_direction, None);
        assert_eq!(snapshot.view_rotation, None);
    }
}

fn copy_vehicle_snapshot(snapshot: &mut GameSnapshot, vehicle: &VehicleSampleState, fresh: bool) {
    snapshot.vehicle_position = fresh.then_some(vehicle.position).flatten();
    snapshot.vehicle_transform = fresh.then_some(vehicle.transform).flatten();
    snapshot.vehicle_forward = fresh.then_some(vehicle.forward).flatten();
    snapshot.vehicle_rotation = fresh.then_some(vehicle.rotation).flatten();
    snapshot.vehicle_velocity = fresh.then_some(vehicle.velocity).flatten();
    snapshot.vehicle_acceleration = fresh.then_some(vehicle.acceleration).flatten();
    snapshot.vehicle_angular_velocity = fresh.then_some(vehicle.angular_velocity).flatten();
    snapshot.vehicle_velocity_last_frame = fresh.then_some(vehicle.velocity_last_frame).flatten();
}

pub(super) fn probe_game_state() {
    if !readable_span(CHARACTER_MANAGER_SINGLETON, size_of::<*mut c_void>()) {
        publish_player_sample(1, None, None);
        publish_vehicle_sample(None);
        return;
    }
    if !is_readable_executable_page(GET_PLAYER_CHARACTER) {
        publish_player_sample(2, None, None);
        publish_vehicle_sample(None);
        return;
    }

    let manager = unsafe { ptr::read_volatile(CHARACTER_MANAGER_SINGLETON as *const *mut c_void) };
    if manager.is_null() || !readable_span(manager as usize, 1) {
        publish_player_sample(1, None, None);
        publish_vehicle_sample(None);
        return;
    }

    let get_player_character: GetPlayerCharacter =
        unsafe { std::mem::transmute(GET_PLAYER_CHARACTER) };
    let player = unsafe { get_player_character(manager.cast::<CharacterManager>()) };
    if player.is_null() {
        publish_player_sample(3, None, None);
        publish_vehicle_sample(None);
        return;
    }
    if !readable_span(player as usize, 1) {
        publish_player_sample(4, None, None);
        publish_vehicle_sample(None);
        return;
    }

    probe_player_transform(player);
    publish_vehicle_sample(sample_vehicle(player));
}
