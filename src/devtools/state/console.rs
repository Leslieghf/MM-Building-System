//! Console UI state: prompt, bounded history, package selection, and status.
//!
//! This state is presentation-only. Requests are submitted through the bounded
//! console transport, and status rows are copied from owned lifecycle events;
//! this state does not own or mutate the script manager.
use crate::api::scripts::console::{ConsoleOutput, ConsoleSeverity};
use bevy_ecs::prelude::Resource;
use std::collections::VecDeque;

pub const MAX_CONSOLE_HISTORY_ENTRIES: usize = 64;
pub const MAX_CONSOLE_STATUS_ROWS: usize = crate::api::scripts::manager::MAX_ACTIVE_SCRIPTS;

#[derive(Debug, Clone)]
pub struct ScriptConsoleStatusRow {
    pub id: crate::api::scripts::ScriptId,
    pub generation: crate::api::scripts::ScriptGeneration,
    pub status: crate::api::scripts::ScriptStatus,
    pub auto_reload: bool,
    pub reload_error: Option<String>,
}

/// Retained only in the UI world. Prompt, history, and selection survive toggles.
#[derive(Resource)]
pub struct ScriptConsoleUiState {
    pub input: String,
    pub input_over_limit: bool,
    pub history: VecDeque<String>,
    pub history_cursor: Option<usize>,
    pub history_draft: String,
    pub follow_output: bool,
    pub transcript_text: String,
    pub scroll_y: f32,
    pub restore_scroll: bool,
    pub focus_prompt: bool,
    pub loaded_scripts: Vec<ScriptConsoleStatusRow>,
    pub selected_script: Option<ScriptConsoleStatusRow>,
    pub active_tab: DevToolsTab,
    pub discovered_scripts: Vec<crate::api::scripts::package::DiscoveredScript>,
    pub selected_package: Option<String>,
    pub discovery_status: String,
    pub discovery_pending: bool,
    pub initial_scan_started: bool,
    pub pending_discovery_request: Option<u64>,
    observed_output_revision: u64,
    observed_transcript_revision: u64,
}

impl Default for ScriptConsoleUiState {
    fn default() -> Self {
        Self {
            // Leave room for an overflow sentinel and ImGui's terminating NUL.
            input: String::with_capacity(crate::api::scripts::console::MAX_COMMAND_BYTES + 2),
            input_over_limit: false,
            history: VecDeque::new(),
            history_cursor: None,
            history_draft: String::new(),
            follow_output: true,
            transcript_text: String::new(),
            scroll_y: 0.0,
            restore_scroll: true,
            focus_prompt: true,
            loaded_scripts: Vec::new(),
            selected_script: None,
            active_tab: DevToolsTab::Console,
            discovered_scripts: Vec::new(),
            selected_package: None,
            discovery_status: "Open Scripts to scan packages.".into(),
            discovery_pending: false,
            initial_scan_started: false,
            pending_discovery_request: None,
            observed_output_revision: 0,
            observed_transcript_revision: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DevToolsTab {
    #[default]
    Console,
    Scripts,
}

impl ScriptConsoleUiState {
    /// Rebuild the plain-text transcript only when retained output changes.
    /// Keeping it as one read-only multiline input gives ImGui native range
    /// selection and copy behavior across command and script groups.
    pub fn refresh_transcript(&mut self, output: &ConsoleOutput) {
        if self.observed_transcript_revision == output.revision() {
            return;
        }
        self.observed_transcript_revision = output.revision();
        self.transcript_text = build_console_transcript(output.events());
    }

    pub fn remember_input(&mut self) {
        if !self.input.trim().is_empty() && self.history.back() != Some(&self.input) {
            self.history.push_back(crate::api::scripts::bounded_text(
                &self.input,
                crate::api::scripts::console::MAX_COMMAND_BYTES,
            ));
            while self.history.len() > MAX_CONSOLE_HISTORY_ENTRIES {
                self.history.pop_front();
            }
        }
        self.history_cursor = None;
        self.history_draft.clear();
    }

    /// Apply only newly received events so old scrollback cannot overwrite current UI state.
    pub fn observe_output(&mut self, output: &ConsoleOutput) {
        let new_events = output
            .revision()
            .saturating_sub(self.observed_output_revision);
        self.observed_output_revision = output.revision();
        let skip = output
            .events()
            .len()
            .saturating_sub(new_events.min(usize::MAX as u64) as usize);
        for event in output.events().iter().skip(skip) {
            self.observe_event(event);
        }
    }

    fn observe_event(&mut self, event: &crate::api::scripts::console::ConsoleEvent) {
        self.observe_discovery_event(event);
        let (Some(id), Some(generation), Some(status)) =
            (&event.script, event.generation, event.status)
        else {
            return;
        };
        let row = self.status_row(event, id, generation, status);
        self.update_selected_status(id, &row);
        self.loaded_scripts.retain(|loaded| loaded.id != *id);
        if status != crate::api::scripts::ScriptStatus::Unloaded {
            self.push_status_row(row);
        }
    }

    fn observe_discovery_event(&mut self, event: &crate::api::scripts::console::ConsoleEvent) {
        let overflow = event.request_id.is_none()
            && (event
                .text
                .starts_with("script discovery result queue overflow")
                || event.text.starts_with("console event queue overload:"));
        let failed_scan = event.request_id == self.pending_discovery_request
            && event.request_id.is_some()
            && event.severity == ConsoleSeverity::Error;
        if overflow {
            self.discovery_status = if event.text.starts_with("console event queue overload:") {
                "Console output overloaded; the scan may be incomplete. Rescan when ready.".into()
            } else {
                event.text.clone()
            };
        } else if failed_scan {
            self.discovery_status = event.text.clone();
        } else {
            return;
        }
        self.discovery_pending = false;
        self.pending_discovery_request = None;
    }

    fn status_row(
        &self,
        event: &crate::api::scripts::console::ConsoleEvent,
        id: &crate::api::scripts::ScriptId,
        generation: crate::api::scripts::ScriptGeneration,
        status: crate::api::scripts::ScriptStatus,
    ) -> ScriptConsoleStatusRow {
        let previous = self.loaded_scripts.iter().find(|row| row.id == *id);
        ScriptConsoleStatusRow {
            id: id.clone(),
            generation,
            status,
            auto_reload: event
                .auto_reload
                .unwrap_or_else(|| previous.is_some_and(|row| row.auto_reload)),
            reload_error: if event.auto_reload.is_some() {
                event.reload_error.clone()
            } else {
                previous.and_then(|row| row.reload_error.clone())
            },
        }
    }

    fn update_selected_status(
        &mut self,
        id: &crate::api::scripts::ScriptId,
        row: &ScriptConsoleStatusRow,
    ) {
        if self
            .selected_script
            .as_ref()
            .is_some_and(|selected| selected.id == *id)
        {
            self.selected_script = Some(row.clone());
        }
    }

    fn push_status_row(&mut self, row: ScriptConsoleStatusRow) {
        if self.loaded_scripts.len() == MAX_CONSOLE_STATUS_ROWS {
            self.loaded_scripts.remove(0);
        }
        if self.selected_script.is_none() {
            self.selected_script = Some(row.clone());
        }
        self.loaded_scripts.push(row);
    }
}

fn build_console_transcript(
    events: &VecDeque<crate::api::scripts::console::ConsoleEvent>,
) -> String {
    use crate::api::scripts::console::{ConsoleEvent, ConsoleSeverity};
    use std::collections::HashSet;

    fn severity_label(severity: ConsoleSeverity) -> &'static str {
        match severity {
            ConsoleSeverity::Info => "INFO",
            ConsoleSeverity::Warning => "WARN",
            ConsoleSeverity::Error => "ERROR",
        }
    }

    fn append_event(
        text: &mut String,
        event: &ConsoleEvent,
        indent: bool,
        in_command: bool,
        last_section: &mut Option<String>,
    ) {
        let lifecycle = matches!(
            event.text.as_str(),
            "Preparing"
                | "Prepared"
                | "Loaded"
                | "Reloaded"
                | "Unloaded"
                | "Queued"
                | "Cancelled"
                | "Superseded"
        );
        if in_command {
            *last_section = None;
            text.push_str("  ");
        } else if let Some(module) = event.source_module.as_ref() {
            if last_section.as_deref() != Some(module.as_str()) {
                if !text.is_empty() && !text.ends_with("\n\n") {
                    text.push('\n');
                }
                text.push_str(&format!("Rust: {module}\n"));
            }
            *last_section = Some(module.clone());
            text.push_str("  ");
        } else if let Some(script) = event.script.as_ref().filter(|_| !lifecycle) {
            let phase = event
                .phase
                .map(|phase| phase.as_str())
                .unwrap_or("callback");
            let section = format!("{} @ {phase}", script.as_str());
            if last_section.as_deref() != Some(section.as_str()) {
                if !text.is_empty() && !text.ends_with("\n\n") {
                    text.push('\n');
                }
                text.push_str(&format!("Rhai: {section}\n"));
            }
            *last_section = Some(section);
            text.push_str("  ");
        } else {
            *last_section = None;
            if indent {
                text.push_str("  ");
            }
        }
        text.push_str(severity_label(event.severity));
        text.push_str(": ");
        text.push_str(&crate::devtools::overlay::plain_console_text(&event.text));
        text.push('\n');
    }

    let mut text = String::new();
    let mut rendered_requests = HashSet::new();
    let mut last_section = None;
    let mut index = 0;
    while index < events.len() {
        let event = &events[index];
        if let Some(command) = event.text.strip_prefix("> ") {
            last_section = None;
            if !text.is_empty() && !text.ends_with("\n\n") {
                text.push('\n');
            }
            text.push_str("Cmd: ");
            text.push_str(command);
            text.push('\n');
            if let Some(request_id) = event.request_id {
                rendered_requests.insert(request_id);
                for response in events.iter().filter(|response| {
                    response.request_id == Some(request_id) && !response.text.starts_with("> ")
                }) {
                    append_event(&mut text, response, true, true, &mut last_section);
                }
            } else {
                let mut response_index = index + 1;
                while response_index < events.len() {
                    let response = &events[response_index];
                    if response.text.starts_with("> ") || response.request_id.is_some() {
                        break;
                    }
                    append_event(&mut text, response, true, true, &mut last_section);
                    response_index += 1;
                }
                index = response_index.saturating_sub(1);
            }
            text.push('\n');
        } else if !event
            .request_id
            .is_some_and(|request_id| rendered_requests.contains(&request_id))
        {
            append_event(&mut text, event, true, false, &mut last_section);
        }
        index += 1;
    }
    text
}
