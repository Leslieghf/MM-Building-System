//! Character-manager and character-method call signatures.
//!
//! Source: `game/charactermanager.h` and `game/go/character.h` at
//! [`SDK_REVISION`](super::super::build::SDK_REVISION). These aliases describe
//! ABI shapes only; they do not certify an address or make an unchecked call
//! safe.

use super::vehicle::Vehicle;

/// Opaque `CCharacterManager` instance.
pub enum CharacterManager {}

/// Opaque `CCharacter` instance.
pub enum Character {}

/// `CCharacterManager::GetPlayerCharacter()` with its explicit C++ `this`.
pub type GetPlayerCharacterFn = unsafe extern "system" fn(*mut CharacterManager) -> *mut Character;

/// `CCharacter::GetVehiclePtr()` with its explicit C++ `this`.
pub type GetCharacterVehicleFn = unsafe extern "system" fn(*mut Character) -> *mut Vehicle;

/// `CCharacter::ForceNeutralState()` with its explicit C++ `this`.
pub type ForceNeutralStateFn = unsafe extern "system" fn(*mut Character);

/// `CCharacterBase::RotateInstantly(float)` with its explicit C++ `this`.
pub type RotateInstantlyFn = unsafe extern "system" fn(*mut Character, f32);
