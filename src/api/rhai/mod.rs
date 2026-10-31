//! Versioned script-facing API registration and reference metadata.
//!
//! `bindings` registers the context and callback-scoped effect collectors;
//! `catalog` is the source for both runtime metadata and generated API
//! documentation; `math` exposes pure helpers that require no capability.
//! This layer is the only script API surface: scripts do not receive Rust ABI
//! types, native addresses, or general filesystem/network access.
pub(crate) mod bindings;
mod catalog;
mod math;

pub(crate) use bindings::register_v1;
pub use bindings::{GamePostSimContext, ScriptApiRegistry};
pub use catalog::{API_CALLBACKS, API_CAPABILITIES, API_VERSION};
