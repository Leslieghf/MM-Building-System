//! In-process Mad Max modloader and code-library foundation.
//!
//! # Extension surfaces
//!
//! [`api::ecs`] provides owned Rust system contracts and typed effects;
//! [`api::scripts`] exposes the managed Rhai package lifecycle. Only Rhai is a
//! supported user-authored extension format. Rust registrations are
//! in-process and do not define a stable external ABI.
//!
//! # Native access
//!
//! Game-specific addresses, layouts, hooks, and calls live in the low-level
//! [`api::game`] implementation module. Extensions should use the owned
//! [`api::ecs`] contracts; they cannot retain native pointers through that API.

pub mod api;
mod devtools;
mod kernel;

pub use api::ecs::{
    SpawnAssetKey, SpawnLifecycle, SpawnRequestError, SpawnRequestId, SpawnTransform,
    spawn_entity_intent,
};
pub use kernel::entry::InitializeASI;
