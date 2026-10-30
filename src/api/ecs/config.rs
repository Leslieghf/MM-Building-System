//! Queue and per-callback limits for the owned modding runtime.
pub const SNAPSHOT_QUEUE_CAPACITY: usize = 8;
pub const INTENT_QUEUE_CAPACITY: usize = 32;
pub const OUTCOME_QUEUE_CAPACITY: usize = 32;
pub const MAX_INTENTS_PER_RENDER: usize = 8;
pub const SPAWN_INTENT_QUEUE_CAPACITY: usize = 1;
pub const MAX_SPAWNS_PER_RENDER: usize = 1;
pub const MAX_SPAWN_ASSET_KEY_BYTES: usize = 240;
use std::sync::atomic::AtomicU64;
pub(crate) static NEXT_INTENT_ID: AtomicU64 = AtomicU64::new(1);
