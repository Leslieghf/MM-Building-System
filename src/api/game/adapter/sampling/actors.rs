//! Player and vehicle sampling copied into owned telemetry.
use super::*;

pub(in crate::api::game::adapter) fn current_player_vehicle() -> Option<*mut Vehicle> {
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

pub(in crate::api::game::adapter) fn probe_player_transform(player: *mut Character) {
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

pub(in crate::api::game::adapter) fn sample_vehicle(
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

pub(in crate::api::game::adapter) fn read_vehicle_vector(
    vehicle: *mut Vehicle,
    slot: usize,
) -> Option<[f32; 3]> {
    let target = vehicle_virtual_function(vehicle, slot)?;
    let get_vector: GetVehicleVector = unsafe { std::mem::transmute(target) };
    let mut vector = [0.0; 3];
    unsafe { get_vector(vehicle, &mut vector) };
    vector
        .iter()
        .all(|component| component.is_finite())
        .then_some(vector)
}

pub(in crate::api::game::adapter) fn read_vehicle_last_frame_vector(
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

pub(in crate::api::game::adapter) fn vehicle_virtual_function(
    vehicle: *mut Vehicle,
    slot: usize,
) -> Option<usize> {
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

pub(in crate::api::game::adapter) fn rotation_xyz_degrees(matrix: &Matrix4x4) -> Option<[f32; 3]> {
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

pub(in crate::api::game::adapter) fn horizontal_forward(matrix: &Matrix4x4) -> Option<[f32; 3]> {
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
