//! Copied frame data imported from the game-native boundary.
//!
//! These values contain owned observations only; native addresses are diagnostic identifiers, not handles.
use super::super::{IntentOutcome, SpawnOutcome};
use crate::api::game::abi::types::Matrix4x4;
use bevy_ecs::prelude::Resource;

#[derive(Resource, Debug, Clone, Copy, Default)]
/// Latest copied game callback data available to ECS systems.
pub struct CurrentGameFrame(pub GameSnapshot);

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
    /// Camera-world position copied from the current camera matrix.
    pub camera_position: Option<[f32; 3]>,
    /// Normalized candidate view direction copied from the camera matrix.
    pub camera_direction: Option<[f32; 3]>,
    /// Latest completed `reticle_ray` script result; absent for misses.
    pub raycast_hit_position: Option<[f32; 3]>,
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
