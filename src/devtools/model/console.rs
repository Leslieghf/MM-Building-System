//! Retained console prompt, history, package selection, and script status.
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
    pub wrap_console_lines: bool,
    pub selected_console_text: Option<String>,
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
            wrap_console_lines: true,
            selected_console_text: None,
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
