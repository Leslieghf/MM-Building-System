//! Mad Max mod runtime, split by native integration, user interface, and game runtime.

mod native;
mod runtime;
pub mod scripting;
pub mod sdk;
mod ui;

pub use runtime::{
    SpawnAssetKey, SpawnLifecycle, SpawnRequestError, SpawnRequestId, SpawnTransform,
    spawn_entity_intent,
};
pub use ui::overlay::InitializeASI;
