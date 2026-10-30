//! Runtime health and callback diagnostics shared as owned ECS data.
use super::{IntentOutcome, SpawnOutcome};
use bevy_ecs::prelude::Resource;

#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct RuntimeDiagnostics {
    pub post_sim_schedule_runs: u64,
    pub render_schedule_runs: u64,
    pub game_mod_system_runs: u64,
    pub ui_mod_system_runs: u64,
    pub snapshots_dropped: u64,
    pub intents_dropped: u64,
    pub intent_transport_full_count: u64,
    pub last_intent_outcome: Option<IntentOutcome>,
    pub last_spawn_outcome: Option<SpawnOutcome>,
    pub callback_thread_id: Option<u32>,
    pub callback_stable_streak: u32,
    pub callback_overlap_count: u64,
    pub callback_thread_mismatch_count: u64,
    pub callback_gate_open: bool,
}
