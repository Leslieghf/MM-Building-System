//! Owned console event schema and script diagnostic metadata.
use crate::api::scripts::{
    MAX_EVENT_BYTES, ScriptGeneration, ScriptId, ScriptPhaseId, ScriptStatus, bounded_text,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleSeverity {
    Info,
    Warning,
    Error,
}
#[derive(Debug, Clone, Copy)]
pub struct SourcePosition {
    pub line: Option<usize>,
    pub column: Option<usize>,
}
#[derive(Debug, Clone)]
pub struct ConsoleEvent {
    pub request_id: Option<u64>,
    pub severity: ConsoleSeverity,
    pub script: Option<ScriptId>,
    pub generation: Option<ScriptGeneration>,
    pub status: Option<ScriptStatus>,
    pub auto_reload: Option<bool>,
    pub reload_error: Option<String>,
    pub phase: Option<ScriptPhaseId>,
    pub source_position: Option<SourcePosition>,
    pub text: String,
}
impl ConsoleEvent {
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
