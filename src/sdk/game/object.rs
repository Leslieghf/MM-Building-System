//! CGameObject virtual method call signatures.
//!
//! Source: `game/go/go.h` at
//! [`SDK_REVISION`](super::super::build::SDK_REVISION). The game object itself
//! remains opaque because its C++ base class and Boost-owned fields are not a
//! safe Rust data structure.

use crate::sdk::types::Matrix4x4;
/// Opaque `CGameObject`; its SDK layout contains engine and Boost-owned state.
pub enum GameObject {}

/// Vtable slot candidate currently used by the game adapter for `GetTransform`.
/// The SDK header gives declaration order, not a numeric slot; this is not
/// independently validated against the supported executable.
pub const GET_TRANSFORM_SLOT_CANDIDATE: usize = 22;

/// Pointer-free `SObjectIDData` / `SObjectID` value.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectId {
    pub hash: u32,
    pub user_data: u32,
}

/// `CGameObject::GetNameHash()` declaration.
pub type GetNameHashFn = unsafe extern "system" fn(*mut GameObject) -> u32;

/// `CGameObject::GetObjectID()` declaration returning an eight-byte value.
pub type GetObjectIdFn = unsafe extern "system" fn(*mut GameObject) -> ObjectId;

#[cfg(test)]
mod tests {
    use super::ObjectId;

    #[test]
    fn object_id_matches_two_u32_fields() {
        assert_eq!(std::mem::size_of::<ObjectId>(), 8);
        assert_eq!(std::mem::align_of::<ObjectId>(), 4);
    }
}

/// `CGameObject::GetTransform(CMatrix*)` using a caller-owned matrix buffer.
pub type GetTransformFn = unsafe extern "system" fn(*mut GameObject, *mut Matrix4x4) -> bool;

/// `CGameObject::SetTransform(CMatrix*)` using a caller-owned matrix buffer.
pub type SetTransformFn = unsafe extern "system" fn(*mut GameObject, *mut Matrix4x4);

/// `CGameObject::InitTransform(const CMatrix&)` using a borrowed matrix.
pub type InitTransformFn = unsafe extern "system" fn(*mut GameObject, *const Matrix4x4);
