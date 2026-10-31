//! Opaque game types, data layouts, and candidate calls from the pinned SDK.
//!
//! Each child module groups declarations by the engine subsystem that owns
//! them. Header-derived signatures and class names do not establish object
//! lifetime, exact executable layout, or vtable slot; only validated adapter
//! paths may make a native call.

pub mod camera;
pub mod character;
pub mod events;
pub mod hierarchy;
pub mod methods;
pub mod object;
pub mod physics;
pub mod spawn;
pub mod state;
pub mod vehicle;
pub mod weapon;
pub mod world_query;
