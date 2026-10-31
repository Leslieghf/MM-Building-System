//! Process-level composition for the in-process ASI library.
//!
//! [`entry`] owns the loader callback, startup worker, and endpoint wiring.
//! [`diagnostics`] provides best-effort logging that does not depend on the
//! game callback or the developer overlay.
//!
//! # Lifecycle
//!
//! Startup creates the game/UI channels, gives each endpoint to its owner, and
//! installs the DX11 hook on a worker thread. The game callback is enabled
//! only after the overlay hook reports successful installation.
pub(crate) mod diagnostics;
pub(crate) mod entry;
