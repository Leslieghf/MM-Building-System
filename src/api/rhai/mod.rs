//! Rhai binding implementation, capability catalog, and pure math helpers.
pub(crate) mod bindings;
mod catalog;
mod math;

pub(crate) use bindings::register_v1;
pub use bindings::{GamePostSimContext, ScriptApiRegistry};
pub use catalog::{API_CALLBACKS, API_CAPABILITIES, API_VERSION};
