//! Candidate input action-map ABI declarations from `core/input.h`.
use crate::api::game::abi::types::HashId;

/// Opaque `Input::CInputActionMap`.
pub enum InputActionMap {}

/// Opaque `Input::IInputDeviceManager` instance.
pub enum InputDeviceManager {}

/// SDK-declared byte offset of `CDeviceManager_STEAM::m_InputManager`.
pub const STEAM_DEVICE_MANAGER_INPUT_MANAGER_OFFSET: usize = 0xA0;

/// SDK-declared byte offset of `CInputDeviceManager::m_IsInFocus` within its
/// base-plus-derived object layout.
pub const INPUT_DEVICE_MANAGER_IS_IN_FOCUS_OFFSET: usize = 0x80;

/// `CInputActionMap::GetValue(const CHashString&)` candidate signature.
pub type GetActionValueFn = unsafe extern "system" fn(*mut InputActionMap, u32) -> f32;

/// `IInputDeviceManager::GetActionMap(const CHashString&)` candidate signature.
pub type GetActionMapFn =
    unsafe extern "system" fn(*mut InputDeviceManager, *const HashId) -> *mut InputActionMap;
