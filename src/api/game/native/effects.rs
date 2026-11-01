//! Validation and application of bounded script effects to the live game.
//!
//! Effects arrive as owned intents from the ECS runtime. This module checks
//! executable identity, callback phase, request bounds, and native address
//! availability before invoking the build-specific implementation. Rejected
//! requests produce outcomes; no script-held pointer or native object handle
//! is accepted.
mod debug_draw;
use super::*;
pub(super) use debug_draw::*;

pub(in crate::api::game::native) fn apply_velocity_intent(
    pending: PendingVelocityIntent,
    overlap_epoch: u64,
) -> IntentResult {
    if CALLBACK_IN_FLIGHT.load(Ordering::Acquire) != 1
        || CALLBACK_OVERLAP_EPOCH.load(Ordering::Acquire) != overlap_epoch
    {
        return IntentResult::CallbackNotExclusive;
    }
    if !pending.velocity.iter().all(|value| value.is_finite()) {
        return IntentResult::InvalidValue;
    }
    let Some(vehicle) = current_player_vehicle() else {
        return IntentResult::NoCurrentVehicle;
    };
    let Some(target) = vehicle_virtual_function(vehicle, VEHICLE_SET_VELOCITY_VTABLE_SLOT) else {
        return IntentResult::SetterUnavailable;
    };
    let set_velocity: SetVehicleVector = unsafe { std::mem::transmute(target) };
    unsafe { set_velocity(vehicle, &pending.velocity) };
    IntentResult::Applied
}

const SPAWN_INFO_BYTES: usize = 0x98;
const SPAWN_INFO_ID_OFFSET: usize = 0x00;
const SPAWN_INFO_PATCH_ID_OFFSET: usize = 0x04;
const SPAWN_INFO_DESPAWN_DISTANCE_OFFSET: usize = 0x08;
const SPAWN_INFO_PRIORITY_OFFSET: usize = 0x4C;

struct SpawnInfoGuard {
    pointer: *mut SpawnInstanceInfo,
    destructor: SpawnInfoDestructorFn,
}

impl Drop for SpawnInfoGuard {
    fn drop(&mut self) {
        unsafe { (self.destructor)(self.pointer) };
    }
}

pub(in crate::api::game::native) fn apply_spawn_intent(
    pending: &PendingSpawnIntent,
    overlap_epoch: u64,
) -> SpawnOutcome {
    if CALLBACK_IN_FLIGHT.load(Ordering::Acquire) != 1
        || CALLBACK_OVERLAP_EPOCH.load(Ordering::Acquire) != overlap_epoch
    {
        return spawn_outcome(
            pending.request_id,
            SpawnRequestResult::CallbackNotExclusive,
            None,
            None,
        );
    }
    let Ok(asset_key) = CString::new(format!("{}.ee", pending.asset_key)) else {
        return spawn_outcome(
            pending.request_id,
            SpawnRequestResult::InvalidRequest,
            None,
            None,
        );
    };
    if !pending
        .matrix
        .values
        .iter()
        .flatten()
        .all(|value| value.is_finite())
        || pending.request_id == 0
    {
        return spawn_outcome(
            pending.request_id,
            SpawnRequestResult::InvalidRequest,
            None,
            None,
        );
    }
    if ![
        SPAWN_INFO_CTOR,
        SPAWN_INFO_DTOR,
        SPAWN_SET_ENTITY_FILE,
        SPAWN_SUBMIT,
    ]
    .into_iter()
    .all(is_readable_executable_page)
    {
        return spawn_outcome(
            pending.request_id,
            SpawnRequestResult::NativeSymbolUnavailable,
            None,
            None,
        );
    }

    submit_spawn_native(pending, &asset_key)
}

fn submit_spawn_native(pending: &PendingSpawnIntent, asset_key: &CString) -> SpawnOutcome {
    let constructor: SpawnInfoConstructorFn = unsafe { std::mem::transmute(SPAWN_INFO_CTOR) };
    let destructor: SpawnInfoDestructorFn = unsafe { std::mem::transmute(SPAWN_INFO_DTOR) };
    let set_entity_file: SetEntityFileFn = unsafe { std::mem::transmute(SPAWN_SET_ENTITY_FILE) };
    let spawn: SpawnFn = unsafe { std::mem::transmute(SPAWN_SUBMIT) };
    let mut storage = MaybeUninit::<[u64; SPAWN_INFO_BYTES / size_of::<u64>()]>::uninit();
    let info = storage.as_mut_ptr().cast::<SpawnInstanceInfo>();
    unsafe { constructor(info) };
    let _guard = SpawnInfoGuard {
        pointer: info,
        destructor,
    };
    unsafe { set_entity_file(info, asset_key.as_ptr()) };

    // The SDK allocator's reference-return ABI is not yet validated in this
    // build. Keep the previously working request-derived ID path until it is.
    let Ok(spawn_id) = u32::try_from(pending.request_id) else {
        return spawn_outcome(
            pending.request_id,
            SpawnRequestResult::SpawnIdUnavailable,
            None,
            None,
        );
    };
    if spawn_id == 0 {
        return spawn_outcome(
            pending.request_id,
            SpawnRequestResult::SpawnIdUnavailable,
            None,
            None,
        );
    }

    let raw_info = info.cast::<u8>();
    unsafe {
        ptr::write_unaligned(raw_info.add(SPAWN_INFO_ID_OFFSET).cast::<u32>(), spawn_id);
        ptr::write_unaligned(
            raw_info.add(SPAWN_INFO_PATCH_ID_OFFSET).cast::<u32>(),
            spawn_id,
        );
        ptr::write_unaligned(
            raw_info
                .add(SPAWN_INFO_DESPAWN_DISTANCE_OFFSET)
                .cast::<f32>(),
            90_000.0,
        );
        ptr::write_unaligned(raw_info.add(SPAWN_INFO_PRIORITY_OFFSET).cast::<i32>(), 2);
    }

    let mut matrix = pending.matrix;
    let native_return = unsafe { spawn(info, &mut matrix) };
    spawn_outcome(
        pending.request_id,
        SpawnRequestResult::NativeCallSubmitted,
        Some(native_return),
        Some(crate::api::game::abi::types::HashId::from_sdk_raw(spawn_id)),
    )
}

pub(in crate::api::game::native) fn apply_spawn_removal_intent(
    pending: PendingSpawnRemoval,
    overlap_epoch: u64,
) -> SpawnOutcome {
    if CALLBACK_IN_FLIGHT.load(Ordering::Acquire) != 1
        || CALLBACK_OVERLAP_EPOCH.load(Ordering::Acquire) != overlap_epoch
    {
        return spawn_outcome(
            pending.request_id,
            SpawnRequestResult::CallbackNotExclusive,
            None,
            Some(pending.native_id),
        );
    }
    if pending.native_id.raw() == 0 || !is_readable_executable_page(SPAWN_DESPAWN) {
        return spawn_outcome(
            pending.request_id,
            SpawnRequestResult::DespawnUnavailable,
            None,
            Some(pending.native_id),
        );
    }

    let despawn: SpawnDespawnFn = unsafe { std::mem::transmute(SPAWN_DESPAWN) };
    unsafe { despawn(&pending.native_id) };
    spawn_outcome(
        pending.request_id,
        SpawnRequestResult::DespawnCallSubmitted,
        None,
        Some(pending.native_id),
    )
}

fn spawn_outcome(
    request_id: u64,
    result: SpawnRequestResult,
    native_return: Option<usize>,
    native_spawn_id: Option<crate::api::game::abi::types::HashId>,
) -> SpawnOutcome {
    SpawnOutcome {
        request_id,
        result,
        native_return,
        native_spawn_id,
    }
}
