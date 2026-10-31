//! Text command grammar and request identity allocation.
use super::MAX_COMMAND_BYTES;
use crate::api::ecs::input_key::InputKey;
use crate::api::scripts::ScriptId;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

/// Shared by console submissions and worker-created automatic reload notices.
pub(crate) fn next_request_id() -> Result<u64, &'static str> {
    NEXT_REQUEST_ID
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
        .map_err(|_| "console request ID counter exhausted")
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Typed console operation submitted to the runtime.
///
/// Most variants are produced by [`ConsoleCommand::parse`]. Auto-reload is a
/// UI-only request and is created through the typed interface.
pub enum ConsoleCommand {
    /// Show command help, optionally scoped to a topic.
    Help(Option<String>),
    /// List supported input-key names.
    ShowKeys,
    /// List current key bindings.
    ShowBinds,
    /// Bind an input key ID to a console command string.
    Bind {
        /// Stable ID returned by [`InputKey::id`](crate::api::ecs::input_key::InputKey::id).
        key: u32,
        /// Quoted command text to execute when the key is pressed.
        command: String,
    },
    /// Remove the binding for an input key ID.
    Unbind {
        /// Stable ID returned by [`InputKey::id`](crate::api::ecs::input_key::InputKey::id).
        key: u32,
    },
    /// Rescan and list script packages without compiling their source.
    ScriptList,
    /// Compile and load one package.
    ScriptLoad(ScriptId),
    /// Compile and activate a replacement package generation.
    ScriptReload(ScriptId),
    /// Invoke one package's registered `on_run` callback.
    ScriptRun(ScriptId),
    /// Unload one package generation.
    ScriptUnload(ScriptId),
    /// Request a status row for one package.
    ScriptStatus(ScriptId),
    /// UI-only typed request; deliberately absent from the text parser.
    ScriptAutoReload { id: ScriptId, enabled: bool },
}
impl ConsoleCommand {
    /// Parse one bounded command string into a typed operation.
    ///
    /// # Errors
    ///
    /// Returns a static usage or validation message for malformed syntax,
    /// unsupported keys, reserved hotkeys, invalid package IDs, or input over
    /// [`super::MAX_COMMAND_BYTES`].
    ///
    /// # Example
    ///
    /// ```
    /// use madmax_bootstrap::api::scripts::console::ConsoleCommand;
    ///
    /// let command = ConsoleCommand::parse("script load vehicle_watch")
    ///     .expect("valid script load command");
    /// assert!(matches!(command, ConsoleCommand::ScriptLoad(_)));
    /// ```
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
    /// Return the affected package ID for a script command.
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
/// Console operation paired with its host-generated correlation ID.
pub struct ConsoleRequest {
    /// Monotonic identifier used to match output to the command.
    pub request_id: u64,
    /// Parsed operation to process at a runtime boundary.
    pub command: ConsoleCommand,
}
