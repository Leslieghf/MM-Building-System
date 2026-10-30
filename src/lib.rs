//! In-process Mad Max modloader and code-library foundation.

pub mod api;
mod devtools;
mod kernel;

pub use api::ecs::{
    SpawnAssetKey, SpawnLifecycle, SpawnRequestError, SpawnRequestId, SpawnTransform,
    spawn_entity_intent,
};
pub use kernel::entry::InitializeASI;
