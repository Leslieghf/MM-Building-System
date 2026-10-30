//! Bounded console requests and script-package lifecycle commands.
use super::*;
use crate::api::scripts::{ScriptId, manager::ScriptListing};

impl GameRuntime {
    pub(super) fn drain_console_requests(&mut self) -> bool {
        for _ in 0..console::MAX_COMMANDS_PER_GAME_CALLBACK {
            match self.endpoint.console_requests.try_recv() {
                Ok(request) => self.process_console_request(request),
                Err(std::sync::mpsc::TryRecvError::Empty) => return true,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.report_console_disconnected();
                    return true;
                }
            }
        }
        false
    }

    fn report_console_disconnected(&mut self) {
        if self.console_requests_disconnected {
            return;
        }
        self.console_requests_disconnected = true;
        self.endpoint.console_events.emit(ConsoleEvent::new(
            None,
            ConsoleSeverity::Warning,
            "UI console request channel disconnected",
        ));
    }

    fn process_console_request(&mut self, request: ConsoleRequest) {
        let request_id = request.request_id;
        match request.command {
            ConsoleCommand::Help(topic) => {
                console::emit_help(&self.endpoint.console_events, request_id, topic.as_deref())
            }
            ConsoleCommand::ShowKeys
            | ConsoleCommand::ShowBinds
            | ConsoleCommand::Bind { .. }
            | ConsoleCommand::Unbind { .. } => self.reject_input_binding_command(request_id),
            ConsoleCommand::ScriptLoad(id) => self.request_script(id, request_id, false),
            ConsoleCommand::ScriptReload(id) => self.request_script(id, request_id, true),
            ConsoleCommand::ScriptAutoReload { id, enabled } => {
                self.request_auto_reload(id, enabled, request_id)
            }
            command @ (ConsoleCommand::ScriptRun(_) | ConsoleCommand::ScriptUnload(_)) => {
                self.run_or_unload(command, request_id)
            }
            ConsoleCommand::ScriptList => self.request_discovery(request_id),
            command @ ConsoleCommand::ScriptStatus(_) => {
                self.report_script_status(command, request_id)
            }
        }
    }

    fn reject_input_binding_command(&self, request_id: u64) {
        self.endpoint.console_events.emit(ConsoleEvent::new(
            Some(request_id),
            ConsoleSeverity::Error,
            "input binding commands are local to the mod-owned render input runtime",
        ));
    }

    fn request_script(&mut self, id: ScriptId, request_id: u64, replace: bool) {
        let request = crate::api::scripts::loader::LoadScriptRequest {
            request_id,
            id,
            replace,
            snapshot: None,
        };
        let _ = self.request_script_load(request);
    }

    fn request_auto_reload(&mut self, id: ScriptId, enabled: bool, request_id: u64) {
        let _ = self
            .app
            .world_mut()
            .resource_mut::<crate::api::scripts::manager::ScriptManager>()
            .set_auto_reload_request(&id, enabled, &self.endpoint.script_loader, Some(request_id));
    }

    fn run_or_unload(&mut self, command: ConsoleCommand, request_id: u64) {
        use crate::api::scripts::manager::ScriptManager;
        let id = command.script().expect("script command").clone();
        let mut scripts = self.app.world_mut().resource_mut::<ScriptManager>();
        match command {
            ConsoleCommand::ScriptRun(_) => {
                let _ = scripts.stage_run_request(id, Some(request_id));
            }
            _ => {
                let _ = scripts.request_unload(&self.endpoint.script_loader, id, Some(request_id));
            }
        }
        scripts.apply_pending_changes();
    }

    fn request_discovery(&self, request_id: u64) {
        let result = self.endpoint.script_loader.try_discover(
            crate::api::scripts::loader::DiscoverScriptsRequest {
                request_id: Some(request_id),
            },
        );
        let (severity, text) = match result {
            Ok(()) => (ConsoleSeverity::Info, "package discovery queued"),
            Err(error) => (ConsoleSeverity::Error, error),
        };
        self.endpoint
            .console_events
            .emit(ConsoleEvent::new(Some(request_id), severity, text));
    }

    fn report_script_status(&self, command: ConsoleCommand, request_id: u64) {
        use crate::api::scripts::manager::ScriptManager;
        let scripts = self.app.world().resource::<ScriptManager>();
        let target = command.script();
        let loaded = scripts
            .list()
            .into_iter()
            .filter(|row| target.is_none_or(|id| id == &row.id));
        let mut found = false;
        for listing in loaded {
            found = true;
            self.emit_loaded_script_status(request_id, listing);
        }
        for (id, _) in scripts
            .pending_loads()
            .into_iter()
            .filter(|(id, _)| target.is_none_or(|target| target == id))
        {
            found = true;
            self.emit_pending_script_status(request_id, id);
        }
        if !found {
            self.emit_missing_script_status(request_id, target);
        }
    }

    fn emit_loaded_script_status(&self, request_id: u64, listing: ScriptListing) {
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
        event.script = Some(listing.id);
        event.generation = Some(listing.generation);
        event.status = Some(listing.status);
        event.auto_reload = Some(listing.auto_reload);
        event.reload_error = listing.reload_error;
        event.phase = Some(crate::api::scripts::ScriptPhaseId::GamePostSim);
        self.endpoint.console_events.emit(event);
    }

    fn emit_pending_script_status(&self, request_id: u64, id: ScriptId) {
        let mut event = ConsoleEvent::new(
            Some(request_id),
            ConsoleSeverity::Info,
            &format!("{}: preparation pending", id.as_str()),
        );
        event.script = Some(id);
        self.endpoint.console_events.emit(event);
    }

    fn emit_missing_script_status(&self, request_id: u64, target: Option<&ScriptId>) {
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
