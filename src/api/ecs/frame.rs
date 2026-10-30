//! Schedule labels and copied game-frame ECS projections.
use super::{IntentOutcome, SpawnOutcome};
use crate::api::game::abi::types::Matrix4x4;
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ScheduleLabel;

#[derive(ScheduleLabel, Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Game phase executed at the host's accepted post-simulation callback.
pub struct MadMaxPostSim;

#[derive(ScheduleLabel, Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Mod-owned render/presentation phase executed from the render hook.
pub struct MadMaxRender;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Ordered sets within [`MadMaxPostSim`].
pub enum MadMaxPostSimSet {
    /// Import current frame data and pending requests.
    Import,
    /// Run compiled Rust systems and managed script callbacks.
    ModSystems,
    /// Validate and export bounded effects and outcomes.
    Export,
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Ordered sets within [`MadMaxRender`].
pub enum MadMaxRenderSet {
    /// Run mod-owned UI preparation systems.
    ModSystems,
    /// Build and present developer-tools UI.
    Presentation,
}

#[derive(Resource, Debug, Clone, Copy, Default)]
/// Latest copied game callback data available to ECS systems.
pub struct CurrentGameFrame(pub GameSnapshot);

/// Diagnostic identity captured from the accepted shape pointer in a native
/// world-query context. These addresses are observations, not safe object handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Observed identity of a candidate shape; addresses are diagnostic only.
pub struct RaycastCandidateShape {
    /// Native address observed for the candidate shape.
    pub address: usize,
    /// Native vtable address observed for the shape.
    pub vtable: usize,
}

#[derive(Debug, Clone, Copy, Default)]
/// Owned values copied from one accepted game callback.
///
/// Optional observations are absent when a value could not be sampled. Native
/// addresses in diagnostic fields are not handles and must not be dereferenced.
pub struct GameSnapshot {
    /// Mod-owned input sampled by the render hook, not a native game keymap.
    pub input_frame: crate::api::ecs::input::InputFrame,
    /// Monotonic host callback count associated with the sample.
    pub callback_sequence: u64,
    /// Whether camera-ray telemetry hook installation succeeded.
    pub camera_raycast_hook_installed: bool,
    /// Captured camera-ray hook invocation count.
    pub camera_raycast_calls: u64,
    /// Whether the general world-query telemetry hook is installed.
    pub world_query_hook_installed: bool,
    /// Captured general world-query invocation count.
    pub world_query_calls: u64,
    /// Last observed return address, for diagnostics only.
    pub world_query_last_return_address: Option<usize>,
    /// Seconds since the previous accepted callback.
    pub callback_delta_seconds: f32,
    /// Seconds elapsed since callback timing began.
    pub callback_elapsed_seconds: f64,
    /// Monotonic sequence of the copied telemetry sample.
    pub sample_sequence: u64,
    /// Age of the copied telemetry in seconds, when known.
    pub sample_age_seconds: Option<f32>,
    /// Number of completed post-simulation schedules.
    pub post_sim_schedule_runs: u64,
    /// Number of completed game mod-system runs.
    pub game_mod_system_runs: u64,
    /// Windows thread identifier of the callback, when available.
    pub callback_thread_id: Option<u32>,
    /// Consecutive callbacks accepted on a stable thread.
    pub callback_stable_streak: u32,
    /// Number of callbacks observed overlapping another callback.
    pub callback_overlap_count: u64,
    /// Number of callbacks rejected for thread mismatch.
    pub callback_thread_mismatch_count: u64,
    /// Whether exclusive callback access is currently accepted.
    pub callback_gate_open: bool,
    /// Number of panics caught at the callback boundary.
    pub callback_panics: u64,
    /// Number of snapshots dropped because bounded transport was full.
    pub dropped_snapshots: u64,
    /// Number of typed effects dropped because bounded transport was full.
    pub dropped_intents: u64,
    /// Most recent velocity-effect outcome.
    pub last_intent_outcome: Option<IntentOutcome>,
    /// Most recent spawn-request outcome.
    pub last_spawn_outcome: Option<SpawnOutcome>,
    /// Sampled player position in game world units.
    pub player_position: Option<[f32; 3]>,
    /// Horizontal projection of the player transform's forward basis.
    pub player_forward: Option<[f32; 3]>,
    /// Copied row-major player transform.
    pub player_transform: Option<Matrix4x4>,
    /// Sampled player rotation values.
    pub player_rotation: Option<[f32; 3]>,
    /// Sampled view/camera rotation values.
    pub view_rotation: Option<[f32; 3]>,
    /// Camera ray submitted through the static-only query filter.
    pub reticle_ray_origin: Option<[f32; 3]>,
    /// Direction of the sampled reticle ray.
    pub reticle_ray_direction: Option<[f32; 3]>,
    /// Whether the static-only reticle query could be called.
    pub reticle_query_available: bool,
    /// Whether the static-only query reported an internal query state.
    pub reticle_query_state_present: bool,
    /// Caller-observed segment fraction; values below one are candidate hits
    /// for this filter/query contract, not a universal WorldQueryHelper rule.
    pub reticle_hit_fraction: Option<f32>,
    /// Point reconstructed as origin + direction * (range * fraction).
    pub reticle_candidate_position: Option<[f32; 3]>,
    pub reticle_candidate_shape: Option<RaycastCandidateShape>,
    /// Whether the player-aim filter query could be called.
    pub player_aim_query_available: bool,
    /// Whether the player-aim query reported an internal state.
    pub player_aim_query_state_present: bool,
    /// Candidate hit fraction for the player-aim filter.
    pub player_aim_hit_fraction: Option<f32>,
    /// Reconstructed candidate point for the player-aim filter.
    pub player_aim_candidate_position: Option<[f32; 3]>,
    /// Candidate shape observed for the player-aim filter.
    pub player_aim_candidate_shape: Option<RaycastCandidateShape>,
    /// Whether the ignore-characters-and-vehicles query could be called.
    pub ignore_characters_vehicles_query_available: bool,
    /// Whether that query reported an internal state.
    pub ignore_characters_vehicles_query_state_present: bool,
    /// Candidate fraction returned for that query.
    pub ignore_characters_vehicles_hit_fraction: Option<f32>,
    /// Reconstructed candidate point for that query.
    pub ignore_characters_vehicles_candidate_position: Option<[f32; 3]>,
    /// Candidate shape observed for that query.
    pub ignore_characters_vehicles_candidate_shape: Option<RaycastCandidateShape>,
    /// Whether the player-owner exclusion query could be called.
    pub ignore_player_owner_query_available: bool,
    /// Whether the owner-exclusion query reported an internal state.
    pub ignore_player_owner_query_state_present: bool,
    /// Candidate fraction returned for the owner-exclusion query.
    pub ignore_player_owner_hit_fraction: Option<f32>,
    /// Reconstructed candidate point for the owner-exclusion query.
    pub ignore_player_owner_candidate_position: Option<[f32; 3]>,
    /// Candidate shape observed for the owner-exclusion query.
    pub ignore_player_owner_candidate_shape: Option<RaycastCandidateShape>,
    /// Sampled player linear velocity.
    pub player_velocity: Option<[f32; 3]>,
    /// Sampled player acceleration.
    pub player_acceleration: Option<[f32; 3]>,
    /// Sampled current vehicle position.
    pub vehicle_position: Option<[f32; 3]>,
    /// Copied row-major current vehicle transform.
    pub vehicle_transform: Option<Matrix4x4>,
    /// Horizontal projection of the current vehicle transform's forward basis.
    pub vehicle_forward: Option<[f32; 3]>,
    /// Sampled current vehicle rotation.
    pub vehicle_rotation: Option<[f32; 3]>,
    /// Sampled current vehicle linear velocity.
    pub vehicle_velocity: Option<[f32; 3]>,
    /// Sampled current vehicle acceleration.
    pub vehicle_acceleration: Option<[f32; 3]>,
    /// Sampled current vehicle angular velocity.
    pub vehicle_angular_velocity: Option<[f32; 3]>,
    /// Previously sampled vehicle linear velocity, when available.
    pub vehicle_velocity_last_frame: Option<[f32; 3]>,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
/// Marker on the ECS entity representing the sampled player character.
pub struct PlayerCharacterProxy;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
/// Marker on the ECS entity representing the current player vehicle.
pub struct CurrentPlayerVehicleProxy;

#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
/// Optional player values projected from the current [`GameSnapshot`].
pub struct PlayerFrameData {
    /// World position in game units.
    pub position: Option<[f32; 3]>,
    /// Horizontal normalized forward direction when available.
    pub forward: Option<[f32; 3]>,
    /// Copied row-major transform.
    pub transform: Option<Matrix4x4>,
    /// Sampled rotation values.
    pub rotation: Option<[f32; 3]>,
    /// Sampled camera/view rotation values.
    pub view_rotation: Option<[f32; 3]>,
    /// Sampled linear velocity.
    pub velocity: Option<[f32; 3]>,
    /// Sampled acceleration.
    pub acceleration: Option<[f32; 3]>,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
/// Optional current-vehicle values projected from the current [`GameSnapshot`].
pub struct VehicleFrameData {
    /// World position in game units.
    pub position: Option<[f32; 3]>,
    /// Horizontal normalized forward direction when available.
    pub forward: Option<[f32; 3]>,
    /// Copied row-major transform.
    pub transform: Option<Matrix4x4>,
    /// Sampled rotation values.
    pub rotation: Option<[f32; 3]>,
    /// Sampled linear velocity.
    pub velocity: Option<[f32; 3]>,
    /// Sampled acceleration.
    pub acceleration: Option<[f32; 3]>,
    /// Sampled angular velocity.
    pub angular_velocity: Option<[f32; 3]>,
    /// Linear velocity from the previous sample, when available.
    pub velocity_last_frame: Option<[f32; 3]>,
}
