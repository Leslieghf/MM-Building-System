use crate::overlay::append_log;
use hudhook::mh::{self, MH_STATUS};
use hudhook::windows::Win32::System::LibraryLoader::GetModuleHandleW;
use std::env;
use std::ffi::c_void;
use std::fs;
use std::mem::{size_of, MaybeUninit};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

const STEAM_TIMESTAMP: u32 = 0x5628_A448;
const STEAM_FILE_SIZE: u64 = 76_814_792;
const STEAM_IMAGE_BASE: usize = 0x14000_0000;
const UPDATE_POST_SIM_RVA: usize = 0x01FA_5A10;
const HOOK_TIMEOUT: Duration = Duration::from_secs(120);

const MEM_COMMIT: u32 = 0x1000;
const PAGE_EXECUTE_READ: u32 = 0x20;
const PAGE_EXECUTE_READWRITE: u32 = 0x40;
const PAGE_EXECUTE_WRITECOPY: u32 = 0x80;
const PAGE_GUARD: u32 = 0x100;

#[repr(C)]
struct MemoryBasicInformation {
    base_address: *mut c_void,
    allocation_base: *mut c_void,
    allocation_protect: u32,
    partition_id: u16,
    region_size: usize,
    state: u32,
    protect: u32,
    kind: u32,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn VirtualQuery(
        address: *const c_void,
        information: *mut MemoryBasicInformation,
        information_length: usize,
    ) -> usize;
}

type UpdatePostSim = unsafe extern "system" fn(*mut c_void);

static ORIGINAL_UPDATE_POST_SIM: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static UPDATE_POST_SIM_INSTALLED: AtomicBool = AtomicBool::new(false);
static UPDATE_POST_SIM_CALLS: AtomicU64 = AtomicU64::new(0);

pub(crate) fn callback_count() -> Option<u64> {
    UPDATE_POST_SIM_INSTALLED
        .load(Ordering::Acquire)
        .then(|| UPDATE_POST_SIM_CALLS.load(Ordering::Acquire))
}

pub(crate) fn start_update_callback_probe() {
    append_log("Starting the UpdatePostSim callback-only probe worker.");
    let worker = thread::Builder::new()
        .name("madmax-update-callback-probe".to_owned())
        .spawn(install_update_post_sim_hook);
    if let Err(error) = worker {
        append_log(&format!("Could not start UpdatePostSim probe: {error}"));
    }
}

fn install_update_post_sim_hook() {
    if !matches_supported_executable() {
        append_log("UpdatePostSim probe refused: executable timestamp or size did not match Steam 2015-10-22.");
        return;
    }

    let module = unsafe { GetModuleHandleW(hudhook::windows::core::PCWSTR::null()) };
    let Ok(module) = module else {
        append_log("UpdatePostSim probe refused: could not read the game image base.");
        return;
    };
    let image_base = module.0 as usize;
    if image_base != STEAM_IMAGE_BASE {
        append_log(&format!(
            "UpdatePostSim probe refused: unexpected image base 0x{image_base:X}."
        ));
        return;
    }

    let Some(target) = image_base.checked_add(UPDATE_POST_SIM_RVA) else {
        append_log("UpdatePostSim probe refused: callback address overflowed.");
        return;
    };

    append_log("Waiting for the Steam UpdatePostSim callback to become readable executable code.");
    let started_at = Instant::now();
    while started_at.elapsed() < HOOK_TIMEOUT {
        if is_readable_executable_page(target) {
            install_hook(target);
            return;
        }
        thread::sleep(Duration::from_millis(250));
    }
    append_log("UpdatePostSim probe refused: callback did not become readable executable code within 120 seconds.");
}

fn matches_supported_executable() -> bool {
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

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let end = offset.checked_add(4)?;
    let value: [u8; 4] = bytes.get(offset..end)?.try_into().ok()?;
    Some(u32::from_le_bytes(value))
}

fn is_readable_executable_page(address: usize) -> bool {
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

fn query_page(address: usize) -> Option<MemoryBasicInformation> {
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

fn install_hook(target: usize) {
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

unsafe extern "system" fn update_post_sim_hook(context: *mut c_void) {
    let original = ORIGINAL_UPDATE_POST_SIM.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    let original: UpdatePostSim = unsafe { std::mem::transmute(original) };
    unsafe { original(context) };
    UPDATE_POST_SIM_CALLS.fetch_add(1, Ordering::Release);
}
