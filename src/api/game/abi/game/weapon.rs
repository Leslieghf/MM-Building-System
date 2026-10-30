//! Weapon-facing ray query recovered from Mad Max's world-query call chain.
//!
//! The method and ABI are mapped from a live code snapshot and passive
//! observation of the game calling it. Its native object lifetime, safe call
//! phase, and hit/miss contract are not exposed as a safe API.

use crate::api::game::abi::types::Vector3;

/// Opaque weapon receiver for the recovered ray-position method.
#[repr(C)]
pub struct CWeapon {
    _opaque: [u8; 0],
}

/// Six-float input block observed by CWeapon::GetRayWorldPosition.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct WeaponRay {
    /// Ray origin in world coordinates.
    pub origin: Vector3,
    /// Ray direction; the game applies a weapon-owned distance scale.
    pub direction: Vector3,
}

/// RVA of CWeapon::GetRayWorldPosition in the inspected Steam executable.
pub const C_WEAPON_GET_RAY_WORLD_POSITION_RVA: usize = 0x01E1_BBF0;

/// Recovered ABI of CWeapon::GetRayWorldPosition.
///
/// The function writes the full-range endpoint first, performs the game's
/// shared world query, and writes a nearer collision point when the query
/// provides a fraction below its 1.0 fallback. The return value is the
/// output pointer in observed calls, not a hit/miss flag. Do not call this
/// until the receiver lifetime and game-thread/phase requirements are known.
pub type CWeaponGetRayWorldPositionFn = unsafe extern "system" fn(
    weapon: *mut CWeapon,
    output_position: *mut Vector3,
    ray: *const WeaponRay,
) -> *mut Vector3;
