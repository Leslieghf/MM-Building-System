//! Overlay-local input capture and appearance preferences.
//!
//! Resources here describe UI preferences and the state of the developer-only
//! menus. They are not game entities or durable mod content; accepted requests
//! are emitted as typed intents for the game runtime to validate.
use bevy_ecs::prelude::Resource;

pub const UI_OPACITY_MIN: f32 = 0.25;
pub const UI_OPACITY_MAX: f32 = 1.0;
pub const UI_OPACITY_DEFAULT: f32 = 0.96;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayMode {
    Hud,
    SpawnMenu,
    UiConfigurator,
    ScriptConsole,
}

impl OverlayMode {
    pub fn owns_game_input(self) -> bool {
        matches!(
            self,
            Self::SpawnMenu | Self::UiConfigurator | Self::ScriptConsole
        )
    }
}

#[derive(Resource, Debug, Clone, Copy)]
pub struct UiPreferences {
    pub mode: OverlayMode,
    /// F1 override; panel transitions restore their default input ownership.
    pub input_capture_override: Option<bool>,
    /// Keep game input captured until Escape, which closed the overlay, is released.
    pub escape_capture_release_pending: bool,
    spawn_menu_restore_capture_override: Option<Option<bool>>,
    pub ui_opacity: f32,
    pub show_player: bool,
    pub show_camera: bool,
    pub show_vehicle: bool,
    pub show_runtime: bool,
    pub show_script_panels: bool,
}

impl Default for UiPreferences {
    fn default() -> Self {
        Self {
            mode: OverlayMode::Hud,
            input_capture_override: None,
            escape_capture_release_pending: false,
            spawn_menu_restore_capture_override: None,
            ui_opacity: UI_OPACITY_DEFAULT,
            show_player: true,
            show_camera: false,
            show_vehicle: true,
            show_runtime: false,
            show_script_panels: true,
        }
    }
}

impl UiPreferences {
    pub fn captures_game_input(self) -> bool {
        self.mode == OverlayMode::SpawnMenu
            || self
                .input_capture_override
                .unwrap_or_else(|| self.mode.owns_game_input())
    }

    pub fn set_mode(&mut self, mode: OverlayMode) {
        let leaving_spawn_menu =
            self.mode == OverlayMode::SpawnMenu && mode != OverlayMode::SpawnMenu;
        self.mode = mode;
        self.input_capture_override = if leaving_spawn_menu && mode == OverlayMode::Hud {
            self.spawn_menu_restore_capture_override.take().flatten()
        } else if mode == OverlayMode::SpawnMenu {
            self.input_capture_override
        } else {
            None
        };
        self.escape_capture_release_pending = false;
    }

    pub fn set_spawn_menu_open(&mut self, open: bool) {
        if open && self.mode != OverlayMode::SpawnMenu {
            self.spawn_menu_restore_capture_override = Some(self.input_capture_override);
            self.mode = OverlayMode::SpawnMenu;
            self.input_capture_override = None;
            self.escape_capture_release_pending = false;
        } else if !open && self.mode == OverlayMode::SpawnMenu {
            self.mode = OverlayMode::Hud;
            self.input_capture_override = self.spawn_menu_restore_capture_override.take().flatten();
            self.escape_capture_release_pending = false;
        }
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

#[cfg(test)]
mod spawn_menu_mode_tests {
    use super::{OverlayMode, UiPreferences};

    #[test]
    fn spawn_menu_captures_input_even_when_capture_override_is_off() {
        let mut preferences = UiPreferences::default();
        preferences.input_capture_override = Some(false);
        preferences.mode = OverlayMode::SpawnMenu;

        assert!(preferences.captures_game_input());
    }

    #[test]
    fn returning_to_hud_restores_the_normal_capture_override() {
        let mut preferences = UiPreferences::default();
        preferences.input_capture_override = Some(true);
        preferences.set_spawn_menu_open(true);
        preferences.set_spawn_menu_open(false);

        assert!(preferences.captures_game_input());
    }

    #[test]
    fn opening_another_input_owning_panel_does_not_inherit_hud_override() {
        let mut preferences = UiPreferences::default();
        preferences.input_capture_override = Some(false);
        preferences.set_spawn_menu_open(true);
        preferences.set_mode(OverlayMode::UiConfigurator);

        assert!(preferences.captures_game_input());
    }
}
