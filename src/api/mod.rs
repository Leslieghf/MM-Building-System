//! Versioned modding API from guarded game access through ECS and Rhai.
pub mod ecs;
pub mod format;
pub mod game;
/// Script lifecycle and event contracts exposed through the modding API.
pub use crate::scripting as scripts;
