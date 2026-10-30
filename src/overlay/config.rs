//! Overlay behavior and Win32 clipboard limits.
pub(super) const ENABLE_RUNTIME_INPUT_SAMPLING: bool = false;
pub(super) const CF_UNICODETEXT: u32 = 13;
pub(super) const GMEM_MOVEABLE: u32 = 0x0002;
pub(super) const MAX_CLIPBOARD_UTF16_UNITS: usize = 262_144;
