//! Native game integration: memory observation, hooks, and diagnostic output.
//! Raw pointers and build-specific addresses stay behind this boundary.
pub(crate) mod game_probe;
pub(crate) mod logging;
