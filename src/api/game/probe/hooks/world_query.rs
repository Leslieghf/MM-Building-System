//! Lifecycle and callbacks for observing native world-query activity.
use super::super::*;

pub(in crate::api::game::probe) fn install_world_query_hook(image_base: usize) {
    let Some(target) = image_base.checked_add(WORLD_QUERY_HELPER_RVA) else {
        append_log("World query tap refused: helper address overflowed.");
        return;
    };
    if !is_readable_executable_page(target) {
        append_log("World query tap refused: helper address is not executable code.");
        return;
    }
    const EXPECTED_ENTRY: [u8; 16] = [
        0x48, 0x89, 0xE0, 0x55, 0x56, 0x57, 0x48, 0x8D, 0x68, 0x98, 0x48, 0x81, 0xEC, 0x50, 0x01,
        0x00,
    ];
    if !readable_span(target, EXPECTED_ENTRY.len()) {
        append_log("World query tap refused: helper entry bytes are not readable.");
        return;
    }
    let entry = unsafe { std::slice::from_raw_parts(target as *const u8, EXPECTED_ENTRY.len()) };
    let entry_text = entry
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ");
    append_log(&format!("World query helper entry bytes: {entry_text}."));
    if entry != EXPECTED_ENTRY {
        append_log(
            "World query tap refused: helper entry signature did not match the observed Steam build.",
        );
        return;
    }

    let address = target as *mut c_void;
    let detour = native_world_query_tap as *const () as *mut c_void;
    let mut original = ptr::null_mut();
    let created = unsafe { mh::MH_CreateHook(address, detour, &mut original) };
    if created != MH_STATUS::MH_OK {
        append_log(&format!(
            "World query tap refused: MinHook could not create the helper hook ({created:?})."
        ));
        return;
    }
    ORIGINAL_WORLD_QUERY.store(original, Ordering::Release);
    install_world_query_candidate_hook(image_base);
    install_raycast_filter_add_hit_hook(image_base);
    append_log(
        "World-query diagnostic build: candidate callback tap only; bounds and group-filter taps are disabled.",
    );

    let enabled = unsafe { mh::MH_EnableHook(address) };
    if enabled != MH_STATUS::MH_OK {
        ORIGINAL_WORLD_QUERY.store(ptr::null_mut(), Ordering::Release);
        append_log(&format!(
            "World query tap refused: MinHook could not enable the helper hook ({enabled:?})."
        ));
        return;
    }

    WORLD_QUERY_HOOK_INSTALLED.store(true, Ordering::Release);
    start_world_query_caller_log_worker();
    append_log(&format!(
        "Passive world query tap installed at 0x{target:X}; recording calls and unique caller addresses."
    ));
}

pub(in crate::api::game::probe) fn install_world_query_bounds_predicate_hook(image_base: usize) {
    if WORLD_QUERY_BOUNDS_PREDICATE_HOOK_INSTALLED.load(Ordering::Acquire) {
        return;
    }
    let Some(target) = image_base.checked_add(WORLD_QUERY_BOUNDS_PREDICATE_RVA) else {
        append_log("World-query bounds-result tap skipped: predicate address overflowed.");
        return;
    };
    const EXPECTED_ENTRY: [u8; 16] = [
        0x48, 0x83, 0xEC, 0x28, 0x48, 0x8B, 0x41, 0x38, 0x0F, 0x29, 0x74, 0x24, 0x10, 0x44, 0x0F,
        0x29,
    ];
    if !is_readable_executable_page(target) || !readable_span(target, EXPECTED_ENTRY.len()) {
        append_log(
            "World-query bounds-result tap skipped: predicate entry is not readable executable code.",
        );
        return;
    }
    let entry = unsafe { std::slice::from_raw_parts(target as *const u8, EXPECTED_ENTRY.len()) };
    if entry != EXPECTED_ENTRY {
        append_log(
            "World-query bounds-result tap skipped: predicate entry signature did not match the observed Steam build.",
        );
        return;
    }

    let address = target as *mut c_void;
    let detour = native_world_query_bounds_predicate_tap as *const () as *mut c_void;
    let mut original = ptr::null_mut();
    let created = unsafe { mh::MH_CreateHook(address, detour, &mut original) };
    if created != MH_STATUS::MH_OK {
        append_log(&format!(
            "World-query bounds-result tap skipped: MinHook could not create predicate hook ({created:?})."
        ));
        return;
    }
    ORIGINAL_WORLD_QUERY_BOUNDS_PREDICATE.store(original, Ordering::Release);
    let enabled = unsafe { mh::MH_EnableHook(address) };
    if enabled != MH_STATUS::MH_OK {
        ORIGINAL_WORLD_QUERY_BOUNDS_PREDICATE.store(ptr::null_mut(), Ordering::Release);
        append_log(&format!(
            "World-query bounds-result tap skipped: MinHook could not enable predicate hook ({enabled:?})."
        ));
        return;
    }

    WORLD_QUERY_BOUNDS_PREDICATE_HOOK_INSTALLED.store(true, Ordering::Release);
    append_log(&format!(
        "World-query bounds-result tap installed at 0x{target:X}; recording at most two predicate calls per sampled helper query."
    ));
}

pub(in crate::api::game::probe) fn install_world_query_candidate_hook(image_base: usize) {
    if WORLD_QUERY_CANDIDATE_HOOK_INSTALLED.load(Ordering::Acquire) {
        return;
    }
    let Some(target) = image_base.checked_add(WORLD_QUERY_CANDIDATE_CALLBACK_RVA) else {
        append_log("World-query candidate tap skipped: callback address overflowed.");
        return;
    };
    if !is_readable_executable_page(target) || !readable_span(target, 16) {
        append_log(
            "World-query candidate tap skipped: callback entry is not readable executable code.",
        );
        return;
    }
    const EXPECTED_ENTRY: [u8; 16] = [
        0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18,
        0x48,
    ];
    let entry = unsafe { std::slice::from_raw_parts(target as *const u8, EXPECTED_ENTRY.len()) };
    if entry != EXPECTED_ENTRY {
        append_log(
            "World-query candidate tap skipped: callback entry signature did not match the supported Steam build.",
        );
        return;
    }

    let address = target as *mut c_void;
    let detour = native_world_query_candidate_tap as *const () as *mut c_void;
    let mut original = ptr::null_mut();
    let created = unsafe { mh::MH_CreateHook(address, detour, &mut original) };
    if created != MH_STATUS::MH_OK {
        append_log(&format!(
            "World-query candidate tap skipped: MinHook could not create callback hook ({created:?})."
        ));
        return;
    }
    ORIGINAL_WORLD_QUERY_CANDIDATE.store(original, Ordering::Release);
    let enabled = unsafe { mh::MH_EnableHook(address) };
    if enabled != MH_STATUS::MH_OK {
        ORIGINAL_WORLD_QUERY_CANDIDATE.store(ptr::null_mut(), Ordering::Release);
        append_log(&format!(
            "World-query candidate tap skipped: MinHook could not enable callback hook ({enabled:?})."
        ));
        return;
    }
    WORLD_QUERY_CANDIDATE_HOOK_INSTALLED.store(true, Ordering::Release);
    append_log(&format!(
        "World-query candidate tap installed at 0x{target:X}; capturing callback ordinals 0,1,2,4,8,16,32 and tracking minimum valid fraction per sampled query."
    ));
}

pub(in crate::api::game::probe) fn install_raycast_filter_add_hit_hook(image_base: usize) {
    if RAYCAST_FILTER_ADD_HIT_HOOK_INSTALLED.load(Ordering::Acquire) {
        return;
    }
    let Some(target) = image_base.checked_add(C_RAYCAST_FILTER_ADD_RAY_HIT_RVA) else {
        append_log("Raycast filter-info tap skipped: callback address overflowed.");
        return;
    };
    const EXPECTED_ENTRY: [u8; 16] = [
        0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18,
        0x57,
    ];
    if !is_readable_executable_page(target) || !readable_span(target, EXPECTED_ENTRY.len()) {
        append_log(
            "Raycast filter-info tap skipped: callback entry is not readable executable code.",
        );
        return;
    }
    let entry = unsafe { std::slice::from_raw_parts(target as *const u8, EXPECTED_ENTRY.len()) };
    if entry != EXPECTED_ENTRY {
        append_log(
            "Raycast filter-info tap skipped: callback signature did not match the captured Steam build.",
        );
        return;
    }

    let address = target as *mut c_void;
    let detour = raycast_filter_add_hit_tap as *const () as *mut c_void;
    let mut original = ptr::null_mut();
    let created = unsafe { mh::MH_CreateHook(address, detour, &mut original) };
    if created != MH_STATUS::MH_OK {
        append_log(&format!(
            "Raycast filter-info tap skipped: MinHook could not create callback hook ({created:?})."
        ));
        return;
    }
    ORIGINAL_RAYCAST_FILTER_ADD_HIT.store(original, Ordering::Release);
    let enabled = unsafe { mh::MH_EnableHook(address) };
    if enabled != MH_STATUS::MH_OK {
        ORIGINAL_RAYCAST_FILTER_ADD_HIT.store(ptr::null_mut(), Ordering::Release);
        append_log(&format!(
            "Raycast filter-info tap skipped: MinHook could not enable callback hook ({enabled:?})."
        ));
        return;
    }
    RAYCAST_FILTER_ADD_HIT_HOOK_INSTALLED.store(true, Ordering::Release);
    append_log(&format!(
        "Scoped CRaycastFilter hit tap installed at 0x{target:X}; captures up to {RAYCAST_FILTER_HIT_CAPTURE_CAPACITY} entries per sampled reticle broadphase candidate."
    ));
}

pub(in crate::api::game::probe) fn install_world_query_group_filter_hook(image_base: usize) {
    if WORLD_QUERY_GROUP_FILTER_HOOK_INSTALLED.load(Ordering::Acquire) {
        return;
    }
    let Some(target) = image_base.checked_add(WORLD_QUERY_GROUP_FILTER_RVA) else {
        append_log("World-query group-filter tap skipped: wrapper address overflowed.");
        return;
    };
    const EXPECTED_ENTRY: [u8; 16] = [
        0x48, 0x83, 0xEC, 0x28, 0x45, 0x8B, 0x49, 0x2C, 0x45, 0x8B, 0x40, 0x24, 0x48, 0x83, 0xC1,
        0xD8,
    ];
    if !is_readable_executable_page(target) || !readable_span(target, EXPECTED_ENTRY.len()) {
        append_log(
            "World-query group-filter tap skipped: wrapper entry is not readable executable code.",
        );
        return;
    }
    let entry = unsafe { std::slice::from_raw_parts(target as *const u8, EXPECTED_ENTRY.len()) };
    if entry != EXPECTED_ENTRY {
        append_log(
            "World-query group-filter tap skipped: wrapper signature did not match the observed Steam build.",
        );
        return;
    }

    let address = target as *mut c_void;
    let detour = native_world_query_group_filter_tap as *const () as *mut c_void;
    let mut original = ptr::null_mut();
    let created = unsafe { mh::MH_CreateHook(address, detour, &mut original) };
    if created != MH_STATUS::MH_OK {
        append_log(&format!(
            "World-query group-filter tap skipped: MinHook could not create wrapper hook ({created:?})."
        ));
        return;
    }
    ORIGINAL_WORLD_QUERY_GROUP_FILTER.store(original, Ordering::Release);
    let enabled = unsafe { mh::MH_EnableHook(address) };
    if enabled != MH_STATUS::MH_OK {
        ORIGINAL_WORLD_QUERY_GROUP_FILTER.store(ptr::null_mut(), Ordering::Release);
        append_log(&format!(
            "World-query group-filter tap skipped: MinHook could not enable wrapper hook ({enabled:?})."
        ));
        return;
    }
    WORLD_QUERY_GROUP_FILTER_HOOK_INSTALLED.store(true, Ordering::Release);
    append_log(&format!(
        "World-query group-filter tap installed at 0x{target:X}; it only reads inputs/results inside a sampled candidate callback."
    ));
}

pub(in crate::api::game::probe) fn start_world_query_caller_log_worker() {
    append_log("World query unique-caller log capture started for this game run.");
    if let Err(error) = thread::Builder::new()
        .name("madmax-world-query-callers".to_owned())
        .spawn(world_query_caller_log_worker)
    {
        append_log(&format!(
            "Could not start world query caller log worker: {error}. Addresses will remain in memory only."
        ));
    }
}

pub(in crate::api::game::probe) fn world_query_caller_log_worker() {
    let mut previous_counts = [0u64; WORLD_QUERY_CALLER_CAPACITY];
    let mut activity_window_start = Instant::now();
    loop {
        for (index, caller) in WORLD_QUERY_CALLERS.iter().enumerate() {
            let address = caller.load(Ordering::Acquire);
            if address == 0 || WORLD_QUERY_CALLERS_LOGGED[index].swap(true, Ordering::AcqRel) {
                continue;
            }
            let rva = address.checked_sub(STEAM_IMAGE_BASE);
            append_log(&format!(
                "World query caller observed: 0x{address:X} (RVA {}).",
                rva.map_or_else(
                    || "outside game image".to_owned(),
                    |value| format!("0x{value:X}")
                )
            ));
        }

        let window_elapsed = activity_window_start.elapsed();
        if window_elapsed >= WORLD_QUERY_CALLER_REPORT_INTERVAL {
            log_world_query_caller_activity(&mut previous_counts, window_elapsed);
            activity_window_start = Instant::now();
        }

        if WORLD_QUERY_CALLER_TABLE_FULL.swap(false, Ordering::AcqRel) {
            append_log(&format!(
                "World query caller address table reached its {WORLD_QUERY_CALLER_CAPACITY}-address limit; additional unique callers may be missing."
            ));
        }
        log_world_query_samples();
        thread::sleep(Duration::from_millis(250));
    }
}

pub(in crate::api::game::probe) fn log_world_query_caller_activity(
    previous_counts: &mut [u64; WORLD_QUERY_CALLER_CAPACITY],
    window: Duration,
) {
    let mut deltas = Vec::with_capacity(WORLD_QUERY_CALLER_CAPACITY);
    for (index, caller) in WORLD_QUERY_CALLERS.iter().enumerate() {
        let address = caller.load(Ordering::Acquire);
        if address == 0 {
            continue;
        }
        let current_count = WORLD_QUERY_CALLER_CALLS[index].load(Ordering::Relaxed);
        let delta = current_count.saturating_sub(previous_counts[index]);
        previous_counts[index] = current_count;
        if delta > 0 {
            deltas.push((address, delta));
        }
    }

    if deltas.is_empty() {
        return;
    }

    deltas.sort_unstable_by(|left, right| right.1.cmp(&left.1));
    let top_callers = deltas
        .iter()
        .take(WORLD_QUERY_CALLER_REPORT_LIMIT)
        .map(|(address, calls)| {
            let caller = address.checked_sub(STEAM_IMAGE_BASE).map_or_else(
                || "outside-game-image".to_owned(),
                |rva| format!("0x{rva:X}"),
            );
            format!("{caller}={calls}")
        })
        .collect::<Vec<_>>()
        .join(",");
    let observed_calls: u64 = deltas.iter().map(|(_, calls)| *calls).sum();
    append_log(&format!(
        "World query caller activity: window={:.1}s observed-calls={observed_calls} active-callers={} top=[{top_callers}].",
        window.as_secs_f32(),
        deltas.len(),
    ));
}

pub(in crate::api::game::probe) fn log_world_query_samples() {
    for (index, sample) in WORLD_QUERY_SAMPLES.iter().enumerate() {
        if WORLD_QUERY_SAMPLES_LOGGED[index].load(Ordering::Acquire)
            || !sample.ready.load(Ordering::Acquire)
            || !sample.result_ready.load(Ordering::Acquire)
        {
            continue;
        }
        let integer_registers: [u64; 7] =
            std::array::from_fn(|slot| sample.integer_registers[slot].load(Ordering::Relaxed));
        let nonvolatile_registers: [u64; 8] =
            std::array::from_fn(|slot| sample.nonvolatile_registers[slot].load(Ordering::Relaxed));
        let xmm_registers: [u64; 12] =
            std::array::from_fn(|slot| sample.xmm_registers[slot].load(Ordering::Relaxed));
        let nonvolatile_xmm_text = sample
            .nonvolatile_xmm_registers
            .chunks_exact(2)
            .enumerate()
            .map(|(index, pair)| {
                format!(
                    "XMM{}=[{:016X}:{:016X}]",
                    index + 6,
                    pair[1].load(Ordering::Relaxed),
                    pair[0].load(Ordering::Relaxed),
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        let query_data: [u64; 12] =
            std::array::from_fn(|slot| sample.query_data[slot].load(Ordering::Relaxed));
        let query_data_after: [u64; 12] =
            std::array::from_fn(|slot| sample.query_data_after[slot].load(Ordering::Relaxed));
        let bounds_frame_origin: [f32; 3] = std::array::from_fn(|slot| {
            f32::from_bits(sample.bounds_frame_origin[slot].load(Ordering::Relaxed))
        });
        let query_data_text = if sample.query_data_valid.load(Ordering::Relaxed) {
            query_data
                .iter()
                .map(|word| format!("{word:016X}"))
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            "unreadable".to_owned()
        };
        let query_data_after_text = if sample.query_data_after_valid.load(Ordering::Relaxed) {
            query_data_after
                .iter()
                .map(|word| format!("{word:016X}"))
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            "unreadable".to_owned()
        };
        let indirect_text = (0..sample.indirect_addresses.len())
            .map(|slot| {
                if !sample.indirect_valid[slot].load(Ordering::Relaxed) {
                    return format!("indirect{slot}=unreadable");
                }
                let address = sample.indirect_addresses[slot].load(Ordering::Relaxed);
                let words: [u64; 12] = std::array::from_fn(|word| {
                    sample.indirect_data[slot][word].load(Ordering::Relaxed)
                });
                format!(
                    "indirect{slot}@0x{address:X}={}",
                    words
                        .iter()
                        .map(|word| format!("{word:016X}"))
                        .collect::<Vec<_>>()
                        .join(" ")
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        let script_argument_text = if sample
            .caller
            .load(Ordering::Relaxed)
            .checked_sub(STEAM_IMAGE_BASE)
            == Some(SCRIPT_RAYCAST_WORLD_QUERY_CALLER_RVA)
        {
            (0..sample.script_argument_addresses.len())
                .map(|slot| {
                    let address = sample.script_argument_addresses[slot].load(Ordering::Relaxed);
                    let words = format_atomic_words(
                        &sample.script_argument_words[slot],
                        sample.script_argument_valid[slot].load(Ordering::Relaxed),
                    );
                    format!("script-arg{slot}@0x{address:X}=[{words}]")
                })
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            "script-args=not-applicable".to_owned()
        };
        let stack_arguments: [u64; 7] =
            std::array::from_fn(|slot| sample.stack_arguments[slot].load(Ordering::Relaxed));
        let stack_arguments_text = stack_arguments
            .iter()
            .enumerate()
            .map(|(slot, argument)| format!("slot{}=0x{argument:X}", slot + 5))
            .collect::<Vec<_>>()
            .join(" ");
        let argument_11_address = stack_arguments[6] as usize;
        let argument_11_override = if argument_11_address == 0 {
            "arg11-pointee=null".to_owned()
        } else if sample.argument_11_override_valid.load(Ordering::Relaxed) {
            format!(
                "arg11-pointee=0x{:08X}",
                sample.argument_11_override.load(Ordering::Relaxed)
            )
        } else {
            format!("arg11-pointee=unreadable@0x{argument_11_address:X}")
        };
        let output_record_before = format_atomic_words(
            &sample.output_record_before,
            sample.output_record_before_valid.load(Ordering::Relaxed),
        );
        let output_record_after = format_atomic_words(
            &sample.output_record_after,
            sample.output_record_after_valid.load(Ordering::Relaxed),
        );
        let query_context_before = format_atomic_words(
            &sample.query_context_before,
            sample.query_context_before_valid.load(Ordering::Relaxed),
        );
        let query_context_after = format_atomic_words(
            &sample.query_context_after,
            sample.query_context_after_valid.load(Ordering::Relaxed),
        );
        let ammo_dispatch_pointer_byte_10 = if sample
            .ammo_dispatch_pointer_byte_10_valid
            .load(Ordering::Relaxed)
        {
            format!(
                "0x{:02X}",
                sample.ammo_dispatch_pointer_byte_10.load(Ordering::Relaxed)
            )
        } else {
            "unreadable".to_owned()
        };
        let candidate_total = sample.candidate_callback_count.load(Ordering::Relaxed);
        let candidate_min_fraction =
            f32::from_bits(sample.candidate_min_fraction.load(Ordering::Relaxed));
        let candidate_min_text = if candidate_min_fraction.is_finite() {
            format!("{candidate_min_fraction:.6}")
        } else {
            "none".to_owned()
        };
        let candidate_text = sample
            .candidate_samples
            .iter()
            .enumerate()
            .filter(|(_, candidate)| candidate.ready.load(Ordering::Acquire))
            .map(|(slot, candidate)| {
                let before = format_atomic_words(
                    &candidate.ray_record_before,
                    candidate.ray_record_before_valid.load(Ordering::Relaxed),
                );
                let after = format_atomic_words(
                    &candidate.ray_record_after,
                    candidate.ray_record_after_valid.load(Ordering::Relaxed),
                );
                let ray_input = format_atomic_words(
                    &candidate.ray_input_words,
                    candidate.ray_input_valid.load(Ordering::Relaxed),
                );
                let collector_before = format_atomic_words(
                    &candidate.collector_state_before,
                    candidate.collector_state_before_valid.load(Ordering::Relaxed),
                );
                let collector_after = format_atomic_words(
                    &candidate.collector_state_after,
                    candidate.collector_state_after_valid.load(Ordering::Relaxed),
                );
                let group_filter = if candidate
                    .group_filter_result_valid
                    .load(Ordering::Acquire)
                {
                    format!(
                        "decision={} ray=0x{:08X} candidate=0x{:08X}",
                        candidate.group_filter_result.load(Ordering::Relaxed),
                        candidate.group_filter_ray_info.load(Ordering::Relaxed),
                        candidate.group_filter_candidate_info.load(Ordering::Relaxed),
                    )
                } else {
                    "unobserved".to_owned()
                };
                let ray_filter_hits = candidate
                    .ray_filter_hits
                    .iter()
                    .filter(|hit| hit.state.load(Ordering::Acquire) == 2)
                    .map(|hit| {
                        let exclusion = hit.exclude_filter_info_mask.load(Ordering::Relaxed);
                        let required = hit.required_filter_info_mask.load(Ordering::Relaxed);
                        let candidate_info = hit.candidate_filter_info.load(Ordering::Relaxed);
                        let selected = hit.selected_shape_after.load(Ordering::Relaxed)
                            == hit.candidate_node.load(Ordering::Relaxed);
                        format!(
                            "vtable=0x{:X},exclude=0x{:08X},require=0x{:08X},info=0x{:08X},mask-pass={},owner=0x{:X},ignored-owner=0x{:X},owner-ignored-match={},selected={} before=0x{:X} after=0x{:X}",
                            hit.filter_vtable.load(Ordering::Relaxed),
                            exclusion,
                            required,
                            candidate_info,
                            (exclusion & candidate_info) == 0
                                && (required & candidate_info) == required,
                            hit.candidate_owner.load(Ordering::Relaxed),
                            hit.filter_ignored_owner.load(Ordering::Relaxed),
                            hit.filter_ignored_owner.load(Ordering::Relaxed) != 0
                                && hit.filter_ignored_owner.load(Ordering::Relaxed)
                                    == hit.candidate_owner.load(Ordering::Relaxed),
                            selected,
                            hit.selected_shape_before.load(Ordering::Relaxed),
                            hit.selected_shape_after.load(Ordering::Relaxed),
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" | ");
                let object_gate_word = if candidate
                    .object_gate_word_valid
                    .load(Ordering::Relaxed)
                {
                    format!("0x{:016X}", candidate.object_gate_word.load(Ordering::Relaxed))
                } else {
                    "unreadable".to_owned()
                };
                let object_local_transform = if candidate
                    .object_local_transform_valid
                    .load(Ordering::Relaxed)
                {
                    candidate
                        .object_local_transform
                        .iter()
                        .map(|word| format!("{:016X}", word.load(Ordering::Relaxed)))
                        .collect::<Vec<_>>()
                        .join(" ")
                } else {
                    "unreadable".to_owned()
                };
                let handle_relative_gate_word = if candidate
                    .handle_relative_gate_word_valid
                    .load(Ordering::Relaxed)
                {
                    format!(
                        "0x{:016X}",
                        candidate.handle_relative_gate_word.load(Ordering::Relaxed)
                    )
                } else {
                    "unreadable".to_owned()
                };
                let handle_relative_transform = if candidate
                    .handle_relative_transform_valid
                    .load(Ordering::Relaxed)
                {
                    candidate
                        .handle_relative_transform
                        .iter()
                        .map(|word| format!("{:016X}", word.load(Ordering::Relaxed)))
                        .collect::<Vec<_>>()
                        .join(" ")
                } else {
                    "unreadable".to_owned()
                };
                let subobject_gate_word = if candidate
                    .subobject_gate_word_valid
                    .load(Ordering::Relaxed)
                {
                    format!(
                        "0x{:016X}",
                        candidate.subobject_gate_word.load(Ordering::Relaxed)
                    )
                } else {
                    "unreadable".to_owned()
                };
                let subobject_transform = if candidate
                    .subobject_transform_valid
                    .load(Ordering::Relaxed)
                {
                    candidate
                        .subobject_transform
                        .iter()
                        .map(|word| format!("{:016X}", word.load(Ordering::Relaxed)))
                        .collect::<Vec<_>>()
                        .join(" ")
                } else {
                    "unreadable".to_owned()
                };
                format!(
                    "candidate[slot={slot},index={}] caster=0x{:X} cast={} handle=0x{:X} relative=0x{:X} object=0x{:X} subobject=0x{:X} vtable=0x{:X}/0x{:X} virtual=0x{:X}/0x{:X} object+0x28={object_gate_word} object+0x140=[{object_local_transform}] relative+0x28={handle_relative_gate_word} relative+0x140=[{handle_relative_transform}] subobject+0x28={subobject_gate_word} subobject+0x140=[{subobject_transform}] group-filter=[{group_filter}] ray-filter-hits=[{ray_filter_hits}] ray-input@0x{:X}=[{ray_input}] record=0x{:X} fraction={:.6} before=[{}] after=[{}] collector@0x{:X} before=[{collector_before}] after=[{collector_after}]",
                    candidate.callback_index.load(Ordering::Relaxed),
                    candidate.caster.load(Ordering::Relaxed),
                    candidate.cast_index.load(Ordering::Relaxed),
                    candidate.handle.load(Ordering::Relaxed),
                    candidate.handle_relative.load(Ordering::Relaxed),
                    candidate.candidate_object.load(Ordering::Relaxed),
                    candidate.object_subobject.load(Ordering::Relaxed),
                    candidate.object_vtable.load(Ordering::Relaxed),
                    candidate.subobject_vtable.load(Ordering::Relaxed),
                    candidate.callback_30.load(Ordering::Relaxed),
                    candidate.callback_80.load(Ordering::Relaxed),
                    candidate.ray_input_record.load(Ordering::Relaxed),
                    candidate.ray_record.load(Ordering::Relaxed),
                    f32::from_bits(candidate.returned_fraction.load(Ordering::Relaxed)),
                    before,
                    after,
                    candidate.collector_state_address.load(Ordering::Relaxed),
                )
            })
            .collect::<Vec<_>>()
            .join(" | ");
        let caller = sample.caller.load(Ordering::Relaxed);
        let ammo_dispatch =
            if caller.checked_sub(STEAM_IMAGE_BASE) == Some(AMMO_WORLD_QUERY_CALLER_RVA) {
                let vtable = if sample.ammo_receiver_vtable_valid.load(Ordering::Relaxed) {
                    format!(
                        "0x{:X}",
                        sample.ammo_receiver_vtable.load(Ordering::Relaxed)
                    )
                } else {
                    "unreadable".to_owned()
                };
                let slot_58 = if sample.ammo_dispatch_58_valid.load(Ordering::Relaxed) {
                    format!("0x{:X}", sample.ammo_dispatch_58.load(Ordering::Relaxed))
                } else {
                    "unreadable".to_owned()
                };
                let slot_60 = if sample.ammo_dispatch_60_valid.load(Ordering::Relaxed) {
                    format!("0x{:X}", sample.ammo_dispatch_60.load(Ordering::Relaxed))
                } else {
                    "unreadable".to_owned()
                };
                format!("vtable={vtable} slot+0x58={slot_58} slot+0x60={slot_60}")
            } else {
                "not-ammo-caller".to_owned()
            };
        let rdx_label = if sample.rdx_label_valid.load(Ordering::Relaxed) {
            let bytes: [u8; WORLD_QUERY_LABEL_CAPACITY] =
                std::array::from_fn(|slot| sample.rdx_label[slot].load(Ordering::Relaxed));
            let length = bytes
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(bytes.len());
            String::from_utf8_lossy(&bytes[..length]).into_owned()
        } else {
            "unresolved".to_owned()
        };
        let output_record_address = sample.output_record_address.load(Ordering::Relaxed);
        let query_context_address = sample.query_context_address.load(Ordering::Relaxed);
        let caller_rva = caller.checked_sub(STEAM_IMAGE_BASE).map_or_else(
            || "outside game image".to_owned(),
            |value| format!("0x{value:X}"),
        );
        let world_query_ray = if sample.query_data_valid.load(Ordering::Relaxed)
            && matches!(
                caller.checked_sub(STEAM_IMAGE_BASE),
                Some(
                    AMMO_WORLD_QUERY_CALLER_RVA
                        | HEIGHT_OVER_GROUND_WORLD_QUERY_CALLER_RVA
                        | SCRIPT_RAYCAST_WORLD_QUERY_CALLER_RVA
                )
            ) {
            let components: [f32; 6] = std::array::from_fn(|index| {
                let word = query_data[index / 2];
                let shift = (index % 2) * 32;
                f32::from_bits((word >> shift) as u32)
            });
            let slot5 = f32::from_bits(stack_arguments[0] as u32);
            match caller.checked_sub(STEAM_IMAGE_BASE) {
                Some(AMMO_WORLD_QUERY_CALLER_RVA) => format!(
                    "length={slot5:.3} origin=({:.3},{:.3},{:.3}) direction=({:.5},{:.5},{:.5})",
                    components[0],
                    components[1],
                    components[2],
                    components[3],
                    components[4],
                    components[5],
                ),
                Some(SCRIPT_RAYCAST_WORLD_QUERY_CALLER_RVA) => {
                    format!(
                        "slot5-scale={slot5:.3} vectors=({:.3},{:.3},{:.3}),({:.5},{:.5},{:.5})",
                        components[0],
                        components[1],
                        components[2],
                        components[3],
                        components[4],
                        components[5],
                    )
                }
                _ => format!(
                    "slot5={slot5:.3} origin=({:.3},{:.3},{:.3}) direction=({:.5},{:.5},{:.5})",
                    components[0],
                    components[1],
                    components[2],
                    components[3],
                    components[4],
                    components[5],
                ),
            }
        } else {
            "unavailable".to_owned()
        };
        let bounds_check_points = if sample.query_data_valid.load(Ordering::Relaxed)
            && sample.bounds_frame_origin_valid.load(Ordering::Relaxed)
        {
            let components: [f32; 6] = std::array::from_fn(|index| {
                let word = query_data[index / 2];
                let shift = (index % 2) * 32;
                f32::from_bits((word >> shift) as u32)
            });
            let first_scale = f32::from_bits(xmm_registers[6] as u32);
            let second_scale = f32::from_bits(stack_arguments[0] as u32);
            let point = |scale: f32| {
                std::array::from_fn::<_, 3, _>(|axis| {
                    components[axis] + components[axis + 3] * scale - bounds_frame_origin[axis]
                })
            };
            let first = point(first_scale);
            let second = point(second_scale);
            format!(
                "frame-origin=({:.3},{:.3},{:.3}) local-points=({:.3},{:.3},{:.3}),({:.3},{:.3},{:.3}) scales=({first_scale:.4},{second_scale:.4})",
                bounds_frame_origin[0],
                bounds_frame_origin[1],
                bounds_frame_origin[2],
                first[0],
                first[1],
                first[2],
                second[0],
                second[1],
                second[2],
            )
        } else {
            "unavailable".to_owned()
        };
        let bounds_check_results = format!(
            "count={} captured=[{}]",
            sample.bounds_check_count.load(Ordering::Relaxed),
            sample
                .bounds_checks
                .iter()
                .enumerate()
                .filter_map(|(ordinal, capture)| {
                    if !capture.valid.load(Ordering::Acquire) {
                        return None;
                    }
                    let point = capture
                        .point
                        .each_ref()
                        .map(|value| f32::from_bits(value.load(Ordering::Relaxed)));
                    let radius = f32::from_bits(capture.radius.load(Ordering::Relaxed));
                    let result = capture.result.load(Ordering::Relaxed) != 0;
                    Some(format!(
                        "#{ordinal} point=({:.3},{:.3},{:.3}) radius={radius:.4} result={result}",
                        point[0], point[1], point[2],
                    ))
                })
                .collect::<Vec<_>>()
                .join(" | "),
        );
        let ammo_result = if caller.checked_sub(STEAM_IMAGE_BASE)
            == Some(AMMO_WORLD_QUERY_CALLER_RVA)
            && sample.query_data_valid.load(Ordering::Relaxed)
        {
            let components: [f32; 6] = std::array::from_fn(|index| {
                let word = query_data[index / 2];
                let shift = (index % 2) * 32;
                f32::from_bits((word >> shift) as u32)
            });
            let ray_length = f32::from_bits(stack_arguments[0] as u32);
            if sample.result.load(Ordering::Relaxed) != 0
                && sample.output_record_after_valid.load(Ordering::Relaxed)
            {
                let output: [u64; 10] = std::array::from_fn(|index| {
                    sample.output_record_after[index].load(Ordering::Relaxed)
                });
                let segment_scale = f32::from_bits((output[2] >> 32) as u32);
                let scaled_distance = segment_scale * ray_length;
                let position = [
                    components[0] + components[3] * scaled_distance,
                    components[1] + components[4] * scaled_distance,
                    components[2] + components[5] * scaled_distance,
                ];
                format!(
                    "true-return-output-scalar={segment_scale:.6} scalar-distance={scaled_distance:.3} dispatch-pointer=0x{:X} caller-position=({:.3},{:.3},{:.3})",
                    output[5], position[0], position[1], position[2],
                )
            } else if sample.result.load(Ordering::Relaxed) == 0 {
                let endpoint = [
                    components[0] + components[3] * ray_length,
                    components[1] + components[4] * ray_length,
                    components[2] + components[5] * ray_length,
                ];
                format!(
                    "false-return-endpoint=({:.3},{:.3},{:.3})",
                    endpoint[0], endpoint[1], endpoint[2],
                )
            } else {
                "true-return-output-unreadable".to_owned()
            }
        } else {
            "unavailable".to_owned()
        };
        append_log(&format!(
            "World query sample #{} capture-ordinal={} caller={} label={:?} result={} ray={} bounds-check={} bounds-results={} ammo-result={} ammo-dispatch-pointer-byte10={} ammo-dispatch={} R8-data-before[0..0x60]={} R8-data-after[0..0x60]={} {} {} stack-slots={} {} candidate-callbacks-query-thread={} candidate-min-fraction-query-thread={} candidates=[{}] output-size=0x50 output@0x{:X} before=[{}] after=[{}] context@0x{:X} before=[{}] after=[{}] RCX=0x{:X} RDX=0x{:X} R8=0x{:X} R9=0x{:X} RAX=0x{:X} R10=0x{:X} R11=0x{:X} RBX=0x{:X} RBP=0x{:X} RSI=0x{:X} RDI=0x{:X} R12=0x{:X} R13=0x{:X} R14=0x{:X} R15=0x{:X} XMM0=[{:016X}:{:016X}] XMM1=[{:016X}:{:016X}] XMM2=[{:016X}:{:016X}] XMM3=[{:016X}:{:016X}] XMM4=[{:016X}:{:016X}] XMM5=[{:016X}:{:016X}] nonvolatile-XMM={}.",
            sample.sequence.load(Ordering::Relaxed),
            sample.capture_ordinal.load(Ordering::Relaxed),
            caller_rva,
            rdx_label,
            sample.result.load(Ordering::Relaxed),
            world_query_ray,
            bounds_check_points,
            bounds_check_results,
            ammo_result,
            ammo_dispatch_pointer_byte_10,
            ammo_dispatch,
            query_data_text,
            query_data_after_text,
            indirect_text,
            script_argument_text,
            stack_arguments_text,
            argument_11_override,
            candidate_total,
            candidate_min_text,
            candidate_text,
            output_record_address,
            output_record_before,
            output_record_after,
            query_context_address,
            query_context_before,
            query_context_after,
            integer_registers[0],
            integer_registers[1],
            integer_registers[2],
            integer_registers[3],
            integer_registers[4],
            integer_registers[5],
            integer_registers[6],
            nonvolatile_registers[0],
            nonvolatile_registers[1],
            nonvolatile_registers[2],
            nonvolatile_registers[3],
            nonvolatile_registers[4],
            nonvolatile_registers[5],
            nonvolatile_registers[6],
            nonvolatile_registers[7],
            xmm_registers[1],
            xmm_registers[0],
            xmm_registers[3],
            xmm_registers[2],
            xmm_registers[5],
            xmm_registers[4],
            xmm_registers[7],
            xmm_registers[6],
            xmm_registers[9],
            xmm_registers[8],
            xmm_registers[11],
            xmm_registers[10],
            nonvolatile_xmm_text,
        ));
        WORLD_QUERY_SAMPLES_LOGGED[index].store(true, Ordering::Release);
    }
}
