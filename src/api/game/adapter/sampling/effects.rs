//! Validated application of script effects to the live game.
use super::*;

pub(in crate::api::game::adapter) fn apply_velocity_intent(
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

pub(in crate::api::game::adapter) fn apply_spawn_intent(
    pending: &PendingSpawnIntent,
    overlap_epoch: u64,
) -> SpawnOutcome {
    let outcome = |result, native_return| SpawnOutcome {
        request_id: pending.request_id,
        result,
        native_return,
    };
    if CALLBACK_IN_FLIGHT.load(Ordering::Acquire) != 1
        || CALLBACK_OVERLAP_EPOCH.load(Ordering::Acquire) != overlap_epoch
    {
        return outcome(SpawnRequestResult::CallbackNotExclusive, None);
    }
    let Ok(asset_key) = CString::new(format!("{}.ee", pending.asset_key)) else {
        return outcome(SpawnRequestResult::InvalidRequest, None);
    };
    if !pending
        .matrix
        .values
        .iter()
        .flatten()
        .all(|value| value.is_finite())
        || pending.request_id == 0
    {
        return outcome(SpawnRequestResult::InvalidRequest, None);
    }
    if pending.request_id > u64::from(u32::MAX) {
        return outcome(SpawnRequestResult::SpawnIdUnavailable, None);
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
        return outcome(SpawnRequestResult::NativeSymbolUnavailable, None);
    }

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

    let raw_info = info.cast::<u8>();
    // This first experiment uses a request-derived nonzero scalar ID because
    // the SDK's CHashString reference-returning allocator has an unverified
    // Rust ABI. It is unique within this process until the u32 range is spent.
    let spawn_id = (pending.request_id as u32).max(1);
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
    outcome(SpawnRequestResult::NativeCallSubmitted, Some(native_return))
}
