//! CVehicle virtual method call signatures and candidate vtable indices.
//!
//! Signatures come from `game/go/vehicle.h` at
//! [`SDK_REVISION`](super::super::build::SDK_REVISION). Slot values mirror the
//! current bridge's SDK-derived candidates; the SDK header declares methods but
//! does not publish numeric vtable indices. These values are not independently
//! verified against the installed executable.

/// Opaque `CVehicle` instance.
pub enum Vehicle {}

/// Candidate index for `CVehicle::SetVelocity` used by the current bridge.
pub const SET_VELOCITY_SLOT_CANDIDATE: usize = 38;
/// Candidate index for `CVehicle::GetAngularVelocity` used by the current bridge.
pub const GET_ANGULAR_VELOCITY_SLOT_CANDIDATE: usize = 57;
/// Candidate index for `CVehicle::GetVelocity` used by the current bridge.
pub const GET_VELOCITY_SLOT_CANDIDATE: usize = 58;
/// Candidate index for `CVehicle::GetVelocityLastFrame` used by the current bridge.
pub const GET_VELOCITY_LAST_FRAME_SLOT_CANDIDATE: usize = 59;
/// Candidate index for `CVehicle::GetAcceleration` used by the current bridge.
pub const GET_ACCELERATION_SLOT_CANDIDATE: usize = 60;

/// `CVehicle::GetVelocity(CVector3f*)` or another output-vector getter.
pub type GetVectorFn = unsafe extern "system" fn(*mut Vehicle, *mut [f32; 3]);

/// `CVehicle::GetVelocityLastFrame()` returning an engine-owned vector pointer.
pub type GetVectorPointerFn = unsafe extern "system" fn(*mut Vehicle) -> *const [f32; 3];

/// `CVehicle::SetVelocity(const CVector3f&)` with a borrowed three-float value.
pub type SetVectorFn = unsafe extern "system" fn(*mut Vehicle, *const [f32; 3]);
