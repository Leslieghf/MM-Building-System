//! Hashing function signature from `core/hash.h`.

use core::ffi::c_char;

/// Candidate signature of the game's `HashString(const char*)` function.
/// Address and text lifetime still require build-specific validation.
pub type HashStringFn = unsafe extern "system" fn(*const c_char) -> u32;
