//! Owned Rust extension contracts and the host-driven game ECS.
//!
//! # Module map
//!
//! [`components`] and [`resources`] separate entity data from runtime state;
//! [`systems`] defines schedule execution; [`transport`] owns cross-thread
//! endpoints; [`runtime`] owns callback orchestration; [`input`] and
//! [`intents`] define mod-facing contracts.
//!
//! Systems use copied values only. Build-specific reads and writes stay behind
//! `api::game::native`; managed scripts use [`crate::api::scripts`].
pub mod components;
mod ids;
pub mod input;
pub mod input_key;
pub mod intents;
pub mod limits;
pub mod resources;
pub mod runtime;
pub mod systems;
pub mod transport;
pub mod types;

pub use components::*;
pub use input::*;
pub use intents::*;
pub use limits::*;
pub use resources::*;
pub use runtime::*;
pub use systems::schedule::{create_game_app, update_game_frame};
pub use systems::schedule_labels::*;
pub use transport::*;
pub use types::*;

#[cfg(test)]
mod tests;
