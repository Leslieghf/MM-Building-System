//! UI-owned state grouped by the developer-tool feature that consumes it.
mod console;
mod overlay;
mod presentation;

pub use console::{DevToolsTab, ScriptConsoleStatusRow, ScriptConsoleUiState};
pub use overlay::{OverlayMode, SpawnTestUiState, UI_OPACITY_MAX, UI_OPACITY_MIN, UiPreferences};
pub use presentation::{
    DebugSection, PendingUiInput, PresentationRow, UiGameSnapshot, UiPresentation,
};
