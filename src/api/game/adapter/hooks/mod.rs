//! Native hook lifecycle and callback entry points.
//!
//! `sampling` owns passive telemetry taps; `world_query` owns experimental
//! query tracing. Both delegate copied observations to the sampling layer and
//! keep native callback work bounded. Neither module is a public extension
//! surface.
mod sampling;
mod world_query;
pub(super) use sampling::*;
pub(super) use world_query::*;
