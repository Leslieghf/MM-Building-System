//! Core input and hashing declarations from the pinned community SDK.
//!
//! These declarations preserve source types and offsets used during adapter
//! work. The hash helper is not used to manufacture IDs in Rust, and input
//! manager pointers remain game-owned; neither child module is a safe runtime
//! API.

pub mod hash;
pub mod input;
