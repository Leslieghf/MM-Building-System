//! Owned, versioned Mad Max scripting boundary.
//!
//! This module defines the host-side package lifecycle and event contract.
//! Script-visible names and semantics are cataloged in [`crate::api::rhai`].
//! Start with [`ScriptId`](crate::api::scripts::ScriptId),
//! [`PreparedScript`](crate::api::scripts::package::PreparedScript), and
//! [`ScriptManager`](crate::api::scripts::manager::ScriptManager); bounded
//! diagnostics use [`ScriptEvent`](crate::api::scripts::ScriptEvent) and
//! [`event_channel`](crate::api::scripts::event_channel). Package files are
//! described in the [scripting guide](https://github.com/Leslieghf/MM-Building-System/blob/main/docs/scripting.md).
pub mod auto_reload;
pub mod console;
pub mod engine;
pub mod loader;
pub mod manager;
pub mod package;

pub use crate::api::rhai::API_VERSION;
/// Maximum encoded length of a validated package identifier.
pub const MAX_SCRIPT_ID_BYTES: usize = 64;
/// Capacity of the bounded script-event channel.
pub const EVENT_QUEUE_CAPACITY: usize = 128;
/// Maximum UTF-8 bytes retained for one script event message.
pub const MAX_EVENT_BYTES: usize = 1024;
/// Maximum log events accepted from one callback invocation.
pub const MAX_LOGS_PER_INVOCATION: usize = 8;

/// Validated package identifier used by script requests and status events.
///
/// IDs are case-sensitive lowercase ASCII and safe to use as one directory
/// component. They are not paths; package discovery also verifies that the
/// manifest ID matches its containing directory.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScriptId(String);
impl ScriptId {
    /// Parse a lowercase package identifier.
    ///
    /// # Errors
    ///
    /// Returns a message when the identifier is empty, longer than 64 bytes,
    /// contains unsupported characters, or uses a reserved Windows device name.
    ///
    /// # Examples
    ///
    /// ```
    /// use madmax_bootstrap::api::scripts::ScriptId;
    ///
    /// let id = ScriptId::parse("vehicle_watch").expect("valid ID");
    /// assert_eq!(id.as_str(), "vehicle_watch");
    /// assert!(ScriptId::parse("Vehicle Watch").is_err());
    /// ```
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
    /// Return the canonical identifier text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
/// Monotonic, host-local generation number assigned when a package activates.
///
/// Reloading a package creates a new generation and resets that package's
/// persistent Rhai `state` map. Generation zero in [`ScriptEvent`] denotes a
/// request or failure that has no active generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptGeneration(pub u64);
/// Callback phase represented in script events and API descriptors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ScriptPhaseId {
    /// Runs after a sampled Mad Max simulation callback.
    GamePostSim,
}
impl ScriptPhaseId {
    /// Return the manifest key for this callback phase.
    pub fn as_str(self) -> &'static str {
        "game.post_sim"
    }
}
/// One generated row in the versioned script API reference.
#[derive(Debug, Clone)]
pub struct ScriptApiDescriptor {
    /// API contract version required by the package.
    pub api_version: u32,
    /// Host phase in which the callback runs.
    pub phase: ScriptPhaseId,
    /// Callback name exposed to the script.
    pub callback: &'static str,
    /// Script-visible values exposed through the callback context.
    pub values: &'static [&'static str],
    /// Script-visible method signatures.
    pub methods: &'static [&'static str],
    /// Representation used when a value is unavailable.
    pub unavailable: &'static str,
    /// Capabilities required by the described context members.
    pub capabilities: &'static [&'static str],
    /// Bounded effect or lifecycle behavior associated with the callback.
    pub effect: &'static str,
}
/// Current lifecycle state of one loaded package generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptStatus {
    /// Callback execution is enabled.
    Active,
    /// Callback execution stopped after a script error.
    Disabled,
    /// No active generation remains for the package.
    Unloaded,
}
/// Classification of a script-host event.
#[derive(Debug, Clone, Copy)]
pub enum ScriptEventKind {
    /// Bounded informational output emitted by a script.
    Log,
    /// Package lifecycle or status update.
    Status,
    /// Validation, compilation, or callback failure.
    Error,
}
/// Owned lifecycle, log, or error event emitted by the script host.
#[derive(Debug, Clone)]
pub struct ScriptEvent {
    /// Request that caused the event, when one exists.
    pub request_id: Option<u64>,
    /// Package associated with the event.
    pub id: ScriptId,
    /// Package generation associated with the event; zero means not active.
    pub generation: ScriptGeneration,
    /// Callback phase associated with the event.
    pub phase: ScriptPhaseId,
    /// Callback or operation name.
    pub callback: String,
    /// Event category.
    pub kind: ScriptEventKind,
    /// Lifecycle state when the event reports one.
    pub status: Option<ScriptStatus>,
    /// Whether file-change reload is enabled for the package.
    pub auto_reload: bool,
    /// Most recent reload error, if present.
    pub reload_error: Option<String>,
    /// One-based source line when reported by the engine.
    pub line: Option<usize>,
    /// One-based source column when reported by the engine.
    pub column: Option<usize>,
    /// UTF-8 message, truncated to [`MAX_EVENT_BYTES`] at emission.
    pub text: String,
}

/// Named native query configuration accepted by the Rhai raycast API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptRaycastFilter {
    /// Mapped static-only filter with its known category mask.
    StaticOnly,
    /// Mapped player-aim candidate predicate.
    PlayerAim,
}

/// Owned ray request emitted by a successful script callback.
#[derive(Debug, Clone)]
pub struct ScriptRaycastRequest {
    /// Ticket returned to the requesting script.
    pub ticket: u64,
    /// World-space ray origin.
    pub origin: [f32; 3],
    /// Normalized world-space direction.
    pub direction: [f32; 3],
    /// Maximum query distance in world units.
    pub max_distance: f32,
    /// Verified filter profile selected by the script.
    pub filter: ScriptRaycastFilter,
    /// Script and generation attribution for lifecycle checks.
    pub source: ScriptEvent,
    /// Callback sequence after which the ticket expires.
    pub expires_at: u64,
}

/// Copied completion for one native ray request.
#[derive(Debug, Clone)]
pub struct ScriptRaycastCompletion {
    /// Ticket associated with the request.
    pub ticket: u64,
    /// Original script generation.
    pub source: ScriptEvent,
    /// Whether the native query API was available and called.
    pub available: bool,
    /// Whether the game helper reported a query state.
    pub state_present: bool,
    /// Optional caller-observed segment fraction.
    pub fraction: Option<f32>,
    /// Optional reconstructed world-space hit position.
    pub hit_position: Option<[f32; 3]>,
}

/// Bounded publisher for script-host events.
#[derive(Clone)]
pub struct ScriptEventSink(pub console::ConsoleEventSink);
impl ScriptEventSink {
    /// Bound and publish an event to the console event channel.
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
/// Create the script event sink and its bounded console receiver.
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
