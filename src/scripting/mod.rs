//! Owned, versioned Mad Max scripting boundary.
pub mod api;
pub mod api_catalog;
pub mod auto_reload;
pub mod console;
pub mod dispatch;
pub mod engine;
pub mod loader;
pub mod manager;
pub mod package;

pub use api_catalog::API_VERSION;
pub const MAX_SCRIPT_ID_BYTES: usize = 64;
pub const EVENT_QUEUE_CAPACITY: usize = 128;
pub const MAX_EVENT_BYTES: usize = 1024;
pub const MAX_LOGS_PER_INVOCATION: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScriptId(String);
impl ScriptId {
    pub fn parse(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        if value.is_empty()
            || value.len() > MAX_SCRIPT_ID_BYTES
            || matches!(value.as_str(), "con" | "prn" | "aux" | "nul")
            || ((value.starts_with("com") || value.starts_with("lpt"))
                && value.len() == 4
                && matches!(value.as_bytes()[3], b'1'..=b'9'))
            || !value
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
        {
            return Err(
                "invalid script ID: use 1..64 lowercase letters, digits, underscores or hyphens",
            );
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptGeneration(pub u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ScriptPhaseId {
    GamePostSim,
}
impl ScriptPhaseId {
    pub fn as_str(self) -> &'static str {
        "game.post_sim"
    }
}
#[derive(Debug, Clone)]
pub struct ScriptApiDescriptor {
    pub api_version: u32,
    pub phase: ScriptPhaseId,
    pub callback: &'static str,
    pub values: &'static [&'static str],
    pub methods: &'static [&'static str],
    pub unavailable: &'static str,
    pub capabilities: &'static [&'static str],
    pub effect: &'static str,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptStatus {
    Active,
    Disabled,
    Unloaded,
}
#[derive(Debug, Clone, Copy)]
pub enum ScriptEventKind {
    Log,
    Status,
    Error,
}
#[derive(Debug, Clone)]
pub struct ScriptEvent {
    pub request_id: Option<u64>,
    pub id: ScriptId,
    pub generation: ScriptGeneration,
    pub phase: ScriptPhaseId,
    pub callback: String,
    pub kind: ScriptEventKind,
    pub status: Option<ScriptStatus>,
    pub auto_reload: bool,
    pub reload_error: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub text: String,
}
#[derive(Clone)]
pub struct ScriptEventSink(pub console::ConsoleEventSink);
impl ScriptEventSink {
    pub fn emit(&self, mut event: ScriptEvent) {
        event.text = bounded_text(&event.text, MAX_EVENT_BYTES);
        event.callback = bounded_text(&event.callback, MAX_SCRIPT_ID_BYTES);
        let source_position = if event.line.is_some() || event.column.is_some() {
            Some(console::SourcePosition {
                line: event.line,
                column: event.column,
            })
        } else {
            None
        };
        self.0.emit(console::ConsoleEvent {
            request_id: event.request_id,
            severity: match event.kind {
                ScriptEventKind::Error => console::ConsoleSeverity::Error,
                _ => console::ConsoleSeverity::Info,
            },
            script: Some(event.id),
            generation: (event.generation.0 != 0).then_some(event.generation),
            status: event.status,
            auto_reload: Some(event.auto_reload),
            reload_error: event
                .reload_error
                .map(|text| bounded_text(&text, MAX_EVENT_BYTES)),
            phase: Some(event.phase),
            source_position,
            text: event.text,
        });
    }
}
pub fn event_channel() -> (ScriptEventSink, console::ConsoleEventReceiver) {
    let (tx, rx) = console::event_channel();
    (ScriptEventSink(tx), rx)
}
pub(crate) fn bounded_text(value: &str, limit: usize) -> String {
    let mut end = value.len().min(limit);
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}
pub mod vector;
