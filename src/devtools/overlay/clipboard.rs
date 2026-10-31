//! Win32 clipboard adapter used by ImGui text fields.
//!
//! Text is copied through the Windows clipboard API with explicit UTF-16
//! conversion and a maximum byte/character bound. The adapter is only used by
//! the overlay's clipboard callbacks and does not expose process or file
//! access to scripts.
use super::config::{CF_UNICODETEXT, GMEM_MOVEABLE, MAX_CLIPBOARD_UTF16_UNITS};
use hudhook::imgui;

#[link(name = "user32")]
unsafe extern "system" {
    fn OpenClipboard(owner: *mut std::ffi::c_void) -> i32;
    fn CloseClipboard() -> i32;
    fn EmptyClipboard() -> i32;
    fn GetClipboardData(format: u32) -> *mut std::ffi::c_void;
    fn SetClipboardData(format: u32, memory: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GlobalAlloc(flags: u32, bytes: usize) -> *mut std::ffi::c_void;
    fn GlobalFree(memory: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    fn GlobalLock(memory: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    fn GlobalUnlock(memory: *mut std::ffi::c_void) -> i32;
    fn GlobalSize(memory: *mut std::ffi::c_void) -> usize;
}

pub(super) struct Win32ClipboardBackend;

impl imgui::ClipboardBackend for Win32ClipboardBackend {
    fn get(&mut self) -> Option<String> {
        unsafe {
            if OpenClipboard(std::ptr::null_mut()) == 0 {
                return None;
            }
            let memory = GetClipboardData(CF_UNICODETEXT);
            if memory.is_null() {
                CloseClipboard();
                return None;
            }
            let bytes = GlobalSize(memory);
            let locked = GlobalLock(memory);
            if locked.is_null() || bytes < std::mem::size_of::<u16>() {
                if !locked.is_null() {
                    GlobalUnlock(memory);
                }
                CloseClipboard();
                return None;
            }
            let unit_count = (bytes / std::mem::size_of::<u16>()).min(MAX_CLIPBOARD_UTF16_UNITS);
            let units = std::slice::from_raw_parts(locked.cast::<u16>(), unit_count);
            let length = units
                .iter()
                .position(|unit| *unit == 0)
                .unwrap_or(unit_count);
            let text = String::from_utf16_lossy(&units[..length]);
            GlobalUnlock(memory);
            CloseClipboard();
            Some(text)
        }
    }

    fn set(&mut self, value: &str) {
        let mut wide: Vec<u16> = value
            .encode_utf16()
            .take(MAX_CLIPBOARD_UTF16_UNITS - 1)
            .collect();
        wide.push(0);
        unsafe {
            if OpenClipboard(std::ptr::null_mut()) == 0 {
                return;
            }
            if EmptyClipboard() == 0 {
                CloseClipboard();
                return;
            }
            let memory = GlobalAlloc(
                GMEM_MOVEABLE,
                wide.len().saturating_mul(std::mem::size_of::<u16>()),
            );
            if memory.is_null() {
                CloseClipboard();
                return;
            }
            let locked = GlobalLock(memory);
            if locked.is_null() {
                GlobalFree(memory);
                CloseClipboard();
                return;
            }
            std::ptr::copy_nonoverlapping(wide.as_ptr(), locked.cast::<u16>(), wide.len());
            GlobalUnlock(memory);
            if SetClipboardData(CF_UNICODETEXT, memory).is_null() {
                GlobalFree(memory);
            }
            CloseClipboard();
        }
    }
}
