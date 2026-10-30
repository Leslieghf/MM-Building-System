//! Bounded cross-callback channels and their typed endpoint bundles.
use super::*;

#[derive(Debug, Clone, Copy)]
pub struct IntentEnvelope {
    pub intent_id: u64,
    pub velocity: [f32; 3],
}

#[derive(Debug, Clone)]
pub struct SpawnIntentEnvelope {
    pub request_id: u64,
    pub asset_key: String,
    pub matrix: Matrix4x4,
}

pub struct GameEndpoint {
    pub console_requests: Receiver<ConsoleRequest>,
    pub console_events: ConsoleEventSink,
    pub script_loader: crate::scripting::loader::ScriptLoader,
    pub discovery_tx: SyncSender<crate::scripting::loader::ScriptDiscoveryResult>,
    pub script_events: crate::scripting::ScriptEventSink,
    pub snapshot_tx: SyncSender<GameSnapshot>,
    pub intent_rx: Receiver<IntentEnvelope>,
    pub outcome_tx: SyncSender<IntentOutcome>,
    pub spawn_rx: Receiver<SpawnIntentEnvelope>,
    pub input_mailbox: crate::api::ecs::input::InputFrameMailbox,
}

pub struct UiEndpoint {
    pub console_requests: ConsoleRequestSender,
    pub console_events: ConsoleEventReceiver,
    pub discovery_requester: crate::scripting::loader::ScriptDiscoveryRequester,
    pub discovery_rx: Receiver<crate::scripting::loader::ScriptDiscoveryResult>,
    pub snapshot_rx: Receiver<GameSnapshot>,
    pub intent_tx: SyncSender<IntentEnvelope>,
    pub outcome_rx: Receiver<IntentOutcome>,
    pub spawn_tx: SyncSender<SpawnIntentEnvelope>,
    pub input_mailbox: crate::api::ecs::input::InputFrameMailbox,
}

pub fn create_endpoints() -> (GameEndpoint, UiEndpoint) {
    let (script_events_tx, script_events_rx) = crate::scripting::event_channel();
    let (console_request_tx, console_request_rx) = console::request_channel();
    let (snapshot_tx, snapshot_rx) = sync_channel(SNAPSHOT_QUEUE_CAPACITY);
    let (discovery_tx, discovery_rx) =
        sync_channel(crate::scripting::loader::LOADER_QUEUE_CAPACITY);
    let (intent_tx, intent_rx) = sync_channel(INTENT_QUEUE_CAPACITY);
    let (outcome_tx, outcome_rx) = sync_channel(OUTCOME_QUEUE_CAPACITY);
    let (spawn_tx, spawn_rx) = sync_channel(SPAWN_INTENT_QUEUE_CAPACITY);
    let input_mailbox = crate::api::ecs::input::InputFrameMailbox::default();
    let script_loader = crate::scripting::loader::ScriptLoader::start_for_game(
        crate::scripting::engine::RhaiEngineFactory,
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
