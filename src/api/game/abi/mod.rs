//! Original Rust declarations derived from the pinned Mad Max SDK revision.
//!
//! Source: <https://github.com/gigaHours/mm_sdk/tree/f2dc640769c88aae9c3965a2aca0aa138f0e62ab>.
//! Explicit `unsafe extern` aliases and absolute addresses are candidate
//! metadata; only individually validated adapters should invoke them.
//!
//! # Scope
//!
//! These declarations describe selected C++ layouts and calling conventions;
//! they do not provide safe game-object ownership or prove that a candidate
//! address is valid for the running executable. Supported extensions should
//! use [`crate::api::ecs`] instead.
//!
//! # Layout documentation
//!
//! Field names mirror the pinned SDK or observed offsets. Their exact layout is
//! more important than synthetic one-line field descriptions, and uncertain
//! observations are intentionally documented at the containing type or module.
#![allow(missing_docs)]

pub mod build;
pub mod core;
pub mod game;
pub mod math;
pub mod types;
