//! Build-specific boundary between Mad Max and the owned modding runtime.
//!
//! # Modules
//!
//! [`abi`] records candidate layouts and call shapes for one pinned executable
//! build. `native` validates the build, guards native access, samples values,
//! and applies bounded effects. The ABI declarations are not a safe extension
//! API; prefer [`crate::api::ecs`] for Rust systems and [`crate::api::rhai`] for
//! scripts.
//!
//! # Safety
//!
//! Build fingerprints and readable-memory checks reduce accidental misuse but
//! do not prove a reverse-engineered call or layout correct. Native pointers
//! must not escape the native-access boundary as persistent handles.
pub mod abi;
pub(crate) mod native;
