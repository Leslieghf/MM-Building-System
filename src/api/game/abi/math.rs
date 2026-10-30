//! Candidate signatures for the game's native math helpers in `math3d.h`.
//!
//! The pure scalar operations live in [`types`](super::types). These aliases
//! only describe native call shapes; several return C++ references and require
//! independent ABI checks before any address is called.

use crate::api::game::abi::types::{Matrix4x4, Quaternion, Vector3, Vector4};

pub type MatrixRotationYawPitchRollFn =
    unsafe extern "system" fn(*mut Matrix4x4, f32, f32, f32) -> *mut Matrix4x4;
pub type MatrixToEulerXyzFn = unsafe extern "system" fn(*mut Matrix4x4) -> *mut Vector3;
pub type MatrixFromEulerXyzFn =
    unsafe extern "system" fn(*mut Matrix4x4, *const Vector3) -> *mut Matrix4x4;
pub type MatrixToEulerFn = unsafe extern "system" fn(*mut Matrix4x4, *mut f32, *mut f32, *mut f32);
pub type QuaternionFromMatrixFn =
    unsafe extern "system" fn(*mut Quaternion, *const Matrix4x4) -> *mut Quaternion;
pub type QuaternionToMatrixFn =
    unsafe extern "system" fn(*mut Quaternion, *mut Matrix4x4) -> *mut Matrix4x4;
pub type QuaternionFromEulerXyzFn =
    unsafe extern "system" fn(*mut Quaternion, *const Vector3) -> *mut Quaternion;
pub type QuaternionFromEulerFn =
    unsafe extern "system" fn(*mut Quaternion, f32, f32, f32) -> *mut Quaternion;
pub type MatrixTransformVector4Fn =
    unsafe extern "system" fn(*mut Matrix4x4, *mut Vector4, *const Vector4) -> *mut Vector4;
pub type QuaternionTransformVector3Fn =
    unsafe extern "system" fn(*mut Quaternion, *mut Vector3, *const Vector3) -> *mut Vector3;
