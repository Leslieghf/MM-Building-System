//! Owned console event schema shared by the script manager and UI.
//!
//! Events carry bounded text, optional package/generation identity, lifecycle
//! phase, and source position. They contain no Rhai AST, scope, or native
//! object. The transport may drop events under pressure, so aggregate drop
//! counters remain the source for transport-health diagnostics.
use crate::api::scripts::{
    MAX_EVENT_BYTES, ScriptGeneration, ScriptId, ScriptPhaseId, ScriptStatus, bounded_text,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// User-visible severity for a console event.
pub enum ConsoleSeverity {
    /// Informational output.
    Info,
    /// Non-fatal issue or recovery notice.
    Warning,
    /// Command, validation, compile, or callback failure.
    Error,
}
#[derive(Debug, Clone, Copy)]
/// Optional one-based source location reported by Rhai.
pub struct SourcePosition {
    /// Source line, when known.
    pub line: Option<usize>,
    /// Source column, when known.
    pub column: Option<usize>,
}
#[derive(Debug, Clone)]
/// Owned console row with optional script lifecycle metadata.
pub struct ConsoleEvent {
    /// Request identifier for matching command output.
    pub request_id: Option<u64>,
    /// Display severity.
    pub severity: ConsoleSeverity,
    /// Associated script package, when applicable.
    pub script: Option<ScriptId>,
    /// Associated loaded generation, when applicable.
    pub generation: Option<ScriptGeneration>,
    /// Reported script lifecycle state, when applicable.
    pub status: Option<ScriptStatus>,
    /// File-change reload state when the event reports it.
    pub auto_reload: Option<bool>,
    /// Latest reload error when reported.
    pub reload_error: Option<String>,
    /// Script callback phase when applicable.
    pub phase: Option<ScriptPhaseId>,
    /// Source location when the engine reports one.
    pub source_position: Option<SourcePosition>,
    /// Bounded display text.
    pub text: String,
}
impl ConsoleEvent {
    /// Create a standalone event, truncating `text` to the host event limit.
    pub fn new(request_id: Option<u64>, severity: ConsoleSeverity, text: &str) -> Self {
        Self {
            request_id,
            severity,
            script: None,
            generation: None,
            status: None,
            auto_reload: None,
            reload_error: None,
            phase: None,
            source_position: None,
            text: bounded_text(text, MAX_EVENT_BYTES),
        }
    }
}
