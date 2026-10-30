//! Game-side Bevy runtime and callback orchestration.
mod commands;
mod effects;
use super::{
    GameEndpoint, GameSnapshot, INTENT_QUEUE_CAPACITY, IntentId, IntentOutcome,
    IntentOutcomeRecord, IntentResult, MAX_SPAWNS_PER_RENDER, MadMaxPostSim, RuntimeDiagnostics,
    SetPlayerVehicleVelocity, SpawnAssetKey, SpawnLifecycle, SpawnOutcome, SpawnRequestId,
    SpawnRequestResult, SpawnTransform, TargetCurrentPlayerVehicle, create_game_app,
    update_game_frame,
};
use crate::api::game::abi::types::Matrix4x4;
use crate::api::scripts::console::{
    self, ConsoleCommand, ConsoleEvent, ConsoleRequest, ConsoleSeverity,
};
use bevy_app::App;
use bevy_ecs::prelude::*;
use std::time::Instant;

/// Runs the owned game ECS runtime at accepted native callback boundaries.
///
/// The host adapter supplies each [`GameSnapshot`]. This type processes bounded
/// requests and returns at most one game intent for native dispatch.
pub struct GameRuntime {
    /// Bevy application containing game resources, schedules, and systems.
    pub app: App,
    pub(super) endpoint: GameEndpoint,
    identity_echo_queued: bool,
    last_callback_at: Option<Instant>,
    elapsed_seconds: f64,
    console_requests_disconnected: bool,
}

#[derive(Debug, Clone, Copy)]
/// Pending validated velocity effect selected for native dispatch.
pub struct PendingVelocityIntent {
    /// Projected entity associated with the effect.
    pub entity: Entity,
    /// Host-local effect identifier.
    pub intent_id: u64,
    /// Requested velocity in game units per second.
    pub velocity: [f32; 3],
}

#[derive(Debug, Clone)]
/// Pending spawn request selected for native dispatch.
pub struct PendingSpawnIntent {
    /// Host-local request identifier.
    pub request_id: u64,
    /// Validated exported-entity key.
    pub asset_key: String,
    /// Finite row-major placement transform.
    pub matrix: Matrix4x4,
}

#[derive(Debug, Clone)]
/// Effect ready for the native adapter to dispatch.
pub enum PendingGameIntent {
    /// Entity spawn request.
    Spawn(PendingSpawnIntent),
    /// Player vehicle velocity request.
    Velocity(PendingVelocityIntent),
}

impl GameRuntime {
    /// Create the host-driven game app from its paired channel endpoint.
    pub fn new(endpoint: GameEndpoint) -> Self {
        let mut app = create_game_app();
        app.insert_resource(crate::api::scripts::manager::ScriptManager::new(
            endpoint.script_events.clone(),
        ));
        crate::api::ecs::modding::register_game_systems(&mut app);
        Self {
            app,
            endpoint,
            identity_echo_queued: false,
            last_callback_at: None,
            elapsed_seconds: 0.0,
            console_requests_disconnected: false,
        }
    }

    /// Route console load/reload commands here at a game callback boundary.
    pub fn request_script_load(
        &mut self,
        request: crate::api::scripts::loader::LoadScriptRequest,
    ) -> Result<(), String> {
        self.app
            .world_mut()
            .resource_mut::<crate::api::scripts::manager::ScriptManager>()
            .request_load(&self.endpoint.script_loader, request)
    }

    /// Import a callback snapshot, run scheduled work, and return one effect.
    ///
    /// `None` means no effect is ready for dispatch on this callback.
    pub fn prepare_and_run(&mut self, mut snapshot: GameSnapshot) -> Option<PendingGameIntent> {
        self.capture_frame(&mut snapshot);
        self.import_pending_intents();
        self.seed_identity_velocity(&snapshot);
        self.process_script_lifecycle();
        self.forward_discovery_results();
        self.run_and_publish(snapshot)
    }

    fn capture_frame(&mut self, snapshot: &mut GameSnapshot) {
        let input = self.endpoint.input_mailbox.take();
        {
            let mut current = self
                .app
                .world_mut()
                .resource_mut::<crate::api::ecs::input::InputFrame>();
            current.clear_edges();
            if let Some(input) = input {
                *current = input;
            }
        }
        snapshot.input_frame = *self
            .app
            .world()
            .resource::<crate::api::ecs::input::InputFrame>();
        let now = Instant::now();
        let delta_seconds = self
            .last_callback_at
            .replace(now)
            .map_or(0.0, |previous| now.duration_since(previous).as_secs_f32());
        self.elapsed_seconds += f64::from(delta_seconds);
        snapshot.callback_delta_seconds = delta_seconds;
        snapshot.callback_elapsed_seconds = self.elapsed_seconds;
        update_game_frame(&mut self.app, *snapshot);
    }

    fn import_pending_intents(&mut self) {
        let mut imported = 0;
        while imported < INTENT_QUEUE_CAPACITY {
            match self.endpoint.intent_rx.try_recv() {
                Ok(envelope) => {
                    self.import_velocity_intent(envelope.intent_id, envelope.velocity);
                    imported += 1;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
            }
        }
        for _ in 0..MAX_SPAWNS_PER_RENDER {
            match self.endpoint.spawn_rx.try_recv() {
                Ok(envelope) => {
                    if let (Ok(asset_key), Ok(transform)) = (
                        SpawnAssetKey::parse(envelope.asset_key),
                        SpawnTransform::new(envelope.matrix),
                    ) {
                        self.app.world_mut().spawn((
                            SpawnRequestId(envelope.request_id),
                            asset_key,
                            transform,
                            SpawnLifecycle::Dispatching,
                        ));
                    } else {
                        self.app
                            .world_mut()
                            .resource_mut::<RuntimeDiagnostics>()
                            .last_spawn_outcome = Some(SpawnOutcome {
                            request_id: envelope.request_id,
                            result: SpawnRequestResult::InvalidRequest,
                            native_return: None,
                        });
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty)
                | Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
            }
        }
    }

    fn seed_identity_velocity(&mut self, snapshot: &GameSnapshot) {
        if !self.identity_echo_queued
            && let Some(velocity) = snapshot.vehicle_velocity
        {
            self.spawn_velocity_intent(velocity);
            self.identity_echo_queued = true;
        }
    }

    fn process_script_lifecycle(&mut self) {
        if self.drain_console_requests() {
            let mut scripts = self
                .app
                .world_mut()
                .resource_mut::<crate::api::scripts::manager::ScriptManager>();
            for id in self.endpoint.script_loader.take_auto_reload_overflow() {
                scripts.report_auto_reload_overflow(id);
            }
            for _ in 0..crate::api::scripts::loader::LOADER_QUEUE_CAPACITY {
                let Some(request) = self.endpoint.script_loader.try_auto_reload() else {
                    break;
                };
                let _ = scripts.request_auto_reload(&self.endpoint.script_loader, request);
            }
            for _ in 0..crate::api::scripts::loader::LOADER_QUEUE_CAPACITY {
                let Some(result) = self.endpoint.script_loader.try_result() else {
                    break;
                };
                scripts.receive_load_result(result);
            }
            scripts.report_loader_overflow(self.endpoint.script_loader.take_overflow());
            scripts.apply_pending_changes();
            scripts.process_deferred_auto_reloads(&self.endpoint.script_loader);
        }
    }

    fn forward_discovery_results(&mut self) {
        for _ in 0..crate::api::scripts::loader::LOADER_QUEUE_CAPACITY {
            let Some(result) = self.endpoint.script_loader.try_discovery_result() else {
                break;
            };
            self.publish_discovery_result(result);
        }
        let dropped_discoveries = self.endpoint.script_loader.take_discovery_overflow();
        if dropped_discoveries != 0 {
            self.endpoint.console_events.emit(ConsoleEvent::new(
                None,
                ConsoleSeverity::Error,
                &format!("script discovery result queue overflow: {dropped_discoveries} result(s) dropped; rescan from Scripts"),
            ));
        }
    }

    fn publish_discovery_result(
        &mut self,
        result: crate::api::scripts::loader::ScriptDiscoveryResult,
    ) {
        if let Some(request_id) = result.request_id {
            self.log_discovery_result(request_id, &result);
        }
        if let Err(error) = self.endpoint.discovery_tx.try_send(result) {
            let (request_id, text) = match error {
                std::sync::mpsc::TrySendError::Full(result) => (
                    result.request_id,
                    "discovery result queue full; rescan from Scripts",
                ),
                std::sync::mpsc::TrySendError::Disconnected(result) => {
                    (result.request_id, "discovery UI channel disconnected")
                }
            };
            self.endpoint.console_events.emit(ConsoleEvent::new(
                request_id,
                ConsoleSeverity::Error,
                text,
            ));
        }
    }

    fn log_discovery_result(
        &self,
        request_id: u64,
        result: &crate::api::scripts::loader::ScriptDiscoveryResult,
    ) {
        self.endpoint.console_events.emit(ConsoleEvent::new(
            Some(request_id),
            ConsoleSeverity::Info,
            "Package scan complete.",
        ));
        for package in &result.packages {
            let metadata = package
                .manifest
                .as_ref()
                .map_or_else(String::new, |manifest| {
                    format!(
                        " | API v{} | callbacks [{}] | capabilities [{}]",
                        manifest.api_version,
                        manifest.callbacks.join(", "),
                        manifest.capabilities.join(", ")
                    )
                });
            let issue = package
                .issue
                .as_ref()
                .map_or_else(String::new, |issue| format!(" | invalid: {issue}"));
            self.endpoint.console_events.emit(ConsoleEvent::new(
                Some(request_id),
                if package.issue.is_some() {
                    ConsoleSeverity::Warning
                } else {
                    ConsoleSeverity::Info
                },
                &format!("{}{}{}", package.package_name, metadata, issue),
            ));
        }
        self.log_loaded_scripts(request_id);
        if let Some(error) = &result.error {
            self.endpoint.console_events.emit(ConsoleEvent::new(
                Some(request_id),
                ConsoleSeverity::Warning,
                error,
            ));
        }
    }

    fn log_loaded_scripts(&self, request_id: u64) {
        for listing in self
            .app
            .world()
            .resource::<crate::api::scripts::manager::ScriptManager>()
            .list()
        {
            self.endpoint.console_events.emit(ConsoleEvent::new(
                Some(request_id),
                ConsoleSeverity::Info,
                &format!(
                    "runtime {}: {:?} | generation {}",
                    listing.id.as_str(),
                    listing.status,
                    listing.generation.0
                ),
            ));
        }
    }

    fn run_and_publish(&mut self, mut snapshot: GameSnapshot) -> Option<PendingGameIntent> {
        self.app.world_mut().run_schedule(MadMaxPostSim);
        {
            let mut diagnostics = self.app.world_mut().resource_mut::<RuntimeDiagnostics>();
            diagnostics.post_sim_schedule_runs =
                diagnostics.post_sim_schedule_runs.saturating_add(1);
            snapshot.post_sim_schedule_runs = diagnostics.post_sim_schedule_runs;
            snapshot.game_mod_system_runs = diagnostics.game_mod_system_runs;
            snapshot.dropped_snapshots = diagnostics.snapshots_dropped;
            snapshot.dropped_intents = diagnostics.intents_dropped;
            snapshot.last_intent_outcome = diagnostics.last_intent_outcome;
            snapshot.last_spawn_outcome = diagnostics.last_spawn_outcome;
        }
        match self.endpoint.snapshot_tx.try_send(snapshot) {
            Ok(()) => {}
            Err(std::sync::mpsc::TrySendError::Full(_)) => {
                let mut diagnostics = self.app.world_mut().resource_mut::<RuntimeDiagnostics>();
                diagnostics.snapshots_dropped = diagnostics.snapshots_dropped.saturating_add(1);
            }
            Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {}
        }
        self.take_next_spawn_intent()
            .map(PendingGameIntent::Spawn)
            .or_else(|| {
                self.take_next_velocity_intent()
                    .map(PendingGameIntent::Velocity)
            })
    }
}
