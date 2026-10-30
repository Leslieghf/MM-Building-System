//! Camera, reticle ray, and raycast-candidate sampling.
use super::*;

pub(in crate::api::game::adapter) fn sample_camera_matrix() -> Option<Matrix4x4> {
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

pub(in crate::api::game::adapter) fn sample_reticle_ray_if_due(
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
        if output.scalar_14 < 1.0
            && let Some(point) = ray.point_at(RETICLE_RAY_MAX_DISTANCE * output.scalar_14)
        {
            sample.candidate_position = Some([point.x, point.y, point.z]);
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
            if player_aim_output.scalar_14 < 1.0
                && let Some(point) =
                    ray.point_at(RETICLE_RAY_MAX_DISTANCE * player_aim_output.scalar_14)
            {
                sample.player_aim_candidate_position = Some([point.x, point.y, point.z]);
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
            if ignore_characters_vehicles_output.scalar_14 < 1.0
                && let Some(point) = ray.point_at(
                    RETICLE_RAY_MAX_DISTANCE * ignore_characters_vehicles_output.scalar_14,
                )
            {
                sample.ignore_characters_vehicles_candidate_position =
                    Some([point.x, point.y, point.z]);
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
                if ignore_player_output.scalar_14 < 1.0
                    && let Some(point) =
                        ray.point_at(RETICLE_RAY_MAX_DISTANCE * ignore_player_output.scalar_14)
                {
                    sample.ignore_player_owner_candidate_position =
                        Some([point.x, point.y, point.z]);
                }
            }
        }
    }

    Some(sample)
}

pub(in crate::api::game::adapter) fn read_query_candidate_shape(
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
