//! Schedule labels and copied game-frame ECS projections.
use super::*;

#[derive(ScheduleLabel, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MadMaxPostSim;

#[derive(ScheduleLabel, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MadMaxRender;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MadMaxPostSimSet {
    Import,
    ModSystems,
    Export,
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MadMaxRenderSet {
    ModSystems,
    Presentation,
}

#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct CurrentGameFrame(pub GameSnapshot);

/// Diagnostic identity captured from the accepted shape pointer in a native
/// world-query context. These addresses are observations, not safe object handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RaycastCandidateShape {
    pub address: usize,
    pub vtable: usize,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct GameSnapshot {
    /// Mod-owned input sampled by the render hook, not a native game keymap.
    pub input_frame: crate::api::ecs::input::InputFrame,
    pub callback_sequence: u64,
    pub camera_raycast_hook_installed: bool,
    pub camera_raycast_calls: u64,
    pub world_query_hook_installed: bool,
    pub world_query_calls: u64,
    pub world_query_last_return_address: Option<usize>,
    pub callback_delta_seconds: f32,
    pub callback_elapsed_seconds: f64,
    pub sample_sequence: u64,
    pub sample_age_seconds: Option<f32>,
    pub post_sim_schedule_runs: u64,
    pub game_mod_system_runs: u64,
    pub callback_thread_id: Option<u32>,
    pub callback_stable_streak: u32,
    pub callback_overlap_count: u64,
    pub callback_thread_mismatch_count: u64,
    pub callback_gate_open: bool,
    pub callback_panics: u64,
    pub dropped_snapshots: u64,
    pub dropped_intents: u64,
    pub last_intent_outcome: Option<IntentOutcome>,
    pub last_spawn_outcome: Option<SpawnOutcome>,
    pub player_position: Option<[f32; 3]>,
    /// Horizontal projection of the player transform's forward basis.
    pub player_forward: Option<[f32; 3]>,
    pub player_transform: Option<Matrix4x4>,
    pub player_rotation: Option<[f32; 3]>,
    pub view_rotation: Option<[f32; 3]>,
    /// Camera ray submitted through the static-only query filter.
    pub reticle_ray_origin: Option<[f32; 3]>,
    pub reticle_ray_direction: Option<[f32; 3]>,
    pub reticle_query_available: bool,
    pub reticle_query_state_present: bool,
    /// Caller-observed segment fraction; values below one are candidate hits
    /// for this filter/query contract, not a universal WorldQueryHelper rule.
    pub reticle_hit_fraction: Option<f32>,
    /// Point reconstructed as origin + direction * (range * fraction).
    pub reticle_candidate_position: Option<[f32; 3]>,
    pub reticle_candidate_shape: Option<RaycastCandidateShape>,
    pub player_aim_query_available: bool,
    pub player_aim_query_state_present: bool,
    pub player_aim_hit_fraction: Option<f32>,
    pub player_aim_candidate_position: Option<[f32; 3]>,
    pub player_aim_candidate_shape: Option<RaycastCandidateShape>,
    pub ignore_characters_vehicles_query_available: bool,
    pub ignore_characters_vehicles_query_state_present: bool,
    pub ignore_characters_vehicles_hit_fraction: Option<f32>,
    pub ignore_characters_vehicles_candidate_position: Option<[f32; 3]>,
    pub ignore_characters_vehicles_candidate_shape: Option<RaycastCandidateShape>,
    pub ignore_player_owner_query_available: bool,
    pub ignore_player_owner_query_state_present: bool,
    pub ignore_player_owner_hit_fraction: Option<f32>,
    pub ignore_player_owner_candidate_position: Option<[f32; 3]>,
    pub ignore_player_owner_candidate_shape: Option<RaycastCandidateShape>,
    pub player_velocity: Option<[f32; 3]>,
    pub player_acceleration: Option<[f32; 3]>,
    pub vehicle_position: Option<[f32; 3]>,
    pub vehicle_transform: Option<Matrix4x4>,
    /// Horizontal projection of the current vehicle transform's forward basis.
    pub vehicle_forward: Option<[f32; 3]>,
    pub vehicle_rotation: Option<[f32; 3]>,
    pub vehicle_velocity: Option<[f32; 3]>,
    pub vehicle_acceleration: Option<[f32; 3]>,
    pub vehicle_angular_velocity: Option<[f32; 3]>,
    pub vehicle_velocity_last_frame: Option<[f32; 3]>,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerCharacterProxy;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurrentPlayerVehicleProxy;

#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct PlayerFrameData {
    pub position: Option<[f32; 3]>,
    pub forward: Option<[f32; 3]>,
    pub transform: Option<Matrix4x4>,
    pub rotation: Option<[f32; 3]>,
    pub view_rotation: Option<[f32; 3]>,
    pub velocity: Option<[f32; 3]>,
    pub acceleration: Option<[f32; 3]>,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct VehicleFrameData {
    pub position: Option<[f32; 3]>,
    pub forward: Option<[f32; 3]>,
    pub transform: Option<Matrix4x4>,
    pub rotation: Option<[f32; 3]>,
    pub velocity: Option<[f32; 3]>,
    pub acceleration: Option<[f32; 3]>,
    pub angular_velocity: Option<[f32; 3]>,
    pub velocity_last_frame: Option<[f32; 3]>,
}
