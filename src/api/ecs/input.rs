//! Owned mod-input frames consumed by game systems and script callbacks.
//!
//! Input is sampled from the overlay backend, not from Mad Max's controller
//! system. Stable [`InputKey`](super::input_key::InputKey) IDs index the
//! keyboard/mouse state. Transition bits accumulate until a game callback
//! consumes or clears them, so missed callbacks do not lose a key edge.

use bevy_ecs::prelude::Resource;
use std::sync::{Arc, Mutex};

/// Stable key IDs index this bounded keyboard/mouse state; no game input is polled.
const INPUT_WORDS: usize = 16;

/// Mod-owned input state sampled by Hudhook/ImGui and copied into the game ECS
/// at callback boundaries. It does not poll Mad Max's input system or controller
/// state. `just_*` transitions accumulate until consumed or cleared.
///
/// Key arguments are stable IDs from
/// [`InputKey`](crate::api::ecs::input_key::InputKey).
/// Unsupported IDs read
/// as released and are ignored by [`InputFrame::set_down`].
///
/// # Edge lifetime
///
/// [`InputFrameMailbox::publish`] merges transitions from sampled UI frames;
/// [`InputFrameMailbox::take`] returns the accumulated state and clears those
/// transitions. Call [`InputFrame::clear_edges`] when advancing a frame without
/// consuming a mailbox sample.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InputFrame {
    down: [u64; INPUT_WORDS],
    just_pressed: [u64; INPUT_WORDS],
    just_released: [u64; INPUT_WORDS],
}

impl InputFrame {
    /// Whether a supported key is held in this frame.
    ///
    /// `key` is the stable numeric ID from [`InputKey::id`](crate::api::ecs::input_key::InputKey::id).
    pub fn is_down(self, key: u32) -> bool {
        bit(self.down, key)
    }
    /// Whether the key changed from released to pressed since the last consume.
    ///
    /// Repeated presses before consumption remain one `true` edge.
    pub fn just_pressed(self, key: u32) -> bool {
        bit(self.just_pressed, key)
    }
    /// Whether the key changed from pressed to released since the last consume.
    ///
    /// Repeated releases before consumption remain one `true` edge.
    pub fn just_released(self, key: u32) -> bool {
        bit(self.just_released, key)
    }

    /// Clear transition edges while preserving held-key state.
    pub fn clear_edges(&mut self) {
        self.just_pressed = [0; INPUT_WORDS];
        self.just_released = [0; INPUT_WORDS];
    }

    /// Apply one key state and record any transition.
    ///
    /// Unsupported key IDs are ignored. A transition stays set until
    /// [`InputFrame::clear_edges`] or a mailbox [`InputFrameMailbox::take`].
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

    /// Replace held keys and merge transitions from a sampled UI frame.
    ///
    /// Repeated samples before consumption preserve all observed press/release
    /// edges while the held state comes from the newest sample.
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

/// Shared mailbox between the render/UI producer and game callback consumer.
///
/// The mailbox retains the latest held-key state and merges transition edges
/// until taken. Both operations are non-blocking: lock contention drops that
/// operation and is reported as `false` or `None`.
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
