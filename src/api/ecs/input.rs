//! Owned input frames consumed by Rust systems and script callbacks.

use bevy_ecs::prelude::Resource;
use std::sync::{Arc, Mutex};

/// Stable key IDs index this bounded keyboard/mouse state; no game input is polled.
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
    /// Whether a supported key is held in this frame.
    pub fn is_down(self, key: u32) -> bool {
        bit(self.down, key)
    }
    /// Whether the key changed from released to pressed since the last consume.
    pub fn just_pressed(self, key: u32) -> bool {
        bit(self.just_pressed, key)
    }
    /// Whether the key changed from pressed to released since the last consume.
    pub fn just_released(self, key: u32) -> bool {
        bit(self.just_released, key)
    }

    /// Clear transition edges while preserving held-key state.
    pub fn clear_edges(&mut self) {
        self.just_pressed = [0; INPUT_WORDS];
        self.just_released = [0; INPUT_WORDS];
    }

    /// Apply one key state; unsupported key IDs are ignored.
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

    /// Replace held keys and accumulate transitions from a sampled frame.
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
    /// Publish a sampled frame without waiting for the consumer.
    ///
    /// Returns `false` if another side currently holds the mailbox lock.
    pub fn publish(&self, sample: InputFrame) -> bool {
        let Ok(mut pending) = self.0.try_lock() else {
            return false;
        };
        pending.accumulate_sample(sample);
        true
    }
    /// Take the accumulated frame without waiting; transition edges are cleared.
    ///
    /// Returns `None` if another side currently holds the mailbox lock.
    pub fn take(&self) -> Option<InputFrame> {
        let Ok(mut pending) = self.0.try_lock() else {
            return None;
        };
        let frame = *pending;
        pending.clear_edges();
        Some(frame)
    }
}
