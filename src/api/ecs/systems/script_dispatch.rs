//! Dispatch Rhai callbacks from owned frames and collect typed effects.
//!
//! This system passes the current copied frame and input snapshot to the script
//! manager. Callback outputs become bounded ECS components for later export;
//! native effects are not invoked during script execution. Completion events
//! retain package and generation identity for UI attribution.
use crate::api::ecs::CurrentGameFrame;
use crate::api::scripts::{
    ScriptEvent, ScriptRaycastRequest,
    manager::{
        MAX_PENDING_VELOCITY_EFFECTS, MAX_RAYCAST_REQUESTS_PER_PHASE, ScriptDebugDrawEffect,
        ScriptManager,
    },
};
use bevy_ecs::prelude::{Commands, Component, Entity, Query, Res, ResMut, Resource};

#[derive(Resource, Default)]
pub(crate) struct PendingScriptDebugDraws(pub Vec<ScriptDebugDrawEffect>);

#[derive(Resource, Default)]
pub(crate) struct PendingScriptRaycasts(pub Vec<ScriptRaycastRequest>);

#[derive(Resource, Default)]
pub(crate) struct PendingScriptPanelPublications(
    pub Vec<crate::api::scripts::ScriptPanelPublication>,
);

/// Native completion attribution stays in the game world, outside Rhai.
#[derive(Component, Clone)]
pub struct ScriptVelocityOrigin(pub ScriptEvent);

/// Keeps a script generation attached to its pending native spawn request.
#[derive(Component, Clone)]
pub struct ScriptSpawnOrigin(pub ScriptEvent, pub String);

pub(crate) fn dispatch_game_post_sim(
    frame: Res<CurrentGameFrame>,
    mut scripts: ResMut<ScriptManager>,
    pending: Query<(Entity, &ScriptVelocityOrigin)>,
    pending_spawns: Query<(Entity, &ScriptSpawnOrigin)>,
    mut commands: Commands,
    mut debug_draws: ResMut<PendingScriptDebugDraws>,
    mut raycasts: ResMut<PendingScriptRaycasts>,
    mut script_panels: ResMut<PendingScriptPanelPublications>,
) {
    scripts.apply_pending_changes();
    let outstanding = pending
        .iter()
        .filter(|(_, origin)| scripts.effect_is_current(&origin.0))
        .count();
    let (effects, frame_debug_draws, frame_raycasts, frame_spawns, frame_panels) = scripts
        .dispatch(
            frame.0,
            MAX_PENDING_VELOCITY_EFFECTS.saturating_sub(outstanding),
        );
    debug_draws.0.extend(frame_debug_draws);
    let raycast_capacity = MAX_RAYCAST_REQUESTS_PER_PHASE.saturating_sub(raycasts.0.len());
    raycasts
        .0
        .extend(frame_raycasts.into_iter().take(raycast_capacity));
    script_panels.0 = frame_panels;
    // Dispatch may disable a generation after an earlier successful invocation.
    // Cancel both old and new effects before the native adapter can consume them.
    for (entity, origin) in &pending {
        if !scripts.effect_is_current(&origin.0) {
            scripts.report_effect_cancelled(origin.0.clone());
            commands.entity(entity).despawn();
        }
    }
    for (entity, origin) in &pending_spawns {
        if !scripts.effect_is_current(&origin.0) {
            scripts.report_effect_cancelled(origin.0.clone());
            commands.entity(entity).despawn();
        }
    }
    for effect in effects {
        commands.spawn((
            crate::api::ecs::player_vehicle_velocity_intent(effect.velocity),
            ScriptVelocityOrigin(effect.origin),
        ));
    }
    for effect in frame_spawns {
        let asset = crate::api::ecs::SpawnAssetKey::parse(effect.asset_key);
        let transform = crate::api::ecs::SpawnTransform::new(effect.matrix);
        match (asset, transform) {
            (Ok(asset), Ok(transform)) => {
                commands.spawn((
                    crate::api::ecs::SpawnRequestId(effect.request_id),
                    asset,
                    transform,
                    crate::api::ecs::SpawnLifecycle::Queued,
                    ScriptSpawnOrigin(effect.origin, effect.catalog_id),
                ));
            }
            _ => scripts.report_spawn_result(
                effect.origin,
                Some(&effect.catalog_id),
                effect.request_id,
                None,
                crate::api::ecs::SpawnRequestResult::InvalidRequest,
                module_path!(),
            ),
        }
    }
}
