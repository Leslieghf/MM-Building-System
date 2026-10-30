//! Read-only sampling that turns validated native observations into owned values.
mod actors;
mod camera;
mod effects;
mod telemetry;
pub(super) use actors::*;
pub(super) use camera::*;
pub(super) use effects::*;
mod world_query_capture;
pub(in crate::api::game::adapter) use telemetry::*;
pub(in crate::api::game::adapter) use world_query_capture::*;

use super::*;

pub(super) fn current_game_snapshot(thread_id: u32) -> GameSnapshot {
    let player = PLAYER_SAMPLE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let player_fresh = player
        .sampled_at
        .is_some_and(|sampled_at| sampled_at.elapsed() <= PLAYER_SAMPLE_MAX_AGE);
    let player_stale = player.status == 10 && !player_fresh;
    let vehicle = VEHICLE_SAMPLE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let vehicle_fresh = vehicle
        .sampled_at
        .is_some_and(|sampled_at| sampled_at.elapsed() <= PLAYER_SAMPLE_MAX_AGE);
    let latest_sample_at = player.sampled_at.max(vehicle.sampled_at);
    let sample_age_seconds = latest_sample_at.map(|sampled_at| sampled_at.elapsed().as_secs_f32());

    GameSnapshot {
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
        player_position: (!player_stale).then_some(player.position).flatten(),
        player_forward: (!player_stale).then_some(player.forward).flatten(),
        player_transform: (!player_stale).then_some(player.transform).flatten(),
        player_rotation: (!player_stale).then_some(player.rotation).flatten(),
        view_rotation: (!player_stale).then_some(player.view_rotation).flatten(),
        reticle_ray_origin: (!player_stale)
            .then_some(player.reticle_ray_origin)
            .flatten(),
        reticle_ray_direction: (!player_stale)
            .then_some(player.reticle_ray_direction)
            .flatten(),
        reticle_query_available: !player_stale && player.reticle_query_available,
        reticle_query_state_present: !player_stale && player.reticle_query_state_present,
        reticle_hit_fraction: (!player_stale)
            .then_some(player.reticle_hit_fraction)
            .flatten(),
        reticle_candidate_position: (!player_stale)
            .then_some(player.reticle_candidate_position)
            .flatten(),
        reticle_candidate_shape: (!player_stale)
            .then_some(player.reticle_candidate_shape)
            .flatten(),
        player_aim_query_available: !player_stale && player.player_aim_query_available,
        player_aim_query_state_present: !player_stale && player.player_aim_query_state_present,
        player_aim_hit_fraction: (!player_stale)
            .then_some(player.player_aim_hit_fraction)
            .flatten(),
        player_aim_candidate_position: (!player_stale)
            .then_some(player.player_aim_candidate_position)
            .flatten(),
        player_aim_candidate_shape: (!player_stale)
            .then_some(player.player_aim_candidate_shape)
            .flatten(),
        ignore_characters_vehicles_query_available: !player_stale
            && player.ignore_characters_vehicles_query_available,
        ignore_characters_vehicles_query_state_present: !player_stale
            && player.ignore_characters_vehicles_query_state_present,
        ignore_characters_vehicles_hit_fraction: (!player_stale)
            .then_some(player.ignore_characters_vehicles_hit_fraction)
            .flatten(),
        ignore_characters_vehicles_candidate_position: (!player_stale)
            .then_some(player.ignore_characters_vehicles_candidate_position)
            .flatten(),
        ignore_characters_vehicles_candidate_shape: (!player_stale)
            .then_some(player.ignore_characters_vehicles_candidate_shape)
            .flatten(),
        ignore_player_owner_query_available: !player_stale
            && player.ignore_player_owner_query_available,
        ignore_player_owner_query_state_present: !player_stale
            && player.ignore_player_owner_query_state_present,
        ignore_player_owner_hit_fraction: (!player_stale)
            .then_some(player.ignore_player_owner_fraction)
            .flatten(),
        ignore_player_owner_candidate_position: (!player_stale)
            .then_some(player.ignore_player_owner_candidate_position)
            .flatten(),
        ignore_player_owner_candidate_shape: (!player_stale)
            .then_some(player.ignore_player_owner_candidate_shape)
            .flatten(),
        player_velocity: (!player_stale).then_some(player.velocity).flatten(),
        player_acceleration: (!player_stale).then_some(player.acceleration).flatten(),
        vehicle_position: vehicle_fresh.then_some(vehicle.position).flatten(),
        vehicle_transform: vehicle_fresh.then_some(vehicle.transform).flatten(),
        vehicle_forward: vehicle_fresh.then_some(vehicle.forward).flatten(),
        vehicle_rotation: vehicle_fresh.then_some(vehicle.rotation).flatten(),
        vehicle_velocity: vehicle_fresh.then_some(vehicle.velocity).flatten(),
        vehicle_acceleration: vehicle_fresh.then_some(vehicle.acceleration).flatten(),
        vehicle_angular_velocity: vehicle_fresh.then_some(vehicle.angular_velocity).flatten(),
        vehicle_velocity_last_frame: vehicle_fresh
            .then_some(vehicle.velocity_last_frame)
            .flatten(),
        ..GameSnapshot::default()
    }
}

pub(super) fn probe_game_state() {
    if !readable_span(CHARACTER_MANAGER_SINGLETON, size_of::<*mut c_void>()) {
        publish_player_sample(1, None, None, None);
        publish_vehicle_sample(None);
        return;
    }
    if !is_readable_executable_page(GET_PLAYER_CHARACTER) {
        publish_player_sample(2, None, None, None);
        publish_vehicle_sample(None);
        return;
    }

    let manager = unsafe { ptr::read_volatile(CHARACTER_MANAGER_SINGLETON as *const *mut c_void) };
    if manager.is_null() || !readable_span(manager as usize, 1) {
        publish_player_sample(1, None, None, None);
        publish_vehicle_sample(None);
        return;
    }

    let get_player_character: GetPlayerCharacter =
        unsafe { std::mem::transmute(GET_PLAYER_CHARACTER) };
    let player = unsafe { get_player_character(manager.cast::<CharacterManager>()) };
    if player.is_null() {
        publish_player_sample(3, None, None, None);
        publish_vehicle_sample(None);
        return;
    }
    if !readable_span(player as usize, 1) {
        publish_player_sample(4, None, None, None);
        publish_vehicle_sample(None);
        return;
    }

    probe_player_transform(player);
    publish_vehicle_sample(sample_vehicle(player));
}
