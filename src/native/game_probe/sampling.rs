//! Read-only sampling that turns validated native observations into owned values.
use super::*;

pub(super) fn current_game_snapshot(thread_id: u32) -> GameSnapshot {
    let player = PLAYER_SAMPLE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let player_fresh = player
        .sampled_at
        .is_some_and(|sampled_at| sampled_at.elapsed() <= PLAYER_SAMPLE_MAX_AGE);
    let player_stale = player.status == 10 && !player_fresh;
    let vehicle = VEHICLE_SAMPLE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let vehicle_fresh = vehicle
        .sampled_at
        .is_some_and(|sampled_at| sampled_at.elapsed() <= PLAYER_SAMPLE_MAX_AGE);
    let latest_sample_at = player.sampled_at.max(vehicle.sampled_at);
    let sample_age_seconds = latest_sample_at.map(|sampled_at| sampled_at.elapsed().as_secs_f32());

    GameSnapshot {
        callback_sequence: UPDATE_POST_SIM_CALLS.load(Ordering::Acquire),
        camera_raycast_hook_installed: CAMERA_RAYCAST_HOOK_INSTALLED.load(Ordering::Acquire),
        camera_raycast_calls: CAMERA_RAYCAST_CALLS.load(Ordering::Acquire),
        world_query_hook_installed: WORLD_QUERY_HOOK_INSTALLED.load(Ordering::Acquire),
        world_query_calls: WORLD_QUERY_CALLS.load(Ordering::Acquire),
        world_query_last_return_address: match WORLD_QUERY_LAST_RETURN_ADDRESS
            .load(Ordering::Acquire)
        {
            0 => None,
            address => Some(address),
        },
        sample_sequence: GAME_SAMPLE_SEQUENCE.load(Ordering::Acquire),
        sample_age_seconds,
        callback_thread_id: Some(thread_id),
        callback_stable_streak: CALLBACK_STABLE_STREAK.load(Ordering::Acquire),
        callback_overlap_count: CALLBACK_OVERLAPS.load(Ordering::Acquire),
        callback_thread_mismatch_count: CALLBACK_THREAD_MISMATCHES.load(Ordering::Acquire),
        callback_gate_open: CALLBACK_GATE_OPEN.load(Ordering::Acquire),
        callback_panics: CALLBACK_RUNTIME_PANICS.load(Ordering::Acquire),
        player_position: (!player_stale).then_some(player.position).flatten(),
        player_forward: (!player_stale).then_some(player.forward).flatten(),
        player_transform: (!player_stale).then_some(player.transform).flatten(),
        player_rotation: (!player_stale).then_some(player.rotation).flatten(),
        view_rotation: (!player_stale).then_some(player.view_rotation).flatten(),
        reticle_ray_origin: (!player_stale)
            .then_some(player.reticle_ray_origin)
            .flatten(),
        reticle_ray_direction: (!player_stale)
            .then_some(player.reticle_ray_direction)
            .flatten(),
        reticle_query_available: !player_stale && player.reticle_query_available,
        reticle_query_state_present: !player_stale && player.reticle_query_state_present,
        reticle_hit_fraction: (!player_stale)
            .then_some(player.reticle_hit_fraction)
            .flatten(),
        reticle_candidate_position: (!player_stale)
            .then_some(player.reticle_candidate_position)
            .flatten(),
        reticle_candidate_shape: (!player_stale)
            .then_some(player.reticle_candidate_shape)
            .flatten(),
        player_aim_query_available: !player_stale && player.player_aim_query_available,
        player_aim_query_state_present: !player_stale && player.player_aim_query_state_present,
        player_aim_hit_fraction: (!player_stale)
            .then_some(player.player_aim_hit_fraction)
            .flatten(),
        player_aim_candidate_position: (!player_stale)
            .then_some(player.player_aim_candidate_position)
            .flatten(),
        player_aim_candidate_shape: (!player_stale)
            .then_some(player.player_aim_candidate_shape)
            .flatten(),
        ignore_characters_vehicles_query_available: !player_stale
            && player.ignore_characters_vehicles_query_available,
        ignore_characters_vehicles_query_state_present: !player_stale
            && player.ignore_characters_vehicles_query_state_present,
        ignore_characters_vehicles_hit_fraction: (!player_stale)
            .then_some(player.ignore_characters_vehicles_hit_fraction)
            .flatten(),
        ignore_characters_vehicles_candidate_position: (!player_stale)
            .then_some(player.ignore_characters_vehicles_candidate_position)
            .flatten(),
        ignore_characters_vehicles_candidate_shape: (!player_stale)
            .then_some(player.ignore_characters_vehicles_candidate_shape)
            .flatten(),
        ignore_player_owner_query_available: !player_stale
            && player.ignore_player_owner_query_available,
        ignore_player_owner_query_state_present: !player_stale
            && player.ignore_player_owner_query_state_present,
        ignore_player_owner_hit_fraction: (!player_stale)
            .then_some(player.ignore_player_owner_fraction)
            .flatten(),
        ignore_player_owner_candidate_position: (!player_stale)
            .then_some(player.ignore_player_owner_candidate_position)
            .flatten(),
        ignore_player_owner_candidate_shape: (!player_stale)
            .then_some(player.ignore_player_owner_candidate_shape)
            .flatten(),
        player_velocity: (!player_stale).then_some(player.velocity).flatten(),
        player_acceleration: (!player_stale).then_some(player.acceleration).flatten(),
        vehicle_position: vehicle_fresh.then_some(vehicle.position).flatten(),
        vehicle_transform: vehicle_fresh.then_some(vehicle.transform).flatten(),
        vehicle_forward: vehicle_fresh.then_some(vehicle.forward).flatten(),
        vehicle_rotation: vehicle_fresh.then_some(vehicle.rotation).flatten(),
        vehicle_velocity: vehicle_fresh.then_some(vehicle.velocity).flatten(),
        vehicle_acceleration: vehicle_fresh.then_some(vehicle.acceleration).flatten(),
        vehicle_angular_velocity: vehicle_fresh.then_some(vehicle.angular_velocity).flatten(),
        vehicle_velocity_last_frame: vehicle_fresh
            .then_some(vehicle.velocity_last_frame)
            .flatten(),
        ..GameSnapshot::default()
    }
}

pub(super) fn probe_game_state() {
    if !readable_span(CHARACTER_MANAGER_SINGLETON, size_of::<*mut c_void>()) {
        publish_player_sample(1, None, None, None);
        publish_vehicle_sample(None);
        return;
    }
    if !is_readable_executable_page(GET_PLAYER_CHARACTER) {
        publish_player_sample(2, None, None, None);
        publish_vehicle_sample(None);
        return;
    }

    let manager = unsafe { ptr::read_volatile(CHARACTER_MANAGER_SINGLETON as *const *mut c_void) };
    if manager.is_null() || !readable_span(manager as usize, 1) {
        publish_player_sample(1, None, None, None);
        publish_vehicle_sample(None);
        return;
    }

    let get_player_character: GetPlayerCharacter =
        unsafe { std::mem::transmute(GET_PLAYER_CHARACTER) };
    let player = unsafe { get_player_character(manager.cast::<CharacterManager>()) };
    if player.is_null() {
        publish_player_sample(3, None, None, None);
        publish_vehicle_sample(None);
        return;
    }
    if !readable_span(player as usize, 1) {
        publish_player_sample(4, None, None, None);
        publish_vehicle_sample(None);
        return;
    }

    probe_player_transform(player);
    publish_vehicle_sample(sample_vehicle(player));
}

pub(super) fn apply_velocity_intent(
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

pub(super) fn apply_spawn_intent(pending: &PendingSpawnIntent, overlap_epoch: u64) -> SpawnOutcome {
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

pub(super) fn current_player_vehicle() -> Option<*mut Vehicle> {
    if !readable_span(CHARACTER_MANAGER_SINGLETON, size_of::<*mut c_void>())
        || !is_readable_executable_page(GET_PLAYER_CHARACTER)
        || !is_readable_executable_page(GET_CHARACTER_VEHICLE)
    {
        return None;
    }
    let manager = unsafe { ptr::read_volatile(CHARACTER_MANAGER_SINGLETON as *const *mut c_void) };
    if manager.is_null() || !readable_span(manager as usize, 1) {
        return None;
    }
    let get_player_character: GetPlayerCharacter =
        unsafe { std::mem::transmute(GET_PLAYER_CHARACTER) };
    let player = unsafe { get_player_character(manager.cast::<CharacterManager>()) };
    if player.is_null() || !readable_span(player as usize, 1) {
        return None;
    }
    let get_vehicle: GetCharacterVehicle = unsafe { std::mem::transmute(GET_CHARACTER_VEHICLE) };
    let vehicle = unsafe { get_vehicle(player) };
    (!vehicle.is_null() && readable_span(vehicle as usize, 1)).then_some(vehicle)
}

pub(super) fn probe_player_transform(player: *mut Character) {
    let Some(game_object_address) = (player as usize).checked_add(CHARACTER_TO_GAME_OBJECT_OFFSET)
    else {
        publish_player_sample(5, None, None, None);
        return;
    };
    if game_object_address % std::mem::align_of::<*const usize>() != 0 {
        publish_player_sample(5, None, None, None);
        return;
    }
    if !readable_span(game_object_address, size_of::<*const usize>()) {
        publish_player_sample(5, None, None, None);
        return;
    }

    let vtable = unsafe { ptr::read_volatile(game_object_address as *const *const usize) };
    if vtable.is_null() {
        publish_player_sample(6, None, None, None);
        return;
    }
    if (vtable as usize) % std::mem::align_of::<usize>() != 0 {
        publish_player_sample(6, None, None, None);
        return;
    }
    let Some(slot_count) = GET_TRANSFORM_VTABLE_SLOT.checked_add(1) else {
        publish_player_sample(6, None, None, None);
        return;
    };
    let Some(vtable_bytes) = slot_count.checked_mul(size_of::<usize>()) else {
        publish_player_sample(6, None, None, None);
        return;
    };
    if !readable_span(vtable as usize, vtable_bytes) {
        publish_player_sample(6, None, None, None);
        return;
    }

    let slot_address = vtable.wrapping_add(GET_TRANSFORM_VTABLE_SLOT);
    let target = unsafe { ptr::read_volatile(slot_address) };
    if target == 0 || !is_readable_executable_page(target) {
        publish_player_sample(7, None, None, None);
        return;
    }

    let get_transform: GetTransform = unsafe { std::mem::transmute(target) };
    let mut transform = Matrix4x4 {
        values: [[0.0; 4]; 4],
    };
    if !unsafe { get_transform(game_object_address as *mut GameObject, &mut transform) } {
        publish_player_sample(8, None, None, None);
        return;
    }

    let position = [
        transform.values[3][0],
        transform.values[3][1],
        transform.values[3][2],
    ];
    if !position.iter().all(|coordinate| coordinate.is_finite()) {
        publish_player_sample(9, None, None, None);
        return;
    }

    let rotation = rotation_xyz_degrees(&transform);
    let camera_matrix = sample_camera_matrix();
    let view_rotation = camera_matrix.as_ref().and_then(rotation_xyz_degrees);
    let reticle_ray_sample = camera_matrix
        .as_ref()
        .and_then(|matrix| sample_reticle_ray_if_due(matrix, player));
    publish_player_sample(10, Some(position), rotation, view_rotation);
    if let Some(sample) = reticle_ray_sample {
        let mut player_sample = PLAYER_SAMPLE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        player_sample.reticle_ray_origin = sample.origin;
        player_sample.reticle_ray_direction = sample.direction;
        player_sample.reticle_query_available = sample.query_available;
        player_sample.reticle_query_state_present = sample.query_state_present;
        player_sample.reticle_hit_fraction = sample.fraction;
        player_sample.reticle_candidate_position = sample.candidate_position;
        player_sample.reticle_candidate_shape = sample.candidate_shape;
        player_sample.player_aim_query_available = sample.player_aim_query_available;
        player_sample.player_aim_query_state_present = sample.player_aim_query_state_present;
        player_sample.player_aim_hit_fraction = sample.player_aim_fraction;
        player_sample.player_aim_candidate_position = sample.player_aim_candidate_position;
        player_sample.player_aim_candidate_shape = sample.player_aim_candidate_shape;
        player_sample.ignore_characters_vehicles_query_available =
            sample.ignore_characters_vehicles_query_available;
        player_sample.ignore_characters_vehicles_query_state_present =
            sample.ignore_characters_vehicles_query_state_present;
        player_sample.ignore_characters_vehicles_hit_fraction =
            sample.ignore_characters_vehicles_fraction;
        player_sample.ignore_characters_vehicles_candidate_position =
            sample.ignore_characters_vehicles_candidate_position;
        player_sample.ignore_characters_vehicles_candidate_shape =
            sample.ignore_characters_vehicles_candidate_shape;
        player_sample.ignore_player_owner_query_available =
            sample.ignore_player_owner_query_available;
        player_sample.ignore_player_owner_query_state_present =
            sample.ignore_player_owner_query_state_present;
        player_sample.ignore_player_owner_fraction = sample.ignore_player_owner_fraction;
        player_sample.ignore_player_owner_candidate_position =
            sample.ignore_player_owner_candidate_position;
        player_sample.ignore_player_owner_candidate_shape =
            sample.ignore_player_owner_candidate_shape;
    }
    let mut player_sample = PLAYER_SAMPLE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    player_sample.transform = Some(transform);
    player_sample.forward = horizontal_forward(&transform);
}

pub(super) fn sample_vehicle(
    player: *mut Character,
) -> Option<(
    Option<Matrix4x4>,
    Option<[f32; 3]>,
    Option<[f32; 3]>,
    Option<[f32; 3]>,
    Option<[f32; 3]>,
)> {
    if !is_readable_executable_page(GET_CHARACTER_VEHICLE) {
        return None;
    }

    let get_vehicle: GetCharacterVehicle = unsafe { std::mem::transmute(GET_CHARACTER_VEHICLE) };
    let vehicle = unsafe { get_vehicle(player) };
    if vehicle.is_null() || !readable_span(vehicle as usize, 1) {
        return None;
    }

    let velocity = read_vehicle_vector(vehicle, VEHICLE_GET_VELOCITY_VTABLE_SLOT);
    let acceleration = read_vehicle_vector(vehicle, VEHICLE_GET_ACCELERATION_VTABLE_SLOT);
    let angular_velocity = read_vehicle_vector(vehicle, VEHICLE_GET_ANGULAR_VELOCITY_VTABLE_SLOT);
    let velocity_last_frame =
        read_vehicle_last_frame_vector(vehicle, VEHICLE_GET_VELOCITY_LAST_FRAME_VTABLE_SLOT);

    let Some(game_object_address) = (vehicle as usize).checked_add(VEHICLE_TO_GAME_OBJECT_OFFSET)
    else {
        return Some((
            None,
            velocity,
            acceleration,
            angular_velocity,
            velocity_last_frame,
        ));
    };
    if game_object_address % std::mem::align_of::<*const usize>() != 0
        || !readable_span(game_object_address, size_of::<*const usize>())
    {
        return Some((
            None,
            velocity,
            acceleration,
            angular_velocity,
            velocity_last_frame,
        ));
    }

    let vtable = unsafe { ptr::read_volatile(game_object_address as *const *const usize) };
    if vtable.is_null() || (vtable as usize) % std::mem::align_of::<usize>() != 0 {
        return Some((
            None,
            velocity,
            acceleration,
            angular_velocity,
            velocity_last_frame,
        ));
    }
    let slot_count = GET_TRANSFORM_VTABLE_SLOT.checked_add(1)?;
    let vtable_bytes = slot_count.checked_mul(size_of::<usize>())?;
    if !readable_span(vtable as usize, vtable_bytes) {
        return Some((
            None,
            velocity,
            acceleration,
            angular_velocity,
            velocity_last_frame,
        ));
    }

    let target = unsafe { ptr::read_volatile(vtable.wrapping_add(GET_TRANSFORM_VTABLE_SLOT)) };
    if target == 0 || !is_readable_executable_page(target) {
        return Some((
            None,
            velocity,
            acceleration,
            angular_velocity,
            velocity_last_frame,
        ));
    }

    let get_transform: GetTransform = unsafe { std::mem::transmute(target) };
    let mut transform = Matrix4x4 {
        values: [[0.0; 4]; 4],
    };
    if !unsafe { get_transform(game_object_address as *mut GameObject, &mut transform) } {
        return Some((
            None,
            velocity,
            acceleration,
            angular_velocity,
            velocity_last_frame,
        ));
    }

    Some((
        Some(transform),
        velocity,
        acceleration,
        angular_velocity,
        velocity_last_frame,
    ))
}

pub(super) fn read_vehicle_vector(vehicle: *mut Vehicle, slot: usize) -> Option<[f32; 3]> {
    let target = vehicle_virtual_function(vehicle, slot)?;
    let get_vector: GetVehicleVector = unsafe { std::mem::transmute(target) };
    let mut vector = [0.0; 3];
    unsafe { get_vector(vehicle, &mut vector) };
    vector
        .iter()
        .all(|component| component.is_finite())
        .then_some(vector)
}

pub(super) fn read_vehicle_last_frame_vector(
    vehicle: *mut Vehicle,
    slot: usize,
) -> Option<[f32; 3]> {
    let target = vehicle_virtual_function(vehicle, slot)?;
    let get_vector: GetVehicleVectorPointer = unsafe { std::mem::transmute(target) };
    let vector_pointer = unsafe { get_vector(vehicle) };
    if vector_pointer.is_null() || !readable_span(vector_pointer as usize, size_of::<[f32; 3]>()) {
        return None;
    }

    let vector = unsafe { ptr::read_volatile(vector_pointer) };
    vector
        .iter()
        .all(|component| component.is_finite())
        .then_some(vector)
}

pub(super) fn vehicle_virtual_function(vehicle: *mut Vehicle, slot: usize) -> Option<usize> {
    let vehicle_address = vehicle as usize;
    if vehicle.is_null()
        || vehicle_address % std::mem::align_of::<*const usize>() != 0
        || !readable_span(vehicle_address, size_of::<*const usize>())
    {
        return None;
    }

    let vtable = unsafe { ptr::read_volatile(vehicle as *const *const usize) };
    if vtable.is_null() || (vtable as usize) % std::mem::align_of::<usize>() != 0 {
        return None;
    }
    let slot_count = slot.checked_add(1)?;
    let vtable_bytes = slot_count.checked_mul(size_of::<usize>())?;
    if !readable_span(vtable as usize, vtable_bytes) {
        return None;
    }

    let target = unsafe { ptr::read_volatile(vtable.wrapping_add(slot)) };
    if target == 0 || !is_readable_executable_page(target) {
        return None;
    }
    Some(target)
}

pub(super) fn rotation_xyz_degrees(matrix: &Matrix4x4) -> Option<[f32; 3]> {
    let values = &matrix.values;
    if values[..3]
        .iter()
        .any(|row| !row[..3].iter().all(|value| value.is_finite()))
    {
        return None;
    }

    let pitch = (-values[0][2]).clamp(-1.0, 1.0).asin();
    let cos_pitch = pitch.cos();
    let (roll, yaw) = if cos_pitch.abs() > 1.0e-6 {
        (
            values[1][2].atan2(values[2][2]),
            values[0][1].atan2(values[0][0]),
        )
    } else {
        ((-values[2][1]).atan2(values[1][1]), 0.0)
    };
    Some([roll.to_degrees(), pitch.to_degrees(), yaw.to_degrees()])
}

pub(super) fn horizontal_forward(matrix: &Matrix4x4) -> Option<[f32; 3]> {
    // The live transformed-axes diagnostic showed actor GetTransform matrices
    // use row 0 = right, row 1 = up, and row 2 = backward. This differs from
    // the generic CMatrix helper names in the community SDK. Actor/vehicle
    // forward is therefore the negated horizontal projection of row 2.
    let backward_x = matrix.values[2][0];
    let backward_z = matrix.values[2][2];
    let magnitude = (backward_x * backward_x + backward_z * backward_z).sqrt();
    (backward_x.is_finite()
        && backward_z.is_finite()
        && magnitude.is_finite()
        && magnitude > 1.0e-6)
        .then_some([-backward_x / magnitude, 0.0, -backward_z / magnitude])
}

pub(super) fn sample_camera_matrix() -> Option<Matrix4x4> {
    if !readable_span(CAMERA_MANAGER_SINGLETON, size_of::<*mut c_void>())
        || !is_readable_executable_page(GET_CAMERA_MATRIX)
    {
        return None;
    }

    let manager = unsafe { ptr::read_volatile(CAMERA_MANAGER_SINGLETON as *const *mut c_void) };
    if manager.is_null() || !readable_span(manager as usize, 1) {
        return None;
    }

    let get_camera_matrix: GetCameraMatrix = unsafe { std::mem::transmute(GET_CAMERA_MATRIX) };
    let mut camera_matrix = Matrix4x4 {
        values: [[0.0; 4]; 4],
    };
    unsafe { get_camera_matrix(manager.cast::<CameraControlManager>(), &mut camera_matrix) };
    camera_matrix
        .values
        .iter()
        .flatten()
        .all(|value| value.is_finite())
        .then_some(camera_matrix)
}

pub(super) fn sample_reticle_ray_if_due(
    camera_matrix: &Matrix4x4,
    player: *mut Character,
) -> Option<ReticleRaySample> {
    let now = Instant::now();
    {
        let mut last_sample = LAST_RETICLE_RAY_SAMPLE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if last_sample.is_some_and(|last| now.duration_since(last) < RETICLE_RAY_SAMPLE_INTERVAL) {
            return None;
        }
        *last_sample = Some(now);
    }

    let Some(ray) = WorldQueryRayInput::from_camera_matrix(camera_matrix) else {
        return Some(ReticleRaySample {
            origin: None,
            direction: None,
            query_available: false,
            query_state_present: false,
            fraction: None,
            candidate_position: None,
            candidate_shape: None,
            player_aim_query_available: false,
            player_aim_query_state_present: false,
            player_aim_fraction: None,
            player_aim_candidate_position: None,
            player_aim_candidate_shape: None,
            ignore_characters_vehicles_query_available: false,
            ignore_characters_vehicles_query_state_present: false,
            ignore_characters_vehicles_fraction: None,
            ignore_characters_vehicles_candidate_position: None,
            ignore_characters_vehicles_candidate_shape: None,
            ignore_player_owner_query_available: false,
            ignore_player_owner_query_state_present: false,
            ignore_player_owner_fraction: None,
            ignore_player_owner_candidate_position: None,
            ignore_player_owner_candidate_shape: None,
        });
    };
    let origin = [ray.origin.x, ray.origin.y, ray.origin.z];
    let direction = [ray.direction.x, ray.direction.y, ray.direction.z];
    let mut sample = ReticleRaySample {
        origin: Some(origin),
        direction: Some(direction),
        query_available: false,
        query_state_present: false,
        fraction: None,
        candidate_position: None,
        candidate_shape: None,
        player_aim_query_available: false,
        player_aim_query_state_present: false,
        player_aim_fraction: None,
        player_aim_candidate_position: None,
        player_aim_candidate_shape: None,
        ignore_characters_vehicles_query_available: false,
        ignore_characters_vehicles_query_state_present: false,
        ignore_characters_vehicles_fraction: None,
        ignore_characters_vehicles_candidate_position: None,
        ignore_characters_vehicles_candidate_shape: None,
        ignore_player_owner_query_available: false,
        ignore_player_owner_query_state_present: false,
        ignore_player_owner_fraction: None,
        ignore_player_owner_candidate_position: None,
        ignore_player_owner_candidate_shape: None,
    };

    let module = unsafe { GetModuleHandleW(hudhook::windows::core::PCWSTR::null()) };
    let Ok(module) = module else {
        return Some(sample);
    };
    let image_base = module.0 as usize;
    if image_base != STEAM_IMAGE_BASE {
        return Some(sample);
    }
    let Some(initializer_address) = image_base.checked_add(WORLD_QUERY_CONTEXT_INITIALIZER_RVA)
    else {
        return Some(sample);
    };
    let Some(option_setter_address) = image_base.checked_add(RAYCAST_FILTER_SET_OPTION_RVA) else {
        return Some(sample);
    };
    let Some(query_address) = image_base.checked_add(WORLD_QUERY_HELPER_RVA) else {
        return Some(sample);
    };
    let Some(filter_vtable) = image_base.checked_add(C_STATIC_ONLY_RAYCAST_FILTER_VFTABLE_RVA)
    else {
        return Some(sample);
    };
    if ![initializer_address, option_setter_address, query_address]
        .into_iter()
        .all(is_readable_executable_page)
    {
        return Some(sample);
    }

    let Some(query_base_global) = image_base.checked_add(WORLD_QUERY_BASE_POINTER_RVA) else {
        return Some(sample);
    };
    if !readable_span(query_base_global, size_of::<*mut c_void>()) {
        return Some(sample);
    }
    let query_base = unsafe { ptr::read_volatile(query_base_global as *const *mut c_void) };
    if query_base.is_null() || !readable_span(query_base as usize, 1) {
        return Some(sample);
    }

    let mut context = RaycastFilterContextStorage([0; 0xD0]);
    let context_pointer = context.0.as_mut_ptr().cast::<c_void>();
    let initialize: WorldQueryContextInitializeFn =
        unsafe { std::mem::transmute(initializer_address) };
    let initialized_context = unsafe { initialize(context_pointer, 0, 0, 0) };
    if initialized_context.is_null() {
        return Some(sample);
    }
    unsafe {
        ptr::write_unaligned(context_pointer.cast::<usize>(), filter_vtable);
        ptr::write_unaligned(
            context
                .0
                .as_mut_ptr()
                .add(C_STATIC_ONLY_FILTER_CATEGORY_MASK_OFFSET)
                .cast::<u32>(),
            3,
        );
    }
    let set_option: RaycastFilterSetOptionFn =
        unsafe { std::mem::transmute(option_setter_address) };
    unsafe { set_option(context_pointer, 1) };

    let mut output = WorldQueryOutputRecord {
        qword_00: 0,
        vector_08: Vector3::default(),
        scalar_14: 1.0,
        qword_18: 0,
        qword_20: 0,
        qword_28: 0,
        words_30: [0; 8],
    };
    let query: WorldQueryHelperFn = unsafe { std::mem::transmute(query_address) };
    let label = b"MadMaxModdingReticleRay\0";
    let state_present = unsafe {
        query(
            query_base,
            label.as_ptr().cast(),
            &ray,
            0.0,
            RETICLE_RAY_MAX_DISTANCE,
            &mut output,
            context_pointer,
            0,
            false,
            0,
            ptr::null(),
        )
    };
    sample.query_available = true;
    sample.query_state_present = state_present;
    sample.candidate_shape = read_query_candidate_shape(&context);
    if output.scalar_14.is_finite() && (0.0..=1.0).contains(&output.scalar_14) {
        sample.fraction = Some(output.scalar_14);
        if output.scalar_14 < 1.0 {
            if let Some(point) = ray.point_at(RETICLE_RAY_MAX_DISTANCE * output.scalar_14) {
                sample.candidate_position = Some([point.x, point.y, point.z]);
            }
        }
    }

    // Repeat the same geometric query with the game's distinct player-aim filter.
    // Its candidate predicate explicitly rejects the current player character and
    // does not consume the static-only category mask at context +0x90.
    let mut player_aim_context = RaycastFilterContextStorage([0; 0xD0]);
    let player_aim_context_pointer = player_aim_context.0.as_mut_ptr().cast::<c_void>();
    let initialized_player_aim_context = unsafe { initialize(player_aim_context_pointer, 0, 0, 0) };
    let Some(player_aim_filter_vtable) =
        image_base.checked_add(C_PLAYER_AIM_RAYCAST_FILTER_VFTABLE_RVA)
    else {
        return Some(sample);
    };
    if !initialized_player_aim_context.is_null() {
        unsafe {
            ptr::write_unaligned(
                player_aim_context_pointer.cast::<usize>(),
                player_aim_filter_vtable,
            );
        }
        unsafe { set_option(player_aim_context_pointer, 1) };
        let mut player_aim_output = WorldQueryOutputRecord {
            qword_00: 0,
            vector_08: Vector3::default(),
            scalar_14: 1.0,
            qword_18: 0,
            qword_20: 0,
            qword_28: 0,
            words_30: [0; 8],
        };
        let player_aim_state_present = unsafe {
            query(
                query_base,
                label.as_ptr().cast(),
                &ray,
                0.0,
                RETICLE_RAY_MAX_DISTANCE,
                &mut player_aim_output,
                player_aim_context_pointer,
                0,
                false,
                0,
                ptr::null(),
            )
        };
        sample.player_aim_query_available = true;
        sample.player_aim_query_state_present = player_aim_state_present;
        sample.player_aim_candidate_shape = read_query_candidate_shape(&player_aim_context);
        if player_aim_output.scalar_14.is_finite()
            && (0.0..=1.0).contains(&player_aim_output.scalar_14)
        {
            sample.player_aim_fraction = Some(player_aim_output.scalar_14);
            if player_aim_output.scalar_14 < 1.0 {
                if let Some(point) =
                    ray.point_at(RETICLE_RAY_MAX_DISTANCE * player_aim_output.scalar_14)
                {
                    sample.player_aim_candidate_position = Some([point.x, point.y, point.z]);
                }
            }
        }
    }

    let Some(ignore_characters_vehicles_filter_vtable) =
        image_base.checked_add(C_IGNORE_CHARACTERS_AND_VEHICLES_RAYCAST_FILTER_VFTABLE_RVA)
    else {
        return Some(sample);
    };
    let mut ignore_characters_vehicles_context = RaycastFilterContextStorage([0; 0xD0]);
    let ignore_characters_vehicles_context_pointer = ignore_characters_vehicles_context
        .0
        .as_mut_ptr()
        .cast::<c_void>();
    let initialized_ignore_characters_vehicles_context =
        unsafe { initialize(ignore_characters_vehicles_context_pointer, 0, 0, 0) };
    if !initialized_ignore_characters_vehicles_context.is_null() {
        unsafe {
            ptr::write_unaligned(
                ignore_characters_vehicles_context_pointer.cast::<usize>(),
                ignore_characters_vehicles_filter_vtable,
            );
        }
        unsafe { set_option(ignore_characters_vehicles_context_pointer, 1) };
        let mut ignore_characters_vehicles_output = WorldQueryOutputRecord {
            qword_00: 0,
            vector_08: Vector3::default(),
            scalar_14: 1.0,
            qword_18: 0,
            qword_20: 0,
            qword_28: 0,
            words_30: [0; 8],
        };
        let ignore_characters_vehicles_label = b"MadMaxModdingReticleIgnoreCharsVehicles\0";
        let ignore_characters_vehicles_state_present = unsafe {
            query(
                query_base,
                ignore_characters_vehicles_label.as_ptr().cast(),
                &ray,
                0.0,
                RETICLE_RAY_MAX_DISTANCE,
                &mut ignore_characters_vehicles_output,
                ignore_characters_vehicles_context_pointer,
                0,
                false,
                0,
                ptr::null(),
            )
        };
        sample.ignore_characters_vehicles_query_available = true;
        sample.ignore_characters_vehicles_query_state_present =
            ignore_characters_vehicles_state_present;
        sample.ignore_characters_vehicles_candidate_shape =
            read_query_candidate_shape(&ignore_characters_vehicles_context);
        if ignore_characters_vehicles_output.scalar_14.is_finite()
            && (0.0..=1.0).contains(&ignore_characters_vehicles_output.scalar_14)
        {
            sample.ignore_characters_vehicles_fraction =
                Some(ignore_characters_vehicles_output.scalar_14);
            if ignore_characters_vehicles_output.scalar_14 < 1.0 {
                if let Some(point) = ray.point_at(
                    RETICLE_RAY_MAX_DISTANCE * ignore_characters_vehicles_output.scalar_14,
                ) {
                    sample.ignore_characters_vehicles_candidate_position =
                        Some([point.x, point.y, point.z]);
                }
            }
        }
    }

    // Repeat the same geometric query with a CIgnoreFilter owner list that
    // contains only the current player object. This isolates owner exclusion
    // from the PlayerAim policy and keeps the shared filter-info exclusion
    // and required masks identical.
    let Some(ignore_filter_vtable) = image_base.checked_add(C_IGNORE_FILTER_VFTABLE_RVA) else {
        return Some(sample);
    };
    if !player.is_null() && readable_span(player as usize, size_of::<usize>()) {
        let mut ignore_player_context = RaycastFilterContextStorage([0; 0xD0]);
        let ignore_player_context_pointer = ignore_player_context.0.as_mut_ptr().cast::<c_void>();
        let initialized_ignore_player_context =
            unsafe { initialize(ignore_player_context_pointer, 0, 0, 0) };
        if !initialized_ignore_player_context.is_null() {
            let ignored_owner = player as usize;
            let ignored_owners = [ignored_owner];
            let owner_begin = ignored_owners.as_ptr() as usize;
            let owner_end = owner_begin + size_of::<usize>();
            unsafe {
                ptr::write_unaligned(
                    ignore_player_context_pointer.cast::<usize>(),
                    ignore_filter_vtable,
                );
                ptr::write_unaligned(
                    ignore_player_context
                        .0
                        .as_mut_ptr()
                        .add(C_IGNORE_FILTER_OWNER_VECTOR_BEGIN_OFFSET)
                        .cast::<usize>(),
                    owner_begin,
                );
                ptr::write_unaligned(
                    ignore_player_context
                        .0
                        .as_mut_ptr()
                        .add(C_IGNORE_FILTER_OWNER_VECTOR_END_OFFSET)
                        .cast::<usize>(),
                    owner_end,
                );
                ptr::write_unaligned(
                    ignore_player_context
                        .0
                        .as_mut_ptr()
                        .add(C_IGNORE_FILTER_OWNER_VECTOR_CAPACITY_OFFSET)
                        .cast::<usize>(),
                    owner_end,
                );
            }
            unsafe { set_option(ignore_player_context_pointer, 1) };
            let mut ignore_player_output = WorldQueryOutputRecord {
                qword_00: 0,
                vector_08: Vector3::default(),
                scalar_14: 1.0,
                qword_18: 0,
                qword_20: 0,
                qword_28: 0,
                words_30: [0; 8],
            };
            let ignore_player_label = b"MadMaxModdingReticleIgnorePlayerOwner\0";
            let ignore_player_state_present = unsafe {
                query(
                    query_base,
                    ignore_player_label.as_ptr().cast(),
                    &ray,
                    0.0,
                    RETICLE_RAY_MAX_DISTANCE,
                    &mut ignore_player_output,
                    ignore_player_context_pointer,
                    0,
                    false,
                    0,
                    ptr::null(),
                )
            };
            sample.ignore_player_owner_query_available = true;
            sample.ignore_player_owner_query_state_present = ignore_player_state_present;
            sample.ignore_player_owner_candidate_shape =
                read_query_candidate_shape(&ignore_player_context);
            if ignore_player_output.scalar_14.is_finite()
                && (0.0..=1.0).contains(&ignore_player_output.scalar_14)
            {
                sample.ignore_player_owner_fraction = Some(ignore_player_output.scalar_14);
                if ignore_player_output.scalar_14 < 1.0 {
                    if let Some(point) =
                        ray.point_at(RETICLE_RAY_MAX_DISTANCE * ignore_player_output.scalar_14)
                    {
                        sample.ignore_player_owner_candidate_position =
                            Some([point.x, point.y, point.z]);
                    }
                }
            }
        }
    }

    Some(sample)
}

pub(super) fn read_query_candidate_shape(
    context: &RaycastFilterContextStorage,
) -> Option<RaycastCandidateShape> {
    let pointer_address = unsafe {
        context
            .0
            .as_ptr()
            .add(WORLD_QUERY_CONTEXT_ACCEPTED_SHAPE_POINTER_OFFSET)
            .cast::<usize>()
    };
    let address = unsafe { ptr::read_unaligned(pointer_address) };
    if address == 0 || !readable_span(address, size_of::<usize>()) {
        return None;
    }
    let vtable = unsafe { ptr::read_unaligned(address as *const usize) };
    (vtable != 0).then_some(RaycastCandidateShape { address, vtable })
}
