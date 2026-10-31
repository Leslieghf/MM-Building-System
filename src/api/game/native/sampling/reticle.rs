//! Bounded native execution of owned Rhai raycast requests.
//!
//! Scripts select only mapped filter profiles and provide copied ray geometry.
//! The native query context is created and discarded inside this module; no
//! engine pointer crosses back into the script or ECS runtime.
use super::*;
use crate::api::game::abi::game::world_query::C_PLAYER_AIM_RAYCAST_FILTER_VFTABLE_RVA;
use crate::api::scripts::{ScriptRaycastCompletion, ScriptRaycastFilter, ScriptRaycastRequest};

struct QueryApi {
    base: *mut c_void,
    initialize: WorldQueryContextInitializeFn,
    set_option: RaycastFilterSetOptionFn,
    query: WorldQueryHelperFn,
}

#[derive(Clone, Copy)]
struct FilterConfig {
    vtable: usize,
    category_mask: Option<u32>,
}

struct FilterResult {
    state_present: bool,
    fraction: Option<f32>,
    position: Option<[f32; 3]>,
}

pub(in crate::api::game::native) fn execute_script_raycast(
    request: ScriptRaycastRequest,
) -> ScriptRaycastCompletion {
    let unavailable = ScriptRaycastCompletion {
        ticket: request.ticket,
        source: request.source.clone(),
        available: false,
        state_present: false,
        fraction: None,
        hit_position: None,
    };
    let Some((api, static_vtable, player_aim_vtable)) = resolve_query_api() else {
        return unavailable;
    };
    let ray = WorldQueryRayInput {
        origin: Vector3::new(request.origin[0], request.origin[1], request.origin[2]),
        direction: Vector3::new(
            request.direction[0],
            request.direction[1],
            request.direction[2],
        ),
    };
    let config = match request.filter {
        ScriptRaycastFilter::StaticOnly => FilterConfig {
            vtable: static_vtable,
            category_mask: Some(3),
        },
        ScriptRaycastFilter::PlayerAim => FilterConfig {
            vtable: player_aim_vtable,
            category_mask: None,
        },
    };
    let Some(result) = query_with_filter(&api, &ray, request.max_distance, config) else {
        return unavailable;
    };
    ScriptRaycastCompletion {
        ticket: request.ticket,
        source: request.source,
        available: true,
        state_present: result.state_present,
        fraction: result.fraction,
        hit_position: result.position,
    }
}

fn resolve_query_api() -> Option<(QueryApi, usize, usize)> {
    let module = unsafe { GetModuleHandleW(hudhook::windows::core::PCWSTR::null()) }.ok()?;
    let image_base = module.0 as usize;
    if image_base != STEAM_IMAGE_BASE {
        return None;
    }
    let initializer_address = image_base.checked_add(WORLD_QUERY_CONTEXT_INITIALIZER_RVA)?;
    let option_setter_address = image_base.checked_add(RAYCAST_FILTER_SET_OPTION_RVA)?;
    let query_address = image_base.checked_add(WORLD_QUERY_HELPER_RVA)?;
    let static_vtable = image_base.checked_add(C_STATIC_ONLY_RAYCAST_FILTER_VFTABLE_RVA)?;
    let player_aim_vtable = image_base.checked_add(C_PLAYER_AIM_RAYCAST_FILTER_VFTABLE_RVA)?;
    if ![initializer_address, option_setter_address, query_address]
        .into_iter()
        .all(is_readable_executable_page)
    {
        return None;
    }
    if !readable_span(static_vtable, size_of::<usize>())
        || !readable_span(player_aim_vtable, size_of::<usize>())
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
        QueryApi {
            base,
            initialize: unsafe { std::mem::transmute(initializer_address) },
            set_option: unsafe { std::mem::transmute(option_setter_address) },
            query: unsafe { std::mem::transmute(query_address) },
        },
        static_vtable,
        player_aim_vtable,
    ))
}

fn query_with_filter(
    api: &QueryApi,
    ray: &WorldQueryRayInput,
    max_distance: f32,
    config: FilterConfig,
) -> Option<FilterResult> {
    let mut context = RaycastFilterContextStorage([0; 0xD0]);
    let context_pointer = context.0.as_mut_ptr().cast::<c_void>();
    if unsafe { (api.initialize)(context_pointer, 0, 0, 0) }.is_null() {
        return None;
    }
    unsafe {
        ptr::write_unaligned(context_pointer.cast::<usize>(), config.vtable);
        if let Some(category_mask) = config.category_mask {
            ptr::write_unaligned(
                context
                    .0
                    .as_mut_ptr()
                    .add(C_STATIC_ONLY_FILTER_CATEGORY_MASK_OFFSET)
                    .cast::<u32>(),
                category_mask,
            );
        }
        (api.set_option)(context_pointer, 1);
    }
    let mut output = WorldQueryOutputRecord {
        qword_00: 0,
        surface_normal: Vector3::default(),
        hit_fraction: 1.0,
        qword_18: 0,
        qword_20: 0,
        qword_28: 0,
        words_30: [0; 8],
    };
    let state_present = unsafe {
        (api.query)(
            api.base,
            b"MadMaxModdingRhaiRaycast\0".as_ptr().cast(),
            ray,
            0.0,
            max_distance,
            &mut output,
            context_pointer,
            0,
            false,
            0,
            ptr::null(),
        )
    };
    let fraction = (output.hit_fraction.is_finite()
        && (0.0..=1.0).contains(&output.hit_fraction))
    .then_some(output.hit_fraction);
    let position = fraction
        .filter(|fraction| *fraction < 1.0)
        .and_then(|fraction| ray.point_at(max_distance * fraction))
        .map(|point| [point.x, point.y, point.z]);
    Some(FilterResult {
        state_present,
        fraction,
        position,
    })
}
