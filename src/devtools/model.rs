//! Overlay preferences, console presentation state, and UI-facing DTOs.
use crate::api::ecs::*;
use crate::api::scripts::console::{ConsoleOutput, ConsoleSeverity};
use crate::api::scripts::*;
use bevy_ecs::prelude::*;
use std::collections::VecDeque;

#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct SpawnTestUiState {
    pub attempted: bool,
}

/// Create an ECS bundle for an in-process mod system to enqueue.
///
/// This is data only: it does not call the SDK or claim the entity key exists
/// in the installed game build.
pub const UI_OPACITY_MIN: f32 = 0.25;
pub const UI_OPACITY_MAX: f32 = 1.0;
pub const UI_OPACITY_DEFAULT: f32 = 0.96;

impl Default for UiPreferences {
    fn default() -> Self {
        Self {
            mode: OverlayMode::Hud,
            input_capture_override: None,
            escape_capture_release_pending: false,
            ui_opacity: UI_OPACITY_DEFAULT,
            show_player: true,
            show_camera: false,
            show_raycast: false,
            show_vehicle: true,
            show_runtime: false,
        }
    }
}

impl UiPreferences {
    pub fn captures_game_input(self) -> bool {
        self.input_capture_override
            .unwrap_or_else(|| self.mode.owns_game_input())
    }

    pub fn set_mode(&mut self, mode: OverlayMode) {
        self.mode = mode;
        self.input_capture_override = None;
        self.escape_capture_release_pending = false;
    }

    pub fn toggle_input_capture(&mut self) {
        self.escape_capture_release_pending = false;
        self.input_capture_override = Some(!self.captures_game_input());
    }

    pub fn set_ui_opacity(&mut self, opacity: f32) {
        self.ui_opacity = if opacity.is_finite() {
            opacity.clamp(UI_OPACITY_MIN, UI_OPACITY_MAX)
        } else {
            UI_OPACITY_DEFAULT
        };
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayMode {
    Hud,
    UiConfigurator,
    ScriptConsole,
}

impl OverlayMode {
    pub fn owns_game_input(self) -> bool {
        matches!(self, Self::UiConfigurator | Self::ScriptConsole)
    }
}

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
            // Keep one overflow sentinel byte plus ImGui's terminating NUL in this fixed buffer.
            input: String::with_capacity(console::MAX_COMMAND_BYTES + 2),
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
                console::MAX_COMMAND_BYTES,
            ));
            while self.history.len() > MAX_CONSOLE_HISTORY_ENTRIES {
                self.history.pop_front();
            }
        }
        self.history_cursor = None;
        self.history_draft.clear();
    }

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
            if event.request_id.is_none()
                && (event
                    .text
                    .starts_with("script discovery result queue overflow")
                    || event.text.starts_with("console event queue overload:"))
            {
                self.discovery_pending = false;
                self.discovery_status = if event.text.starts_with("console event queue overload:") {
                    "Console output overloaded; the scan may be incomplete. Rescan when ready."
                        .into()
                } else {
                    event.text.clone()
                };
                self.pending_discovery_request = None;
            }
            if event.request_id.is_some() && event.request_id == self.pending_discovery_request {
                if event.severity == ConsoleSeverity::Error {
                    self.discovery_pending = false;
                    self.discovery_status = event.text.clone();
                    self.pending_discovery_request = None;
                }
            }
            let (Some(id), Some(generation), Some(status)) =
                (&event.script, event.generation, event.status)
            else {
                continue;
            };
            let previous = self.loaded_scripts.iter().find(|row| row.id == *id);
            let row = ScriptConsoleStatusRow {
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
            };
            if self
                .selected_script
                .as_ref()
                .is_some_and(|selected| selected.id == *id)
            {
                self.selected_script = Some(row.clone());
            }
            self.loaded_scripts.retain(|loaded| loaded.id != *id);
            if status != crate::api::scripts::ScriptStatus::Unloaded {
                if self.loaded_scripts.len() == MAX_CONSOLE_STATUS_ROWS {
                    self.loaded_scripts.remove(0);
                }
                if self.selected_script.is_none() {
                    self.selected_script = Some(row.clone());
                }
                self.loaded_scripts.push(row);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugSection {
    Player,
    Camera,
    Raycast,
    Vehicle,
    Runtime,
}

#[derive(Debug, Clone)]
pub struct PresentationRow {
    pub section: DebugSection,
    pub label: &'static str,
    pub value: String,
}

#[derive(Resource, Debug, Default)]
pub struct UiPresentation(pub Vec<PresentationRow>);

#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct UiGameSnapshot(pub GameSnapshot);

#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct PendingUiInput(pub crate::api::ecs::input::InputFrame);

#[derive(Resource, Debug, Clone, Copy)]
pub struct UiPreferences {
    pub mode: OverlayMode,
    /// F1 override; panel transitions restore their default input ownership.
    pub input_capture_override: Option<bool>,
    /// Keep game input captured until an Escape key that closed the overlay is released.
    pub escape_capture_release_pending: bool,
    pub ui_opacity: f32,
    pub show_player: bool,
    pub show_camera: bool,
    pub show_raycast: bool,
    pub show_vehicle: bool,
    pub show_runtime: bool,
}
