//! Bounded capture of candidate shapes from world-query outputs.
use super::*;

pub(in crate::api::game::adapter) fn is_reticle_query_label_bytes(
    label: &[u8; WORLD_QUERY_LABEL_CAPACITY],
) -> bool {
    [
        &b"MadMaxModdingReticleRay\0"[..],
        &b"MadMaxModdingReticleIgnoreCharsVehicles\0"[..],
        &b"MadMaxModdingReticleIgnorePlayerOwner\0"[..],
    ]
    .iter()
    .any(|expected| {
        label
            .iter()
            .zip(*expected)
            .all(|(actual, expected)| *actual == *expected)
    })
}

pub(in crate::api::game::adapter) fn is_reticle_query_label(sample: &WorldQuerySample) -> bool {
    if !sample.rdx_label_valid.load(Ordering::Acquire) {
        return false;
    }
    [
        &b"MadMaxModdingReticleRay\0"[..],
        &b"MadMaxModdingReticleIgnoreCharsVehicles\0"[..],
        &b"MadMaxModdingReticleIgnorePlayerOwner\0"[..],
    ]
    .iter()
    .any(|expected| {
        expected
            .iter()
            .enumerate()
            .all(|(index, expected)| sample.rdx_label[index].load(Ordering::Relaxed) == *expected)
    })
}

pub(in crate::api::game::adapter) fn register_reticle_query_caller(
    return_address: usize,
) -> Option<usize> {
    if return_address == 0 {
        return None;
    }
    for (index, slot) in RETICLE_QUERY_CALLERS.iter().enumerate() {
        let observed = slot.load(Ordering::Acquire);
        if observed == return_address {
            return Some(index);
        }
        if observed == 0
            && slot
                .compare_exchange(0, return_address, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
        {
            return Some(index);
        }
    }
    None
}

pub(in crate::api::game::adapter) fn begin_raycast_filter_hit_capture(
    filter: *mut c_void,
    candidate_chain: *mut c_void,
) -> Option<RaycastFilterHitCapture> {
    if WORLD_QUERY_ACTIVE_SAMPLES.load(Ordering::Relaxed) == 0 {
        return None;
    }
    let frame = WORLD_QUERY_CANDIDATE_STACK
        .try_with(|stack| stack.try_borrow().ok().and_then(|stack| stack.current()))
        .ok()
        .flatten()?;
    let sample = WORLD_QUERY_SAMPLES.get(frame.sample_index)?;
    if !sample.ready.load(Ordering::Acquire) || !is_reticle_query_label(sample) {
        return None;
    }
    let callback_slot = frame.callback_slot?;
    if callback_slot >= RAYCAST_FILTER_CAPTURE_BROADPHASE_SLOTS {
        return None;
    }
    let candidate_sample = sample.candidate_samples.get(callback_slot)?;
    let filter_address = filter as usize;
    let chain_address = candidate_chain as usize;
    if filter_address < 0x10000
        || !readable_span(filter_address, 0x88)
        || chain_address < 0x10000
        || !readable_span(chain_address, size_of::<usize>())
    {
        return None;
    }

    let candidate_node = unsafe { ptr::read_unaligned(chain_address as *const usize) };
    if candidate_node < 0x10000 || !readable_span(candidate_node, 0x30) {
        return None;
    }
    let candidate_info = unsafe { ptr::read_unaligned((candidate_node + 0x18) as *const u32) };
    let candidate_owner = unsafe { ptr::read_unaligned((candidate_node + 0x28) as *const usize) };
    let filter_vtable = unsafe { ptr::read_unaligned(filter_address as *const usize) };
    let exclude_filter_info_mask =
        unsafe { ptr::read_unaligned((filter_address + 0x70) as *const u32) };
    let required_filter_info_mask =
        unsafe { ptr::read_unaligned((filter_address + 0x74) as *const u32) };
    let filter_ignored_owner = if filter_vtable == STEAM_IMAGE_BASE + C_IGNORE_FILTER_VFTABLE_RVA
        && readable_span(filter_address, 0xA8)
    {
        let begin_pointer = unsafe {
            ptr::read_unaligned(
                (filter_address + C_IGNORE_FILTER_OWNER_VECTOR_BEGIN_OFFSET) as *const usize,
            )
        };
        let end_pointer = unsafe {
            ptr::read_unaligned(
                (filter_address + C_IGNORE_FILTER_OWNER_VECTOR_END_OFFSET) as *const usize,
            )
        };
        if begin_pointer < end_pointer
            && end_pointer - begin_pointer <= 0x1000
            && readable_span(begin_pointer, size_of::<usize>())
        {
            unsafe { ptr::read_unaligned(begin_pointer as *const usize) }
        } else {
            0
        }
    } else {
        0
    };
    let selected_shape_before =
        unsafe { ptr::read_unaligned((filter_address + 0x80) as *const usize) };

    let hit = candidate_sample.ray_filter_hits.iter().find(|hit| {
        hit.state
            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    })?;
    hit.filter_context.store(filter_address, Ordering::Relaxed);
    hit.filter_vtable.store(filter_vtable, Ordering::Relaxed);
    hit.exclude_filter_info_mask
        .store(exclude_filter_info_mask, Ordering::Relaxed);
    hit.required_filter_info_mask
        .store(required_filter_info_mask, Ordering::Relaxed);
    hit.candidate_node.store(candidate_node, Ordering::Relaxed);
    hit.candidate_filter_info
        .store(candidate_info, Ordering::Relaxed);
    hit.candidate_owner
        .store(candidate_owner, Ordering::Relaxed);
    hit.filter_ignored_owner
        .store(filter_ignored_owner, Ordering::Relaxed);
    hit.selected_shape_before
        .store(selected_shape_before, Ordering::Relaxed);
    Some(RaycastFilterHitCapture {
        sample: hit,
        filter_context: filter_address,
    })
}

pub(in crate::api::game::adapter) fn finish_raycast_filter_hit_capture(
    capture: RaycastFilterHitCapture,
) {
    if readable_span(capture.filter_context, 0x88) {
        let selected_shape_after =
            unsafe { ptr::read_unaligned((capture.filter_context + 0x80) as *const usize) };
        capture
            .sample
            .selected_shape_after
            .store(selected_shape_after, Ordering::Relaxed);
    }
    capture.sample.state.store(2, Ordering::Release);
}

pub(in crate::api::game::adapter) fn world_query_candidate_capture_slot(
    callback_ordinal: u32,
) -> Option<usize> {
    match callback_ordinal {
        0 => Some(0),
        1 => Some(1),
        2 => Some(2),
        4 => Some(3),
        8 => Some(4),
        16 => Some(5),
        32 => Some(6),
        _ => None,
    }
}

pub(in crate::api::game::adapter) fn current_world_query_sample_index() -> Option<usize> {
    WORLD_QUERY_RETURN_STACK
        .try_with(|stack| {
            stack
                .try_borrow()
                .ok()
                .and_then(|stack| stack.current_sample_index())
        })
        .ok()
        .flatten()
}

pub(in crate::api::game::adapter) unsafe extern "system" fn native_world_query_bounds_predicate_tap(
    owner: *mut c_void,
    point: *const f32,
    radius: f32,
) -> u8 {
    let original = ORIGINAL_WORLD_QUERY_BOUNDS_PREDICATE.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    let original: WorldQueryBoundsPredicate = unsafe { std::mem::transmute(original) };
    let sample_index = current_world_query_sample_index()
        .filter(|sample_index| *sample_index < WORLD_QUERY_SAMPLES.len());
    let input_point = sample_index.and_then(|_| {
        (!point.is_null() && readable_span(point as usize, size_of::<[f32; 3]>()))
            .then(|| unsafe { ptr::read_unaligned(point.cast::<[f32; 3]>()) })
    });
    let result = unsafe { original(owner, point, radius) };

    let Some(sample_index) = sample_index else {
        return result;
    };
    let Some(sample) = WORLD_QUERY_SAMPLES.get(sample_index) else {
        return result;
    };
    let ordinal = sample.bounds_check_count.fetch_add(1, Ordering::Relaxed) as usize;
    let Some(capture) = sample.bounds_checks.get(ordinal) else {
        return result;
    };
    let Some(input_point) = input_point else {
        return result;
    };
    for (destination, value) in capture.point.iter().zip(input_point) {
        destination.store(value.to_bits(), Ordering::Relaxed);
    }
    capture.radius.store(radius.to_bits(), Ordering::Relaxed);
    capture.result.store(result, Ordering::Relaxed);
    capture.valid.store(true, Ordering::Release);
    result
}

pub(in crate::api::game::adapter) fn pointer_at(address: usize) -> usize {
    if address >= 0x10000 && readable_span(address, size_of::<usize>()) {
        unsafe { ptr::read_unaligned(address as *const usize) }
    } else {
        0
    }
}

pub(in crate::api::game::adapter) fn candidate_virtual_target(object: usize, slot: usize) -> usize {
    pointer_at(pointer_at(object).checked_add(slot).unwrap_or_default())
}

pub(in crate::api::game::adapter) fn begin_world_query_candidate_capture(
    caster: *mut c_void,
    handle: *mut c_void,
    cast_index: u32,
) -> Option<WorldQueryCandidateBefore> {
    if WORLD_QUERY_ACTIVE_SAMPLES.load(Ordering::Relaxed) == 0 {
        return None;
    }
    let sample_index = current_world_query_sample_index()?;
    let sample = WORLD_QUERY_SAMPLES.get(sample_index)?;
    if !sample.ready.load(Ordering::Acquire) {
        return None;
    }
    let callback_ordinal = sample
        .candidate_callback_count
        .fetch_add(1, Ordering::Relaxed);
    let Some(capture_slot) = world_query_candidate_capture_slot(callback_ordinal) else {
        return Some(WorldQueryCandidateBefore {
            sample_index,
            callback_slot: None,
            ray_record: 0,
            collector_state_address: 0,
        });
    };
    let candidate_sample = sample.candidate_samples.get(capture_slot)?;
    candidate_sample.ready.store(false, Ordering::Relaxed);
    candidate_sample
        .group_filter_result_valid
        .store(false, Ordering::Relaxed);
    for filter_hit in &candidate_sample.ray_filter_hits {
        filter_hit.state.store(0, Ordering::Relaxed);
    }
    candidate_sample
        .ray_record_before_valid
        .store(false, Ordering::Relaxed);
    candidate_sample
        .ray_record_after_valid
        .store(false, Ordering::Relaxed);
    candidate_sample
        .collector_state_before_valid
        .store(false, Ordering::Relaxed);
    candidate_sample
        .collector_state_after_valid
        .store(false, Ordering::Relaxed);

    let caster_address = caster as usize;
    let handle_address = handle as usize;
    let output_base = pointer_at(caster_address.checked_add(0x18).unwrap_or_default());
    let stride = caster_address
        .checked_add(0x20)
        .filter(|address| readable_span(*address, size_of::<u32>()))
        .map(|address| unsafe { ptr::read_unaligned(address as *const u32) as usize })
        .unwrap_or_default();
    let ray_record = output_base
        .checked_add(stride.checked_mul(cast_index as usize).unwrap_or_default())
        .unwrap_or_default();
    let ray_record_before_valid = ray_record >= 0x10000 && readable_span(ray_record, 0x30);
    let ray_record_before = if ray_record_before_valid {
        unsafe { ptr::read_unaligned(ray_record as *const [u64; 6]) }
    } else {
        [0; 6]
    };

    let handle_relative = handle_address
        .checked_add(5)
        .filter(|address| readable_span(*address, 1))
        .map(|address| unsafe { ptr::read_unaligned(address as *const i8) as isize })
        .and_then(|offset| handle_address.checked_add_signed(offset))
        .unwrap_or_default();
    let candidate_object = pointer_at(handle_relative);
    let subobject_offset = handle_relative
        .checked_add(0x20)
        .filter(|address| readable_span(*address, 1))
        .map(|address| unsafe { ptr::read_unaligned(address as *const i8) as isize })
        .unwrap_or_default();
    let object_subobject = handle_relative
        .checked_add_signed(subobject_offset)
        .unwrap_or_default();
    let object_vtable = pointer_at(candidate_object);
    let subobject_vtable = pointer_at(object_subobject);
    let callback_30 = candidate_virtual_target(candidate_object, 0x30);
    let callback_80 = candidate_virtual_target(object_subobject, 0x80);
    let object_gate_word = candidate_object
        .checked_add(0x28)
        .filter(|address| readable_span(*address, size_of::<u64>()))
        .map(|address| unsafe { ptr::read_unaligned(address as *const u64) });
    let object_local_transform = candidate_object
        .checked_add(0x140)
        .filter(|address| readable_span(*address, size_of::<[u64; 6]>()))
        .map(|address| unsafe { ptr::read_unaligned(address as *const [u64; 6]) });
    let handle_relative_gate_word = handle_relative
        .checked_add(0x28)
        .filter(|address| readable_span(*address, size_of::<u64>()))
        .map(|address| unsafe { ptr::read_unaligned(address as *const u64) });
    let handle_relative_transform = handle_relative
        .checked_add(0x140)
        .filter(|address| readable_span(*address, size_of::<[u64; 6]>()))
        .map(|address| unsafe { ptr::read_unaligned(address as *const [u64; 6]) });
    let subobject_gate_word = object_subobject
        .checked_add(0x28)
        .filter(|address| readable_span(*address, size_of::<u64>()))
        .map(|address| unsafe { ptr::read_unaligned(address as *const u64) });
    let subobject_transform = object_subobject
        .checked_add(0x140)
        .filter(|address| readable_span(*address, size_of::<[u64; 6]>()))
        .map(|address| unsafe { ptr::read_unaligned(address as *const [u64; 6]) });

    candidate_sample
        .callback_index
        .store(callback_ordinal, Ordering::Relaxed);
    candidate_sample
        .caster
        .store(caster_address, Ordering::Relaxed);
    candidate_sample
        .handle
        .store(handle_address, Ordering::Relaxed);
    candidate_sample
        .cast_index
        .store(cast_index, Ordering::Relaxed);
    candidate_sample
        .handle_relative
        .store(handle_relative, Ordering::Relaxed);
    candidate_sample
        .candidate_object
        .store(candidate_object, Ordering::Relaxed);
    candidate_sample
        .object_subobject
        .store(object_subobject, Ordering::Relaxed);
    candidate_sample
        .object_vtable
        .store(object_vtable, Ordering::Relaxed);
    candidate_sample
        .subobject_vtable
        .store(subobject_vtable, Ordering::Relaxed);
    candidate_sample
        .callback_30
        .store(callback_30, Ordering::Relaxed);
    candidate_sample
        .callback_80
        .store(callback_80, Ordering::Relaxed);
    candidate_sample
        .object_gate_word
        .store(object_gate_word.unwrap_or_default(), Ordering::Relaxed);
    candidate_sample
        .object_gate_word_valid
        .store(object_gate_word.is_some(), Ordering::Relaxed);
    if let Some(transform) = object_local_transform {
        for (destination, value) in candidate_sample
            .object_local_transform
            .iter()
            .zip(transform)
        {
            destination.store(value, Ordering::Relaxed);
        }
    }
    candidate_sample
        .object_local_transform_valid
        .store(object_local_transform.is_some(), Ordering::Relaxed);
    candidate_sample.handle_relative_gate_word.store(
        handle_relative_gate_word.unwrap_or_default(),
        Ordering::Relaxed,
    );
    candidate_sample
        .handle_relative_gate_word_valid
        .store(handle_relative_gate_word.is_some(), Ordering::Relaxed);
    if let Some(transform) = handle_relative_transform {
        for (destination, value) in candidate_sample
            .handle_relative_transform
            .iter()
            .zip(transform)
        {
            destination.store(value, Ordering::Relaxed);
        }
    }
    candidate_sample
        .handle_relative_transform_valid
        .store(handle_relative_transform.is_some(), Ordering::Relaxed);
    candidate_sample
        .subobject_gate_word
        .store(subobject_gate_word.unwrap_or_default(), Ordering::Relaxed);
    candidate_sample
        .subobject_gate_word_valid
        .store(subobject_gate_word.is_some(), Ordering::Relaxed);
    if let Some(transform) = subobject_transform {
        for (destination, value) in candidate_sample.subobject_transform.iter().zip(transform) {
            destination.store(value, Ordering::Relaxed);
        }
    }
    candidate_sample
        .subobject_transform_valid
        .store(subobject_transform.is_some(), Ordering::Relaxed);
    candidate_sample
        .ray_record
        .store(ray_record, Ordering::Relaxed);
    for (destination, value) in candidate_sample
        .ray_record_before
        .iter()
        .zip(ray_record_before)
    {
        destination.store(value, Ordering::Relaxed);
    }
    candidate_sample
        .ray_record_before_valid
        .store(ray_record_before_valid, Ordering::Relaxed);
    let ray_input_base = pointer_at(caster_address.checked_add(0x08).unwrap_or_default());
    let ray_input_record = ray_input_base
        .checked_add(stride.checked_mul(cast_index as usize).unwrap_or_default())
        .unwrap_or_default();
    candidate_sample
        .ray_input_record
        .store(ray_input_record, Ordering::Relaxed);
    let ray_input_valid =
        ray_input_record >= 0x10000 && readable_span(ray_input_record, size_of::<[u64; 6]>());
    if ray_input_valid {
        let words = unsafe { ptr::read_unaligned(ray_input_record as *const [u64; 6]) };
        for (destination, value) in candidate_sample.ray_input_words.iter().zip(words) {
            destination.store(value, Ordering::Relaxed);
        }
    }
    candidate_sample
        .ray_input_valid
        .store(ray_input_valid, Ordering::Relaxed);
    let collector_state_address = caster_address.checked_add(0x30).unwrap_or_default();
    candidate_sample
        .collector_state_address
        .store(collector_state_address, Ordering::Relaxed);
    let collector_before_valid = collector_state_address >= 0x10000
        && readable_span(collector_state_address, size_of::<[u64; 8]>());
    if collector_before_valid {
        let words = unsafe { ptr::read_unaligned(collector_state_address as *const [u64; 8]) };
        for (destination, value) in candidate_sample.collector_state_before.iter().zip(words) {
            destination.store(value, Ordering::Relaxed);
        }
    }
    candidate_sample
        .collector_state_before_valid
        .store(collector_before_valid, Ordering::Relaxed);

    Some(WorldQueryCandidateBefore {
        sample_index,
        callback_slot: Some(capture_slot),
        ray_record,
        collector_state_address,
    })
}

pub(in crate::api::game::adapter) fn finish_world_query_candidate_capture(
    capture: WorldQueryCandidateBefore,
    fraction: f32,
) {
    let Some(sample) = WORLD_QUERY_SAMPLES.get(capture.sample_index) else {
        return;
    };
    if fraction.is_finite() && (0.0..=1.0).contains(&fraction) {
        sample
            .candidate_min_fraction
            .fetch_min(fraction.to_bits(), Ordering::Relaxed);
    }
    let Some(callback_slot) = capture.callback_slot else {
        return;
    };
    let Some(candidate) = sample.candidate_samples.get(callback_slot) else {
        return;
    };
    candidate
        .returned_fraction
        .store(fraction.to_bits(), Ordering::Relaxed);
    if capture.ray_record >= 0x10000 && readable_span(capture.ray_record, 0x30) {
        let words = unsafe { ptr::read_unaligned(capture.ray_record as *const [u64; 6]) };
        for (destination, value) in candidate.ray_record_after.iter().zip(words) {
            destination.store(value, Ordering::Relaxed);
        }
        candidate
            .ray_record_after_valid
            .store(true, Ordering::Relaxed);
    }
    if capture.collector_state_address >= 0x10000
        && readable_span(capture.collector_state_address, size_of::<[u64; 8]>())
    {
        let words =
            unsafe { ptr::read_unaligned(capture.collector_state_address as *const [u64; 8]) };
        for (destination, value) in candidate.collector_state_after.iter().zip(words) {
            destination.store(value, Ordering::Relaxed);
        }
        candidate
            .collector_state_after_valid
            .store(true, Ordering::Relaxed);
    }
    candidate.ready.store(true, Ordering::Release);
}

pub(in crate::api::game::adapter) unsafe extern "system" fn native_world_query_candidate_tap(
    caster: *mut c_void,
    handle: *mut c_void,
    cast_index: u32,
) -> f32 {
    let original = ORIGINAL_WORLD_QUERY_CANDIDATE.load(Ordering::Acquire);
    if original.is_null() {
        return 0.0;
    }
    let original: WorldQueryCandidateCallback = unsafe { std::mem::transmute(original) };
    let capture = begin_world_query_candidate_capture(caster, handle, cast_index);
    let pushed = capture.is_some_and(|capture| {
        WORLD_QUERY_CANDIDATE_STACK
            .try_with(|stack| {
                stack
                    .try_borrow_mut()
                    .ok()
                    .map(|mut stack| {
                        let _ = stack.push(WorldQueryCandidateFrame {
                            sample_index: capture.sample_index,
                            callback_slot: capture.callback_slot,
                        });
                        true
                    })
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    });
    let fraction = unsafe { original(caster, handle, cast_index) };
    if pushed {
        let _ = WORLD_QUERY_CANDIDATE_STACK.try_with(|stack| {
            if let Ok(mut stack) = stack.try_borrow_mut() {
                stack.pop();
            }
        });
    }
    if let Some(capture) = capture {
        finish_world_query_candidate_capture(capture, fraction);
    }
    fraction
}

pub(in crate::api::game::adapter) unsafe extern "system" fn raycast_filter_add_hit_tap(
    filter: *mut c_void,
    candidate_chain: *mut c_void,
    query_output: *mut c_void,
) {
    let original = ORIGINAL_RAYCAST_FILTER_ADD_HIT.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    let original: CRaycastFilterAddRayHit = unsafe { std::mem::transmute(original) };
    let capture = begin_raycast_filter_hit_capture(filter, candidate_chain);
    unsafe { original(filter, candidate_chain, query_output) };
    if let Some(capture) = capture {
        finish_raycast_filter_hit_capture(capture);
    }
}

pub(in crate::api::game::adapter) unsafe extern "system" fn native_world_query_group_filter_tap(
    filter: *mut c_void,
    output: *mut u8,
    ray: *const c_void,
    candidate: *const c_void,
) -> *mut u8 {
    let original = ORIGINAL_WORLD_QUERY_GROUP_FILTER.load(Ordering::Acquire);
    if original.is_null() {
        return output;
    }
    let original: WorldQueryGroupFilter = unsafe { std::mem::transmute(original) };
    let frame = WORLD_QUERY_CANDIDATE_STACK
        .try_with(|stack| stack.try_borrow().ok().and_then(|stack| stack.current()))
        .ok()
        .flatten();
    let capture = frame.and_then(|frame| {
        let slot = frame.callback_slot?;
        let sample = WORLD_QUERY_SAMPLES.get(frame.sample_index)?;
        let candidate_sample = sample.candidate_samples.get(slot)?;
        let ray_info = (ray as usize)
            .checked_add(0x24)
            .filter(|address| readable_span(*address, size_of::<u32>()))
            .map(|address| unsafe { ptr::read_unaligned(address as *const u32) })?;
        let candidate_info = (candidate as usize)
            .checked_add(0x2C)
            .filter(|address| readable_span(*address, size_of::<u32>()))
            .map(|address| unsafe { ptr::read_unaligned(address as *const u32) })?;
        Some((candidate_sample, ray_info, candidate_info))
    });
    let result_pointer = unsafe { original(filter, output, ray, candidate) };
    if let Some((candidate_sample, ray_info, candidate_info)) = capture
        && !result_pointer.is_null()
        && readable_span(result_pointer as usize, size_of::<u8>())
    {
        candidate_sample
            .group_filter_ray_info
            .store(ray_info, Ordering::Relaxed);
        candidate_sample
            .group_filter_candidate_info
            .store(candidate_info, Ordering::Relaxed);
        candidate_sample.group_filter_result.store(
            unsafe { ptr::read_unaligned(result_pointer) },
            Ordering::Relaxed,
        );
        candidate_sample
            .group_filter_result_valid
            .store(true, Ordering::Release);
    }
    result_pointer
}

// Only the selected diagnostic calls replace the return address. All calls
// tail-jump into the original trampoline with the caller's stack unchanged.
#[unsafe(naked)]
pub(in crate::api::game::adapter) unsafe extern "system" fn native_world_query_tap() -> ! {
    core::arch::naked_asm!(
        "sub rsp, 0x1E8",
        "mov qword ptr [rsp + 0x20], rcx",
        "mov qword ptr [rsp + 0x28], rdx",
        "mov qword ptr [rsp + 0x30], r8",
        "mov qword ptr [rsp + 0x38], r9",
        "mov qword ptr [rsp + 0x40], rax",
        "mov qword ptr [rsp + 0x48], r10",
        "mov qword ptr [rsp + 0x50], r11",
        "movdqu xmmword ptr [rsp + 0x58], xmm0",
        "movdqu xmmword ptr [rsp + 0x68], xmm1",
        "movdqu xmmword ptr [rsp + 0x78], xmm2",
        "movdqu xmmword ptr [rsp + 0x88], xmm3",
        "movdqu xmmword ptr [rsp + 0x98], xmm4",
        "movdqu xmmword ptr [rsp + 0xA8], xmm5",
        "mov qword ptr [rsp + 0xB8], 0",
        // Entry RSP is now at +0x1E8. Win64 stack slot five starts at entry
        // RSP+0x28; copy slots five through eleven into the saved record.
        "mov rax, qword ptr [rsp + 0x210]",
        "mov qword ptr [rsp + 0xC0], rax",
        "mov rax, qword ptr [rsp + 0x218]",
        "mov qword ptr [rsp + 0xC8], rax",
        "mov rax, qword ptr [rsp + 0x220]",
        "mov qword ptr [rsp + 0xD0], rax",
        "mov rax, qword ptr [rsp + 0x228]",
        "mov qword ptr [rsp + 0xD8], rax",
        "mov rax, qword ptr [rsp + 0x230]",
        "mov qword ptr [rsp + 0xE0], rax",
        "mov rax, qword ptr [rsp + 0x238]",
        "mov qword ptr [rsp + 0xE8], rax",
        "mov rax, qword ptr [rsp + 0x240]",
        "mov qword ptr [rsp + 0xF0], rax",
        "mov qword ptr [rsp + 0xF8], rbx",
        "mov qword ptr [rsp + 0x100], rbp",
        "mov qword ptr [rsp + 0x108], rsi",
        "mov qword ptr [rsp + 0x110], rdi",
        "mov qword ptr [rsp + 0x118], r12",
        "mov qword ptr [rsp + 0x120], r13",
        "mov qword ptr [rsp + 0x128], r14",
        "mov qword ptr [rsp + 0x130], r15",
        "movdqu xmmword ptr [rsp + 0x138], xmm6",
        "movdqu xmmword ptr [rsp + 0x148], xmm7",
        "movdqu xmmword ptr [rsp + 0x158], xmm8",
        "movdqu xmmword ptr [rsp + 0x168], xmm9",
        "movdqu xmmword ptr [rsp + 0x178], xmm10",
        "movdqu xmmword ptr [rsp + 0x188], xmm11",
        "movdqu xmmword ptr [rsp + 0x198], xmm12",
        "movdqu xmmword ptr [rsp + 0x1A8], xmm13",
        "movdqu xmmword ptr [rsp + 0x1B8], xmm14",
        "movdqu xmmword ptr [rsp + 0x1C8], xmm15",
        "mov rcx, qword ptr [rsp + 0x1E8]",
        "lea rdx, [rsp + 0x20]",
        "call {record_call}",
        "mov qword ptr [rsp + 0x1E0], rax",
        "movdqu xmm0, xmmword ptr [rsp + 0x58]",
        "movdqu xmm1, xmmword ptr [rsp + 0x68]",
        "movdqu xmm2, xmmword ptr [rsp + 0x78]",
        "movdqu xmm3, xmmword ptr [rsp + 0x88]",
        "movdqu xmm4, xmmword ptr [rsp + 0x98]",
        "movdqu xmm5, xmmword ptr [rsp + 0xA8]",
        "mov rcx, qword ptr [rsp + 0x20]",
        "mov rdx, qword ptr [rsp + 0x28]",
        "mov r8, qword ptr [rsp + 0x30]",
        "mov r9, qword ptr [rsp + 0x38]",
        "mov r10, qword ptr [rsp + 0x48]",
        "mov r11, qword ptr [rsp + 0x50]",
        "mov rax, qword ptr [rsp + 0x40]",
        "cmp qword ptr [rsp + 0xB8], 0",
        "je 2f",
        "lea rax, [rip + {return_tap}]",
        "mov qword ptr [rsp + 0x1E8], rax",
        "2:",
        "mov rax, qword ptr [rsp + 0x1E0]",
        "add rsp, 0x1E8",
        "jmp rax",
        record_call = sym record_world_query_call,
        return_tap = sym native_world_query_return_tap,
    )
}

#[unsafe(naked)]
pub(in crate::api::game::adapter) unsafe extern "system" fn native_world_query_return_tap() -> ! {
    core::arch::naked_asm!(
        "push rax",
        "sub rsp, 0x28",
        "mov rcx, qword ptr [rsp + 0x28]",
        "call {record_return}",
        "mov r11, rax",
        "mov rax, qword ptr [rsp + 0x28]",
        "add rsp, 0x28",
        "pop rax",
        "jmp r11",
        record_return = sym record_world_query_return,
    )
}

#[inline(never)]
extern "C" fn record_world_query_call(
    return_address: usize,
    registers: *mut WorldQuerySavedRegisters,
) -> usize {
    let sequence = WORLD_QUERY_CALLS.fetch_add(1, Ordering::Relaxed) + 1;
    WORLD_QUERY_LAST_RETURN_ADDRESS.store(return_address, Ordering::Relaxed);
    let caller_index = record_world_query_caller(return_address);
    if let Some(index) = caller_index {
        WORLD_QUERY_CALLER_CALLS[index].fetch_add(1, Ordering::Relaxed);
    }

    let caller_rva = return_address.checked_sub(STEAM_IMAGE_BASE);
    let saved_registers = if registers.is_null() {
        None
    } else {
        Some(unsafe { ptr::read_unaligned(registers) })
    };
    let reticle_label = saved_registers
        .as_ref()
        .and_then(|snapshot| snapshot_printable_c_string(snapshot.rdx as usize));
    let is_reticle_call = reticle_label
        .as_ref()
        .is_some_and(is_reticle_query_label_bytes);
    let reticle_caller_slot = is_reticle_call
        .then(|| register_reticle_query_caller(return_address))
        .flatten();
    let repeat_caller_index = caller_rva.and_then(|rva| {
        WORLD_QUERY_REPEAT_CALLERS
            .iter()
            .position(|candidate| *candidate == rva)
    });
    let spaced_repeat_caller_index = caller_rva.and_then(|rva| {
        WORLD_QUERY_SPACED_REPEAT_CALLERS
            .iter()
            .position(|candidate| *candidate == rva)
    });
    let capture_ordinal = if is_reticle_call {
        reticle_caller_slot.and_then(|index| {
            let call_ordinal = RETICLE_QUERY_CALL_COUNTS[index].fetch_add(1, Ordering::Relaxed);
            RETICLE_QUERY_SAMPLE_ORDINALS
                .iter()
                .position(|scheduled| *scheduled == call_ordinal)
                .map(|sample_ordinal| sample_ordinal as u32)
        })
    } else {
        match caller_rva {
            Some(AMMO_WORLD_QUERY_CALLER_RVA) => {
                let ordinal = WORLD_QUERY_AMMO_SAMPLE_CALLS.fetch_add(1, Ordering::Relaxed);
                (ordinal < 320).then_some(ordinal)
            }
            Some(HEIGHT_OVER_GROUND_WORLD_QUERY_CALLER_RVA) => {
                let ordinal = WORLD_QUERY_FAN_SAMPLE_CALLS.fetch_add(1, Ordering::Relaxed);
                (ordinal < 8).then_some(ordinal)
            }
            Some(SCRIPT_RAYCAST_WORLD_QUERY_CALLER_RVA) => {
                let ordinal =
                    WORLD_QUERY_SCRIPT_RAYCAST_SAMPLE_CALLS.fetch_add(1, Ordering::Relaxed);
                (ordinal < 160).then_some(ordinal)
            }
            _ => {
                if let Some(index) = spaced_repeat_caller_index {
                    let call_ordinal = WORLD_QUERY_SPACED_REPEAT_CALL_COUNTS[index]
                        .fetch_add(1, Ordering::Relaxed);
                    WORLD_QUERY_SPACED_REPEAT_CALL_ORDINALS
                        .iter()
                        .position(|scheduled| *scheduled == call_ordinal)
                        .map(|sample_ordinal| sample_ordinal as u32)
                } else {
                    repeat_caller_index.map_or_else(
                        || {
                            caller_index
                                .filter(|index| {
                                    !WORLD_QUERY_CALLERS_SAMPLED[*index]
                                        .swap(true, Ordering::AcqRel)
                                })
                                .map(|_| 0)
                        },
                        |index| {
                            let ordinal = WORLD_QUERY_REPEAT_CALL_COUNTS[index]
                                .fetch_add(1, Ordering::Relaxed);
                            (ordinal < WORLD_QUERY_REPEAT_SAMPLE_LIMIT).then_some(ordinal)
                        },
                    )
                }
            }
        }
    };
    let mut return_frame_installed = false;
    if let Some(capture_ordinal) = capture_ordinal.filter(|_| !registers.is_null()) {
        let sample_index = if let Some(reticle_caller_slot) = reticle_caller_slot {
            RETICLE_QUERY_SAMPLE_BASE
                + reticle_caller_slot * RETICLE_QUERY_SAMPLE_ORDINALS.len()
                + capture_ordinal as usize
        } else if caller_rva == Some(SCRIPT_RAYCAST_WORLD_QUERY_CALLER_RVA) {
            capture_ordinal as usize
        } else {
            let ordinal = WORLD_QUERY_SAMPLE_COUNT.fetch_add(1, Ordering::Relaxed);
            if ordinal < WORLD_QUERY_NON_CAMERA_SAMPLE_CAPACITY {
                WORLD_QUERY_NON_CAMERA_SAMPLE_BASE + ordinal
            } else {
                WORLD_QUERY_SAMPLE_CAPACITY
            }
        };
        if let Some(sample) = WORLD_QUERY_SAMPLES.get(sample_index) {
            let snapshot = saved_registers.expect("non-null query register snapshot");
            if let Some(label) = reticle_label {
                for (destination, value) in sample.rdx_label.iter().zip(label) {
                    destination.store(value, Ordering::Relaxed);
                }
                sample.rdx_label_valid.store(true, Ordering::Relaxed);
            }
            let integer_registers = [
                snapshot.rcx,
                snapshot.rdx,
                snapshot.r8,
                snapshot.r9,
                snapshot.rax,
                snapshot.r10,
                snapshot.r11,
            ];
            let nonvolatile_registers = snapshot.nonvolatile_registers;
            let xmm_registers = snapshot.xmm.into_iter().flatten();
            let nonvolatile_xmm_registers = snapshot.nonvolatile_xmm.into_iter().flatten();
            for (destination, value) in sample.stack_arguments.iter().zip(snapshot.stack_arguments)
            {
                destination.store(value, Ordering::Relaxed);
            }
            // The shared helper's live disassembly loads its output/context
            // pointers from stack arguments 6 and 7 for every caller.
            let output_address = snapshot.stack_arguments[1] as usize;
            let context_address = snapshot.stack_arguments[2] as usize;

            if caller_rva == Some(SCRIPT_RAYCAST_WORLD_QUERY_CALLER_RVA) {
                // Static disassembly shows the wrapper keeps original RCX in
                // RDI and original RDX in RSI until the helper call. Snapshot
                // only 0x20 raw bytes for each of the already-capped samples.
                let script_arguments = [
                    nonvolatile_registers[3] as usize,
                    nonvolatile_registers[2] as usize,
                ];
                for (slot, address) in script_arguments.into_iter().enumerate() {
                    sample.script_argument_addresses[slot].store(address, Ordering::Relaxed);
                    if address >= 0x10000 && readable_span(address, size_of::<[u64; 4]>()) {
                        let words = unsafe { ptr::read_unaligned(address as *const [u64; 4]) };
                        for (destination, value) in
                            sample.script_argument_words[slot].iter().zip(words)
                        {
                            destination.store(value, Ordering::Relaxed);
                        }
                        sample.script_argument_valid[slot].store(true, Ordering::Relaxed);
                    }
                }
            }

            sample
                .output_record_address
                .store(output_address, Ordering::Relaxed);
            sample
                .query_context_address
                .store(context_address, Ordering::Relaxed);
            if readable_span(output_address, size_of::<[u64; 10]>()) {
                let words = unsafe { ptr::read_unaligned(output_address as *const [u64; 10]) };
                for (destination, value) in sample.output_record_before.iter().zip(words) {
                    destination.store(value, Ordering::Relaxed);
                }
                sample
                    .output_record_before_valid
                    .store(true, Ordering::Relaxed);
            }
            if readable_span(context_address, size_of::<[u64; 21]>()) {
                let words = unsafe { ptr::read_unaligned(context_address as *const [u64; 21]) };
                for (destination, value) in sample.query_context_before.iter().zip(words) {
                    destination.store(value, Ordering::Relaxed);
                }
                sample
                    .query_context_before_valid
                    .store(true, Ordering::Relaxed);
            }
            let query_data = if readable_span(snapshot.r8 as usize, size_of::<[u64; 12]>()) {
                Some(unsafe { ptr::read_unaligned(snapshot.r8 as *const [u64; 12]) })
            } else {
                None
            };
            let bounds_frame_origin = snapshot.rcx.checked_add(0x40).and_then(|address| {
                readable_span(address as usize, size_of::<[f32; 3]>())
                    .then(|| unsafe { ptr::read_unaligned(address as *const [f32; 3]) })
            });
            let argument_11_override = usize::try_from(snapshot.stack_arguments[6])
                .ok()
                .filter(|address| *address != 0 && readable_span(*address, size_of::<u32>()))
                .map(|address| unsafe { ptr::read_unaligned(address as *const u32) });
            sample.sequence.store(sequence, Ordering::Relaxed);
            sample
                .capture_ordinal
                .store(capture_ordinal, Ordering::Relaxed);
            sample.caller.store(return_address, Ordering::Relaxed);
            for (destination, value) in sample.integer_registers.iter().zip(integer_registers) {
                destination.store(value, Ordering::Relaxed);
            }
            for (destination, value) in sample
                .nonvolatile_registers
                .iter()
                .zip(nonvolatile_registers)
            {
                destination.store(value, Ordering::Relaxed);
            }
            if caller_rva == Some(AMMO_WORLD_QUERY_CALLER_RVA) {
                capture_ammo_receiver_dispatch(sample, nonvolatile_registers[0] as usize);
            }
            for (destination, value) in sample.xmm_registers.iter().zip(xmm_registers) {
                destination.store(value, Ordering::Relaxed);
            }
            for (destination, value) in sample
                .nonvolatile_xmm_registers
                .iter()
                .zip(nonvolatile_xmm_registers)
            {
                destination.store(value, Ordering::Relaxed);
            }
            if let Some(value) = argument_11_override {
                sample.argument_11_override.store(value, Ordering::Relaxed);
                sample
                    .argument_11_override_valid
                    .store(true, Ordering::Relaxed);
            }
            if let Some(origin) = bounds_frame_origin {
                for (destination, value) in sample.bounds_frame_origin.iter().zip(origin) {
                    destination.store(value.to_bits(), Ordering::Relaxed);
                }
                sample
                    .bounds_frame_origin_valid
                    .store(true, Ordering::Relaxed);
            }
            if let Some(query_data) = query_data {
                for (destination, value) in sample.query_data.iter().zip(query_data) {
                    destination.store(value, Ordering::Relaxed);
                }
                sample.query_data_valid.store(true, Ordering::Relaxed);
            }
            sample.ready.store(true, Ordering::Release);
            let frame = PendingWorldQueryReturn {
                sample_index,
                return_address,
                query_data_address: snapshot.r8 as usize,
            };
            let (pushed, invalidated_samples) = WORLD_QUERY_RETURN_STACK
                .try_with(|stack| match stack.try_borrow_mut() {
                    Ok(mut stack) => stack.push(frame),
                    Err(_) => (false, 0),
                })
                .unwrap_or((false, 0));
            if invalidated_samples > 0 {
                WORLD_QUERY_ACTIVE_SAMPLES.fetch_sub(invalidated_samples, Ordering::Relaxed);
            }
            if pushed {
                WORLD_QUERY_ACTIVE_SAMPLES.fetch_add(1, Ordering::Relaxed);
                unsafe {
                    ptr::addr_of_mut!((*registers).capture_return).write_unaligned(1);
                }
                return_frame_installed = true;
            } else {
                sample.result.store(2, Ordering::Relaxed);
                sample.result_claimed.store(true, Ordering::Relaxed);
                sample.result_ready.store(true, Ordering::Release);
            }
        }
    }

    if !return_frame_installed && !registers.is_null() {
        let frame = PendingWorldQueryReturn {
            sample_index: WORLD_QUERY_NO_SAMPLE,
            return_address,
            query_data_address: 0,
        };
        let (pushed, invalidated_samples) = WORLD_QUERY_RETURN_STACK
            .try_with(|stack| match stack.try_borrow_mut() {
                Ok(mut stack) => stack.push(frame),
                Err(_) => (false, 0),
            })
            .unwrap_or((false, 0));
        if invalidated_samples > 0 {
            WORLD_QUERY_ACTIVE_SAMPLES.fetch_sub(invalidated_samples, Ordering::Relaxed);
        }
        if pushed {
            unsafe {
                ptr::addr_of_mut!((*registers).capture_return).write_unaligned(1);
            }
        }
    }

    ORIGINAL_WORLD_QUERY.load(Ordering::Acquire) as usize
}
