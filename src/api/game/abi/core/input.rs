//! Candidate input action-map declarations from the pinned `core/input.h`.
//!
//! The module records opaque manager types, observed field offsets, and SDK
//! call shapes. The current mod-owned input snapshot comes from ImGui; these
//! declarations do not poll the game's controller state or authorize native
//! input calls.
/// Opaque `Input::CInputActionMap`.
pub enum InputActionMap {}

/// Opaque `Input::IInputDeviceManager` instance.
pub enum InputDeviceManager {}

/// SDK-declared byte offset of `CDeviceManager_STEAM::m_InputManager`.
pub const STEAM_DEVICE_MANAGER_INPUT_MANAGER_OFFSET: usize = 0xA0;

/// SDK-declared byte offset of `CInputDeviceManager::m_IsInFocus` within its
/// base-plus-derived object layout.
pub const INPUT_DEVICE_MANAGER_IS_IN_FOCUS_OFFSET: usize = 0x80;
