use crate::overlay::append_log;
use hudhook::mh::{self, MH_STATUS};
use hudhook::windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use hudhook::windows::Win32::System::LibraryLoader::GetModuleHandleW;
use hudhook::windows::Win32::System::Threading::GetCurrentProcess;
use hudhook::windows::core::PCWSTR;
use std::env;
use std::ffi::c_void;
use std::fs;
use std::mem::{size_of, MaybeUninit};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const STEAM_TIMESTAMP: u32 = 0x5628_A448;
const STEAM_FILE_SIZE: u64 = 76_814_792;
const STEAM_IMAGE_BASE: usize = 0x14000_0000;
const UPDATE_POST_SIM_RVA: usize = 0x01FA_5A10;
const STARTUP_RETURN_ADDRESS: usize = 0x1430_3978D;
const CHARACTER_MANAGER_SINGLETON_RVA: usize = 0x017F_5338;
const GET_PLAYER_CHARACTER_RVA: usize = 0x01C9_1250;
// SDK layout: CPhysicsGameObject places its one-vptr IPfxGameObject base first,
// then its CGameObject base. GetTransform follows CRtti's virtual destructor
// and 21 earlier CGameObject virtual methods.
const GAME_OBJECT_SUBOBJECT_OFFSET: usize = 8;
const GET_TRANSFORM_VTABLE_SLOT: usize = 22;
const HOOK_TIMEOUT: Duration = Duration::from_secs(120);
const SAMPLE_INTERVAL_MS: u64 = 500;
const MAX_REASONABLE_COORDINATE: f32 = 1_000_000.0;

const MEM_COMMIT: u32 = 0x1000;
const PAGE_READONLY: u32 = 0x02;
const PAGE_READWRITE: u32 = 0x04;
const PAGE_WRITECOPY: u32 = 0x08;
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
    fn GetStartupInfoW(information: *mut c_void);
    fn VirtualQuery(
        address: *const c_void,
        information: *mut MemoryBasicInformation,
        information_length: usize,
    ) -> usize;
}

type GetStartupInfoW = unsafe extern "system" fn(*mut c_void);
type UpdatePostSim = unsafe extern "system" fn(*mut c_void);
type GetPlayerCharacter = unsafe extern "system" fn(*mut c_void) -> *mut c_void;
type GetTransform = unsafe extern "system" fn(*mut c_void, *mut Matrix4x4) -> bool;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Matrix4x4 {
    values: [[f32; 4]; 4],
}

static ORIGINAL_GET_STARTUP_INFO: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_UPDATE_POST_SIM: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static STARTUP_GATE_TRIGGERED: AtomicBool = AtomicBool::new(false);
static POSITION_X: AtomicU32 = AtomicU32::new(0);
static POSITION_Y: AtomicU32 = AtomicU32::new(0);
static POSITION_Z: AtomicU32 = AtomicU32::new(0);
static POSITION_UPDATED_AT: AtomicU64 = AtomicU64::new(0);
static POSITION_SAMPLE_IN_PROGRESS: AtomicBool = AtomicBool::new(false);
static LAST_SAMPLE_AT: AtomicU64 = AtomicU64::new(0);
static LAST_FAILURE_LOG_AT: AtomicU64 = AtomicU64::new(0);
static LAST_POSITION_LOG_AT: AtomicU64 = AtomicU64::new(0);

pub(crate) fn latest_player_position() -> Option<(f32, f32, f32)> {
    let updated_at = POSITION_UPDATED_AT.load(Ordering::Acquire);
    if updated_at == 0 || unix_millis().saturating_sub(updated_at) > 3_000 {
        return None;
    }

    Some((
        f32::from_bits(POSITION_X.load(Ordering::Relaxed)),
        f32::from_bits(POSITION_Y.load(Ordering::Relaxed)),
        f32::from_bits(POSITION_Z.load(Ordering::Relaxed)),
    ))
}


#[unsafe(naked)]
unsafe extern "system" fn startup_info_detour(_information: *mut c_void) {
    std::arch::naked_asm!(
        "mov rdx, qword ptr [rsp]",
        "jmp {handler}",
        handler = sym startup_info_detour_body,
    )
}

unsafe extern "system" fn startup_info_detour_body(
    information: *mut c_void,
    return_address: *const c_void,
) {
    let original = ORIGINAL_GET_STARTUP_INFO.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    let original: GetStartupInfoW = unsafe { std::mem::transmute(original) };
    unsafe { original(information) };

    if return_address as usize != STARTUP_RETURN_ADDRESS
        || STARTUP_GATE_TRIGGERED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
            .is_err()
    {
        return;
    }

    append_log("SDK startup gate matched _tmainCRTStartup; starting the player-position hook worker.");
    let worker = thread::Builder::new()
        .name("madmax-player-position-probe".to_owned())
        .spawn(install_player_position_hook);
    if let Err(error) = worker {
        append_log(&format!("Could not start player-position probe: {error}"));
    }
}

pub(crate) fn install_startup_gate() {
    if !matches_supported_executable() {
        append_log("Player-position probe refused: executable timestamp or size did not match Steam 2015-10-22.");
        return;
    }

    let module = unsafe { GetModuleHandleW(PCWSTR::null()) };
    let Ok(module) = module else {
        append_log("Player-position probe refused: could not read the game image base.");
        return;
    };
    let image_base = module.0 as usize;
    if image_base != STEAM_IMAGE_BASE {
        append_log(&format!(
            "Player-position probe refused: unexpected image base 0x{image_base:X}."
        ));
        return;
    }

    let initialized = unsafe { mh::MH_Initialize() };
    if initialized != MH_STATUS::MH_OK && initialized != MH_STATUS::MH_ERROR_ALREADY_INITIALIZED {
        append_log(&format!(
            "Player-position probe refused: MinHook initialization failed ({initialized:?})."
        ));
        return;
    }

    let target = unsafe { GetStartupInfoW as *const () as *mut c_void };
    let detour = startup_info_detour as *const () as *mut c_void;
    let mut original = ptr::null_mut();
    let created = unsafe { mh::MH_CreateHook(target, detour, &mut original) };
    if created != MH_STATUS::MH_OK {
        append_log(&format!(
            "Player-position probe refused: MinHook could not create the GetStartupInfoW gate ({created:?})."
        ));
        return;
    }
    ORIGINAL_GET_STARTUP_INFO.store(original, Ordering::Release);

    let enabled = unsafe { mh::MH_EnableHook(target) };
    if enabled != MH_STATUS::MH_OK {
        ORIGINAL_GET_STARTUP_INFO.store(ptr::null_mut(), Ordering::Release);
        append_log(&format!(
            "Player-position probe refused: MinHook could not enable the GetStartupInfoW gate ({enabled:?})."
        ));
        return;
    }
    append_log("GetStartupInfoW startup gate installed; waiting for the SDK's _tmainCRTStartup return address.");
}
fn install_player_position_hook() {
    if !matches_supported_executable() {
        append_log("Player-position probe refused: executable timestamp or size did not match Steam 2015-10-22.");
        return;
    }

    let module = unsafe { GetModuleHandleW(PCWSTR::null()) };
    let Ok(module) = module else {
        append_log("Player-position probe refused: could not read the game image base.");
        return;
    };
    let image_base = module.0 as usize;
    if image_base != STEAM_IMAGE_BASE {
        append_log(&format!(
            "Player-position probe refused: unexpected image base 0x{image_base:X}."
        ));
        return;
    }

    let Some(update_post_sim) = image_base.checked_add(UPDATE_POST_SIM_RVA) else {
        append_log("Player-position probe refused: UpdatePostSim address overflowed.");
        return;
    };
    let Some(get_player_character) = image_base.checked_add(GET_PLAYER_CHARACTER_RVA) else {
        append_log("Player-position probe refused: GetPlayerCharacter address overflowed.");
        return;
    };
    let Some(character_manager_singleton) =
        image_base.checked_add(CHARACTER_MANAGER_SINGLETON_RVA)
    else {
        append_log("Player-position probe refused: character-manager address overflowed.");
        return;
    };

    append_log("Waiting for the Steam UpdatePostSim callback to become readable executable code.");
    let started_at = std::time::Instant::now();
    while started_at.elapsed() < HOOK_TIMEOUT {
        if is_readable_executable_page(update_post_sim) {
            install_hook(update_post_sim, get_player_character, character_manager_singleton);
            return;
        }
        thread::sleep(Duration::from_millis(250));
    }
    append_log("Player-position probe refused: UpdatePostSim did not become readable executable code within 120 seconds.");
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

fn is_readable_page(address: usize) -> bool {
    let Some(information) = query_page(address) else {
        return false;
    };
    if information.state != MEM_COMMIT || information.protect & PAGE_GUARD != 0 {
        return false;
    }

    matches!(
        information.protect & 0xFF,
        PAGE_READONLY
            | PAGE_READWRITE
            | PAGE_WRITECOPY
            | PAGE_EXECUTE_READ
            | PAGE_EXECUTE_READWRITE
            | PAGE_EXECUTE_WRITECOPY
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

fn read_current_process(address: usize, output: &mut [u8]) -> bool {
    if output.is_empty() || !is_readable_page(address) {
        return false;
    }
    let Some(last_address) = address.checked_add(output.len() - 1) else {
        return false;
    };
    let Some(first_page) = query_page(address) else {
        return false;
    };
    let first_page_end = (first_page.base_address as usize).saturating_add(first_page.region_size);
    if last_address >= first_page_end || !is_readable_page(last_address) {
        return false;
    }

    let process = unsafe { GetCurrentProcess() };
    let mut bytes_read = 0;
    let result = unsafe {
        ReadProcessMemory(
            process,
            address as *const c_void,
            output.as_mut_ptr().cast(),
            output.len(),
            Some(&mut bytes_read),
        )
    };
    result.is_ok() && bytes_read == output.len()
}

fn read_pointer(address: usize) -> Option<usize> {
    let mut bytes = [0u8; size_of::<usize>()];
    if !read_current_process(address, &mut bytes) {
        return None;
    }
    Some(usize::from_ne_bytes(bytes))
}

fn install_hook(target: usize, get_player_character: usize, character_manager_singleton: usize) {
    let initialized = unsafe { mh::MH_Initialize() };
    if initialized != MH_STATUS::MH_OK && initialized != MH_STATUS::MH_ERROR_ALREADY_INITIALIZED {
        append_log(&format!(
            "Player-position probe refused: MinHook initialization failed ({initialized:?})."
        ));
        return;
    }

    GET_PLAYER_CHARACTER_ADDRESS.store(get_player_character, Ordering::Release);
    CHARACTER_MANAGER_SINGLETON_ADDRESS.store(character_manager_singleton, Ordering::Release);
    let address = target as *mut c_void;
    let detour = update_post_sim_hook as *const () as *mut c_void;
    let mut original = ptr::null_mut();
    let created = unsafe { mh::MH_CreateHook(address, detour, &mut original) };
    if created != MH_STATUS::MH_OK {
        append_log(&format!(
            "Player-position probe refused: MinHook could not create UpdatePostSim hook ({created:?})."
        ));
        return;
    }
    ORIGINAL_UPDATE_POST_SIM.store(original, Ordering::Release);

    let enabled = unsafe { mh::MH_EnableHook(address) };
    if enabled != MH_STATUS::MH_OK {
        ORIGINAL_UPDATE_POST_SIM.store(ptr::null_mut(), Ordering::Release);
        append_log(&format!(
            "Player-position probe refused: MinHook could not enable UpdatePostSim hook ({enabled:?})."
        ));
        return;
    }

    append_log(&format!(
        "Player-position probe installed on UpdatePostSim at 0x{target:X}; waiting for a valid player transform."
    ));
}

static GET_PLAYER_CHARACTER_ADDRESS: AtomicUsize = AtomicUsize::new(0);
static CHARACTER_MANAGER_SINGLETON_ADDRESS: AtomicUsize = AtomicUsize::new(0);

unsafe extern "system" fn update_post_sim_hook(context: *mut c_void) {
    let original = ORIGINAL_UPDATE_POST_SIM.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    let original: UpdatePostSim = unsafe { std::mem::transmute(original) };
    unsafe { original(context) };
    sample_player_position();
}

fn sample_player_position() {
    let now = unix_millis();
    let last_sample = LAST_SAMPLE_AT.load(Ordering::Relaxed);
    if now.saturating_sub(last_sample) < SAMPLE_INTERVAL_MS
        || POSITION_SAMPLE_IN_PROGRESS
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
    {
        return;
    }
    LAST_SAMPLE_AT.store(now, Ordering::Relaxed);
    let _sample_guard = SampleGuard;

    let manager_address = CHARACTER_MANAGER_SINGLETON_ADDRESS.load(Ordering::Acquire);
    let Some(manager) = read_pointer(manager_address) else {
        log_probe_failure("character-manager singleton could not be read");
        return;
    };
    if manager == 0 || !is_readable_page(manager) {
        log_probe_failure("character manager is null or unreadable");
        return;
    }

    let getter_address = GET_PLAYER_CHARACTER_ADDRESS.load(Ordering::Acquire);
    if !is_readable_executable_page(getter_address) {
        log_probe_failure("GetPlayerCharacter is not readable executable code");
        return;
    }
    let getter: GetPlayerCharacter = unsafe { std::mem::transmute(getter_address) };
    let player = unsafe { getter(manager as *mut c_void) };
    if player.is_null() {
        log_probe_failure("GetPlayerCharacter returned null");
        return;
    }

    let Some(game_object) = (player as usize).checked_add(GAME_OBJECT_SUBOBJECT_OFFSET) else {
        log_probe_failure("player object address overflowed");
        return;
    };
    let Some(vtable) = read_pointer(game_object) else {
        log_probe_failure("player game-object vtable could not be read");
        return;
    };
    let Some(method_offset) = GET_TRANSFORM_VTABLE_SLOT.checked_mul(size_of::<usize>()) else {
        log_probe_failure("GetTransform vtable offset overflowed");
        return;
    };
    let Some(method_slot) = vtable.checked_add(method_offset) else {
        log_probe_failure("GetTransform vtable address overflowed");
        return;
    };
    let Some(method) = read_pointer(method_slot) else {
        log_probe_failure("GetTransform vtable entry could not be read");
        return;
    };
    if method == 0 || !is_readable_executable_page(method) {
        log_probe_failure("GetTransform vtable entry is not readable executable code");
        return;
    }

    let get_transform: GetTransform = unsafe { std::mem::transmute(method) };
    let mut transform = Matrix4x4::default();
    if !unsafe { get_transform(game_object as *mut c_void, &mut transform) } {
        log_probe_failure("GetTransform returned false");
        return;
    }

    let position = (transform.values[3][0], transform.values[3][1], transform.values[3][2]);
    if !position_is_reasonable(position) {
        log_probe_failure("GetTransform returned non-finite or implausible coordinates");
        return;
    }

    POSITION_X.store(position.0.to_bits(), Ordering::Relaxed);
    POSITION_Y.store(position.1.to_bits(), Ordering::Relaxed);
    POSITION_Z.store(position.2.to_bits(), Ordering::Relaxed);
    POSITION_UPDATED_AT.store(now, Ordering::Release);
    let last_logged = LAST_POSITION_LOG_AT.load(Ordering::Relaxed);
    if now.saturating_sub(last_logged) >= 2_000
        && LAST_POSITION_LOG_AT
            .compare_exchange(last_logged, now, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
    {
        append_log(&format!(
            "Player position from GetTransform: x={:.3}, y={:.3}, z={:.3}.",
            position.0, position.1, position.2
        ));
    }
}

struct SampleGuard;

impl Drop for SampleGuard {
    fn drop(&mut self) {
        POSITION_SAMPLE_IN_PROGRESS.store(false, Ordering::Release);
    }
}

fn position_is_reasonable(position: (f32, f32, f32)) -> bool {
    [position.0, position.1, position.2]
        .into_iter()
        .all(|value| value.is_finite() && value.abs() <= MAX_REASONABLE_COORDINATE)
}

fn log_probe_failure(reason: &str) {
    let now = unix_millis();
    let last_logged = LAST_FAILURE_LOG_AT.load(Ordering::Relaxed);
    if now.saturating_sub(last_logged) >= 2_000
        && LAST_FAILURE_LOG_AT
            .compare_exchange(last_logged, now, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
    {
        append_log(&format!("Player-position sample skipped: {reason}."));
    }
}

fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}