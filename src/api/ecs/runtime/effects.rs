//! Drain callback-produced effects and record adapter completion outcomes.
//!
//! Requests stay as owned ECS data until the host adapter accepts one at a
//! valid game callback. The runtime records typed outcomes for later UI
//! presentation; this module does not call native game functions itself.
use super::*;

impl GameRuntime {
    pub(crate) fn take_script_debug_draws(
        &mut self,
    ) -> Vec<crate::api::scripts::manager::ScriptDebugDrawEffect> {
        std::mem::take(
            &mut self
                .app
                .world_mut()
                .resource_mut::<crate::api::ecs::systems::script_dispatch::PendingScriptDebugDraws>(
                )
                .0,
        )
    }

    pub(crate) fn take_script_raycast_requests(
        &mut self,
    ) -> Vec<crate::api::scripts::ScriptRaycastRequest> {
        std::mem::take(
            &mut self
                .app
                .world_mut()
                .resource_mut::<crate::api::ecs::systems::script_dispatch::PendingScriptRaycasts>()
                .0,
        )
    }

    pub(crate) fn complete_script_raycast(
        &mut self,
        result: crate::api::scripts::ScriptRaycastCompletion,
    ) -> bool {
        self.app
            .world_mut()
            .resource_mut::<crate::api::scripts::manager::ScriptManager>()
            .complete_script_raycast(result)
    }

    pub(crate) fn complete_spawn_intent(
        &mut self,
        pending: PendingSpawnIntent,
        outcome: SpawnOutcome,
    ) {
        let world = self.app.world_mut();
        let mut query = world.query_filtered::<(Entity, &SpawnRequestId), With<SpawnTransform>>();
        if let Some((entity, _)) = query.iter(world).find(|(_, id)| id.0 == pending.request_id) {
            world.entity_mut(entity).despawn();
        }
        world
            .resource_mut::<RuntimeDiagnostics>()
            .last_spawn_outcome = Some(outcome);
    }

    pub(super) fn take_next_spawn_intent(&mut self) -> Option<PendingSpawnIntent> {
        let world = self.app.world_mut();
        let candidate = {
            let mut query = world.query_filtered::<
                (&SpawnRequestId, &SpawnAssetKey, &SpawnTransform),
                With<SpawnLifecycle>,
            >();
            query
                .iter(world)
                .min_by_key(|(id, _, _)| id.0)
                .map(|(id, key, transform)| (id.0, key.as_str().to_owned(), transform.matrix))
        }?;
        Some(PendingSpawnIntent {
            request_id: candidate.0,
            asset_key: candidate.1,
            matrix: candidate.2,
        })
    }

    /// Record native dispatch completion, emit script attribution, and remove
    /// the pending ECS entity.
    ///
    /// The result is also published to the bounded UI outcome queue. If that
    /// queue is full, the runtime increments its dropped-intent diagnostic.
    pub fn complete_intent(&mut self, pending: PendingVelocityIntent, result: IntentResult) {
        if let Some(origin) = self
            .app
            .world()
            .get::<crate::api::ecs::systems::script_dispatch::ScriptVelocityOrigin>(pending.entity)
            .cloned()
        {
            self.app
                .world()
                .resource::<crate::api::scripts::manager::ScriptManager>()
                .report_velocity_result(origin.0, pending.intent_id, result);
        }
        self.app.world_mut().entity_mut(pending.entity).despawn();
        let outcome = IntentOutcome {
            intent_id: pending.intent_id,
            result,
        };
        self.app
            .world_mut()
            .resource_mut::<RuntimeDiagnostics>()
            .last_intent_outcome = Some(outcome);
        match self.endpoint.outcome_tx.try_send(outcome) {
            Ok(()) => {}
            Err(std::sync::mpsc::TrySendError::Full(_)) => {
                let mut diagnostics = self.app.world_mut().resource_mut::<RuntimeDiagnostics>();
                diagnostics.intents_dropped = diagnostics.intents_dropped.saturating_add(1);
            }
            Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {}
        }
    }

    pub(super) fn spawn_velocity_intent(&mut self, velocity: [f32; 3]) {
        let intent = crate::api::ecs::player_vehicle_velocity_intent(velocity);
        self.app.world_mut().spawn(intent);
    }

    pub(super) fn import_velocity_intent(&mut self, id: u64, velocity: [f32; 3]) {
        self.app.world_mut().spawn((
            IntentId(id),
            SetPlayerVehicleVelocity(velocity),
            TargetCurrentPlayerVehicle,
        ));
    }

    pub(super) fn take_next_velocity_intent(&mut self) -> Option<PendingVelocityIntent> {
        let world = self.app.world_mut();
        let candidate = {
            let mut query = world.query_filtered::<
                (Entity, &IntentId, &SetPlayerVehicleVelocity),
                Without<IntentOutcomeRecord>,
            >();
            query
                .iter(world)
                .min_by_key(|(_, id, _)| id.0)
                .map(|(entity, id, velocity)| (entity, id.0, velocity.0))
        }?;
        Some(PendingVelocityIntent {
            entity: candidate.0,
            intent_id: candidate.1,
            velocity: candidate.2,
        })
    }
}
