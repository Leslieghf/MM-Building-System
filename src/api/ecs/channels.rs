//! Bounded cross-callback channels and their typed endpoint bundles.
use super::{
    GameSnapshot, INTENT_QUEUE_CAPACITY, IntentOutcome, OUTCOME_QUEUE_CAPACITY,
    SNAPSHOT_QUEUE_CAPACITY, SPAWN_INTENT_QUEUE_CAPACITY,
};
use crate::api::game::abi::types::Matrix4x4;
use crate::api::scripts::console::{
    self, ConsoleEventReceiver, ConsoleEventSink, ConsoleRequest, ConsoleRequestSender,
};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};

#[derive(Debug, Clone, Copy)]
/// Velocity effect copied from the UI-side producer to the game callback.
pub struct IntentEnvelope {
    /// Host-local identifier used to correlate the completion outcome.
    pub intent_id: u64,
    /// Requested velocity in game units per second.
    pub velocity: [f32; 3],
}

#[derive(Debug, Clone)]
/// Spawn request copied from the producer to the game callback.
pub struct SpawnIntentEnvelope {
    /// Host-local request identifier.
    pub request_id: u64,
    /// Validated exported-entity key text.
    pub asset_key: String,
    /// Finite row-major placement transform.
    pub matrix: Matrix4x4,
}

/// Channel bundle consumed by game callback orchestration.
pub struct GameEndpoint {
    /// Script console requests waiting for game-boundary processing.
    pub console_requests: Receiver<ConsoleRequest>,
    /// Sink for user-visible script console events.
    pub console_events: ConsoleEventSink,
    /// Worker-backed package compiler and discovery service.
    pub script_loader: crate::api::scripts::loader::ScriptLoader,
    /// Publisher for completed package discovery results.
    pub discovery_tx: SyncSender<crate::api::scripts::loader::ScriptDiscoveryResult>,
    /// Publisher for script lifecycle and callback events.
    pub script_events: crate::api::scripts::ScriptEventSink,
    /// Bounded publisher for owned game snapshots.
    pub snapshot_tx: SyncSender<GameSnapshot>,
    /// Receiver for validated velocity effects.
    pub intent_rx: Receiver<IntentEnvelope>,
    /// Publisher for completed velocity outcomes.
    pub outcome_tx: SyncSender<IntentOutcome>,
    /// Receiver for bounded spawn requests.
    pub spawn_rx: Receiver<SpawnIntentEnvelope>,
    /// Non-blocking input mailbox shared with render presentation.
    pub input_mailbox: crate::api::ecs::input::InputFrameMailbox,
}

/// Channel bundle consumed by render/UI presentation.
pub struct UiEndpoint {
    /// Sender for bounded console commands.
    pub console_requests: ConsoleRequestSender,
    /// Receiver for script and console output.
    pub console_events: ConsoleEventReceiver,
    /// Handle for asynchronous package discovery requests.
    pub discovery_requester: crate::api::scripts::loader::ScriptDiscoveryRequester,
    /// Receiver for package discovery results.
    pub discovery_rx: Receiver<crate::api::scripts::loader::ScriptDiscoveryResult>,
    /// Receiver for recent owned game snapshots.
    pub snapshot_rx: Receiver<GameSnapshot>,
    /// Sender for typed velocity effects.
    pub intent_tx: SyncSender<IntentEnvelope>,
    /// Receiver for completed velocity outcomes.
    pub outcome_rx: Receiver<IntentOutcome>,
    /// Sender for bounded spawn requests.
    pub spawn_tx: SyncSender<SpawnIntentEnvelope>,
    /// Non-blocking input mailbox shared with game callbacks.
    pub input_mailbox: crate::api::ecs::input::InputFrameMailbox,
}

/// Create paired game and UI endpoints with the configured bounded channels.
pub fn create_endpoints() -> (GameEndpoint, UiEndpoint) {
    let (script_events_tx, script_events_rx) = crate::api::scripts::event_channel();
    let (console_request_tx, console_request_rx) = console::request_channel();
    let (snapshot_tx, snapshot_rx) = sync_channel(SNAPSHOT_QUEUE_CAPACITY);
    let (discovery_tx, discovery_rx) =
        sync_channel(crate::api::scripts::loader::LOADER_QUEUE_CAPACITY);
    let (intent_tx, intent_rx) = sync_channel(INTENT_QUEUE_CAPACITY);
    let (outcome_tx, outcome_rx) = sync_channel(OUTCOME_QUEUE_CAPACITY);
    let (spawn_tx, spawn_rx) = sync_channel(SPAWN_INTENT_QUEUE_CAPACITY);
    let input_mailbox = crate::api::ecs::input::InputFrameMailbox::default();
    let script_loader = crate::api::scripts::loader::ScriptLoader::start_for_game(
        crate::api::scripts::engine::RhaiEngineFactory,
    );
    let discovery_requester = script_loader.discovery_requester();

    (
        GameEndpoint {
            console_requests: console_request_rx,
            console_events: script_events_tx.0.clone(),
            script_loader,
            discovery_tx,
            script_events: script_events_tx,
            snapshot_tx,
            intent_rx,
            outcome_tx,
            spawn_rx,
            input_mailbox: input_mailbox.clone(),
        },
        UiEndpoint {
            console_requests: console_request_tx,
            console_events: script_events_rx,
            discovery_requester,
            discovery_rx,
            snapshot_rx,
            intent_tx,
            outcome_rx,
            spawn_tx,
            input_mailbox,
        },
    )
}
