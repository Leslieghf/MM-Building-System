//! Guarded reads and memory-region checks used before native memory access.
use super::*;

pub(super) fn matches_supported_executable() -> bool {
    let Some(path) = env::current_exe().ok() else {
        return false;
    };
    let Ok(bytes) = fs::read(path) else {
        return false;
    };
    if bytes.len() as u64 != STEAM_FILE_SIZE || bytes.get(0..2) != Some(b"MZ") {
        return false;
    }
    let Some(pe_offset) = read_u32(&bytes, 0x3C).map(|offset| offset as usize) else {
        return false;
    };
    let Some(signature_end) = pe_offset.checked_add(4) else {
        return false;
    };
    if bytes.get(pe_offset..signature_end) != Some(b"PE\0\0") {
        return false;
    }
    let Some(timestamp_offset) = pe_offset.checked_add(8) else {
        return false;
    };
    read_u32(&bytes, timestamp_offset) == Some(STEAM_TIMESTAMP)
}

pub(super) fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let end = offset.checked_add(4)?;
    let value: [u8; 4] = bytes.get(offset..end)?.try_into().ok()?;
    Some(u32::from_le_bytes(value))
}

pub(super) fn is_readable_executable_page(address: usize) -> bool {
    let Some(information) = query_page(address) else {
        return false;
    };
    if information.state != MEM_COMMIT || information.protect & PAGE_GUARD != 0 {
        return false;
    }

    matches!(
        information.protect & 0xFF,
        PAGE_EXECUTE_READ | PAGE_EXECUTE_READWRITE | PAGE_EXECUTE_WRITECOPY
    )
}

pub(super) fn readable_span(address: usize, length: usize) -> bool {
    let Some(end) = address.checked_add(length) else {
        return false;
    };
    if length == 0 {
        return true;
    }

    let Some(information) = query_page(address) else {
        return false;
    };
    let region_start = information.base_address as usize;
    let Some(region_end) = region_start.checked_add(information.region_size) else {
        return false;
    };
    if information.state != MEM_COMMIT
        || information.protect & PAGE_GUARD != 0
        || !matches!(
            information.protect & 0xFF,
            PAGE_READONLY
                | PAGE_READWRITE
                | PAGE_WRITECOPY
                | PAGE_EXECUTE_READ
                | PAGE_EXECUTE_READWRITE
                | PAGE_EXECUTE_WRITECOPY
        )
    {
        return false;
    }

    address >= region_start && end <= region_end
}

pub(super) fn writable_span(address: usize, length: usize) -> bool {
    let Some(end) = address.checked_add(length) else {
        return false;
    };
    if length == 0 {
        return true;
    }

    let Some(information) = query_page(address) else {
        return false;
    };
    let region_start = information.base_address as usize;
    let Some(region_end) = region_start.checked_add(information.region_size) else {
        return false;
    };
    information.state == MEM_COMMIT
        && information.protect & PAGE_GUARD == 0
        && information.protect & PAGE_WRITABLE_MASK != 0
        && address >= region_start
        && end <= region_end
}

pub(super) fn input_focus_address() -> Option<usize> {
    if DEVICE_MANAGER_SINGLETON == 0
        || !readable_span(DEVICE_MANAGER_SINGLETON, size_of::<*mut c_void>())
    {
        return None;
    }
    let device_manager =
        unsafe { ptr::read_volatile(DEVICE_MANAGER_SINGLETON as *const *mut c_void) as usize };
    if device_manager == 0 {
        return None;
    }
    let input_manager = device_manager.checked_add(STEAM_DEVICE_MANAGER_INPUT_MANAGER_OFFSET)?;
    let focus_address = input_manager.checked_add(INPUT_DEVICE_MANAGER_IS_IN_FOCUS_OFFSET)?;
    readable_span(focus_address, size_of::<u8>()).then_some(focus_address)
}

pub(super) fn synchronize_game_input_focus(overlap_epoch: u64) {
    if CALLBACK_IN_FLIGHT.load(Ordering::Acquire) != 1
        || CALLBACK_OVERLAP_EPOCH.load(Ordering::Acquire) != overlap_epoch
    {
        return;
    }
    let Some(address) = input_focus_address() else {
        return;
    };
    let current = unsafe { ptr::read_volatile(address as *const u8) };
    if current > 1 {
        return;
    }

    let capture_requested = UI_INPUT_CAPTURE_REQUESTED.load(Ordering::Acquire);
    let mut saved_focus = SAVED_GAME_INPUT_FOCUS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let target = if capture_requested {
        let _ = saved_focus.get_or_insert(current != 0);
        Some(false)
    } else {
        saved_focus.map(|was_focused| was_focused && GAME_WINDOW_ACTIVE.load(Ordering::Acquire))
    };
    let Some(target) = target else {
        return;
    };
    if current == u8::from(target) {
        if !capture_requested {
            *saved_focus = None;
        }
        return;
    }
    if CALLBACK_IN_FLIGHT.load(Ordering::Acquire) != 1
        || CALLBACK_OVERLAP_EPOCH.load(Ordering::Acquire) != overlap_epoch
        || !writable_span(address, size_of::<u8>())
    {
        return;
    }
    unsafe { ptr::write_volatile(address as *mut u8, u8::from(target)) };
    if !capture_requested {
        *saved_focus = None;
    }
}

pub(super) fn query_page(address: usize) -> Option<MemoryBasicInformation> {
    let mut information = MaybeUninit::<MemoryBasicInformation>::zeroed();
    let result = unsafe {
        VirtualQuery(
            address as *const c_void,
            information.as_mut_ptr(),
            size_of::<MemoryBasicInformation>(),
        )
    };
    (result == size_of::<MemoryBasicInformation>()).then(|| unsafe { information.assume_init() })
}

pub(super) fn install_hook(target: usize) {
    let initialized = unsafe { mh::MH_Initialize() };
    if initialized != MH_STATUS::MH_OK && initialized != MH_STATUS::MH_ERROR_ALREADY_INITIALIZED {
        append_log(&format!(
            "UpdatePostSim probe refused: MinHook initialization failed ({initialized:?})."
        ));
        return;
    }

    let address = target as *mut c_void;
    let detour = update_post_sim_hook as *const () as *mut c_void;
    let mut original = ptr::null_mut();
    let created = unsafe { mh::MH_CreateHook(address, detour, &mut original) };
    if created != MH_STATUS::MH_OK {
        append_log(&format!(
            "UpdatePostSim probe refused: MinHook could not create the callback hook ({created:?})."
        ));
        return;
    }
    ORIGINAL_UPDATE_POST_SIM.store(original, Ordering::Release);

    let enabled = unsafe { mh::MH_EnableHook(address) };
    if enabled != MH_STATUS::MH_OK {
        ORIGINAL_UPDATE_POST_SIM.store(ptr::null_mut(), Ordering::Release);
        append_log(&format!(
            "UpdatePostSim probe refused: MinHook could not enable the callback hook ({enabled:?})."
        ));
        return;
    }

    UPDATE_POST_SIM_INSTALLED.store(true, Ordering::Release);
    append_log(&format!(
        "Callback-only hook installed on UpdatePostSim at 0x{target:X}; waiting for callback invocations."
    ));
}
