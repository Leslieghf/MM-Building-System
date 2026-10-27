use crate::overlay::append_log;
use hudhook::mh::{self, MH_STATUS};
use hudhook::windows::Win32::System::LibraryLoader::GetModuleHandleW;
use std::env;
use std::ffi::c_void;
use std::fs;
use std::mem::{MaybeUninit, size_of};
use std::ptr;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

const STEAM_TIMESTAMP: u32 = 0x5628_A448;
const STEAM_FILE_SIZE: u64 = 76_814_792;
const STEAM_IMAGE_BASE: usize = 0x14000_0000;
const UPDATE_POST_SIM_RVA: usize = 0x01FA_5A10;
const CHARACTER_MANAGER_SINGLETON: usize = 0x1417_F5338;
const GET_PLAYER_CHARACTER: usize = 0x141C_91250;
// Steam address added to the community SDK after the revision used for the
// initial player-transform research; CCharacter::GetVehiclePtr.
const GET_CHARACTER_VEHICLE: usize = 0x141C_82910;
// Steam addresses in the community SDK's CCameraControlManager declarations.
const CAMERA_MANAGER_SINGLETON: usize = 0x1417_F5378;
const GET_CAMERA_MATRIX: usize = 0x1422_2DA60;
// SDK-derived layout candidates from the pinned mm_sdk declarations; neither
// value is runtime-proven. CPhysicsGameObject lists IPfxGameObject before
// CGameObject, and GetTransform is the final CGameObject virtual.
const CHARACTER_TO_GAME_OBJECT_OFFSET: usize = 0x8;
// CVehicle follows the same multiple-inheritance path through CPhysicsGameObject.
const VEHICLE_TO_GAME_OBJECT_OFFSET: usize = 0x8;
const GET_TRANSFORM_VTABLE_SLOT: usize = 22;
const HOOK_TIMEOUT: Duration = Duration::from_secs(120);
const PLAYER_SAMPLE_MAX_AGE: Duration = Duration::from_secs(1);

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
struct PlayerSampleState {
    status: u8,
    position: Option<[f32; 3]>,
    rotation: Option<[f32; 3]>,
    view_rotation: Option<[f32; 3]>,
    velocity: Option<[f32; 3]>,
    acceleration: Option<[f32; 3]>,
    sampled_at: Option<Instant>,
}

struct VehicleSampleState {
    position: Option<[f32; 3]>,
    rotation: Option<[f32; 3]>,
    velocity: Option<[f32; 3]>,
    acceleration: Option<[f32; 3]>,
    sampled_at: Option<Instant>,
}

static PLAYER_SAMPLE: Mutex<PlayerSampleState> = Mutex::new(PlayerSampleState {
    status: 0,
    position: None,
    rotation: None,
    view_rotation: None,
    velocity: None,
    acceleration: None,
    sampled_at: None,
});

static VEHICLE_SAMPLE: Mutex<VehicleSampleState> = Mutex::new(VehicleSampleState {
    position: None,
    rotation: None,
    velocity: None,
    acceleration: None,
    sampled_at: None,
});

type GetPlayerCharacter = unsafe extern "system" fn(*mut c_void) -> *mut c_void;
type GetCharacterVehicle = unsafe extern "system" fn(*mut c_void) -> *mut c_void;
type GetTransform = unsafe extern "system" fn(*mut c_void, *mut Matrix4x4) -> bool;
type GetCameraMatrix = unsafe extern "system" fn(*mut c_void, *mut Matrix4x4);

#[repr(C)]
struct Matrix4x4 {
    values: [[f32; 4]; 4],
}

pub(crate) fn callback_count() -> Option<u64> {
    UPDATE_POST_SIM_INSTALLED
        .load(Ordering::Acquire)
        .then(|| UPDATE_POST_SIM_CALLS.load(Ordering::Acquire))
}

pub(crate) fn player_sample() -> Option<(
    &'static str,
    Option<[f32; 3]>,
    Option<[f32; 3]>,
    Option<[f32; 3]>,
    Option<[f32; 3]>,
    Option<[f32; 3]>,
)> {
    if !UPDATE_POST_SIM_INSTALLED.load(Ordering::Acquire) {
        return None;
    }

    let sample = PLAYER_SAMPLE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if sample.status == 10 {
        let fresh = sample
            .sampled_at
            .is_some_and(|sampled_at| sampled_at.elapsed() <= PLAYER_SAMPLE_MAX_AGE);
        if !fresh {
            return Some(("position stale", None, None, None, None, None));
        }
    }

    Some((
        player_status_text(sample.status),
        sample.position,
        sample.rotation,
        sample.view_rotation,
        sample.velocity,
        sample.acceleration,
    ))
}

pub(crate) fn vehicle_sample() -> Option<(
    Option<[f32; 3]>,
    Option<[f32; 3]>,
    Option<[f32; 3]>,
    Option<[f32; 3]>,
)> {
    if !UPDATE_POST_SIM_INSTALLED.load(Ordering::Acquire) {
        return None;
    }

    let sample = VEHICLE_SAMPLE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let fresh = sample
        .sampled_at
        .is_some_and(|sampled_at| sampled_at.elapsed() <= PLAYER_SAMPLE_MAX_AGE);
    if !fresh {
        return Some((None, None, None, None));
    }

    Some((
        sample.position,
        sample.rotation,
        sample.velocity,
        sample.acceleration,
    ))
}

fn publish_vehicle_sample(transform: Option<Matrix4x4>) {
    let now = Instant::now();
    let (position, rotation) = transform.map_or((None, None), |transform| {
        let position = [
            transform.values[3][0],
            transform.values[3][1],
            transform.values[3][2],
        ];
        if position.iter().all(|coordinate| coordinate.is_finite()) {
            (Some(position), rotation_xyz_degrees(&transform))
        } else {
            (None, None)
        }
    });

    let mut sample = VEHICLE_SAMPLE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let (velocity, acceleration) =
        if let (Some(position), Some(previous_position), Some(previous_at)) =
            (position, sample.position, sample.sampled_at)
        {
            let elapsed = now.duration_since(previous_at).as_secs_f32();
            if elapsed > 0.0 {
                let velocity = vector_difference(position, previous_position, elapsed);
                let acceleration =
                    sample
                        .velocity
                        .zip(velocity)
                        .and_then(|(previous_velocity, velocity)| {
                            vector_difference(velocity, previous_velocity, elapsed)
                        });
                (velocity, acceleration)
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

    sample.position = position;
    sample.rotation = rotation;
    sample.velocity = velocity;
    sample.acceleration = acceleration;
    sample.sampled_at = position.map(|_| now);
}

fn player_status_text(status: u8) -> &'static str {
    match status {
        0 => "waiting",
        1 => "manager unavailable",
        2 => "getter unavailable",
        3 => "no player character",
        4 => "player object unreadable",
        5 => "candidate subobject unreadable",
        6 => "candidate vtable unreadable",
        7 => "candidate slot is not executable",
        8 => "GetTransform returned false",
        9 => "position contains non-finite values",
        10 => "position available",
        _ => "unknown",
    }
}

fn publish_player_sample(
    status: u8,
    position: Option<[f32; 3]>,
    rotation: Option<[f32; 3]>,
    view_rotation: Option<[f32; 3]>,
) {
    let now = Instant::now();
    let mut sample = PLAYER_SAMPLE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let (velocity, acceleration) =
        if let (Some(position), Some(previous_position), Some(previous_at)) =
            (position, sample.position, sample.sampled_at)
        {
            let elapsed = now.duration_since(previous_at).as_secs_f32();
            if elapsed > 0.0 {
                let velocity = vector_difference(position, previous_position, elapsed);
                let acceleration =
                    sample
                        .velocity
                        .zip(velocity)
                        .and_then(|(previous_velocity, velocity)| {
                            vector_difference(velocity, previous_velocity, elapsed)
                        });
                (velocity, acceleration)
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

    sample.status = status;
    sample.position = position;
    sample.rotation = rotation;
    sample.view_rotation = view_rotation;
    sample.velocity = velocity;
    sample.acceleration = acceleration;
    sample.sampled_at = position.map(|_| now);
}

fn vector_difference(current: [f32; 3], previous: [f32; 3], elapsed: f32) -> Option<[f32; 3]> {
    let difference = [
        (current[0] - previous[0]) / elapsed,
        (current[1] - previous[1]) / elapsed,
        (current[2] - previous[2]) / elapsed,
    ];
    difference
        .iter()
        .all(|component| component.is_finite())
        .then_some(difference)
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
        append_log(
            "UpdatePostSim probe refused: executable timestamp or size did not match Steam 2015-10-22.",
        );
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
    append_log(
        "UpdatePostSim probe refused: callback did not become readable executable code within 120 seconds.",
    );
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

fn readable_span(address: usize, length: usize) -> bool {
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
    UPDATE_POST_SIM_CALLS.fetch_add(1, Ordering::AcqRel);
    probe_game_state();
}

fn probe_game_state() {
    if !readable_span(CHARACTER_MANAGER_SINGLETON, size_of::<*mut c_void>()) {
        publish_player_sample(1, None, None, None);
        publish_vehicle_sample(None);
        return;
    }
    if !is_readable_executable_page(GET_PLAYER_CHARACTER) {
        publish_player_sample(2, None, None, None);
        publish_vehicle_sample(None);
        return;
    }

    let manager = unsafe { ptr::read_volatile(CHARACTER_MANAGER_SINGLETON as *const *mut c_void) };
    if manager.is_null() || !readable_span(manager as usize, 1) {
        publish_player_sample(1, None, None, None);
        publish_vehicle_sample(None);
        return;
    }

    let get_player_character: GetPlayerCharacter =
        unsafe { std::mem::transmute(GET_PLAYER_CHARACTER) };
    let player = unsafe { get_player_character(manager) };
    if player.is_null() {
        publish_player_sample(3, None, None, None);
        publish_vehicle_sample(None);
        return;
    }
    if !readable_span(player as usize, 1) {
        publish_player_sample(4, None, None, None);
        publish_vehicle_sample(None);
        return;
    }

    probe_player_transform(player);
    publish_vehicle_sample(sample_vehicle_transform(player));
}

fn probe_player_transform(player: *mut c_void) {
    let Some(game_object_address) = (player as usize).checked_add(CHARACTER_TO_GAME_OBJECT_OFFSET)
    else {
        publish_player_sample(5, None, None, None);
        return;
    };
    if game_object_address % std::mem::align_of::<*const usize>() != 0 {
        publish_player_sample(5, None, None, None);
        return;
    }
    if !readable_span(game_object_address, size_of::<*const usize>()) {
        publish_player_sample(5, None, None, None);
        return;
    }

    let vtable = unsafe { ptr::read_volatile(game_object_address as *const *const usize) };
    if vtable.is_null() {
        publish_player_sample(6, None, None, None);
        return;
    }
    if (vtable as usize) % std::mem::align_of::<usize>() != 0 {
        publish_player_sample(6, None, None, None);
        return;
    }
    let Some(slot_count) = GET_TRANSFORM_VTABLE_SLOT.checked_add(1) else {
        publish_player_sample(6, None, None, None);
        return;
    };
    let Some(vtable_bytes) = slot_count.checked_mul(size_of::<usize>()) else {
        publish_player_sample(6, None, None, None);
        return;
    };
    if !readable_span(vtable as usize, vtable_bytes) {
        publish_player_sample(6, None, None, None);
        return;
    }

    let slot_address = vtable.wrapping_add(GET_TRANSFORM_VTABLE_SLOT);
    let target = unsafe { ptr::read_volatile(slot_address) };
    if target == 0 || !is_readable_executable_page(target) {
        publish_player_sample(7, None, None, None);
        return;
    }

    let get_transform: GetTransform = unsafe { std::mem::transmute(target) };
    let mut transform = Matrix4x4 {
        values: [[0.0; 4]; 4],
    };
    if !unsafe { get_transform(game_object_address as *mut c_void, &mut transform) } {
        publish_player_sample(8, None, None, None);
        return;
    }

    let position = [
        transform.values[3][0],
        transform.values[3][1],
        transform.values[3][2],
    ];
    if !position.iter().all(|coordinate| coordinate.is_finite()) {
        publish_player_sample(9, None, None, None);
        return;
    }

    let rotation = rotation_xyz_degrees(&transform);
    let view_rotation = sample_view_rotation();
    publish_player_sample(10, Some(position), rotation, view_rotation);
}

fn sample_vehicle_transform(player: *mut c_void) -> Option<Matrix4x4> {
    if !is_readable_executable_page(GET_CHARACTER_VEHICLE) {
        return None;
    }

    let get_vehicle: GetCharacterVehicle = unsafe { std::mem::transmute(GET_CHARACTER_VEHICLE) };
    let vehicle = unsafe { get_vehicle(player) };
    if vehicle.is_null() || !readable_span(vehicle as usize, 1) {
        return None;
    }

    let Some(game_object_address) = (vehicle as usize).checked_add(VEHICLE_TO_GAME_OBJECT_OFFSET)
    else {
        return None;
    };
    if game_object_address % std::mem::align_of::<*const usize>() != 0
        || !readable_span(game_object_address, size_of::<*const usize>())
    {
        return None;
    }

    let vtable = unsafe { ptr::read_volatile(game_object_address as *const *const usize) };
    if vtable.is_null() || (vtable as usize) % std::mem::align_of::<usize>() != 0 {
        return None;
    }
    let slot_count = GET_TRANSFORM_VTABLE_SLOT.checked_add(1)?;
    let vtable_bytes = slot_count.checked_mul(size_of::<usize>())?;
    if !readable_span(vtable as usize, vtable_bytes) {
        return None;
    }

    let target = unsafe { ptr::read_volatile(vtable.wrapping_add(GET_TRANSFORM_VTABLE_SLOT)) };
    if target == 0 || !is_readable_executable_page(target) {
        return None;
    }

    let get_transform: GetTransform = unsafe { std::mem::transmute(target) };
    let mut transform = Matrix4x4 {
        values: [[0.0; 4]; 4],
    };
    if !unsafe { get_transform(game_object_address as *mut c_void, &mut transform) } {
        return None;
    }

    Some(transform)
}

fn rotation_xyz_degrees(matrix: &Matrix4x4) -> Option<[f32; 3]> {
    let values = &matrix.values;
    if values[..3]
        .iter()
        .any(|row| !row[..3].iter().all(|value| value.is_finite()))
    {
        return None;
    }

    let pitch = (-values[0][2]).clamp(-1.0, 1.0).asin();
    let cos_pitch = pitch.cos();
    let (roll, yaw) = if cos_pitch.abs() > 1.0e-6 {
        (
            values[1][2].atan2(values[2][2]),
            values[0][1].atan2(values[0][0]),
        )
    } else {
        ((-values[2][1]).atan2(values[1][1]), 0.0)
    };
    Some([roll.to_degrees(), pitch.to_degrees(), yaw.to_degrees()])
}

fn sample_view_rotation() -> Option<[f32; 3]> {
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
    unsafe { get_camera_matrix(manager, &mut camera_matrix) };
    rotation_xyz_degrees(&camera_matrix)
}
