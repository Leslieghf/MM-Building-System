//! Overlay-local input capture, appearance preferences, and test-panel state.
//!
//! Resources here describe UI preferences and the state of the developer-only
//! spawn panel. They are not game entities or durable mod content; accepted
//! requests are emitted as typed intents for the game runtime to validate.
use bevy_ecs::prelude::Resource;

#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct SpawnTestUiState {
    pub attempted: bool,
}

pub const UI_OPACITY_MIN: f32 = 0.25;
pub const UI_OPACITY_MAX: f32 = 1.0;
pub const UI_OPACITY_DEFAULT: f32 = 0.96;

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

#[derive(Resource, Debug, Clone, Copy)]
pub struct UiPreferences {
    pub mode: OverlayMode,
    /// F1 override; panel transitions restore their default input ownership.
    pub input_capture_override: Option<bool>,
    /// Keep game input captured until Escape, which closed the overlay, is released.
    pub escape_capture_release_pending: bool,
    pub ui_opacity: f32,
    pub show_player: bool,
    pub show_camera: bool,
    pub show_raycast: bool,
    pub show_vehicle: bool,
    pub show_runtime: bool,
}

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
