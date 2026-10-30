//! Game callback runtime, schedule execution, and intent application.
use super::*;

pub struct GameRuntime {
    pub app: App,
    endpoint: GameEndpoint,
    identity_echo_queued: bool,
    last_callback_at: Option<Instant>,
    elapsed_seconds: f64,
    console_requests_disconnected: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct PendingVelocityIntent {
    pub entity: Entity,
    pub intent_id: u64,
    pub velocity: [f32; 3],
}

#[derive(Debug, Clone)]
pub struct PendingSpawnIntent {
    pub request_id: u64,
    pub asset_key: String,
    pub matrix: Matrix4x4,
}

#[derive(Debug, Clone)]
pub enum PendingGameIntent {
    Spawn(PendingSpawnIntent),
    Velocity(PendingVelocityIntent),
}

impl GameRuntime {
    pub fn new(endpoint: GameEndpoint) -> Self {
        let mut app = create_game_app();
        app.insert_resource(crate::scripting::manager::ScriptManager::new(
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
        request: crate::scripting::loader::LoadScriptRequest,
    ) -> Result<(), String> {
        self.app
            .world_mut()
            .resource_mut::<crate::scripting::manager::ScriptManager>()
            .request_load(&self.endpoint.script_loader, request)
    }

    fn drain_console_requests(&mut self) -> bool {
        for _ in 0..console::MAX_COMMANDS_PER_GAME_CALLBACK {
            match self.endpoint.console_requests.try_recv() {
                Ok(request) => self.process_console_request(request),
                Err(std::sync::mpsc::TryRecvError::Empty) => return true,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    if !self.console_requests_disconnected {
                        self.console_requests_disconnected = true;
                        self.endpoint.console_events.emit(ConsoleEvent::new(
                            None,
                            ConsoleSeverity::Warning,
                            "UI console request channel disconnected",
                        ));
                    }
                    return true;
                }
            }
        }
        // Preserve the command budget; defer async work until the UI queue drains.
        false
    }

    fn process_console_request(&mut self, request: ConsoleRequest) {
        use crate::scripting::{loader::LoadScriptRequest, manager::ScriptManager};
        let request_id = request.request_id;
        match request.command {
            ConsoleCommand::Help(topic) => {
                console::emit_help(&self.endpoint.console_events, request_id, topic.as_deref())
            }
            ConsoleCommand::ShowKeys
            | ConsoleCommand::ShowBinds
            | ConsoleCommand::Bind { .. }
            | ConsoleCommand::Unbind { .. } => {
                self.endpoint.console_events.emit(ConsoleEvent::new(
                    Some(request_id),
                    ConsoleSeverity::Error,
                    "input binding commands are local to the mod-owned render input runtime",
                ));
            }
            ConsoleCommand::ScriptLoad(id) => {
                let _ = self.request_script_load(LoadScriptRequest {
                    request_id,
                    id,
                    replace: false,
                    snapshot: None,
                });
            }
            ConsoleCommand::ScriptReload(id) => {
                let _ = self.request_script_load(LoadScriptRequest {
                    request_id,
                    id,
                    replace: true,
                    snapshot: None,
                });
            }
            ConsoleCommand::ScriptAutoReload { id, enabled } => {
                let _ = self
                    .app
                    .world_mut()
                    .resource_mut::<ScriptManager>()
                    .set_auto_reload_request(
                        &id,
                        enabled,
                        &self.endpoint.script_loader,
                        Some(request_id),
                    );
            }
            command @ (ConsoleCommand::ScriptRun(_) | ConsoleCommand::ScriptUnload(_)) => {
                let id = command.script().expect("script command").clone();
                let mut scripts = self.app.world_mut().resource_mut::<ScriptManager>();
                let result = match command {
                    ConsoleCommand::ScriptRun(_) => {
                        scripts.stage_run_request(id.clone(), Some(request_id))
                    }
                    _ => scripts.request_unload(
                        &self.endpoint.script_loader,
                        id.clone(),
                        Some(request_id),
                    ),
                };
                // Manager lifecycle errors already carry one correlated console event.
                let _ = result;
                // Complete each lifecycle command before inspecting the next one.
                scripts.apply_pending_changes();
            }
            ConsoleCommand::ScriptList => {
                let result = self.endpoint.script_loader.try_discover(
                    crate::scripting::loader::DiscoverScriptsRequest {
                        request_id: Some(request_id),
                    },
                );
                let (severity, text) = match result {
                    Ok(()) => (ConsoleSeverity::Info, "package discovery queued"),
                    Err(error) => (ConsoleSeverity::Error, error),
                };
                self.endpoint.console_events.emit(ConsoleEvent::new(
                    Some(request_id),
                    severity,
                    text,
                ));
            }
            command @ ConsoleCommand::ScriptStatus(_) => {
                let scripts = self.app.world().resource::<ScriptManager>();
                let target = command.script();
                let mut found = false;
                for listing in scripts
                    .list()
                    .iter()
                    .filter(|row| target.is_none_or(|id| id == &row.id))
                {
                    found = true;
                    let callbacks = listing
                        .callbacks
                        .iter()
                        .map(|(phase, callback)| format!("{}={callback}", phase.as_str()))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let mut event = ConsoleEvent::new(
                        Some(request_id),
                        ConsoleSeverity::Info,
                        &format!(
                            "{}: {:?}; callbacks: [{}]; on_run: {}",
                            listing.id.as_str(),
                            listing.status,
                            callbacks,
                            listing.has_on_run
                        ),
                    );
                    event.script = Some(listing.id.clone());
                    event.generation = Some(listing.generation);
                    event.status = Some(listing.status);
                    event.auto_reload = Some(listing.auto_reload);
                    event.reload_error = listing.reload_error.clone();
                    event.phase = Some(crate::scripting::ScriptPhaseId::GamePostSim);
                    self.endpoint.console_events.emit(event);
                }
                for (id, _load_request) in scripts
                    .pending_loads()
                    .into_iter()
                    .filter(|(id, _)| target.is_none_or(|target| target == id))
                {
                    found = true;
                    let mut event = ConsoleEvent::new(
                        Some(request_id),
                        ConsoleSeverity::Info,
                        &format!("{}: preparation pending", id.as_str()),
                    );
                    event.script = Some(id);
                    self.endpoint.console_events.emit(event);
                }
                if !found {
                    let mut event = ConsoleEvent::new(
                        Some(request_id),
                        if target.is_some() {
                            ConsoleSeverity::Error
                        } else {
                            ConsoleSeverity::Info
                        },
                        if target.is_some() {
                            "unknown script ID: script is not loaded or preparing"
                        } else {
                            "no scripts loaded or preparing"
                        },
                    );
                    event.script = target.cloned();
                    self.endpoint.console_events.emit(event);
                }
            }
        }
    }

    pub(crate) fn take_script_debug_draws(
        &mut self,
    ) -> Vec<crate::scripting::manager::ScriptDebugDrawEffect> {
        std::mem::take(
            &mut self
                .app
                .world_mut()
                .resource_mut::<crate::scripting::dispatch::PendingScriptDebugDraws>()
                .0,
        )
    }

    pub fn prepare_and_run(&mut self, mut snapshot: GameSnapshot) -> Option<PendingGameIntent> {
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
        update_game_frame(&mut self.app, snapshot);
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

        if !self.identity_echo_queued {
            if let Some(velocity) = snapshot.vehicle_velocity {
                self.spawn_velocity_intent(velocity);
                self.identity_echo_queued = true;
            }
        }

        if self.drain_console_requests() {
            let mut scripts = self
                .app
                .world_mut()
                .resource_mut::<crate::scripting::manager::ScriptManager>();
            for id in self.endpoint.script_loader.take_auto_reload_overflow() {
                scripts.report_auto_reload_overflow(id);
            }
            for _ in 0..crate::scripting::loader::LOADER_QUEUE_CAPACITY {
                let Some(request) = self.endpoint.script_loader.try_auto_reload() else {
                    break;
                };
                let _ = scripts.request_auto_reload(&self.endpoint.script_loader, request);
            }
            for _ in 0..crate::scripting::loader::LOADER_QUEUE_CAPACITY {
                let Some(result) = self.endpoint.script_loader.try_result() else {
                    break;
                };
                scripts.receive_load_result(result);
            }
            scripts.report_loader_overflow(self.endpoint.script_loader.take_overflow());
            scripts.apply_pending_changes();
            scripts.process_deferred_auto_reloads(&self.endpoint.script_loader);
        }
        for _ in 0..crate::scripting::loader::LOADER_QUEUE_CAPACITY {
            let Some(result) = self.endpoint.script_loader.try_discovery_result() else {
                break;
            };
            if let Some(request_id) = result.request_id {
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
                for listing in self
                    .app
                    .world()
                    .resource::<crate::scripting::manager::ScriptManager>()
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
                if let Some(error) = &result.error {
                    self.endpoint.console_events.emit(ConsoleEvent::new(
                        Some(request_id),
                        ConsoleSeverity::Warning,
                        error,
                    ));
                }
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
        let dropped_discoveries = self.endpoint.script_loader.take_discovery_overflow();
        if dropped_discoveries != 0 {
            self.endpoint.console_events.emit(ConsoleEvent::new(
                None,
                ConsoleSeverity::Error,
                &format!("script discovery result queue overflow: {dropped_discoveries} result(s) dropped; rescan from Scripts"),
            ));
        }
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

    pub fn complete_spawn_intent(&mut self, pending: PendingSpawnIntent, outcome: SpawnOutcome) {
        let world = self.app.world_mut();
        let mut query = world.query_filtered::<(Entity, &SpawnRequestId), With<SpawnTransform>>();
        if let Some((entity, _)) = query.iter(world).find(|(_, id)| id.0 == pending.request_id) {
            world.entity_mut(entity).despawn();
        }
        world
            .resource_mut::<RuntimeDiagnostics>()
            .last_spawn_outcome = Some(outcome);
    }

    fn take_next_spawn_intent(&mut self) -> Option<PendingSpawnIntent> {
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

    pub fn complete_intent(&mut self, pending: PendingVelocityIntent, result: IntentResult) {
        if let Some(origin) = self
            .app
            .world()
            .get::<crate::scripting::dispatch::ScriptVelocityOrigin>(pending.entity)
            .cloned()
        {
            self.app
                .world()
                .resource::<crate::scripting::manager::ScriptManager>()
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

    fn spawn_velocity_intent(&mut self, velocity: [f32; 3]) {
        let intent = crate::api::ecs::player_vehicle_velocity_intent(velocity);
        self.app.world_mut().spawn(intent);
    }

    fn import_velocity_intent(&mut self, id: u64, velocity: [f32; 3]) {
        self.app.world_mut().spawn((
            IntentId(id),
            SetPlayerVehicleVelocity(velocity),
            TargetCurrentPlayerVehicle,
        ));
    }

    fn take_next_velocity_intent(&mut self) -> Option<PendingVelocityIntent> {
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
