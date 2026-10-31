//! Transfer queued UI intents to the game callback through bounded channels.
//!
//! # Retry behavior
//!
//! Intents are ordered by their assigned request ID before submission. A
//! successful send removes the UI entity; a full channel leaves pending
//! entities for the next render. A disconnected endpoint stops the transfer
//! without blocking the render thread.
use super::*;

/// Transfer UI-world intent entities into the bounded game-world transport.
/// Full queues leave the entities in the UI world to retry on a later render.
pub(crate) fn flush_ui_intents(app: &mut App, endpoint: &UiEndpoint) {
    let world = app.world_mut();
    flush_velocity_intents(world, endpoint);
    flush_spawn_intents(world, endpoint);
}

fn flush_velocity_intents(world: &mut bevy_ecs::world::World, endpoint: &UiEndpoint) {
    let mut pending: Vec<_> = {
        let mut query = world.query_filtered::<
            (Entity, &IntentId, &SetPlayerVehicleVelocity),
            Without<IntentOutcomeRecord>,
        >();
        query
            .iter(world)
            .map(|(entity, id, velocity)| (entity, id.0, velocity.0))
            .collect()
    };
    pending.sort_by_key(|(_, intent_id, _)| *intent_id);

    let mut sent = 0;
    for (entity, intent_id, velocity) in pending {
        if sent >= MAX_INTENTS_PER_RENDER {
            break;
        }
        match endpoint.intent_tx.try_send(IntentEnvelope {
            intent_id,
            velocity,
        }) {
            Ok(()) => {
                world.entity_mut(entity).despawn();
                sent += 1;
            }
            Err(std::sync::mpsc::TrySendError::Full(_)) => {
                let mut diagnostics = world.resource_mut::<RuntimeDiagnostics>();
                diagnostics.intent_transport_full_count =
                    diagnostics.intent_transport_full_count.saturating_add(1);
                break;
            }
            Err(std::sync::mpsc::TrySendError::Disconnected(_)) => break,
        }
    }
}

fn flush_spawn_intents(world: &mut bevy_ecs::world::World, endpoint: &UiEndpoint) {
    let mut pending_spawns: Vec<_> = {
        let mut query = world.query_filtered::<
            (Entity, &SpawnRequestId, &SpawnAssetKey, &SpawnTransform),
            Without<SpawnOutcomeRecord>,
        >();
        query
            .iter(world)
            .map(|(entity, id, key, transform)| {
                (entity, id.0, key.as_str().to_owned(), transform.matrix)
            })
            .collect()
    };
    pending_spawns.sort_by_key(|(_, request_id, _, _)| *request_id);
    for (entity, request_id, asset_key, matrix) in
        pending_spawns.into_iter().take(MAX_SPAWNS_PER_RENDER)
    {
        match endpoint.spawn_tx.try_send(SpawnIntentEnvelope {
            request_id,
            asset_key,
            matrix,
        }) {
            Ok(()) => {
                world.entity_mut(entity).despawn();
            }
            Err(std::sync::mpsc::TrySendError::Full(_)) => break,
            Err(std::sync::mpsc::TrySendError::Disconnected(_)) => break,
        }
    }
}
