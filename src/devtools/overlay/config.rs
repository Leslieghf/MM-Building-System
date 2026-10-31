//! Overlay behavior settings and platform clipboard bounds.
//!
//! Values here constrain UI-only behavior such as input capture, history, and
//! clipboard text size. They do not configure the game adapter or script
//! capabilities.
pub(super) const ENABLE_RUNTIME_INPUT_SAMPLING: bool = false;
pub(super) const CF_UNICODETEXT: u32 = 13;
pub(super) const GMEM_MOVEABLE: u32 = 0x0002;
pub(super) const MAX_CLIPBOARD_UTF16_UNITS: usize = 262_144;
