//! Camera-manager call signature.
//!
//! Source: `game/cameramanager.h` at
//! [`SDK_REVISION`](super::super::build::SDK_REVISION). The output buffer is
//! caller-owned, while the manager pointer remains engine-owned and opaque.

use crate::api::game::abi::types::Matrix4x4;
/// Opaque `CCameraControlManager` instance.
pub enum CameraControlManager {}

/// `CCameraControlManager::GetCameraMatrix(CMatrix4f&)`.
pub type GetCameraMatrixFn = unsafe extern "system" fn(*mut CameraControlManager, *mut Matrix4x4);
