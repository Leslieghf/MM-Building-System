//! Versioned modding API from guarded game access through ECS and Rhai.
//!
//! # Modules
//!
//! - [`game`] contains the supported executable's raw ABI and guarded adapter.
//!   It is an implementation boundary, not a supported extension surface.
//! - [`ecs`] defines owned frame data, mod-system schedules, and bounded effects.
//! - [`rhai`] defines the versioned script context and generated API catalog.
//! - [`scripts`] owns package validation, compilation, lifecycle, and console transport.
//! - [`mod@format`] shares display formatting without depending on UI code.
//!
//! # Safety boundary
//!
//! Native pointers stay inside [`game`]. Callers exchange copied snapshots and
//! typed requests; a successful Rust build does not establish runtime safety
//! for a particular game executable.
pub mod ecs;
pub mod format;
pub mod game;
pub mod rhai;
/// Script lifecycle and event contracts exposed through the modding API.
pub mod scripts;
