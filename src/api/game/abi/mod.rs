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
//!
//! # Math declarations
//!
//! The pinned SDK's `math3d.h` declares helpers for matrix rotation and Euler
//! conversion, quaternion conversion, and vector transformation. Their Rust
//! call shapes are preserved below as a source reference only. No executable
//! addresses are mapped for these helpers, and the adapter does not call them.
//! Use the owned value operations in [`types`] or the script-facing helpers in
//! [`crate::api::rhai`] instead.
//!
//! ```text
//! type MatrixRotationYawPitchRollFn = unsafe extern "system" fn(
//!     *mut Matrix4x4, f32, f32, f32,
//! ) -> *mut Matrix4x4;
//! type MatrixToEulerXyzFn = unsafe extern "system" fn(*mut Matrix4x4) -> *mut Vector3;
//! type MatrixFromEulerXyzFn = unsafe extern "system" fn(
//!     *mut Matrix4x4, *const Vector3,
//! ) -> *mut Matrix4x4;
//! type MatrixToEulerFn = unsafe extern "system" fn(
//!     *mut Matrix4x4, *mut f32, *mut f32, *mut f32,
//! );
//! type QuaternionFromMatrixFn = unsafe extern "system" fn(
//!     *mut Quaternion, *const Matrix4x4,
//! ) -> *mut Quaternion;
//! type QuaternionToMatrixFn = unsafe extern "system" fn(
//!     *mut Quaternion, *mut Matrix4x4,
//! ) -> *mut Matrix4x4;
//! type QuaternionFromEulerXyzFn = unsafe extern "system" fn(
//!     *mut Quaternion, *const Vector3,
//! ) -> *mut Quaternion;
//! type QuaternionFromEulerFn = unsafe extern "system" fn(
//!     *mut Quaternion, f32, f32, f32,
//! ) -> *mut Quaternion;
//! type MatrixTransformVector4Fn = unsafe extern "system" fn(
//!     *mut Matrix4x4, *mut Vector4, *const Vector4,
//! ) -> *mut Vector4;
//! type QuaternionTransformVector3Fn = unsafe extern "system" fn(
//!     *mut Quaternion, *mut Vector3, *const Vector3,
//! ) -> *mut Vector3;
//! ```
//!
//! These declarations are not a Rust API: their ownership, angle units,
//! coordinate convention, and reference-return ABI have not been validated
//! against the supported executable. Source: [`pinned SDK revision`].
//!
//! [`pinned SDK revision`]: https://github.com/gigaHours/mm_sdk/tree/f2dc640769c88aae9c3965a2aca0aa138f0e62ab
#![allow(missing_docs)]

pub mod build;
pub mod core;
pub mod game;
pub mod types;
