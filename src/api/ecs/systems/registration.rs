//! Registration points and ordering for first-party systems in both schedules.
//!
//! Game systems run inside the accepted post-simulation callback and may use
//! owned ECS state and typed intents only. UI systems run in the separate
//! render schedule and must not access game resources. User-authored Rhai
//! callbacks are dispatched by the host in the designated callback set.
use crate::api::ecs::{
    MadMaxPostSim, MadMaxPostSimSet, MadMaxRender, MadMaxRenderSet, RuntimeDiagnostics,
};
use bevy_app::App;
use bevy_ecs::prelude::ResMut;
use bevy_ecs::schedule::IntoScheduleConfigs;

/// Register first-party Rust systems into the host-driven game phase here.
/// Mod systems should use `MadMaxPostSimSet::ModSystems` and operate only on
/// ECS resources/components; native game calls stay in the adapter.
pub(crate) fn register_game_systems(app: &mut App) {
    app.add_systems(
        MadMaxPostSim,
        (
            count_game_mod_phase,
            super::script_dispatch::dispatch_game_post_sim,
        )
            .in_set(MadMaxPostSimSet::ModSystems),
    );
}

/// Register first-party Rust systems that prepare or extend ECS-backed UI.
/// The ImGui callback remains the only renderer and input backend.
pub(crate) fn register_ui_systems(app: &mut App) {
    app.add_systems(
        MadMaxRender,
        count_ui_mod_phase.in_set(MadMaxRenderSet::ModSystems),
    );
}

fn count_game_mod_phase(mut diagnostics: ResMut<RuntimeDiagnostics>) {
    diagnostics.game_mod_system_runs = diagnostics.game_mod_system_runs.saturating_add(1);
}

fn count_ui_mod_phase(mut diagnostics: ResMut<RuntimeDiagnostics>) {
    diagnostics.ui_mod_system_runs = diagnostics.ui_mod_system_runs.saturating_add(1);
}
