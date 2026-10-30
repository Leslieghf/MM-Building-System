//! Owned mod input frames and the stable named-key mapping used by scripts.

use bevy_ecs::prelude::Resource;
use std::sync::{Arc, Mutex};

/// ImGui named-key IDs occupy a fixed range below 1024. The runtime stores
/// only this bounded keyboard/mouse state; it does not poll Mad Max input.
const INPUT_WORDS: usize = 16;

/// Mod-owned input state sampled by Hudhook/ImGui and copied into the game ECS
/// at callback boundaries. `just_*` transitions accumulate until consumed.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InputFrame {
    down: [u64; INPUT_WORDS],
    just_pressed: [u64; INPUT_WORDS],
    just_released: [u64; INPUT_WORDS],
}

impl InputFrame {
    pub fn is_down(self, key: u32) -> bool {
        bit(self.down, key)
    }
    pub fn just_pressed(self, key: u32) -> bool {
        bit(self.just_pressed, key)
    }
    pub fn just_released(self, key: u32) -> bool {
        bit(self.just_released, key)
    }

    pub fn clear_edges(&mut self) {
        self.just_pressed = [0; INPUT_WORDS];
        self.just_released = [0; INPUT_WORDS];
    }

    pub fn set_down(&mut self, key: u32, is_down: bool) {
        if key as usize >= INPUT_WORDS * 64 {
            return;
        }
        let was_down = bit(self.down, key);
        set_bit(&mut self.down, key, is_down);
        if is_down && !was_down {
            set_bit(&mut self.just_pressed, key, true);
        }
        if !is_down && was_down {
            set_bit(&mut self.just_released, key, true);
        }
    }

    pub fn accumulate_sample(&mut self, sample: Self) {
        self.down = sample.down;
        for i in 0..INPUT_WORDS {
            self.just_pressed[i] |= sample.just_pressed[i];
            self.just_released[i] |= sample.just_released[i];
        }
    }
}

fn bit(bits: [u64; INPUT_WORDS], key: u32) -> bool {
    let key = key as usize;
    key < INPUT_WORDS * 64 && bits[key / 64] & (1 << (key % 64)) != 0
}
fn set_bit(bits: &mut [u64; INPUT_WORDS], key: u32, value: bool) {
    let key = key as usize;
    if key >= INPUT_WORDS * 64 {
        return;
    }
    let mask = 1 << (key % 64);
    if value {
        bits[key / 64] |= mask;
    } else {
        bits[key / 64] &= !mask;
    }
}

/// Shared latest-sample mailbox; both sides use `try_lock`, so neither the
/// render callback nor Mad Max callback waits for the other runtime.
#[derive(Clone, Default)]
pub struct InputFrameMailbox(Arc<Mutex<InputFrame>>);
impl InputFrameMailbox {
    pub fn publish(&self, sample: InputFrame) -> bool {
        let Ok(mut pending) = self.0.try_lock() else {
            return false;
        };
        pending.accumulate_sample(sample);
        true
    }
    pub fn take(&self) -> Option<InputFrame> {
        let Ok(mut pending) = self.0.try_lock() else {
            return None;
        };
        let frame = *pending;
        pending.clear_edges();
        Some(frame)
    }
}

/// Parse the stable console spelling into the named-key ID used by ImGui.
pub fn parse_key(name: &str) -> Option<hudhook::imgui::Key> {
    let normalized = name.to_ascii_lowercase();
    Some(match normalized.as_str() {
        "tab" => hudhook::imgui::Key::Tab,
        "left" => hudhook::imgui::Key::LeftArrow,
        "right" => hudhook::imgui::Key::RightArrow,
        "up" => hudhook::imgui::Key::UpArrow,
        "down" => hudhook::imgui::Key::DownArrow,
        "pageup" => hudhook::imgui::Key::PageUp,
        "pagedown" => hudhook::imgui::Key::PageDown,
        "home" => hudhook::imgui::Key::Home,
        "end" => hudhook::imgui::Key::End,
        "insert" => hudhook::imgui::Key::Insert,
        "delete" => hudhook::imgui::Key::Delete,
        "backspace" => hudhook::imgui::Key::Backspace,
        "space" => hudhook::imgui::Key::Space,
        "enter" => hudhook::imgui::Key::Enter,
        "escape" => hudhook::imgui::Key::Escape,
        "lctrl" => hudhook::imgui::Key::LeftCtrl,
        "lshift" => hudhook::imgui::Key::LeftShift,
        "lalt" => hudhook::imgui::Key::LeftAlt,
        "rctrl" => hudhook::imgui::Key::RightCtrl,
        "rshift" => hudhook::imgui::Key::RightShift,
        "ralt" => hudhook::imgui::Key::RightAlt,
        "lsuper" => hudhook::imgui::Key::LeftSuper,
        "rsuper" => hudhook::imgui::Key::RightSuper,
        "menu" => hudhook::imgui::Key::Menu,
        "0" => hudhook::imgui::Key::Alpha0,
        "1" => hudhook::imgui::Key::Alpha1,
        "2" => hudhook::imgui::Key::Alpha2,
        "3" => hudhook::imgui::Key::Alpha3,
        "4" => hudhook::imgui::Key::Alpha4,
        "5" => hudhook::imgui::Key::Alpha5,
        "6" => hudhook::imgui::Key::Alpha6,
        "7" => hudhook::imgui::Key::Alpha7,
        "8" => hudhook::imgui::Key::Alpha8,
        "9" => hudhook::imgui::Key::Alpha9,
        "a" => hudhook::imgui::Key::A,
        "b" => hudhook::imgui::Key::B,
        "c" => hudhook::imgui::Key::C,
        "d" => hudhook::imgui::Key::D,
        "e" => hudhook::imgui::Key::E,
        "f" => hudhook::imgui::Key::F,
        "g" => hudhook::imgui::Key::G,
        "h" => hudhook::imgui::Key::H,
        "i" => hudhook::imgui::Key::I,
        "j" => hudhook::imgui::Key::J,
        "k" => hudhook::imgui::Key::K,
        "l" => hudhook::imgui::Key::L,
        "m" => hudhook::imgui::Key::M,
        "n" => hudhook::imgui::Key::N,
        "o" => hudhook::imgui::Key::O,
        "p" => hudhook::imgui::Key::P,
        "q" => hudhook::imgui::Key::Q,
        "r" => hudhook::imgui::Key::R,
        "s" => hudhook::imgui::Key::S,
        "t" => hudhook::imgui::Key::T,
        "u" => hudhook::imgui::Key::U,
        "v" => hudhook::imgui::Key::V,
        "w" => hudhook::imgui::Key::W,
        "x" => hudhook::imgui::Key::X,
        "y" => hudhook::imgui::Key::Y,
        "z" => hudhook::imgui::Key::Z,
        "f1" => hudhook::imgui::Key::F1,
        "f2" => hudhook::imgui::Key::F2,
        "f3" => hudhook::imgui::Key::F3,
        "f4" => hudhook::imgui::Key::F4,
        "f5" => hudhook::imgui::Key::F5,
        "f6" => hudhook::imgui::Key::F6,
        "f7" => hudhook::imgui::Key::F7,
        "f8" => hudhook::imgui::Key::F8,
        "f9" => hudhook::imgui::Key::F9,
        "f10" => hudhook::imgui::Key::F10,
        "f11" => hudhook::imgui::Key::F11,
        "f12" => hudhook::imgui::Key::F12,
        "mouse1" => hudhook::imgui::Key::MouseLeft,
        "mouse2" => hudhook::imgui::Key::MouseRight,
        "mouse3" => hudhook::imgui::Key::MouseMiddle,
        "mouse4" => hudhook::imgui::Key::MouseX1,
        "mouse5" => hudhook::imgui::Key::MouseX2,
        _ => return None,
    })
}

pub fn key_name(key: hudhook::imgui::Key) -> Option<&'static str> {
    Some(match key {
        hudhook::imgui::Key::Tab => "tab",
        hudhook::imgui::Key::LeftArrow => "left",
        hudhook::imgui::Key::RightArrow => "right",
        hudhook::imgui::Key::UpArrow => "up",
        hudhook::imgui::Key::DownArrow => "down",
        hudhook::imgui::Key::PageUp => "pageup",
        hudhook::imgui::Key::PageDown => "pagedown",
        hudhook::imgui::Key::Home => "home",
        hudhook::imgui::Key::End => "end",
        hudhook::imgui::Key::Insert => "insert",
        hudhook::imgui::Key::Delete => "delete",
        hudhook::imgui::Key::Backspace => "backspace",
        hudhook::imgui::Key::Space => "space",
        hudhook::imgui::Key::Enter => "enter",
        hudhook::imgui::Key::Escape => "escape",
        hudhook::imgui::Key::LeftCtrl => "lctrl",
        hudhook::imgui::Key::LeftShift => "lshift",
        hudhook::imgui::Key::LeftAlt => "lalt",
        hudhook::imgui::Key::RightCtrl => "rctrl",
        hudhook::imgui::Key::RightShift => "rshift",
        hudhook::imgui::Key::RightAlt => "ralt",
        hudhook::imgui::Key::LeftSuper => "lsuper",
        hudhook::imgui::Key::RightSuper => "rsuper",
        hudhook::imgui::Key::Menu => "menu",
        hudhook::imgui::Key::Alpha0 => "0",
        hudhook::imgui::Key::Alpha1 => "1",
        hudhook::imgui::Key::Alpha2 => "2",
        hudhook::imgui::Key::Alpha3 => "3",
        hudhook::imgui::Key::Alpha4 => "4",
        hudhook::imgui::Key::Alpha5 => "5",
        hudhook::imgui::Key::Alpha6 => "6",
        hudhook::imgui::Key::Alpha7 => "7",
        hudhook::imgui::Key::Alpha8 => "8",
        hudhook::imgui::Key::Alpha9 => "9",
        hudhook::imgui::Key::A => "a",
        hudhook::imgui::Key::B => "b",
        hudhook::imgui::Key::C => "c",
        hudhook::imgui::Key::D => "d",
        hudhook::imgui::Key::E => "e",
        hudhook::imgui::Key::F => "f",
        hudhook::imgui::Key::G => "g",
        hudhook::imgui::Key::H => "h",
        hudhook::imgui::Key::I => "i",
        hudhook::imgui::Key::J => "j",
        hudhook::imgui::Key::K => "k",
        hudhook::imgui::Key::L => "l",
        hudhook::imgui::Key::M => "m",
        hudhook::imgui::Key::N => "n",
        hudhook::imgui::Key::O => "o",
        hudhook::imgui::Key::P => "p",
        hudhook::imgui::Key::Q => "q",
        hudhook::imgui::Key::R => "r",
        hudhook::imgui::Key::S => "s",
        hudhook::imgui::Key::T => "t",
        hudhook::imgui::Key::U => "u",
        hudhook::imgui::Key::V => "v",
        hudhook::imgui::Key::W => "w",
        hudhook::imgui::Key::X => "x",
        hudhook::imgui::Key::Y => "y",
        hudhook::imgui::Key::Z => "z",
        hudhook::imgui::Key::F1 => "f1",
        hudhook::imgui::Key::F2 => "f2",
        hudhook::imgui::Key::F3 => "f3",
        hudhook::imgui::Key::F4 => "f4",
        hudhook::imgui::Key::F5 => "f5",
        hudhook::imgui::Key::F6 => "f6",
        hudhook::imgui::Key::F7 => "f7",
        hudhook::imgui::Key::F8 => "f8",
        hudhook::imgui::Key::F9 => "f9",
        hudhook::imgui::Key::F10 => "f10",
        hudhook::imgui::Key::F11 => "f11",
        hudhook::imgui::Key::F12 => "f12",
        hudhook::imgui::Key::MouseLeft => "mouse1",
        hudhook::imgui::Key::MouseRight => "mouse2",
        hudhook::imgui::Key::MouseMiddle => "mouse3",
        hudhook::imgui::Key::MouseX1 => "mouse4",
        hudhook::imgui::Key::MouseX2 => "mouse5",
        _ => return None,
    })
}
