//! Bounded nonblocking request and event channels.
use super::command::ConsoleRequest;
use super::{
    ConsoleCommand, ConsoleEvent, ConsoleSeverity, REQUEST_QUEUE_CAPACITY, next_request_id,
};
use crate::api::scripts::{MAX_EVENT_BYTES, bounded_text};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{Receiver, SyncSender, TryRecvError, TrySendError, sync_channel},
    },
};

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
    let (sender, receiver) = sync_channel(crate::api::scripts::EVENT_QUEUE_CAPACITY);
    let dropped = Arc::new(AtomicU64::new(0));
    (
        ConsoleEventSink {
            sender,
            dropped: dropped.clone(),
        },
        ConsoleEventReceiver { receiver, dropped },
    )
}
