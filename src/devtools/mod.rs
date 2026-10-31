//! Developer-only presentation, local UI state, and input translation.
//!
//! # Ownership
//!
//! [`state`] owns UI state; [`presentation`] projects copied runtime data and
//! transfers queued requests; [`overlay`] renders that presentation through
//! DX11/ImGui; [`input`] translates the overlay backend into the stable input
//! snapshot. Native game pointers and script execution remain outside this
//! layer.
pub(crate) mod input;
pub(crate) mod overlay;
pub(crate) mod presentation;
pub(crate) mod state;
