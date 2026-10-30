//! Bounded console requests and script-package lifecycle commands.
use super::*;

impl GameRuntime {
    pub(super) fn drain_console_requests(&mut self) -> bool {
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
        use crate::api::scripts::{loader::LoadScriptRequest, manager::ScriptManager};
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
                    crate::api::scripts::loader::DiscoverScriptsRequest {
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
                    event.phase = Some(crate::api::scripts::ScriptPhaseId::GamePostSim);
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
}
