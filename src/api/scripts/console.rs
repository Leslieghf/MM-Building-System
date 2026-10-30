//! Bounded, owned console messages. Neither callback waits for queue capacity.
use super::{
    MAX_EVENT_BYTES, ScriptGeneration, ScriptId, ScriptPhaseId, ScriptStatus, bounded_text,
};
use crate::api::ecs::input_key::InputKey;
use bevy_ecs::prelude::Resource;
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{Receiver, SyncSender, TryRecvError, TrySendError, sync_channel},
    },
};

pub const MAX_COMMAND_BYTES: usize = 256;
pub const REQUEST_QUEUE_CAPACITY: usize = 32;
pub const MAX_COMMANDS_PER_GAME_CALLBACK: usize = 8;
pub const MAX_EVENTS_PER_RENDER: usize = super::EVENT_QUEUE_CAPACITY;
pub const MAX_SCROLLBACK_ENTRIES: usize = 256;
pub const MAX_SCROLLBACK_BYTES: usize = 128 * 1024;
pub const MAX_ANSI_SGR_SPANS: usize = 128;
static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

/// Shared by console submissions and worker-created automatic reload notices.
pub(super) fn next_request_id() -> Result<u64, &'static str> {
    NEXT_REQUEST_ID
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
        .map_err(|_| "console request ID counter exhausted")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsoleCommand {
    Help(Option<String>),
    ShowKeys,
    ShowBinds,
    Bind {
        key: u32,
        command: String,
    },
    Unbind {
        key: u32,
    },
    ScriptList,
    ScriptLoad(ScriptId),
    ScriptReload(ScriptId),
    ScriptRun(ScriptId),
    ScriptUnload(ScriptId),
    ScriptStatus(ScriptId),
    /// UI-only typed request; deliberately absent from the text parser.
    ScriptAutoReload {
        id: ScriptId,
        enabled: bool,
    },
}
impl ConsoleCommand {
    pub fn parse(input: &str) -> Result<Self, &'static str> {
        if input.len() > MAX_COMMAND_BYTES {
            return Err("console input exceeds 256 UTF-8 bytes");
        }
        if input.split_whitespace().next() == Some("bind") {
            return parse_bind_command(input);
        }
        let mut words = input.split_whitespace();
        let first = words.next();
        if first == Some("help") {
            let topic = words.collect::<Vec<_>>().join(" ");
            return Ok(Self::Help((!topic.is_empty()).then_some(topic)));
        }
        let second = words.next();
        let third = words.next();
        let fourth = words.next();
        let fifth = words.next();
        if words.next().is_some() {
            return Err("too many arguments; use help");
        }
        match (first, second, third, fourth, fifth) {
            (Some("show_keys"), None, None, None, None) => Ok(Self::ShowKeys),
            (Some("show_binds"), None, None, None, None) => Ok(Self::ShowBinds),
            (Some("unbind"), Some(key), None, None, None) => {
                let key = InputKey::parse(key).ok_or("unknown key; use show_keys")?;
                if matches!(key, InputKey::F1 | InputKey::F2 | InputKey::F3) {
                    return Err("F1, F2 and F3 are reserved mod controls");
                }
                Ok(Self::Unbind { key: key.id() })
            }
            (Some("script"), Some("list"), None, None, None) => Ok(Self::ScriptList),
            (Some("script"), Some(action), Some(id), None, None) => {
                let constructor = match action {
                    "load" => Self::ScriptLoad,
                    "reload" => Self::ScriptReload,
                    "run" => Self::ScriptRun,
                    "unload" => Self::ScriptUnload,
                    "status" => Self::ScriptStatus,
                    "list" => return Err("script list takes no arguments"),
                    _ => return Err("unknown script command; use help"),
                };
                Ok(constructor(ScriptId::parse(id)?))
            }
            (None, _, _, _, _) => Err("enter a command; use help"),
            (Some("bind"), _, _, _, _) => Err("usage: bind <key> \"<console command>\""),
            (Some("unbind"), _, _, _, _) => Err("usage: unbind <key>"),
            (Some("script"), _, _, _, _) => {
                Err("usage: script list or script <load|reload|run|unload|status> <id>")
            }
            _ => Err("unknown command; use help"),
        }
    }
    pub fn script(&self) -> Option<&ScriptId> {
        match self {
            Self::ScriptLoad(id)
            | Self::ScriptReload(id)
            | Self::ScriptRun(id)
            | Self::ScriptUnload(id)
            | Self::ScriptStatus(id)
            | Self::ScriptAutoReload { id, .. } => Some(id),
            _ => None,
        }
    }
}

fn parse_bind_command(input: &str) -> Result<ConsoleCommand, &'static str> {
    let trimmed = input.trim();
    let key_start = trimmed
        .find(char::is_whitespace)
        .ok_or("usage: bind <key> \"<console command>\"")?;
    let arguments = trimmed[key_start..].trim_start();
    let key_end = arguments
        .find(char::is_whitespace)
        .ok_or("usage: bind <key> \"<console command>\"")?;
    let key = InputKey::parse(&arguments[..key_end]).ok_or("unknown key; use show_keys")?;
    if matches!(key, InputKey::F1 | InputKey::F2 | InputKey::F3) {
        return Err("F1, F2 and F3 are reserved mod controls");
    }

    let quoted = arguments[key_end..].trim();
    if quoted.len() < 2 || !quoted.starts_with('"') || !quoted.ends_with('"') {
        return Err("usage: bind <key> \"<console command>\"");
    }

    let mut command = String::with_capacity(quoted.len() - 2);
    let mut chars = quoted[1..quoted.len() - 1].chars();
    while let Some(character) = chars.next() {
        if character == '\\' {
            match chars.next() {
                Some('"') => command.push('"'),
                Some('\\') => command.push('\\'),
                Some(other) => {
                    command.push('\\');
                    command.push(other);
                }
                None => command.push('\\'),
            }
        } else {
            command.push(character);
        }
    }
    if command.trim().is_empty() {
        return Err("bind command cannot be empty");
    }
    if command.len() > MAX_COMMAND_BYTES {
        return Err("bind command exceeds 256 UTF-8 bytes");
    }
    Ok(ConsoleCommand::Bind {
        key: key.id(),
        command,
    })
}

#[derive(Debug, Clone)]
pub struct ConsoleRequest {
    pub request_id: u64,
    pub command: ConsoleCommand,
}
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

/// Exactly one UI producer owns this handle: ID allocation and enqueue cannot
/// race/reorder on a shared channel. The private sender is deliberately not Clone.
pub struct ConsoleRequestSender {
    sender: SyncSender<ConsoleRequest>,
    bindings: BTreeMap<u32, String>,
}
impl ConsoleRequestSender {
    pub fn bindings(&self) -> &BTreeMap<u32, String> {
        &self.bindings
    }
    pub fn bind(&mut self, key: u32, command: String) {
        self.bindings.insert(key, command);
    }
    pub fn unbind(&mut self, key: u32) -> bool {
        self.bindings.remove(&key).is_some()
    }
    pub fn submit(&mut self, command: ConsoleCommand) -> Result<u64, ConsoleEvent> {
        let request_id = next_request_id()
            .map_err(|text| ConsoleEvent::new(None, ConsoleSeverity::Error, text))?;
        let script = command.script().cloned();
        self.sender
            .try_send(ConsoleRequest {
                request_id,
                command,
            })
            .map_err(|error| {
                let text = match error {
                    TrySendError::Full(_) => "console request queue full; command rejected; retry",
                    TrySendError::Disconnected(_) => {
                        "game console request channel disconnected; command rejected"
                    }
                };
                let mut event = ConsoleEvent::new(Some(request_id), ConsoleSeverity::Error, text);
                event.script = script;
                event
            })?;
        Ok(request_id)
    }
    pub fn submit_input(&mut self, input: &str) -> Result<u64, ConsoleEvent> {
        let command = ConsoleCommand::parse(input)
            .map_err(|text| ConsoleEvent::new(None, ConsoleSeverity::Error, text))?;
        self.submit(command)
    }
}
pub fn request_channel() -> (ConsoleRequestSender, Receiver<ConsoleRequest>) {
    let (sender, receiver) = sync_channel(REQUEST_QUEUE_CAPACITY);
    (
        ConsoleRequestSender {
            sender,
            bindings: BTreeMap::new(),
        },
        receiver,
    )
}

#[derive(Clone)]
pub struct ConsoleEventSink {
    sender: SyncSender<ConsoleEvent>,
    dropped: Arc<AtomicU64>,
}
pub struct ConsoleEventReceiver {
    receiver: Receiver<ConsoleEvent>,
    dropped: Arc<AtomicU64>,
}
impl ConsoleEventSink {
    pub fn emit(&self, mut event: ConsoleEvent) {
        event.text = bounded_text(&event.text, MAX_EVENT_BYTES);
        if self.sender.try_send(event).is_err() {
            // Saturating retained state, never recursively report into a full queue.
            let _ = self
                .dropped
                .try_update(Ordering::Relaxed, Ordering::Relaxed, |count| {
                    Some(count.saturating_add(1))
                });
        }
    }
}
impl ConsoleEventReceiver {
    pub fn try_recv(&self) -> Result<ConsoleEvent, TryRecvError> {
        self.receiver.try_recv()
    }
    pub fn take_dropped(&self) -> u64 {
        self.dropped.swap(0, Ordering::Relaxed)
    }
}
pub fn event_channel() -> (ConsoleEventSink, ConsoleEventReceiver) {
    let (sender, receiver) = sync_channel(super::EVENT_QUEUE_CAPACITY);
    let dropped = Arc::new(AtomicU64::new(0));
    (
        ConsoleEventSink {
            sender,
            dropped: dropped.clone(),
        },
        ConsoleEventReceiver { receiver, dropped },
    )
}

/// UI-owned retained output, ready for the Task 4 panel. No engine or game API.
#[derive(Resource, Default)]
pub struct ConsoleOutput {
    events: VecDeque<ConsoleEvent>,
    text_bytes: usize,
    disconnected: bool,
    revision: u64,
}
impl ConsoleOutput {
    pub fn events(&self) -> &VecDeque<ConsoleEvent> {
        &self.events
    }
    /// Counts retained and evicted events so UI caches observe each event once.
    pub fn revision(&self) -> u64 {
        self.revision
    }
    /// Clears retained console output without crossing the game request channel.
    pub fn clear(&mut self) {
        self.events.clear();
        self.text_bytes = 0;
        self.revision = self.revision.saturating_add(1);
    }
    pub fn record(&mut self, mut event: ConsoleEvent) {
        self.revision = self.revision.saturating_add(1);
        event.text = bounded_text(&event.text, MAX_EVENT_BYTES);
        self.text_bytes += event.text.len();
        self.events.push_back(event);
        while self.events.len() > MAX_SCROLLBACK_ENTRIES || self.text_bytes > MAX_SCROLLBACK_BYTES {
            if let Some(old) = self.events.pop_front() {
                self.text_bytes -= old.text.len();
            }
        }
    }
    /// Records local parse/transport failures directly, even when game queues are full.
    pub fn submit_input(&mut self, sender: &mut ConsoleRequestSender, input: &str) -> Option<u64> {
        match sender.submit_input(input) {
            Ok(id) => {
                self.record(ConsoleEvent::new(
                    Some(id),
                    ConsoleSeverity::Info,
                    &format!("> {}", bounded_text(input, MAX_COMMAND_BYTES)),
                ));
                Some(id)
            }
            Err(event) => {
                self.record(ConsoleEvent::new(
                    None,
                    ConsoleSeverity::Info,
                    &format!("> {}", bounded_text(input, MAX_COMMAND_BYTES)),
                ));
                self.record(event);
                None
            }
        }
    }
    pub fn drain(&mut self, receiver: &ConsoleEventReceiver) {
        for _ in 0..MAX_EVENTS_PER_RENDER {
            match receiver.try_recv() {
                Ok(event) => self.record(event),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    if !self.disconnected {
                        self.disconnected = true;
                        self.record(ConsoleEvent::new(
                            None,
                            ConsoleSeverity::Error,
                            "game console event channel disconnected",
                        ));
                    }
                    break;
                }
            }
        }
        let dropped = receiver.take_dropped();
        if dropped != 0 {
            self.record(ConsoleEvent::new(None, ConsoleSeverity::Warning,
                &format!("console event queue overload: {dropped} events dropped; output/results may be missing; use script list/status to refresh")));
        }
    }
}

pub fn emit_help(sink: &ConsoleEventSink, request_id: u64, topic: Option<&str>) {
    let lines: &[&str] = match topic {
        None => &[
            "Commands",
            "  help [command]                 Show all commands or command details",
            "  clear                          Clear console output",
            "  show_keys                      List bindable key names",
            "  show_binds                     List current key bindings",
            "  bind <key> \"<command>\"         Bind a console command to a key",
            "  unbind <key>                   Remove a key binding",
            "  script list                    Discover script packages",
            "  script load <id>               Prepare and load a script",
            "  script reload <id>             Prepare and replace a loaded script",
            "  script run <id>                Run its on_run callback once",
            "  script unload <id>             Unload a script",
            "  script status <id>             Show a loaded script's status",
            "Full script API reference: repository docs/rhai-api.md.",
            "Use help <command> for details, such as help bind or help script load.",
        ],
        Some("help") => &[
            "Show the command index, or detailed help for one command.",
            "Examples: help bind | help script list | help script load",
        ],
        Some("clear") => &["Clear the retained console output."],
        Some("show_keys") => &["List key names accepted by bind and unbind."],
        Some("show_binds") => &["List the current key-to-command bindings."],
        Some("bind") => &[
            "Run the quoted console command when the mod runtime detects that key press.",
            "F1, F2, and F3 are reserved. Example: bind g \"script run vehicle_jump\"",
        ],
        Some("unbind") => &["Remove the binding for that key."],
        Some("script list") => &[
            "Scan package manifests and report discovered packages. This does not execute scripts.",
        ],
        Some("script load") => &[
            "Validate and compile a discovered package, then activate it at the game callback boundary.",
            "Phases: Preparing → Prepared → Loaded.",
        ],
        Some("script reload") => &[
            "Prepare a replacement and activate it at the game callback boundary.",
            "Phases: Preparing → Prepared → Reloaded.",
        ],
        Some("script run") => {
            &["Queue the loaded script's on_run entry point for the next game.post_sim callback."]
        }
        Some("script unload") => {
            &["Unload the script and cancel its queued one-shot callbacks and pending effects."]
        }
        Some("script status") => {
            &["Show that script's status, callbacks, and pending preparation."]
        }
        Some(unknown) => {
            sink.emit(ConsoleEvent::new(
                Some(request_id),
                ConsoleSeverity::Error,
                &format!("No help topic for '{unknown}'. Use help for the command index."),
            ));
            return;
        }
    };
    for line in lines {
        sink.emit(ConsoleEvent::new(
            Some(request_id),
            ConsoleSeverity::Info,
            line,
        ));
    }
}
