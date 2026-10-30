//! UI-owned bounded scrollback and local command result presentation.
use super::{
    ConsoleEvent, ConsoleEventReceiver, ConsoleEventSink, ConsoleRequestSender, ConsoleSeverity,
    MAX_COMMAND_BYTES, MAX_EVENTS_PER_RENDER, MAX_SCROLLBACK_BYTES, MAX_SCROLLBACK_ENTRIES,
};
use crate::api::scripts::{MAX_EVENT_BYTES, bounded_text};
use bevy_ecs::prelude::Resource;
use std::collections::VecDeque;
use std::sync::mpsc::TryRecvError;

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
    let lines = match help_lines(topic) {
        Ok(lines) => lines,
        Err(unknown) => {
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

fn help_lines(topic: Option<&str>) -> Result<&'static [&'static str], &str> {
    Ok(match topic {
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
        Some(unknown) => return Err(unknown),
    })
}
