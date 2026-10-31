//! Capacity and work limits for bounded ECS transports and callback phases.
//!
//! Queue capacities bound retained cross-thread data; per-render and
//! per-callback limits bound work performed at a single boundary. Producers
//! must handle a full queue explicitly rather than blocking the game or render
//! thread.
/// Number of owned snapshots retained by cross-callback transport.
pub const SNAPSHOT_QUEUE_CAPACITY: usize = 8;
/// Number of typed velocity effects retained by cross-callback transport.
pub const INTENT_QUEUE_CAPACITY: usize = 32;
/// Number of completed velocity outcomes retained for presentation.
pub const OUTCOME_QUEUE_CAPACITY: usize = 32;
/// Maximum general intents imported during one render callback.
pub const MAX_INTENTS_PER_RENDER: usize = 8;
/// Capacity reserved for queued spawn requests.
pub const SPAWN_INTENT_QUEUE_CAPACITY: usize = 1;
/// Maximum spawn requests dispatched during one render callback.
pub const MAX_SPAWNS_PER_RENDER: usize = 1;
/// Maximum UTF-8 bytes accepted for an exported entity key.
pub const MAX_SPAWN_ASSET_KEY_BYTES: usize = 240;
use std::sync::atomic::AtomicU64;
pub(crate) static NEXT_INTENT_ID: AtomicU64 = AtomicU64::new(1);
