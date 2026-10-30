//! Mad Max mod runtime, split by native integration, user interface, and game runtime.

pub mod api;
mod devtools;
mod kernel;
mod runtime;
pub mod scripting;
pub use api::game::abi as sdk;

pub use devtools::overlay::InitializeASI;
pub use runtime::{
    SpawnAssetKey, SpawnLifecycle, SpawnRequestError, SpawnRequestId, SpawnTransform,
    spawn_entity_intent,
};
