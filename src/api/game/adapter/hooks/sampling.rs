//! Passive player, vehicle, camera, and weapon hook setup and callbacks.
use super::super::*;

pub(in crate::api::game::adapter) fn install_camera_raycast_hook(image_base: usize) {
    let Some(target) = image_base.checked_add(RAYCAST_STATIC_FROM_CAMERA_RVA) else {
        append_log("Camera ray tap refused: callback address overflowed.");
        return;
    };
    if !is_readable_executable_page(target) {
        append_log("Camera ray tap refused: callback address is not executable code.");
        return;
    }

    let address = target as *mut c_void;
    let detour = camera_raycast_hook as *const () as *mut c_void;
    let mut original = ptr::null_mut();
    let created = unsafe { mh::MH_CreateHook(address, detour, &mut original) };
    if created != MH_STATUS::MH_OK {
        append_log(&format!(
            "Camera ray tap refused: MinHook could not create the callback hook ({created:?})."
        ));
        return;
    }
    ORIGINAL_CAMERA_RAYCAST.store(original, Ordering::Release);

    let enabled = unsafe { mh::MH_EnableHook(address) };
    if enabled != MH_STATUS::MH_OK {
        ORIGINAL_CAMERA_RAYCAST.store(ptr::null_mut(), Ordering::Release);
        append_log(&format!(
            "Camera ray tap refused: MinHook could not enable the callback hook ({enabled:?})."
        ));
        return;
    }

    CAMERA_RAYCAST_HOOK_INSTALLED.store(true, Ordering::Release);
    append_log(&format!(
        "Passive camera ray callback tap installed at 0x{target:X}; waiting for game-script calls."
    ));
}

pub(in crate::api::game::adapter) unsafe extern "system" fn camera_raycast_hook(
    arg0: *mut c_void,
    arg1: *mut c_void,
) -> u32 {
    CAMERA_RAYCAST_CALLS.fetch_add(1, Ordering::Relaxed);
    let original = ORIGINAL_CAMERA_RAYCAST.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    let original: CameraRaycastCallback = unsafe { std::mem::transmute(original) };
    unsafe { original(arg0, arg1) }
}

pub(in crate::api::game::adapter) fn install_weapon_ray_position_hook(image_base: usize) {
    let Some(target) = image_base.checked_add(WEAPON_RAY_POSITION_RVA) else {
        append_log("Weapon ray position tap refused: function address overflowed.");
        return;
    };
    if !is_readable_executable_page(target) {
        append_log("Weapon ray position tap refused: function address is not executable code.");
        return;
    }

    const EXPECTED_ENTRY: [u8; 16] = [
        0x48, 0x89, 0xE0, 0x55, 0x53, 0x56, 0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x48, 0x8D,
        0xA8,
    ];
    if !readable_span(target, EXPECTED_ENTRY.len()) {
        append_log("Weapon ray position tap refused: function entry bytes are not readable.");
        return;
    }
    let entry = unsafe { std::slice::from_raw_parts(target as *const u8, EXPECTED_ENTRY.len()) };
    let entry_text = entry
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ");
    append_log(&format!("Weapon ray position entry bytes: {entry_text}."));
    if entry != EXPECTED_ENTRY {
        append_log(
            "Weapon ray position tap refused: function signature did not match the observed Steam build.",
        );
        return;
    }

    let address = target as *mut c_void;
    let detour = weapon_ray_position_hook as *const () as *mut c_void;
    let mut original = ptr::null_mut();
    let created = unsafe { mh::MH_CreateHook(address, detour, &mut original) };
    if created != MH_STATUS::MH_OK {
        append_log(&format!(
            "Weapon ray position tap refused: MinHook could not create the hook ({created:?})."
        ));
        return;
    }
    ORIGINAL_WEAPON_RAY_POSITION.store(original, Ordering::Release);

    let enabled = unsafe { mh::MH_EnableHook(address) };
    if enabled != MH_STATUS::MH_OK {
        ORIGINAL_WEAPON_RAY_POSITION.store(ptr::null_mut(), Ordering::Release);
        append_log(&format!(
            "Weapon ray position tap refused: MinHook could not enable the hook ({enabled:?})."
        ));
        return;
    }

    WEAPON_RAY_POSITION_HOOK_INSTALLED.store(true, Ordering::Release);
    start_weapon_ray_sample_log_worker();
    append_log(&format!(
        "Passive CWeapon::GetRayWorldPosition observer installed at 0x{target:X}; recording up to {WEAPON_RAY_SAMPLE_CAPACITY} sampled ray inputs and returned positions."
    ));
}

pub(in crate::api::game::adapter) unsafe extern "system" fn weapon_ray_position_hook(
    weapon: *mut c_void,
    output: *mut f32,
    ray: *const f32,
) -> *mut f32 {
    let call = WEAPON_RAY_POSITION_CALLS.fetch_add(1, Ordering::Relaxed) + 1;
    let sample_this_call = call <= 8 || call.is_multiple_of(30);
    let ray_values =
        if sample_this_call && !ray.is_null() && readable_span(ray as usize, size_of::<[f32; 6]>())
        {
            Some(unsafe { ptr::read_unaligned(ray.cast::<[f32; 6]>()) })
        } else {
            None
        };

    let original = ORIGINAL_WEAPON_RAY_POSITION.load(Ordering::Acquire);
    if original.is_null() {
        return ptr::null_mut();
    }
    let original: WeaponRayPosition = unsafe { std::mem::transmute(original) };
    let returned = unsafe { original(weapon, output, ray) };

    let Some(ray_values) = ray_values else {
        return returned;
    };
    if output.is_null() || !readable_span(output as usize, size_of::<[f32; 3]>()) {
        return returned;
    }
    let result = unsafe { ptr::read_unaligned(output.cast::<[f32; 3]>()) };
    let slot_index = WEAPON_RAY_SAMPLE_COUNT.fetch_add(1, Ordering::Relaxed);
    let Some(slot) = WEAPON_RAY_SAMPLES.get(slot_index) else {
        return returned;
    };
    slot.call.store(call, Ordering::Relaxed);
    slot.weapon.store(weapon as usize, Ordering::Relaxed);
    for (destination, value) in slot.ray.iter().zip(ray_values) {
        destination.store(value.to_bits(), Ordering::Relaxed);
    }
    for (destination, value) in slot.result.iter().zip(result) {
        destination.store(value.to_bits(), Ordering::Relaxed);
    }
    slot.returned.store(returned as usize, Ordering::Relaxed);
    slot.ready.store(true, Ordering::Release);
    returned
}

pub(in crate::api::game::adapter) fn install_player_aim_filter_candidate_test_hook(
    image_base: usize,
) {
    let Some(target) = image_base.checked_add(PLAYER_AIM_FILTER_CANDIDATE_TEST_RVA) else {
        append_log("Player aim filter tap refused: predicate address overflowed.");
        return;
    };
    if !is_readable_executable_page(target) {
        append_log("Player aim filter tap refused: predicate address is not executable code.");
        return;
    }

    if !readable_span(target, PLAYER_AIM_FILTER_CANDIDATE_TEST_ENTRY.len()) {
        append_log("Player aim filter tap refused: predicate entry bytes are not readable.");
        return;
    }
    let entry = unsafe {
        std::slice::from_raw_parts(
            target as *const u8,
            PLAYER_AIM_FILTER_CANDIDATE_TEST_ENTRY.len(),
        )
    };
    let entry_text = entry
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ");
    append_log(&format!(
        "Player aim filter predicate entry bytes: {entry_text}."
    ));
    if entry != PLAYER_AIM_FILTER_CANDIDATE_TEST_ENTRY {
        append_log(
            "Player aim filter tap refused: predicate signature did not match the observed Steam build.",
        );
        return;
    }

    let address = target as *mut c_void;
    let detour = player_aim_filter_candidate_test_hook as *const () as *mut c_void;
    let mut original = ptr::null_mut();
    let created = unsafe { mh::MH_CreateHook(address, detour, &mut original) };
    if created != MH_STATUS::MH_OK {
        append_log(&format!(
            "Player aim filter tap refused: MinHook could not create the predicate hook ({created:?})."
        ));
        return;
    }
    ORIGINAL_PLAYER_AIM_FILTER_CANDIDATE_TEST.store(original, Ordering::Release);

    let enabled = unsafe { mh::MH_EnableHook(address) };
    if enabled != MH_STATUS::MH_OK {
        ORIGINAL_PLAYER_AIM_FILTER_CANDIDATE_TEST.store(ptr::null_mut(), Ordering::Release);
        append_log(&format!(
            "Player aim filter tap refused: MinHook could not enable the predicate hook ({enabled:?})."
        ));
        return;
    }

    PLAYER_AIM_FILTER_CANDIDATE_TEST_HOOK_INSTALLED.store(true, Ordering::Release);
    start_player_aim_filter_activity_log_worker();
    append_log(&format!(
        "Passive CPlayerAimRaycastFilter candidate-test tap installed at 0x{target:X}; call counts only."
    ));
}

pub(in crate::api::game::adapter) unsafe extern "system" fn player_aim_filter_candidate_test_hook(
    filter: *mut c_void,
    candidate: *mut c_void,
) -> u8 {
    PLAYER_AIM_FILTER_CANDIDATE_TEST_CALLS.fetch_add(1, Ordering::Relaxed);
    let original = ORIGINAL_PLAYER_AIM_FILTER_CANDIDATE_TEST.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    let original: PlayerAimFilterCandidateTest = unsafe { std::mem::transmute(original) };
    unsafe { original(filter, candidate) }
}

pub(in crate::api::game::adapter) fn start_player_aim_filter_activity_log_worker() {
    if let Err(error) = thread::Builder::new()
        .name("madmax-player-aim-filter-log".to_owned())
        .spawn(player_aim_filter_activity_log_worker)
    {
        append_log(&format!(
            "Could not start player aim filter activity logger: {error}."
        ));
    }
}

pub(in crate::api::game::adapter) fn start_player_aim_filter_hook_install_worker(
    image_base: usize,
) {
    if let Err(error) = thread::Builder::new()
        .name("madmax-player-aim-filter-hook".to_owned())
        .spawn(move || {
            append_log("Waiting for the player aim filter predicate code to materialize.");
            let started_at = Instant::now();
            while started_at.elapsed() < HOOK_TIMEOUT {
                let Some(target) = image_base.checked_add(PLAYER_AIM_FILTER_CANDIDATE_TEST_RVA)
                else {
                    append_log("Player aim filter tap refused: predicate address overflowed.");
                    return;
                };
                if is_readable_executable_page(target)
                    && readable_span(target, PLAYER_AIM_FILTER_CANDIDATE_TEST_ENTRY.len())
                {
                    let entry = unsafe {
                        std::slice::from_raw_parts(
                            target as *const u8,
                            PLAYER_AIM_FILTER_CANDIDATE_TEST_ENTRY.len(),
                        )
                    };
                    if entry == PLAYER_AIM_FILTER_CANDIDATE_TEST_ENTRY {
                        install_player_aim_filter_candidate_test_hook(image_base);
                        return;
                    }
                }
                thread::sleep(Duration::from_millis(250));
            }
            append_log(
                "Player aim filter tap not installed: predicate code/signature did not become available within 120 seconds.",
            );
        })
    {
        append_log(&format!(
            "Could not start player aim filter hook installer: {error}."
        ));
    }
}

pub(in crate::api::game::adapter) fn player_aim_filter_activity_log_worker() {
    let mut previous_total = 0;
    loop {
        thread::sleep(WORLD_QUERY_CALLER_REPORT_INTERVAL);
        let total = PLAYER_AIM_FILTER_CANDIDATE_TEST_CALLS.load(Ordering::Relaxed);
        let calls = total.saturating_sub(previous_total);
        append_log(&format!(
            "Player aim filter activity: window={}s calls={} total={total}.",
            WORLD_QUERY_CALLER_REPORT_INTERVAL.as_secs(),
            calls,
        ));
        previous_total = total;
    }
}

pub(in crate::api::game::adapter) fn start_weapon_ray_sample_log_worker() {
    append_log(
        "Weapon ray sample logger started; it records the first 8 calls and then every 30th call.",
    );
    if let Err(error) = thread::Builder::new()
        .name("madmax-weapon-ray-samples".to_owned())
        .spawn(weapon_ray_sample_log_worker)
    {
        append_log(&format!(
            "Could not start weapon ray sample logger: {error}. Samples will remain in memory only."
        ));
    }
}

pub(in crate::api::game::adapter) fn weapon_ray_sample_log_worker() {
    let mut logged = [false; WEAPON_RAY_SAMPLE_CAPACITY];
    loop {
        for (index, slot) in WEAPON_RAY_SAMPLES.iter().enumerate() {
            if logged[index] || !slot.ready.load(Ordering::Acquire) {
                continue;
            }
            let ray: [f32; 6] = std::array::from_fn(|index| {
                f32::from_bits(slot.ray[index].load(Ordering::Relaxed))
            });
            let result: [f32; 3] = std::array::from_fn(|index| {
                f32::from_bits(slot.result[index].load(Ordering::Relaxed))
            });
            append_log(&format!(
                "Weapon ray sample #{}: weapon=0x{:X}, origin=({:.3}, {:.3}, {:.3}), direction=({:.5}, {:.5}, {:.5}), result=({:.3}, {:.3}, {:.3}), returned=0x{:X}.",
                slot.call.load(Ordering::Relaxed),
                slot.weapon.load(Ordering::Relaxed),
                ray[0],
                ray[1],
                ray[2],
                ray[3],
                ray[4],
                ray[5],
                result[0],
                result[1],
                result[2],
                slot.returned.load(Ordering::Relaxed),
            ));
            logged[index] = true;
        }
        thread::sleep(Duration::from_millis(250));
    }
}
