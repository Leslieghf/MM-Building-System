//! Owned runtime health counters and latest callback/effect outcomes.
//!
//! Counters are cumulative for the runtime lifetime; latest-outcome fields are
//! replaced when a newer result arrives. Diagnostics contain no native handles
//! and may be copied to the UI for presentation without entering game code.
use super::super::{IntentOutcome, SpawnOutcome};
use bevy_ecs::prelude::Resource;

#[derive(Resource, Debug, Default, Clone, Copy)]
/// Owned health counters and most recent effect outcomes for the runtime.
pub struct RuntimeDiagnostics {
    /// Completed game post-simulation schedules.
    pub post_sim_schedule_runs: u64,
    /// Completed UI render schedules.
    pub render_schedule_runs: u64,
    /// Completed game mod-system phase executions.
    pub game_mod_system_runs: u64,
    /// Completed UI mod-system phase executions.
    pub ui_mod_system_runs: u64,
    /// Snapshots dropped because bounded transport was full.
    pub snapshots_dropped: u64,
    /// Effects dropped because bounded transport was full.
    pub intents_dropped: u64,
    /// Number of transport-full conditions observed.
    pub intent_transport_full_count: u64,
    /// Most recent velocity effect result.
    pub last_intent_outcome: Option<IntentOutcome>,
    /// Most recent spawn request result.
    pub last_spawn_outcome: Option<SpawnOutcome>,
    /// Thread identifier currently accepted for game callbacks.
    pub callback_thread_id: Option<u32>,
    /// Consecutive callback count on the accepted thread.
    pub callback_stable_streak: u32,
    /// Count of overlapping callback entries.
    pub callback_overlap_count: u64,
    /// Count of callback entries rejected for a thread mismatch.
    pub callback_thread_mismatch_count: u64,
    /// Whether the callback exclusivity gate currently permits game work.
    pub callback_gate_open: bool,
}
