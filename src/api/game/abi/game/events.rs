//! Event declaration markers from `game/events.h`.
//!
//! The SDK header declares handler shapes but does not provide an external
//! subscribe/unsubscribe lifecycle. This module therefore exposes no event
//! registration or dispatch functions.

/// Opaque SDK event implementation.
pub enum EventImpl {}

/// Opaque SDK event handler base.
pub enum EventHandler {}
