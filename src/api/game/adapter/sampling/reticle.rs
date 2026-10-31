//! Rate-limited reticle ray sampling with comparable filter-policy outputs.
//!
//! A validated player/camera ray is queried with the static, player-aim,
//! character/vehicle exclusion, and player-owner exclusion policies. Each
//! result is copied into telemetry with availability tracked separately from
//! hit/miss state. Native contexts and temporary owner storage stay alive for
//! the duration of their query and are never published.
use super::*;

struct QueryApi {
    base: *mut c_void,
    initialize: WorldQueryContextInitializeFn,
    set_option: RaycastFilterSetOptionFn,
    query: WorldQueryHelperFn,
}

#[derive(Clone, Copy)]
struct FilterConfig {
    vtable: usize,
    label: &'static [u8],
    category_mask: Option<u32>,
    ignored_owner: Option<usize>,
}

#[derive(Default)]
struct FilterResult {
    state_present: bool,
    fraction: Option<f32>,
    position: Option<[f32; 3]>,
    shape: Option<RaycastCandidateShape>,
}

pub(in crate::api::game::adapter) fn sample_reticle_ray_if_due(
    camera_matrix: &Matrix4x4,
    player: *mut Character,
) -> Option<ReticleRaySample> {
    if !reticle_sample_is_due() {
        return None;
    }
    let Some(ray) = WorldQueryRayInput::from_camera_matrix(camera_matrix) else {
        return Some(ReticleRaySample::default());
    };
    let mut sample = ReticleRaySample {
        origin: Some([ray.origin.x, ray.origin.y, ray.origin.z]),
        direction: Some([ray.direction.x, ray.direction.y, ray.direction.z]),
        ..ReticleRaySample::default()
    };
    let Some((image_base, api, static_vtable)) = resolve_query_api() else {
        return Some(sample);
    };

    sample_static_only(&mut sample, &api, static_vtable, &ray);
    let Some(player_aim_vtable) = image_base.checked_add(C_PLAYER_AIM_RAYCAST_FILTER_VFTABLE_RVA)
    else {
        return Some(sample);
    };
    sample_player_aim(&mut sample, &api, player_aim_vtable, &ray);
    let Some(ignored_actors_vtable) =
        image_base.checked_add(C_IGNORE_CHARACTERS_AND_VEHICLES_RAYCAST_FILTER_VFTABLE_RVA)
    else {
        return Some(sample);
    };
    sample_ignoring_actors(&mut sample, &api, ignored_actors_vtable, &ray);
    let Some(owner_filter_vtable) = image_base.checked_add(C_IGNORE_FILTER_VFTABLE_RVA) else {
        return Some(sample);
    };
    sample_ignoring_player(&mut sample, &api, owner_filter_vtable, &ray, player);
    Some(sample)
}

fn reticle_sample_is_due() -> bool {
    let now = Instant::now();
    let mut last_sample = LAST_RETICLE_RAY_SAMPLE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if last_sample.is_some_and(|last| now.duration_since(last) < RETICLE_RAY_SAMPLE_INTERVAL) {
        return false;
    }
    *last_sample = Some(now);
    true
}

fn resolve_query_api() -> Option<(usize, QueryApi, usize)> {
    let module = unsafe { GetModuleHandleW(hudhook::windows::core::PCWSTR::null()) }.ok()?;
    let image_base = module.0 as usize;
    if image_base != STEAM_IMAGE_BASE {
        return None;
    }
    let initializer_address = image_base.checked_add(WORLD_QUERY_CONTEXT_INITIALIZER_RVA)?;
    let option_setter_address = image_base.checked_add(RAYCAST_FILTER_SET_OPTION_RVA)?;
    let query_address = image_base.checked_add(WORLD_QUERY_HELPER_RVA)?;
    let filter_vtable = image_base.checked_add(C_STATIC_ONLY_RAYCAST_FILTER_VFTABLE_RVA)?;
    if ![initializer_address, option_setter_address, query_address]
        .into_iter()
        .all(is_readable_executable_page)
    {
        return None;
    }
    let query_base_global = image_base.checked_add(WORLD_QUERY_BASE_POINTER_RVA)?;
    if !readable_span(query_base_global, size_of::<*mut c_void>()) {
        return None;
    }
    let base = unsafe { ptr::read_volatile(query_base_global as *const *mut c_void) };
    if base.is_null() || !readable_span(base as usize, 1) {
        return None;
    }
    Some((
        image_base,
        QueryApi {
            base,
            initialize: unsafe { std::mem::transmute(initializer_address) },
            set_option: unsafe { std::mem::transmute(option_setter_address) },
            query: unsafe { std::mem::transmute(query_address) },
        },
        filter_vtable,
    ))
}

fn sample_static_only(
    sample: &mut ReticleRaySample,
    api: &QueryApi,
    vtable: usize,
    ray: &WorldQueryRayInput,
) {
    let config = FilterConfig {
        vtable,
        label: b"MadMaxModdingReticleRay\0",
        category_mask: Some(3),
        ignored_owner: None,
    };
    if let Some(result) = query_with_filter(api, ray, config) {
        sample.query_available = true;
        sample.query_state_present = result.state_present;
        sample.fraction = result.fraction;
        sample.candidate_position = result.position;
        sample.candidate_shape = result.shape;
    }
}

fn sample_player_aim(
    sample: &mut ReticleRaySample,
    api: &QueryApi,
    vtable: usize,
    ray: &WorldQueryRayInput,
) {
    let config = FilterConfig {
        vtable,
        label: b"MadMaxModdingReticleRay\0",
        category_mask: None,
        ignored_owner: None,
    };
    if let Some(result) = query_with_filter(api, ray, config) {
        sample.player_aim_query_available = true;
        sample.player_aim_query_state_present = result.state_present;
        sample.player_aim_fraction = result.fraction;
        sample.player_aim_candidate_position = result.position;
        sample.player_aim_candidate_shape = result.shape;
    }
}

fn sample_ignoring_actors(
    sample: &mut ReticleRaySample,
    api: &QueryApi,
    vtable: usize,
    ray: &WorldQueryRayInput,
) {
    let config = FilterConfig {
        vtable,
        label: b"MadMaxModdingReticleIgnoreCharsVehicles\0",
        category_mask: None,
        ignored_owner: None,
    };
    if let Some(result) = query_with_filter(api, ray, config) {
        sample.ignore_characters_vehicles_query_available = true;
        sample.ignore_characters_vehicles_query_state_present = result.state_present;
        sample.ignore_characters_vehicles_fraction = result.fraction;
        sample.ignore_characters_vehicles_candidate_position = result.position;
        sample.ignore_characters_vehicles_candidate_shape = result.shape;
    }
}

fn sample_ignoring_player(
    sample: &mut ReticleRaySample,
    api: &QueryApi,
    vtable: usize,
    ray: &WorldQueryRayInput,
    player: *mut Character,
) {
    if player.is_null() || !readable_span(player as usize, size_of::<usize>()) {
        return;
    }
    let config = FilterConfig {
        vtable,
        label: b"MadMaxModdingReticleIgnorePlayerOwner\0",
        category_mask: None,
        ignored_owner: Some(player as usize),
    };
    if let Some(result) = query_with_filter(api, ray, config) {
        sample.ignore_player_owner_query_available = true;
        sample.ignore_player_owner_query_state_present = result.state_present;
        sample.ignore_player_owner_fraction = result.fraction;
        sample.ignore_player_owner_candidate_position = result.position;
        sample.ignore_player_owner_candidate_shape = result.shape;
    }
}

fn query_with_filter(
    api: &QueryApi,
    ray: &WorldQueryRayInput,
    config: FilterConfig,
) -> Option<FilterResult> {
    let mut context = RaycastFilterContextStorage([0; 0xD0]);
    let context_pointer = context.0.as_mut_ptr().cast::<c_void>();
    if unsafe { (api.initialize)(context_pointer, 0, 0, 0) }.is_null() {
        return None;
    }
    // Keep the owner backing storage alive until the native query has returned.
    let ignored_owners = config.ignored_owner.map(|owner| [owner]);
    initialize_filter_context(
        &mut context,
        context_pointer,
        config,
        ignored_owners.as_ref(),
    );
    unsafe { (api.set_option)(context_pointer, 1) };
    let mut output = empty_query_output();
    let state_present = unsafe {
        (api.query)(
            api.base,
            config.label.as_ptr().cast(),
            ray,
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
    Some(filter_result(
        state_present,
        output.scalar_14,
        ray,
        &context,
    ))
}

fn initialize_filter_context(
    context: &mut RaycastFilterContextStorage,
    pointer: *mut c_void,
    config: FilterConfig,
    ignored_owners: Option<&[usize; 1]>,
) {
    unsafe {
        ptr::write_unaligned(pointer.cast::<usize>(), config.vtable);
        if let Some(mask) = config.category_mask {
            ptr::write_unaligned(
                context
                    .0
                    .as_mut_ptr()
                    .add(C_STATIC_ONLY_FILTER_CATEGORY_MASK_OFFSET)
                    .cast::<u32>(),
                mask,
            );
        }
        if let Some(ignored_owners) = ignored_owners {
            let begin = ignored_owners.as_ptr() as usize;
            let end = begin + size_of::<usize>();
            write_owner_vector(context, begin, end);
        }
    }
}

fn write_owner_vector(context: &mut RaycastFilterContextStorage, begin: usize, end: usize) {
    for (offset, value) in [
        (C_IGNORE_FILTER_OWNER_VECTOR_BEGIN_OFFSET, begin),
        (C_IGNORE_FILTER_OWNER_VECTOR_END_OFFSET, end),
        (C_IGNORE_FILTER_OWNER_VECTOR_CAPACITY_OFFSET, end),
    ] {
        unsafe {
            ptr::write_unaligned(context.0.as_mut_ptr().add(offset).cast::<usize>(), value);
        }
    }
}

fn empty_query_output() -> WorldQueryOutputRecord {
    WorldQueryOutputRecord {
        qword_00: 0,
        vector_08: Vector3::default(),
        scalar_14: 1.0,
        qword_18: 0,
        qword_20: 0,
        qword_28: 0,
        words_30: [0; 8],
    }
}

fn filter_result(
    state_present: bool,
    fraction: f32,
    ray: &WorldQueryRayInput,
    context: &RaycastFilterContextStorage,
) -> FilterResult {
    let fraction = (fraction.is_finite() && (0.0..=1.0).contains(&fraction)).then_some(fraction);
    let position = fraction
        .filter(|fraction| *fraction < 1.0)
        .and_then(|fraction| ray.point_at(RETICLE_RAY_MAX_DISTANCE * fraction))
        .map(|point| [point.x, point.y, point.z]);
    FilterResult {
        state_present,
        fraction,
        position,
        shape: read_query_candidate_shape(context),
    }
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
