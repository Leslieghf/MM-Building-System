//! Bounded, short-lived camera-pose requests from script callbacks.
//!
//! The native camera hook only reads this copied scalar state. It never keeps
//! a script value or game pointer, and a missing refresh expires the override.
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

static VERSION: AtomicU64 = AtomicU64::new(0);
static FRAME_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static EXPIRES_AFTER: AtomicU64 = AtomicU64::new(0);
static ENABLED: AtomicBool = AtomicBool::new(false);
static HOOK_AVAILABLE: AtomicBool = AtomicBool::new(false);
static EXTERNAL_GETTER_CALLS: AtomicU64 = AtomicU64::new(0);
static EXTERNAL_OVERRIDE_CALLS: AtomicU64 = AtomicU64::new(0);
thread_local! {
    static INTERNAL_GETTER_SAMPLE: Cell<bool> = const { Cell::new(false) };
}
static VALUES: [AtomicU32; 6] = [
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
];

#[derive(Clone, Copy)]
pub(crate) struct CameraOverride {
    pub offset: [f32; 3],
    pub euler_degrees: [f32; 3],
}

pub(crate) fn advance_frame(sequence: u64) {
    FRAME_SEQUENCE.store(sequence, Ordering::Release);
}

pub(crate) fn set_hook_available(available: bool) {
    HOOK_AVAILABLE.store(available, Ordering::Release);
}

pub(crate) fn hook_available() -> bool {
    HOOK_AVAILABLE.load(Ordering::Acquire)
}

pub(crate) fn external_getter_calls() -> u64 {
    EXTERNAL_GETTER_CALLS.load(Ordering::Relaxed)
}

pub(crate) fn external_override_calls() -> u64 {
    EXTERNAL_OVERRIDE_CALLS.load(Ordering::Relaxed)
}

pub(crate) fn with_internal_getter_sample<T>(sample: impl FnOnce() -> T) -> T {
    INTERNAL_GETTER_SAMPLE.with(|internal| {
        let was_internal = internal.replace(true);
        let result = sample();
        internal.set(was_internal);
        result
    })
}

pub(crate) fn record_getter_call() -> bool {
    let external = INTERNAL_GETTER_SAMPLE.with(|internal| !internal.get());
    if external {
        EXTERNAL_GETTER_CALLS.fetch_add(1, Ordering::Relaxed);
    }
    external
}

pub(crate) fn record_external_override_call() {
    EXTERNAL_OVERRIDE_CALLS.fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn publish(value: CameraOverride, sequence: u64) {
    VERSION.fetch_add(1, Ordering::AcqRel);
    for (slot, value) in VALUES
        .iter()
        .zip(value.offset.into_iter().chain(value.euler_degrees))
    {
        slot.store(value.to_bits(), Ordering::Relaxed);
    }
    EXPIRES_AFTER.store(sequence.saturating_add(2), Ordering::Relaxed);
    ENABLED.store(true, Ordering::Relaxed);
    VERSION.fetch_add(1, Ordering::Release);
}

pub(crate) fn clear() {
    VERSION.fetch_add(1, Ordering::AcqRel);
    ENABLED.store(false, Ordering::Relaxed);
    VERSION.fetch_add(1, Ordering::Release);
}

pub(crate) fn current() -> Option<CameraOverride> {
    for _ in 0..3 {
        let before = VERSION.load(Ordering::Acquire);
        if before & 1 != 0 || !ENABLED.load(Ordering::Relaxed) {
            return None;
        }
        let expiry = EXPIRES_AFTER.load(Ordering::Relaxed);
        let values: [f32; 6] =
            std::array::from_fn(|index| f32::from_bits(VALUES[index].load(Ordering::Relaxed)));
        let after = VERSION.load(Ordering::Acquire);
        if before == after {
            if FRAME_SEQUENCE.load(Ordering::Acquire) > expiry
                || values.iter().any(|value| !value.is_finite())
            {
                return None;
            }
            return Some(CameraOverride {
                offset: [values[0], values[1], values[2]],
                euler_degrees: [values[3], values[4], values[5]],
            });
        }
    }
    None
}
