//! UI-owned Bevy resources grouped by the developer-tool feature that consumes them.
//!
//! # Boundary
//!
//! These resources hold presentation preferences, transient selections, and
//! copied display data. They do not own the game runtime, script manager, or
//! native handles. Modules are split by the UI feature that reads or updates
//! each resource; see [`presentation`], [`overlay`], and [`console`].
mod console;
mod overlay;
mod presentation;

pub use console::{DevToolsTab, ScriptConsoleStatusRow, ScriptConsoleUiState};
pub use overlay::{OverlayMode, SpawnTestUiState, UI_OPACITY_MAX, UI_OPACITY_MIN, UiPreferences};
pub use presentation::{
    DebugSection, PendingUiInput, PresentationRow, UiGameSnapshot, UiPresentation,
};
