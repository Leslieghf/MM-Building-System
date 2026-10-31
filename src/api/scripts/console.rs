//! Bounded command grammar, cross-thread transport, and transcript retention.
//!
//! [`ConsoleCommand::parse`](crate::api::scripts::console::ConsoleCommand::parse)
//! turns text into a typed request; it does not run a command.
//! the [`bounded request channel`](crate::api::scripts::console::request_channel) transfers
//! requests to game callback processing, while the
//! [`bounded event channel`](crate::api::scripts::console::event_channel) carries
//! bounded results back to presentation.
//!
//! # Text commands
//!
//! ```text
//! help [topic]                 show command help
//! show_keys | show_binds       inspect input names and bindings
//! bind <key> "<command>"      bind a key to a console command
//! unbind <key>                 remove a binding
//! script list                  discover packages without compiling them
//! script <load|reload|run|unload|status> <id>
//! ```
//!
//! `F1`, `F2`, and `F3` are reserved controls. `script auto_reload` is a typed
//! UI request and intentionally has no text-command spelling. Queue capacities
//! and per-callback drain budgets are exposed below.
mod command;
mod event;
mod output;
mod transport;

pub use command::ConsoleCommand;
pub use command::ConsoleRequest;
pub(crate) use command::next_request_id;
pub use event::{ConsoleEvent, ConsoleSeverity, SourcePosition};
pub use output::{ConsoleOutput, emit_help};
pub use transport::{
    ConsoleEventReceiver, ConsoleEventSink, ConsoleRequestSender, event_channel, request_channel,
};

/// Maximum UTF-8 bytes accepted for one console command.
pub const MAX_COMMAND_BYTES: usize = 256;
/// Capacity of the bounded console request channel.
pub const REQUEST_QUEUE_CAPACITY: usize = 32;
/// Maximum console commands consumed at one game callback.
pub const MAX_COMMANDS_PER_GAME_CALLBACK: usize = 8;
/// Maximum console events drained during one render.
pub const MAX_EVENTS_PER_RENDER: usize = super::EVENT_QUEUE_CAPACITY;
/// Maximum retained console transcript entries.
pub const MAX_SCROLLBACK_ENTRIES: usize = 256;
/// Maximum total UTF-8 bytes retained by the transcript.
pub const MAX_SCROLLBACK_BYTES: usize = 128 * 1024;
/// Maximum parsed ANSI SGR spans retained for one transcript entry.
pub const MAX_ANSI_SGR_SPANS: usize = 128;
