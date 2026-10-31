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
    copy_vehicle_snapshot(&mut snapshot, &vehicle, vehicle_fresh);
    snapshot
}

fn copy_player_snapshot(snapshot: &mut GameSnapshot, player: &PlayerSampleState, stale: bool) {
    snapshot.player_position = (!stale).then_some(player.position).flatten();
    snapshot.player_forward = (!stale).then_some(player.forward).flatten();
    snapshot.player_transform = (!stale).then_some(player.transform).flatten();
    snapshot.player_rotation = (!stale).then_some(player.rotation).flatten();
    snapshot.view_rotation = (!stale).then_some(player.view_rotation).flatten();
    snapshot.reticle_ray_origin = (!stale).then_some(player.reticle_ray_origin).flatten();
    snapshot.reticle_ray_direction = (!stale).then_some(player.reticle_ray_direction).flatten();
    snapshot.reticle_query_available = !stale && player.reticle_query_available;
    snapshot.reticle_query_state_present = !stale && player.reticle_query_state_present;
    snapshot.reticle_hit_fraction = (!stale).then_some(player.reticle_hit_fraction).flatten();
    snapshot.reticle_candidate_position = (!stale)
        .then_some(player.reticle_candidate_position)
        .flatten();
    snapshot.reticle_candidate_shape = (!stale).then_some(player.reticle_candidate_shape).flatten();
    snapshot.player_aim_query_available = !stale && player.player_aim_query_available;
    snapshot.player_aim_query_state_present = !stale && player.player_aim_query_state_present;
    snapshot.player_aim_hit_fraction = (!stale).then_some(player.player_aim_hit_fraction).flatten();
    snapshot.player_aim_candidate_position = (!stale)
        .then_some(player.player_aim_candidate_position)
        .flatten();
    snapshot.player_aim_candidate_shape = (!stale)
        .then_some(player.player_aim_candidate_shape)
        .flatten();
    snapshot.ignore_characters_vehicles_query_available =
        !stale && player.ignore_characters_vehicles_query_available;
    snapshot.ignore_characters_vehicles_query_state_present =
        !stale && player.ignore_characters_vehicles_query_state_present;
    snapshot.ignore_characters_vehicles_hit_fraction = (!stale)
        .then_some(player.ignore_characters_vehicles_hit_fraction)
        .flatten();
    snapshot.ignore_characters_vehicles_candidate_position = (!stale)
        .then_some(player.ignore_characters_vehicles_candidate_position)
        .flatten();
    snapshot.ignore_characters_vehicles_candidate_shape = (!stale)
        .then_some(player.ignore_characters_vehicles_candidate_shape)
        .flatten();
    snapshot.ignore_player_owner_query_available =
        !stale && player.ignore_player_owner_query_available;
    snapshot.ignore_player_owner_query_state_present =
        !stale && player.ignore_player_owner_query_state_present;
    snapshot.ignore_player_owner_hit_fraction = (!stale)
        .then_some(player.ignore_player_owner_fraction)
        .flatten();
    snapshot.ignore_player_owner_candidate_position = (!stale)
        .then_some(player.ignore_player_owner_candidate_position)
        .flatten();
    snapshot.ignore_player_owner_candidate_shape = (!stale)
        .then_some(player.ignore_player_owner_candidate_shape)
        .flatten();
    snapshot.player_velocity = (!stale).then_some(player.velocity).flatten();
    snapshot.player_acceleration = (!stale).then_some(player.acceleration).flatten();
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
