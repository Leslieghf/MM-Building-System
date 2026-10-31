//! Camera-matrix sampling from the supported game build.
//!
//! The camera matrix is copied only after the camera-manager singleton and
//! method target pass build-specific validation. The copied matrix is telemetry
//! input; the game-owned manager and all temporary pointers remain local to the
//! adapter call.
use super::*;

pub(in crate::api::game::native) fn sample_camera_matrix() -> Option<Matrix4x4> {
    if !readable_span(CAMERA_MANAGER_SINGLETON, size_of::<*mut c_void>())
        || !is_readable_executable_page(GET_CAMERA_MATRIX)
    {
        return None;
    }

    let manager = unsafe { ptr::read_volatile(CAMERA_MANAGER_SINGLETON as *const *mut c_void) };
    if manager.is_null() || !readable_span(manager as usize, 1) {
        return None;
    }

    let get_camera_matrix: GetCameraMatrix = unsafe { std::mem::transmute(GET_CAMERA_MATRIX) };
    let mut camera_matrix = Matrix4x4 {
        values: [[0.0; 4]; 4],
    };
    unsafe { get_camera_matrix(manager.cast::<CameraControlManager>(), &mut camera_matrix) };
    camera_matrix
        .values
        .iter()
        .flatten()
        .all(|value| value.is_finite())
        .then_some(camera_matrix)
}
