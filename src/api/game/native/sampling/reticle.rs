//! Bounded native execution of owned Rhai raycast requests.
//!
//! Scripts select only mapped filter profiles and provide copied ray geometry.
//! The native query context is created and discarded inside this module; no
//! engine pointer crosses back into the script or ECS runtime.
use super::*;
use crate::api::game::abi::game::world_query::C_PLAYER_AIM_RAYCAST_FILTER_VFTABLE_RVA;
use crate::api::scripts::{ScriptRaycastCompletion, ScriptRaycastFilter, ScriptRaycastRequest};
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct QueryKey {
    origin: [u32; 3],
    direction: [u32; 3],
    max_distance: u32,
    filter: ScriptRaycastFilter,
}

impl From<&ScriptRaycastRequest> for QueryKey {
    fn from(request: &ScriptRaycastRequest) -> Self {
        Self {
            origin: request.origin.map(f32::to_bits),
            direction: request.direction.map(f32::to_bits),
            max_distance: request.max_distance.to_bits(),
            filter: request.filter,
        }
    }
}

/// Shared result resource for one accepted game frame. It is populated lazily
/// on demand and dropped after that frame's script requests are completed.
#[derive(Default)]
struct FrameRaycastResults(HashMap<QueryKey, ScriptRaycastCompletion>);

impl FrameRaycastResults {
    fn resolve(&mut self, request: ScriptRaycastRequest) -> ScriptRaycastCompletion {
        let key = QueryKey::from(&request);
        if let Some(shared) = self.0.get(&key) {
            let mut completion = shared.clone();
            completion.ticket = request.ticket;
            completion.source = request.source;
            return completion;
        }
        let completion = execute_script_raycast(request);
        self.0.insert(key, completion.clone());
        completion
    }
}

/// Execute only demanded requests and share equivalent results in this game frame.
/// The shared result table expires with the frame, avoiding stale camera hits.
pub(in crate::api::game::native) fn execute_script_raycast_batch(
    requests: Vec<ScriptRaycastRequest>,
    mut should_continue: impl FnMut() -> bool,
    mut complete: impl FnMut(ScriptRaycastCompletion),
) {
    if requests.is_empty() {
        return;
    }
    if requests.len() == 1 {
        if should_continue() {
            complete(execute_script_raycast(
                requests.into_iter().next().expect("one request"),
            ));
        }
        return;
    }
    let mut frame_results = FrameRaycastResults::default();
    for request in requests {
        if !should_continue() {
            break;
        }
        complete(frame_results.resolve(request));
    }
}

struct QueryApi {
    base: *mut c_void,
    initialize: WorldQueryContextInitializeFn,
    set_option: RaycastFilterSetOptionFn,
    query: WorldQueryHelperFn,
}

#[derive(Clone, Copy)]
struct QueryApiAddresses {
    base_pointer: usize,
    initialize: usize,
    set_option: usize,
    query: usize,
    static_vtable: usize,
    player_aim_vtable: usize,
}

static QUERY_API_ADDRESSES: OnceLock<QueryApiAddresses> = OnceLock::new();
static LAST_WORLD_QUERY_MICROS: AtomicU64 = AtomicU64::new(0);

pub(in crate::api::game::native) fn last_world_query_micros() -> u64 {
    LAST_WORLD_QUERY_MICROS.load(Ordering::Relaxed)
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
    let addresses = if let Some(addresses) = QUERY_API_ADDRESSES.get() {
        *addresses
    } else {
        let module = unsafe { GetModuleHandleW(hudhook::windows::core::PCWSTR::null()) }.ok()?;
        let image_base = module.0 as usize;
        if image_base != STEAM_IMAGE_BASE {
            return None;
        }
        let addresses = QueryApiAddresses {
            base_pointer: image_base.checked_add(WORLD_QUERY_BASE_POINTER_RVA)?,
            initialize: image_base.checked_add(WORLD_QUERY_CONTEXT_INITIALIZER_RVA)?,
            set_option: image_base.checked_add(RAYCAST_FILTER_SET_OPTION_RVA)?,
            query: image_base.checked_add(WORLD_QUERY_HELPER_RVA)?,
            static_vtable: image_base.checked_add(C_STATIC_ONLY_RAYCAST_FILTER_VFTABLE_RVA)?,
            player_aim_vtable: image_base.checked_add(C_PLAYER_AIM_RAYCAST_FILTER_VFTABLE_RVA)?,
        };
        if ![addresses.initialize, addresses.set_option, addresses.query]
            .into_iter()
            .all(is_readable_executable_page)
            || !readable_span(addresses.static_vtable, size_of::<usize>())
            || !readable_span(addresses.player_aim_vtable, size_of::<usize>())
            || !readable_span(addresses.base_pointer, size_of::<*mut c_void>())
        {
            return None;
        }
        let _ = QUERY_API_ADDRESSES.set(addresses);
        *QUERY_API_ADDRESSES.get()?
    };
    let base = unsafe { ptr::read_volatile(addresses.base_pointer as *const *mut c_void) };
    if base.is_null() || !readable_span(base as usize, 1) {
        return None;
    }
    Some((
        QueryApi {
            base,
            initialize: unsafe { std::mem::transmute(addresses.initialize) },
            set_option: unsafe { std::mem::transmute(addresses.set_option) },
            query: unsafe { std::mem::transmute(addresses.query) },
        },
        addresses.static_vtable,
        addresses.player_aim_vtable,
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
    let query_started = Instant::now();
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
    LAST_WORLD_QUERY_MICROS.store(
        query_started
            .elapsed()
            .as_micros()
            .min(u128::from(u64::MAX)) as u64,
        Ordering::Relaxed,
    );
    let fraction = (output.hit_fraction.is_finite() && (0.0..=1.0).contains(&output.hit_fraction))
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
