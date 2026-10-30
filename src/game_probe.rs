use crate::overlay::append_log;
use crate::runtime::{
    GameEndpoint, GameRuntime, GameSnapshot, IntentResult, PendingGameIntent, PendingSpawnIntent,
    PendingVelocityIntent, RaycastCandidateShape, SpawnOutcome, SpawnRequestResult,
};
use crate::scripting::api::DebugPrimitive;
use crate::sdk::build::steam::{
    CAMERA_MANAGER_SINGLETON, CHARACTER_MANAGER_SINGLETON, DEVICE_MANAGER_SINGLETON,
    GET_CAMERA_MATRIX, GET_CHARACTER_VEHICLE, GET_PLAYER_CHARACTER, SPAWN_INFO_CTOR,
    SPAWN_INFO_DTOR, SPAWN_SET_ENTITY_FILE, SPAWN_SUBMIT,
};
use crate::sdk::core::input::{
    INPUT_DEVICE_MANAGER_IS_IN_FOCUS_OFFSET, STEAM_DEVICE_MANAGER_INPUT_MANAGER_OFFSET,
};
use crate::sdk::game::camera::{CameraControlManager, GetCameraMatrixFn as GetCameraMatrix};
use crate::sdk::game::character::{
    Character, CharacterManager, GetCharacterVehicleFn as GetCharacterVehicle,
    GetPlayerCharacterFn as GetPlayerCharacter,
};
use crate::sdk::game::object::{
    GET_TRANSFORM_SLOT_CANDIDATE as GET_TRANSFORM_VTABLE_SLOT, GameObject,
    GetTransformFn as GetTransform,
};
use crate::sdk::game::spawn::{
    SetEntityFileFn, SpawnFn, SpawnInfoConstructorFn, SpawnInfoDestructorFn, SpawnInstanceInfo,
};
use crate::sdk::game::vehicle::{
    GET_ACCELERATION_SLOT_CANDIDATE as VEHICLE_GET_ACCELERATION_VTABLE_SLOT,
    GET_ANGULAR_VELOCITY_SLOT_CANDIDATE as VEHICLE_GET_ANGULAR_VELOCITY_VTABLE_SLOT,
    GET_VELOCITY_LAST_FRAME_SLOT_CANDIDATE as VEHICLE_GET_VELOCITY_LAST_FRAME_VTABLE_SLOT,
    GET_VELOCITY_SLOT_CANDIDATE as VEHICLE_GET_VELOCITY_VTABLE_SLOT,
    GetVectorFn as GetVehicleVector, GetVectorPointerFn as GetVehicleVectorPointer,
    SET_VELOCITY_SLOT_CANDIDATE as VEHICLE_SET_VELOCITY_VTABLE_SLOT,
    SetVectorFn as SetVehicleVector, Vehicle,
};
use crate::sdk::game::world_query::{
    C_IGNORE_CHARACTERS_AND_VEHICLES_RAYCAST_FILTER_VFTABLE_RVA,
    C_IGNORE_FILTER_OWNER_VECTOR_BEGIN_OFFSET, C_IGNORE_FILTER_OWNER_VECTOR_CAPACITY_OFFSET,
    C_IGNORE_FILTER_OWNER_VECTOR_END_OFFSET, C_IGNORE_FILTER_VFTABLE_RVA,
    C_PLAYER_AIM_RAYCAST_FILTER_VFTABLE_RVA, C_RAYCAST_FILTER_ADD_RAY_HIT_RVA,
    C_STATIC_ONLY_FILTER_CATEGORY_MASK_OFFSET, C_STATIC_ONLY_RAYCAST_FILTER_VFTABLE_RVA,
    RAYCAST_FILTER_SET_OPTION_RVA, RaycastFilterSetOptionFn, WORLD_QUERY_BASE_POINTER_RVA,
    WORLD_QUERY_CONTEXT_ACCEPTED_SHAPE_POINTER_OFFSET, WORLD_QUERY_CONTEXT_INITIALIZER_RVA,
    WorldQueryContextInitializeFn, WorldQueryHelperFn, WorldQueryOutputRecord, WorldQueryRayInput,
};
use crate::sdk::types::{Matrix4x4, Vector3};
use hudhook::mh::{self, MH_STATUS};
use hudhook::windows::Win32::System::LibraryLoader::GetModuleHandleW;
use std::cell::RefCell;
use std::env;
use std::ffi::{CString, c_void};
use std::fs;
use std::mem::{MaybeUninit, size_of};
use std::panic::{self, AssertUnwindSafe};
use std::ptr;
use std::sync::atomic::{
    AtomicBool, AtomicPtr, AtomicU8, AtomicU32, AtomicU64, AtomicUsize, Ordering,
};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

const STEAM_TIMESTAMP: u32 = 0x5628_A448;
const STEAM_FILE_SIZE: u64 = 76_814_792;
const STEAM_IMAGE_BASE: usize = 0x14000_0000;
const UPDATE_POST_SIM_RVA: usize = 0x01FA_5A10;
// CDebugRenderer facade, its line and auto-width arrow slots, verified in the
// Ghidra project from the captured Steam executable overlays.
const CDEBUG_RENDERER_INSTANCE_GLOBAL_RVA: usize = 0x017C_F0E8;
const CDEBUG_RENDERER_FACADE_VTABLE_RVA: usize = 0x0134_6938;
const CDEBUG_RENDERER_LINE_SLOT: usize = 1;
const CDEBUG_RENDERER_ARROW_SLOT: usize = 9;
const CDEBUG_RENDERER_AXES_SLOT: usize = 25;
const CDEBUG_RENDERER_SPHERE_SLOT: usize = 26;
const CDEBUG_RENDERER_SPARSE_SPHERE_SLOT: usize = 27;
const CDEBUG_RENDERER_TESSELLATED_SPHERE_SLOT: usize = 28;
const CDEBUG_RENDERER_AABB_SLOT: usize = 31;
const CDEBUG_RENDERER_TRANSFORMED_AABB_SLOT: usize = 34;
const CDEBUG_RENDERER_QUEUED_TRANSFORMED_AABB_SLOT: usize = 35;
const CDEBUG_RENDERER_BOUNDS_SPHERE_SLOT: usize = 36;
const CDEBUG_RENDERER_BOUNDS_CYLINDER_SLOT: usize = 37;
const CDEBUG_RENDERER_BOUNDS_CAPSULE_SLOT: usize = 38;
const CDEBUG_RENDERER_BOUNDS_CONVEX_VERTICES_SLOT: usize = 39;
const CDEBUG_RENDERER_LINE_METHOD_RVA: usize = 0x0297_A1B0;
const CDEBUG_RENDERER_ARROW_METHOD_RVA: usize = 0x0297_A420;
const CDEBUG_RENDERER_AXES_METHOD_RVA: usize = 0x0297_A940;
const CDEBUG_RENDERER_SPHERE_METHOD_RVA: usize = 0x0297_A960;
const CDEBUG_RENDERER_SPARSE_SPHERE_METHOD_RVA: usize = 0x0297_A990;
const CDEBUG_RENDERER_TESSELLATED_SPHERE_METHOD_RVA: usize = 0x0297_A9D0;
const CDEBUG_RENDERER_AABB_METHOD_RVA: usize = 0x0297_AA80;
const CDEBUG_RENDERER_TRANSFORMED_AABB_METHOD_RVA: usize = 0x0297_AB30;
const CDEBUG_RENDERER_QUEUED_TRANSFORMED_AABB_METHOD_RVA: usize = 0x0297_AB70;
const CDEBUG_RENDERER_BOUNDS_SPHERE_METHOD_RVA: usize = 0x0297_ABA0;
const CDEBUG_RENDERER_BOUNDS_CYLINDER_METHOD_RVA: usize = 0x0297_ABE0;
const CDEBUG_RENDERER_BOUNDS_CAPSULE_METHOD_RVA: usize = 0x0297_AC20;
const CDEBUG_RENDERER_BOUNDS_CONVEX_VERTICES_METHOD_RVA: usize = 0x0297_AC60;
const PRIMITIVE_RENDERER_LINE_RVA: usize = 0x028B_BA40;
const PRIMITIVE_RENDERER_ARROW_RVA: usize = 0x028B_DF60;
const PRIMITIVE_RENDERER_AXES_RVA: usize = 0x028B_79C0;
static SCRIPT_DEBUG_DRAW_SUBMISSION_LOGGED: AtomicBool = AtomicBool::new(false);
// Binary-derived callback registered as `RaycastStaticFromCamera` in the
// Steam 1.0.3.0 executable. The tap below only counts and forwards calls.
const RAYCAST_STATIC_FROM_CAMERA_RVA: usize = 0x01EA_C4C0;
// Shared lower-level world-query helper reached by the script trace wrapper
// and many gameplay call sites in the supported Steam executable.
const WORLD_QUERY_HELPER_RVA: usize = 0x0252_EDF0;
// Point-in-world-bounds predicate called twice by the shared query helper.
const WORLD_QUERY_BOUNDS_PREDICATE_RVA: usize = 0x0252_67E0;
const WORLD_QUERY_BOUNDS_CHECKS_PER_QUERY: usize = 2;
// hkProxyRayCaster::addBroadPhaseHandle in the supported Steam executable.
const WORLD_QUERY_CANDIDATE_CALLBACK_RVA: usize = 0x0252_6AA0;
const WORLD_QUERY_CANDIDATE_CAPTURE_CAPACITY: usize = 7;
// RTTI-resolved CCustomSampledHeightFieldShape vtable targets. Their bodies
// are absent from the on-disk .xpdata import, so capture runtime bytes once.
const SAMPLED_HEIGHT_FIELD_RAY_CAST_RVA: usize = 0x024F_6880;
const SAMPLED_HEIGHT_FIELD_PROXY_QUERY_RVA: usize = 0x02B1_B0A0;
const SHAPE_VIRTUAL_CODE_CAPTURE_LENGTH: usize = 96;
// hkpCustomGroupFilter ray-query virtual observed through the world filter's
// subobject vtable at +0x08.
const WORLD_QUERY_GROUP_FILTER_RVA: usize = 0x024E_7580;
const WORLD_QUERY_RETURN_STACK_CAPACITY: usize = 256;
const WORLD_QUERY_NO_SAMPLE: usize = usize::MAX;
const WORLD_QUERY_CANDIDATE_STACK_CAPACITY: usize = 16;
const RAYCAST_FILTER_HIT_CAPTURE_CAPACITY: usize = 4;
const RAYCAST_FILTER_CAPTURE_BROADPHASE_SLOTS: usize = 2;
// RTTI-confirmed CPlayerAimRaycastFilter vtable slot +0x10 candidate test.
// The observer only counts calls and always forwards to the original method.
const PLAYER_AIM_FILTER_CANDIDATE_TEST_RVA: usize = 0x0203_9E90;
const PLAYER_AIM_FILTER_CANDIDATE_TEST_ENTRY: [u8; 16] = [
    0x57, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0x0D, 0x9C, 0xB4, 0x7B, 0xFF, 0x48, 0x89, 0xD7, 0xE8,
];
// CWeapon::GetRayWorldPosition in the observed Steam executable. The entry
// signature is checked before installing the passive observer.
const WEAPON_RAY_POSITION_RVA: usize = 0x01E1_BBF0;
// SDK-derived layout candidates from the pinned mm_sdk declarations; neither
// value is runtime-proven. CPhysicsGameObject lists IPfxGameObject before
// CGameObject, and GetTransform is the final CGameObject virtual.
const CHARACTER_TO_GAME_OBJECT_OFFSET: usize = 0x8;
// CVehicle follows the same multiple-inheritance path through CPhysicsGameObject.
const VEHICLE_TO_GAME_OBJECT_OFFSET: usize = 0x8;
const HOOK_TIMEOUT: Duration = Duration::from_secs(120);
const GAME_STATE_SAMPLE_INTERVAL: Duration = Duration::from_millis(50);
const RETICLE_RAY_SAMPLE_INTERVAL: Duration = Duration::from_millis(250);
const RETICLE_RAY_MAX_DISTANCE: f32 = 500.0;
const PLAYER_SAMPLE_MAX_AGE: Duration = Duration::from_secs(1);
const CALLBACK_STABLE_STREAK_REQUIRED: u32 = 60;
// The observed helper has dozens of call sites. Keep a fixed, allocation-free
// set on the hook path; a background worker writes newly seen entries to disk.
const WORLD_QUERY_CALLER_CAPACITY: usize = 256;
const WORLD_QUERY_CALLER_REPORT_INTERVAL: Duration = Duration::from_secs(5);
const WORLD_QUERY_CALLER_REPORT_LIMIT: usize = 12;
// Reserve a dedicated region for the camera-script samples: high-rate
// unrelated callers can otherwise consume the shared array before the player
// aims. The remaining region covers the first 320 ammo calls, 8
// HeightOverGround calls, three samples for each of ten important callers,
// three spaced samples for each of two occlusion callers, and one for each
// of the remaining 40 direct sites (404 expected samples), leaving 12 slots.
const WORLD_QUERY_CAMERA_SAMPLE_CAPACITY: usize = 160;
const WORLD_QUERY_NON_CAMERA_SAMPLE_BASE: usize = WORLD_QUERY_CAMERA_SAMPLE_CAPACITY;
const WORLD_QUERY_NON_CAMERA_SAMPLE_CAPACITY: usize = 416;
const RETICLE_QUERY_CALLER_CAPACITY: usize = 4;
// The reticle probe runs every 250 ms; these samples span about 18 seconds.
const RETICLE_QUERY_SAMPLE_ORDINALS: [u64; 5] = [0, 4, 16, 40, 72];
const RETICLE_QUERY_SAMPLE_BASE: usize =
    WORLD_QUERY_NON_CAMERA_SAMPLE_BASE + WORLD_QUERY_NON_CAMERA_SAMPLE_CAPACITY;
const WORLD_QUERY_SAMPLE_CAPACITY: usize =
    RETICLE_QUERY_SAMPLE_BASE + RETICLE_QUERY_CALLER_CAPACITY * RETICLE_QUERY_SAMPLE_ORDINALS.len();
const WORLD_QUERY_LABEL_CAPACITY: usize = 64;
const WEAPON_RAY_SAMPLE_CAPACITY: usize = 64;
const AMMO_WORLD_QUERY_CALLER_RVA: usize = 0x01DA_5892;
const HEIGHT_OVER_GROUND_WORLD_QUERY_CALLER_RVA: usize = 0x01CA_3731;
const SCRIPT_RAYCAST_WORLD_QUERY_CALLER_RVA: usize = 0x01EA_C351;
const WORLD_QUERY_REPEAT_CALLERS: [usize; 10] = [
    0x0205_0E4E, // CForcePulse::CheckPlayerRaycast
    0x01E1_BE5B, // CWeapon::GetRayWorldPosition
    0x0207_77E9, // CRayCasterObject::CastBulletRay
    0x020C_8899, // CCharacterModule::CanSee
    0x01C7_067A, // CRayCastCondition::Evaluate
    0x01C2_84DA, // CRayCastFanCondition::Evaluate
    0x01CA_F3F1, // CRayCastFanCondition::Evaluate (second callsite)
    0x01FA_783D, // CInteractionClearPathCondition::Evaluate
    0x01FA_78B7, // CInteractionClearPathCondition::Evaluate (second callsite)
    0x01EA_A014, // NodeIsVisibleObject
];
const WORLD_QUERY_REPEAT_SAMPLE_LIMIT: u32 = 3;
const WORLD_QUERY_SPACED_REPEAT_CALLERS: [usize; 2] = [
    0x01FC_0E0E, // wind_occlusion
    0x021D_C95A, // vehicle_sound_occlusion
];
const WORLD_QUERY_SPACED_REPEAT_CALL_ORDINALS: [u64; 3] = [0, 256, 2048];

const MEM_COMMIT: u32 = 0x1000;
const PAGE_READONLY: u32 = 0x02;
const PAGE_READWRITE: u32 = 0x04;
const PAGE_WRITECOPY: u32 = 0x08;
const PAGE_EXECUTE_READ: u32 = 0x20;
const PAGE_EXECUTE_READWRITE: u32 = 0x40;
const PAGE_EXECUTE_WRITECOPY: u32 = 0x80;
const PAGE_GUARD: u32 = 0x100;
const PAGE_WRITABLE_MASK: u32 =
    PAGE_READWRITE | PAGE_WRITECOPY | PAGE_EXECUTE_READWRITE | PAGE_EXECUTE_WRITECOPY;

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
    fn GetCurrentThreadId() -> u32;
}

type UpdatePostSim = unsafe extern "system" fn(*mut c_void);
type CDebugRendererLine = unsafe extern "system" fn(
    *mut c_void,
    *const [f32; 3],
    *const [f32; 3],
    *const [f32; 4],
    *const [f32; 4],
);
type CDebugRendererArrow = unsafe extern "system" fn(
    *mut c_void,
    *const [f32; 3],
    *const [f32; 3],
    *const [f32; 4],
    *const [f32; 4],
);
type CDebugRendererAxes = unsafe extern "system" fn(*mut c_void, *const [f32; 16], f32);
type CDebugRendererSphere =
    unsafe extern "system" fn(*mut c_void, *const [f32; 3], f32, *const [f32; 4]);
type CDebugRendererAabb =
    unsafe extern "system" fn(*mut c_void, *const [f32; 3], *const [f32; 3], *const [f32; 4]);
type CDebugRendererTransformedAabb = unsafe extern "system" fn(
    *mut c_void,
    *const [f32; 16],
    *const [f32; 3],
    *const [f32; 3],
    *const [f32; 4],
);
type CDebugRendererBoundsShape = unsafe extern "system" fn(
    *mut c_void,
    *const [f32; 16],
    *const [f32; 3],
    *const [f32; 3],
    *const [f32; 4],
);
type CameraRaycastCallback = unsafe extern "system" fn(*mut c_void, *mut c_void) -> u32;
type WeaponRayPosition = unsafe extern "system" fn(*mut c_void, *mut f32, *const f32) -> *mut f32;
type PlayerAimFilterCandidateTest = unsafe extern "system" fn(*mut c_void, *mut c_void) -> u8;

#[repr(C)]
#[derive(Clone, Copy)]
struct WorldQuerySavedRegisters {
    rcx: u64,
    rdx: u64,
    r8: u64,
    r9: u64,
    rax: u64,
    r10: u64,
    r11: u64,
    xmm: [[u64; 2]; 6],
    capture_return: u64,
    // Win64 stack slots 5 through 11, copied by the naked tap before it calls
    // Rust. At the inspected ammo and HeightOverGround callsites, slots 5–7
    // are ray length, output, and context. The helper reads slot 9 as a flag,
    // slot 10 as an index input, and optionally dereferences slot 11 as a u32
    // override when it is non-null. Keep all slots raw in the capture because
    // other callsites may use different values and meanings.
    stack_arguments: [u64; 7],
    // Capture Win64 nonvolatile registers too. In particular, RBX remains the
    // ammo receiver across the shared query and owns the post-query dispatch.
    nonvolatile_registers: [u64; 8],
    // The script ray path keeps the scalar subtracted from its segment-length
    // calculation in XMM10 at the helper boundary. Preserve XMM6-XMM15 so the
    // bounded sample can expose that input and other caller-specific values.
    nonvolatile_xmm: [[u64; 2]; 10],
}

const _: () = assert!(
    size_of::<WorldQuerySavedRegisters>() == 0x1B8
        && std::mem::offset_of!(WorldQuerySavedRegisters, capture_return) == 0x98
        && std::mem::offset_of!(WorldQuerySavedRegisters, stack_arguments) == 0xA0
        && std::mem::offset_of!(WorldQuerySavedRegisters, nonvolatile_registers) == 0xD8
        && std::mem::offset_of!(WorldQuerySavedRegisters, nonvolatile_xmm) == 0x118
);

struct WorldQueryCandidateSample {
    ready: AtomicBool,
    callback_index: AtomicU32,
    caster: AtomicUsize,
    handle: AtomicUsize,
    cast_index: AtomicU32,
    handle_relative: AtomicUsize,
    candidate_object: AtomicUsize,
    object_subobject: AtomicUsize,
    object_vtable: AtomicUsize,
    subobject_vtable: AtomicUsize,
    callback_30: AtomicUsize,
    callback_80: AtomicUsize,
    object_gate_word: AtomicU64,
    object_gate_word_valid: AtomicBool,
    object_local_transform: [AtomicU64; 6],
    object_local_transform_valid: AtomicBool,
    handle_relative_gate_word: AtomicU64,
    handle_relative_gate_word_valid: AtomicBool,
    handle_relative_transform: [AtomicU64; 6],
    handle_relative_transform_valid: AtomicBool,
    subobject_gate_word: AtomicU64,
    subobject_gate_word_valid: AtomicBool,
    subobject_transform: [AtomicU64; 6],
    subobject_transform_valid: AtomicBool,
    ray_record: AtomicUsize,
    ray_record_before: [AtomicU64; 6],
    ray_record_after: [AtomicU64; 6],
    ray_record_before_valid: AtomicBool,
    ray_record_after_valid: AtomicBool,
    ray_input_record: AtomicUsize,
    ray_input_words: [AtomicU64; 6],
    ray_input_valid: AtomicBool,
    collector_state_address: AtomicUsize,
    collector_state_before: [AtomicU64; 8],
    collector_state_after: [AtomicU64; 8],
    collector_state_before_valid: AtomicBool,
    collector_state_after_valid: AtomicBool,
    group_filter_result: AtomicU8,
    group_filter_result_valid: AtomicBool,
    group_filter_ray_info: AtomicU32,
    group_filter_candidate_info: AtomicU32,
    ray_filter_hits: [RaycastFilterHitSample; RAYCAST_FILTER_HIT_CAPTURE_CAPACITY],
    returned_fraction: AtomicU32,
}

struct RaycastFilterHitSample {
    state: AtomicU8,
    filter_context: AtomicUsize,
    filter_vtable: AtomicUsize,
    exclude_filter_info_mask: AtomicU32,
    required_filter_info_mask: AtomicU32,
    candidate_node: AtomicUsize,
    candidate_filter_info: AtomicU32,
    candidate_owner: AtomicUsize,
    filter_ignored_owner: AtomicUsize,
    selected_shape_before: AtomicUsize,
    selected_shape_after: AtomicUsize,
}

impl RaycastFilterHitSample {
    const fn new() -> Self {
        Self {
            state: AtomicU8::new(0),
            filter_context: AtomicUsize::new(0),
            filter_vtable: AtomicUsize::new(0),
            exclude_filter_info_mask: AtomicU32::new(0),
            required_filter_info_mask: AtomicU32::new(0),
            candidate_node: AtomicUsize::new(0),
            candidate_filter_info: AtomicU32::new(0),
            candidate_owner: AtomicUsize::new(0),
            filter_ignored_owner: AtomicUsize::new(0),
            selected_shape_before: AtomicUsize::new(0),
            selected_shape_after: AtomicUsize::new(0),
        }
    }
}

impl WorldQueryCandidateSample {
    const fn new() -> Self {
        Self {
            ready: AtomicBool::new(false),
            callback_index: AtomicU32::new(0),
            caster: AtomicUsize::new(0),
            handle: AtomicUsize::new(0),
            cast_index: AtomicU32::new(0),
            handle_relative: AtomicUsize::new(0),
            candidate_object: AtomicUsize::new(0),
            object_subobject: AtomicUsize::new(0),
            object_vtable: AtomicUsize::new(0),
            subobject_vtable: AtomicUsize::new(0),
            callback_30: AtomicUsize::new(0),
            callback_80: AtomicUsize::new(0),
            object_gate_word: AtomicU64::new(0),
            object_gate_word_valid: AtomicBool::new(false),
            object_local_transform: [const { AtomicU64::new(0) }; 6],
            object_local_transform_valid: AtomicBool::new(false),
            handle_relative_gate_word: AtomicU64::new(0),
            handle_relative_gate_word_valid: AtomicBool::new(false),
            handle_relative_transform: [const { AtomicU64::new(0) }; 6],
            handle_relative_transform_valid: AtomicBool::new(false),
            subobject_gate_word: AtomicU64::new(0),
            subobject_gate_word_valid: AtomicBool::new(false),
            subobject_transform: [const { AtomicU64::new(0) }; 6],
            subobject_transform_valid: AtomicBool::new(false),
            ray_record: AtomicUsize::new(0),
            ray_record_before: [const { AtomicU64::new(0) }; 6],
            ray_record_after: [const { AtomicU64::new(0) }; 6],
            ray_record_before_valid: AtomicBool::new(false),
            ray_record_after_valid: AtomicBool::new(false),
            ray_input_record: AtomicUsize::new(0),
            ray_input_words: [const { AtomicU64::new(0) }; 6],
            ray_input_valid: AtomicBool::new(false),
            collector_state_address: AtomicUsize::new(0),
            collector_state_before: [const { AtomicU64::new(0) }; 8],
            collector_state_after: [const { AtomicU64::new(0) }; 8],
            collector_state_before_valid: AtomicBool::new(false),
            collector_state_after_valid: AtomicBool::new(false),
            group_filter_result: AtomicU8::new(0),
            group_filter_result_valid: AtomicBool::new(false),
            group_filter_ray_info: AtomicU32::new(0),
            group_filter_candidate_info: AtomicU32::new(0),
            ray_filter_hits: [const { RaycastFilterHitSample::new() };
                RAYCAST_FILTER_HIT_CAPTURE_CAPACITY],
            returned_fraction: AtomicU32::new(0),
        }
    }
}

#[derive(Clone, Copy)]
struct PendingWorldQueryReturn {
    sample_index: usize,
    return_address: usize,
    query_data_address: usize,
}

struct WorldQueryReturnStack {
    frames: [PendingWorldQueryReturn; WORLD_QUERY_RETURN_STACK_CAPACITY],
    length: usize,
}

impl WorldQueryReturnStack {
    const fn new() -> Self {
        Self {
            frames: [const {
                PendingWorldQueryReturn {
                    sample_index: WORLD_QUERY_NO_SAMPLE,
                    return_address: 0,
                    query_data_address: 0,
                }
            }; WORLD_QUERY_RETURN_STACK_CAPACITY],
            length: 0,
        }
    }

    fn push(&mut self, frame: PendingWorldQueryReturn) -> (bool, usize) {
        let Some(slot) = self.frames.get_mut(self.length) else {
            let mut invalidated_samples = 0;
            for active in &mut self.frames[..self.length] {
                if active.sample_index < WORLD_QUERY_SAMPLE_CAPACITY {
                    active.sample_index = WORLD_QUERY_NO_SAMPLE;
                    invalidated_samples += 1;
                }
            }
            return (false, invalidated_samples);
        };
        *slot = frame;
        self.length += 1;
        (true, 0)
    }

    fn pop(&mut self) -> Option<PendingWorldQueryReturn> {
        self.length = self.length.checked_sub(1)?;
        self.frames.get(self.length).copied()
    }

    fn current_sample_index(&self) -> Option<usize> {
        let frame = self.frames.get(self.length.checked_sub(1)?)?;
        (frame.sample_index < WORLD_QUERY_SAMPLE_CAPACITY).then_some(frame.sample_index)
    }
}

#[derive(Clone, Copy)]
struct WorldQueryCandidateFrame {
    sample_index: usize,
    callback_slot: Option<usize>,
}

struct WorldQueryCandidateStack {
    frames: [WorldQueryCandidateFrame; WORLD_QUERY_CANDIDATE_STACK_CAPACITY],
    length: usize,
    overflow_depth: usize,
}

impl WorldQueryCandidateStack {
    const fn new() -> Self {
        Self {
            frames: [const {
                WorldQueryCandidateFrame {
                    sample_index: WORLD_QUERY_NO_SAMPLE,
                    callback_slot: None,
                }
            }; WORLD_QUERY_CANDIDATE_STACK_CAPACITY],
            length: 0,
            overflow_depth: 0,
        }
    }

    fn push(&mut self, frame: WorldQueryCandidateFrame) -> bool {
        if self.overflow_depth > 0 {
            self.overflow_depth = self.overflow_depth.saturating_add(1);
            return false;
        }
        let Some(slot) = self.frames.get_mut(self.length) else {
            self.overflow_depth = 1;
            return false;
        };
        *slot = frame;
        self.length += 1;
        true
    }

    fn pop(&mut self) {
        if self.overflow_depth > 0 {
            self.overflow_depth -= 1;
        } else {
            self.length = self.length.saturating_sub(1);
        }
    }

    fn current(&self) -> Option<WorldQueryCandidateFrame> {
        if self.overflow_depth > 0 {
            return None;
        }
        self.frames.get(self.length.checked_sub(1)?).copied()
    }
}

struct WorldQuerySample {
    ready: AtomicBool,
    result_ready: AtomicBool,
    result_claimed: AtomicBool,
    query_data_valid: AtomicBool,
    query_data_after_valid: AtomicBool,
    result: AtomicU64,
    sequence: AtomicU64,
    capture_ordinal: AtomicU32,
    caller: AtomicUsize,
    rdx_label: [AtomicU8; WORLD_QUERY_LABEL_CAPACITY],
    rdx_label_valid: AtomicBool,
    integer_registers: [AtomicU64; 7],
    nonvolatile_registers: [AtomicU64; 8],
    xmm_registers: [AtomicU64; 12],
    nonvolatile_xmm_registers: [AtomicU64; 20],
    bounds_frame_origin: [AtomicU32; 3],
    bounds_frame_origin_valid: AtomicBool,
    bounds_check_count: AtomicU32,
    bounds_checks: [WorldQueryBoundsCheck; WORLD_QUERY_BOUNDS_CHECKS_PER_QUERY],
    query_data: [AtomicU64; 12],
    query_data_after: [AtomicU64; 12],
    // GraphScriptRaycastStatic retains its original RCX/RDX inputs in RDI/RSI
    // across the shared helper. Keep 0x20 raw bytes for each capped sample;
    // leave their typed-value layout uninterpreted until runtime evidence.
    script_argument_addresses: [AtomicUsize; 2],
    script_argument_words: [[AtomicU64; 4]; 2],
    script_argument_valid: [AtomicBool; 2],
    indirect_addresses: [AtomicUsize; 2],
    indirect_data: [[AtomicU64; 12]; 2],
    indirect_valid: [AtomicBool; 2],
    stack_arguments: [AtomicU64; 7],
    argument_11_override: AtomicU32,
    argument_11_override_valid: AtomicBool,
    output_record_address: AtomicUsize,
    output_record_before: [AtomicU64; 10],
    output_record_after: [AtomicU64; 10],
    output_record_before_valid: AtomicBool,
    output_record_after_valid: AtomicBool,
    ammo_dispatch_pointer_byte_10: AtomicU8,
    ammo_dispatch_pointer_byte_10_valid: AtomicBool,
    ammo_receiver_vtable: AtomicUsize,
    ammo_receiver_vtable_valid: AtomicBool,
    ammo_dispatch_58: AtomicUsize,
    ammo_dispatch_58_valid: AtomicBool,
    ammo_dispatch_60: AtomicUsize,
    ammo_dispatch_60_valid: AtomicBool,
    query_context_address: AtomicUsize,
    query_context_before: [AtomicU64; 21],
    query_context_after: [AtomicU64; 21],
    query_context_before_valid: AtomicBool,
    query_context_after_valid: AtomicBool,
    candidate_callback_count: AtomicU32,
    candidate_min_fraction: AtomicU32,
    candidate_samples: [WorldQueryCandidateSample; WORLD_QUERY_CANDIDATE_CAPTURE_CAPACITY],
}

impl WorldQuerySample {
    const fn new() -> Self {
        Self {
            ready: AtomicBool::new(false),
            result_ready: AtomicBool::new(false),
            result_claimed: AtomicBool::new(false),
            query_data_valid: AtomicBool::new(false),
            query_data_after_valid: AtomicBool::new(false),
            result: AtomicU64::new(0),
            sequence: AtomicU64::new(0),
            capture_ordinal: AtomicU32::new(0),
            caller: AtomicUsize::new(0),
            rdx_label: [const { AtomicU8::new(0) }; WORLD_QUERY_LABEL_CAPACITY],
            rdx_label_valid: AtomicBool::new(false),
            integer_registers: [const { AtomicU64::new(0) }; 7],
            nonvolatile_registers: [const { AtomicU64::new(0) }; 8],
            xmm_registers: [const { AtomicU64::new(0) }; 12],
            nonvolatile_xmm_registers: [const { AtomicU64::new(0) }; 20],
            bounds_frame_origin: [const { AtomicU32::new(0) }; 3],
            bounds_frame_origin_valid: AtomicBool::new(false),
            bounds_check_count: AtomicU32::new(0),
            bounds_checks: [const { WorldQueryBoundsCheck::new() };
                WORLD_QUERY_BOUNDS_CHECKS_PER_QUERY],
            query_data: [const { AtomicU64::new(0) }; 12],
            query_data_after: [const { AtomicU64::new(0) }; 12],
            script_argument_addresses: [const { AtomicUsize::new(0) }; 2],
            script_argument_words: [const { [const { AtomicU64::new(0) }; 4] }; 2],
            script_argument_valid: [const { AtomicBool::new(false) }; 2],
            indirect_addresses: [const { AtomicUsize::new(0) }; 2],
            indirect_data: [const { [const { AtomicU64::new(0) }; 12] }; 2],
            indirect_valid: [const { AtomicBool::new(false) }; 2],
            stack_arguments: [const { AtomicU64::new(0) }; 7],
            argument_11_override: AtomicU32::new(0),
            argument_11_override_valid: AtomicBool::new(false),
            output_record_address: AtomicUsize::new(0),
            output_record_before: [const { AtomicU64::new(0) }; 10],
            output_record_after: [const { AtomicU64::new(0) }; 10],
            output_record_before_valid: AtomicBool::new(false),
            output_record_after_valid: AtomicBool::new(false),
            ammo_dispatch_pointer_byte_10: AtomicU8::new(0),
            ammo_dispatch_pointer_byte_10_valid: AtomicBool::new(false),
            ammo_receiver_vtable: AtomicUsize::new(0),
            ammo_receiver_vtable_valid: AtomicBool::new(false),
            ammo_dispatch_58: AtomicUsize::new(0),
            ammo_dispatch_58_valid: AtomicBool::new(false),
            ammo_dispatch_60: AtomicUsize::new(0),
            ammo_dispatch_60_valid: AtomicBool::new(false),
            query_context_address: AtomicUsize::new(0),
            query_context_before: [const { AtomicU64::new(0) }; 21],
            query_context_after: [const { AtomicU64::new(0) }; 21],
            query_context_before_valid: AtomicBool::new(false),
            query_context_after_valid: AtomicBool::new(false),
            candidate_callback_count: AtomicU32::new(0),
            candidate_min_fraction: AtomicU32::new(f32::INFINITY.to_bits()),
            candidate_samples: [const { WorldQueryCandidateSample::new() };
                WORLD_QUERY_CANDIDATE_CAPTURE_CAPACITY],
        }
    }
}

struct WorldQueryBoundsCheck {
    point: [AtomicU32; 3],
    radius: AtomicU32,
    result: AtomicU8,
    valid: AtomicBool,
}

impl WorldQueryBoundsCheck {
    const fn new() -> Self {
        Self {
            point: [const { AtomicU32::new(0) }; 3],
            radius: AtomicU32::new(0),
            result: AtomicU8::new(0),
            valid: AtomicBool::new(false),
        }
    }
}

struct WeaponRaySample {
    ready: AtomicBool,
    call: AtomicU64,
    weapon: AtomicUsize,
    ray: [AtomicU32; 6],
    result: [AtomicU32; 3],
    returned: AtomicUsize,
}

impl WeaponRaySample {
    const fn new() -> Self {
        Self {
            ready: AtomicBool::new(false),
            call: AtomicU64::new(0),
            weapon: AtomicUsize::new(0),
            ray: [const { AtomicU32::new(0) }; 6],
            result: [const { AtomicU32::new(0) }; 3],
            returned: AtomicUsize::new(0),
        }
    }
}

static ORIGINAL_UPDATE_POST_SIM: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static UPDATE_POST_SIM_INSTALLED: AtomicBool = AtomicBool::new(false);
static ORIGINAL_CAMERA_RAYCAST: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static CAMERA_RAYCAST_HOOK_INSTALLED: AtomicBool = AtomicBool::new(false);
static CAMERA_RAYCAST_CALLS: AtomicU64 = AtomicU64::new(0);
static ORIGINAL_WEAPON_RAY_POSITION: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static WEAPON_RAY_POSITION_HOOK_INSTALLED: AtomicBool = AtomicBool::new(false);
static WEAPON_RAY_POSITION_CALLS: AtomicU64 = AtomicU64::new(0);
static ORIGINAL_PLAYER_AIM_FILTER_CANDIDATE_TEST: AtomicPtr<c_void> =
    AtomicPtr::new(ptr::null_mut());
static PLAYER_AIM_FILTER_CANDIDATE_TEST_HOOK_INSTALLED: AtomicBool = AtomicBool::new(false);
static PLAYER_AIM_FILTER_CANDIDATE_TEST_CALLS: AtomicU64 = AtomicU64::new(0);
static WEAPON_RAY_SAMPLE_COUNT: AtomicUsize = AtomicUsize::new(0);
static WEAPON_RAY_SAMPLES: [WeaponRaySample; WEAPON_RAY_SAMPLE_CAPACITY] =
    [const { WeaponRaySample::new() }; WEAPON_RAY_SAMPLE_CAPACITY];
static ORIGINAL_WORLD_QUERY: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_WORLD_QUERY_BOUNDS_PREDICATE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_WORLD_QUERY_CANDIDATE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_RAYCAST_FILTER_ADD_HIT: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static RAYCAST_FILTER_ADD_HIT_HOOK_INSTALLED: AtomicBool = AtomicBool::new(false);
static WORLD_QUERY_CANDIDATE_HOOK_INSTALLED: AtomicBool = AtomicBool::new(false);
static WORLD_QUERY_HOOK_INSTALLED: AtomicBool = AtomicBool::new(false);
static WORLD_QUERY_BOUNDS_PREDICATE_HOOK_INSTALLED: AtomicBool = AtomicBool::new(false);
static WORLD_QUERY_GROUP_FILTER_HOOK_INSTALLED: AtomicBool = AtomicBool::new(false);
static WORLD_QUERY_CALLS: AtomicU64 = AtomicU64::new(0);
static WORLD_QUERY_ACTIVE_SAMPLES: AtomicUsize = AtomicUsize::new(0);
static WORLD_QUERY_LAST_RETURN_ADDRESS: AtomicUsize = AtomicUsize::new(0);
static WORLD_QUERY_CALLERS: [AtomicUsize; WORLD_QUERY_CALLER_CAPACITY] =
    [const { AtomicUsize::new(0) }; WORLD_QUERY_CALLER_CAPACITY];
static WORLD_QUERY_CALLER_CALLS: [AtomicU64; WORLD_QUERY_CALLER_CAPACITY] =
    [const { AtomicU64::new(0) }; WORLD_QUERY_CALLER_CAPACITY];
static WORLD_QUERY_CALLERS_LOGGED: [AtomicBool; WORLD_QUERY_CALLER_CAPACITY] =
    [const { AtomicBool::new(false) }; WORLD_QUERY_CALLER_CAPACITY];
static WORLD_QUERY_CALLERS_SAMPLED: [AtomicBool; WORLD_QUERY_CALLER_CAPACITY] =
    [const { AtomicBool::new(false) }; WORLD_QUERY_CALLER_CAPACITY];
static WORLD_QUERY_REPEAT_CALL_COUNTS: [AtomicU32; WORLD_QUERY_REPEAT_CALLERS.len()] =
    [const { AtomicU32::new(0) }; WORLD_QUERY_REPEAT_CALLERS.len()];
static WORLD_QUERY_SPACED_REPEAT_CALL_COUNTS: [AtomicU64; WORLD_QUERY_SPACED_REPEAT_CALLERS.len()] =
    [const { AtomicU64::new(0) }; WORLD_QUERY_SPACED_REPEAT_CALLERS.len()];
static WORLD_QUERY_CALLER_TABLE_FULL: AtomicBool = AtomicBool::new(false);
static WORLD_QUERY_SAMPLE_COUNT: AtomicUsize = AtomicUsize::new(0);
static WORLD_QUERY_AMMO_SAMPLE_CALLS: AtomicU32 = AtomicU32::new(0);
static WORLD_QUERY_FAN_SAMPLE_CALLS: AtomicU32 = AtomicU32::new(0);
static WORLD_QUERY_SCRIPT_RAYCAST_SAMPLE_CALLS: AtomicU32 = AtomicU32::new(0);
static RETICLE_QUERY_CALLERS: [AtomicUsize; RETICLE_QUERY_CALLER_CAPACITY] =
    [const { AtomicUsize::new(0) }; RETICLE_QUERY_CALLER_CAPACITY];
static RETICLE_QUERY_CALL_COUNTS: [AtomicU64; RETICLE_QUERY_CALLER_CAPACITY] =
    [const { AtomicU64::new(0) }; RETICLE_QUERY_CALLER_CAPACITY];
static WORLD_QUERY_SAMPLES: [WorldQuerySample; WORLD_QUERY_SAMPLE_CAPACITY] =
    [const { WorldQuerySample::new() }; WORLD_QUERY_SAMPLE_CAPACITY];
static WORLD_QUERY_SAMPLES_LOGGED: [AtomicBool; WORLD_QUERY_SAMPLE_CAPACITY] =
    [const { AtomicBool::new(false) }; WORLD_QUERY_SAMPLE_CAPACITY];
static UPDATE_POST_SIM_CALLS: AtomicU64 = AtomicU64::new(0);
static GAME_RUNTIME_TRACE_STAGES: AtomicU32 = AtomicU32::new(0);
static CALLBACK_OWNER_THREAD_ID: AtomicU32 = AtomicU32::new(0);
static CALLBACK_STABLE_STREAK: AtomicU32 = AtomicU32::new(0);
static CALLBACK_IN_FLIGHT: AtomicUsize = AtomicUsize::new(0);
static CALLBACK_OVERLAPS: AtomicU64 = AtomicU64::new(0);
static CALLBACK_OVERLAP_EPOCH: AtomicU64 = AtomicU64::new(0);
static CALLBACK_THREAD_MISMATCHES: AtomicU64 = AtomicU64::new(0);
static CALLBACK_GATE_OPEN: AtomicBool = AtomicBool::new(false);
static GAME_ENDPOINT: OnceLock<Mutex<Option<GameEndpoint>>> = OnceLock::new();
static CALLBACK_RUNTIME_PANICS: AtomicU64 = AtomicU64::new(0);
static GAME_SAMPLE_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static UI_INPUT_CAPTURE_REQUESTED: AtomicBool = AtomicBool::new(false);
static GAME_WINDOW_ACTIVE: AtomicBool = AtomicBool::new(true);
static SAVED_GAME_INPUT_FOCUS: Mutex<Option<bool>> = Mutex::new(None);
thread_local! {
    static GAME_RUNTIME: RefCell<Option<GameRuntime>> = const { RefCell::new(None) };
    static WORLD_QUERY_RETURN_STACK: RefCell<WorldQueryReturnStack> = const {
        RefCell::new(WorldQueryReturnStack::new())
    };
    static WORLD_QUERY_CANDIDATE_STACK: RefCell<WorldQueryCandidateStack> = const {
        RefCell::new(WorldQueryCandidateStack::new())
    };
}
static LAST_GAME_STATE_SAMPLE: Mutex<Option<Instant>> = Mutex::new(None);

pub(crate) fn set_ui_input_capture(active: bool) {
    UI_INPUT_CAPTURE_REQUESTED.store(active, Ordering::Release);
}

pub(crate) fn set_game_window_active(active: bool) {
    GAME_WINDOW_ACTIVE.store(active, Ordering::Release);
}
struct PlayerSampleState {
    status: u8,
    position: Option<[f32; 3]>,
    transform: Option<Matrix4x4>,
    forward: Option<[f32; 3]>,
    rotation: Option<[f32; 3]>,
    view_rotation: Option<[f32; 3]>,
    reticle_ray_origin: Option<[f32; 3]>,
    reticle_ray_direction: Option<[f32; 3]>,
    reticle_query_available: bool,
    reticle_query_state_present: bool,
    reticle_hit_fraction: Option<f32>,
    reticle_candidate_position: Option<[f32; 3]>,
    reticle_candidate_shape: Option<RaycastCandidateShape>,
    player_aim_query_available: bool,
    player_aim_query_state_present: bool,
    player_aim_hit_fraction: Option<f32>,
    player_aim_candidate_position: Option<[f32; 3]>,
    player_aim_candidate_shape: Option<RaycastCandidateShape>,
    ignore_characters_vehicles_query_available: bool,
    ignore_characters_vehicles_query_state_present: bool,
    ignore_characters_vehicles_hit_fraction: Option<f32>,
    ignore_characters_vehicles_candidate_position: Option<[f32; 3]>,
    ignore_characters_vehicles_candidate_shape: Option<RaycastCandidateShape>,
    ignore_player_owner_query_available: bool,
    ignore_player_owner_query_state_present: bool,
    ignore_player_owner_fraction: Option<f32>,
    ignore_player_owner_candidate_position: Option<[f32; 3]>,
    ignore_player_owner_candidate_shape: Option<RaycastCandidateShape>,
    velocity: Option<[f32; 3]>,
    acceleration: Option<[f32; 3]>,
    sampled_at: Option<Instant>,
}

struct VehicleSampleState {
    position: Option<[f32; 3]>,
    forward: Option<[f32; 3]>,
    transform: Option<Matrix4x4>,
    rotation: Option<[f32; 3]>,
    velocity: Option<[f32; 3]>,
    acceleration: Option<[f32; 3]>,
    angular_velocity: Option<[f32; 3]>,
    velocity_last_frame: Option<[f32; 3]>,
    sampled_at: Option<Instant>,
}

static PLAYER_SAMPLE: Mutex<PlayerSampleState> = Mutex::new(PlayerSampleState {
    status: 0,
    position: None,
    transform: None,
    forward: None,
    rotation: None,
    view_rotation: None,
    reticle_ray_origin: None,
    reticle_ray_direction: None,
    reticle_query_available: false,
    reticle_query_state_present: false,
    reticle_hit_fraction: None,
    reticle_candidate_position: None,
    reticle_candidate_shape: None,
    player_aim_query_available: false,
    player_aim_query_state_present: false,
    player_aim_hit_fraction: None,
    player_aim_candidate_position: None,
    player_aim_candidate_shape: None,
    ignore_characters_vehicles_query_available: false,
    ignore_characters_vehicles_query_state_present: false,
    ignore_characters_vehicles_hit_fraction: None,
    ignore_characters_vehicles_candidate_position: None,
    ignore_characters_vehicles_candidate_shape: None,
    ignore_player_owner_query_available: false,
    ignore_player_owner_query_state_present: false,
    ignore_player_owner_fraction: None,
    ignore_player_owner_candidate_position: None,
    ignore_player_owner_candidate_shape: None,
    velocity: None,
    acceleration: None,
    sampled_at: None,
});
static LAST_RETICLE_RAY_SAMPLE: Mutex<Option<Instant>> = Mutex::new(None);

#[repr(align(16))]
struct RaycastFilterContextStorage([u8; 0xD0]);
const _: () = assert!(
    std::mem::size_of::<RaycastFilterContextStorage>()
        >= C_STATIC_ONLY_FILTER_CATEGORY_MASK_OFFSET + std::mem::size_of::<u32>()
        && std::mem::size_of::<RaycastFilterContextStorage>()
            >= C_IGNORE_FILTER_OWNER_VECTOR_CAPACITY_OFFSET + std::mem::size_of::<usize>()
);

#[derive(Clone, Copy)]
struct ReticleRaySample {
    origin: Option<[f32; 3]>,
    direction: Option<[f32; 3]>,
    query_available: bool,
    query_state_present: bool,
    fraction: Option<f32>,
    candidate_position: Option<[f32; 3]>,
    candidate_shape: Option<RaycastCandidateShape>,
    player_aim_query_available: bool,
    player_aim_query_state_present: bool,
    player_aim_fraction: Option<f32>,
    player_aim_candidate_position: Option<[f32; 3]>,
    player_aim_candidate_shape: Option<RaycastCandidateShape>,
    ignore_characters_vehicles_query_available: bool,
    ignore_characters_vehicles_query_state_present: bool,
    ignore_characters_vehicles_fraction: Option<f32>,
    ignore_characters_vehicles_candidate_position: Option<[f32; 3]>,
    ignore_characters_vehicles_candidate_shape: Option<RaycastCandidateShape>,
    ignore_player_owner_query_available: bool,
    ignore_player_owner_query_state_present: bool,
    ignore_player_owner_fraction: Option<f32>,
    ignore_player_owner_candidate_position: Option<[f32; 3]>,
    ignore_player_owner_candidate_shape: Option<RaycastCandidateShape>,
}

static VEHICLE_SAMPLE: Mutex<VehicleSampleState> = Mutex::new(VehicleSampleState {
    position: None,
    forward: None,
    transform: None,
    rotation: None,
    velocity: None,
    acceleration: None,
    angular_velocity: None,
    velocity_last_frame: None,
    sampled_at: None,
});

pub(crate) fn set_runtime_endpoint(endpoint: GameEndpoint) {
    if GAME_ENDPOINT.set(Mutex::new(Some(endpoint))).is_err() {
        append_log("Bevy runtime endpoint was already initialized; keeping the original endpoint.");
    }
}

fn take_runtime_endpoint() -> Option<GameEndpoint> {
    GAME_ENDPOINT
        .get()?
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take()
}

fn publish_vehicle_sample(
    sample_data: Option<(
        Option<Matrix4x4>,
        Option<[f32; 3]>,
        Option<[f32; 3]>,
        Option<[f32; 3]>,
        Option<[f32; 3]>,
    )>,
) {
    let now = Instant::now();
    let (transform, velocity, acceleration, angular_velocity, velocity_last_frame) =
        sample_data.unwrap_or((None, None, None, None, None));
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
    sample.position = position;
    sample.forward = transform.as_ref().and_then(horizontal_forward);
    sample.transform = transform;
    sample.rotation = rotation;
    sample.velocity = velocity;
    sample.acceleration = acceleration;
    sample.angular_velocity = angular_velocity;
    sample.velocity_last_frame = velocity_last_frame;
    sample.sampled_at = (position.is_some()
        || velocity.is_some()
        || acceleration.is_some()
        || angular_velocity.is_some()
        || velocity_last_frame.is_some())
    .then_some(now);
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
    sample.transform = None;
    sample.forward = None;
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
            if UPDATE_POST_SIM_INSTALLED.load(Ordering::Acquire) {
                log_sampled_height_field_virtual_code(image_base);
                install_camera_raycast_hook(image_base);
                install_weapon_ray_position_hook(image_base);
                wait_for_stable_callback_then_install_world_query(image_base);
            }
            return;
        }
        thread::sleep(Duration::from_millis(250));
    }
    append_log(
        "UpdatePostSim probe refused: callback did not become readable executable code within 120 seconds.",
    );
}

fn log_sampled_height_field_virtual_code(image_base: usize) {
    for (label, rva) in [
        (
            "CCustomSampledHeightFieldShape::vtable+0x30",
            SAMPLED_HEIGHT_FIELD_RAY_CAST_RVA,
        ),
        (
            "CCustomSampledHeightFieldShape::vtable+0x80",
            SAMPLED_HEIGHT_FIELD_PROXY_QUERY_RVA,
        ),
    ] {
        let Some(address) = image_base.checked_add(rva) else {
            append_log(&format!(
                "Runtime code snapshot skipped for {label}: address overflow."
            ));
            continue;
        };
        if !is_readable_executable_page(address) {
            append_log(&format!(
                "Runtime code snapshot skipped for {label} at RVA 0x{rva:X}: target is not readable executable memory."
            ));
            continue;
        }
        if !readable_span(address, SHAPE_VIRTUAL_CODE_CAPTURE_LENGTH) {
            append_log(&format!(
                "Runtime code snapshot skipped for {label} at RVA 0x{rva:X}: 0x{SHAPE_VIRTUAL_CODE_CAPTURE_LENGTH:X} bytes are not in one readable memory region."
            ));
            continue;
        }

        let bytes = unsafe {
            std::slice::from_raw_parts(address as *const u8, SHAPE_VIRTUAL_CODE_CAPTURE_LENGTH)
        };
        let hex = bytes
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join(" ");
        append_log(&format!(
            "Runtime code snapshot {label} RVA=0x{rva:X} VA=0x{address:X} length=0x{:X} bytes={hex}.",
            bytes.len(),
        ));
    }
}

fn install_camera_raycast_hook(image_base: usize) {
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

unsafe extern "system" fn camera_raycast_hook(arg0: *mut c_void, arg1: *mut c_void) -> u32 {
    CAMERA_RAYCAST_CALLS.fetch_add(1, Ordering::Relaxed);
    let original = ORIGINAL_CAMERA_RAYCAST.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    let original: CameraRaycastCallback = unsafe { std::mem::transmute(original) };
    unsafe { original(arg0, arg1) }
}

fn install_weapon_ray_position_hook(image_base: usize) {
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

unsafe extern "system" fn weapon_ray_position_hook(
    weapon: *mut c_void,
    output: *mut f32,
    ray: *const f32,
) -> *mut f32 {
    let call = WEAPON_RAY_POSITION_CALLS.fetch_add(1, Ordering::Relaxed) + 1;
    let sample_this_call = call <= 8 || call % 30 == 0;
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

fn install_player_aim_filter_candidate_test_hook(image_base: usize) {
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

unsafe extern "system" fn player_aim_filter_candidate_test_hook(
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

fn start_player_aim_filter_activity_log_worker() {
    if let Err(error) = thread::Builder::new()
        .name("madmax-player-aim-filter-log".to_owned())
        .spawn(player_aim_filter_activity_log_worker)
    {
        append_log(&format!(
            "Could not start player aim filter activity logger: {error}."
        ));
    }
}

fn start_player_aim_filter_hook_install_worker(image_base: usize) {
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

fn player_aim_filter_activity_log_worker() {
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

fn start_weapon_ray_sample_log_worker() {
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

fn weapon_ray_sample_log_worker() {
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

fn wait_for_stable_callback_then_install_world_query(image_base: usize) {
    append_log(
        "Waiting for 60 stable UpdatePostSim callbacks before checking the world query helper.",
    );
    let started_at = Instant::now();
    while started_at.elapsed() < HOOK_TIMEOUT {
        if CALLBACK_STABLE_STREAK.load(Ordering::Acquire) >= CALLBACK_STABLE_STREAK_REQUIRED {
            start_player_aim_filter_hook_install_worker(image_base);
            install_world_query_hook(image_base);
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
    append_log(
        "World query tap not attempted: UpdatePostSim did not reach 60 stable callbacks within 120 seconds.",
    );
}

fn install_world_query_hook(image_base: usize) {
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

fn install_world_query_bounds_predicate_hook(image_base: usize) {
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

fn install_world_query_candidate_hook(image_base: usize) {
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

fn install_raycast_filter_add_hit_hook(image_base: usize) {
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

fn install_world_query_group_filter_hook(image_base: usize) {
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

fn start_world_query_caller_log_worker() {
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

fn world_query_caller_log_worker() {
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

fn log_world_query_caller_activity(
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

fn log_world_query_samples() {
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

fn format_atomic_words<const N: usize>(words: &[AtomicU64; N], valid: bool) -> String {
    if !valid {
        return "unreadable".to_owned();
    }
    words
        .iter()
        .map(|word| format!("{:016X}", word.load(Ordering::Relaxed)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn snapshot_printable_c_string(address: usize) -> Option<[u8; WORLD_QUERY_LABEL_CAPACITY]> {
    if !readable_span(address, WORLD_QUERY_LABEL_CAPACITY) {
        return None;
    }
    let bytes = unsafe { ptr::read_unaligned(address as *const [u8; WORLD_QUERY_LABEL_CAPACITY]) };
    let length = bytes.iter().position(|byte| *byte == 0)?;
    (length > 0
        && bytes[..length]
            .iter()
            .all(|byte| (0x20..=0x7E).contains(byte)))
    .then_some(bytes)
}

#[inline(never)]
extern "C" fn record_world_query_result(
    sample_index: usize,
    return_address: usize,
    query_data_address: usize,
    result: u64,
) {
    let Some(sample) = WORLD_QUERY_SAMPLES.get(sample_index) else {
        return;
    };
    if !sample.ready.load(Ordering::Acquire)
        || sample.caller.load(Ordering::Relaxed) != return_address
        || sample.result_claimed.swap(true, Ordering::AcqRel)
    {
        return;
    }

    sample.result.store(result & 0xFF, Ordering::Relaxed);
    let output_record_address = sample.output_record_address.load(Ordering::Relaxed);
    if readable_span(output_record_address, size_of::<[u64; 10]>()) {
        let words = unsafe { ptr::read_unaligned(output_record_address as *const [u64; 10]) };
        if return_address.checked_sub(STEAM_IMAGE_BASE) == Some(AMMO_WORLD_QUERY_CALLER_RVA) {
            let dispatch_pointer_byte_address = (words[5] as usize).checked_add(0x10);
            if let Some(address) =
                dispatch_pointer_byte_address.filter(|address| readable_span(*address, 1))
            {
                let value = unsafe { ptr::read_unaligned(address as *const u8) };
                sample
                    .ammo_dispatch_pointer_byte_10
                    .store(value, Ordering::Relaxed);
                sample
                    .ammo_dispatch_pointer_byte_10_valid
                    .store(true, Ordering::Relaxed);
            }
        }
        for (destination, value) in sample.output_record_after.iter().zip(words) {
            destination.store(value, Ordering::Relaxed);
        }
        sample
            .output_record_after_valid
            .store(true, Ordering::Relaxed);
    }
    let query_context_address = sample.query_context_address.load(Ordering::Relaxed);
    if readable_span(query_context_address, size_of::<[u64; 21]>()) {
        let words = unsafe { ptr::read_unaligned(query_context_address as *const [u64; 21]) };
        for (destination, value) in sample.query_context_after.iter().zip(words) {
            destination.store(value, Ordering::Relaxed);
        }
        sample
            .query_context_after_valid
            .store(true, Ordering::Relaxed);
    }
    if readable_span(query_data_address, size_of::<[u64; 12]>()) {
        let data = unsafe { ptr::read_unaligned(query_data_address as *const [u64; 12]) };
        for (destination, value) in sample.query_data_after.iter().zip(data) {
            destination.store(value, Ordering::Relaxed);
        }
        sample.query_data_after_valid.store(true, Ordering::Relaxed);

        // The helper's caller-owned ray block can contain pointers to
        // additional result records. Capture readable candidates while the
        // caller's stack frame is still live. Duplicate pointers are recorded
        // once, and capture remains bounded to two records per sample.
        let mut captured = 0usize;
        let mut captured_addresses = [0usize; 2];
        for pointer_slot in [3usize, 4, 5] {
            let address = sample.query_data[pointer_slot].load(Ordering::Relaxed) as usize;
            if address < 0x10000
                || captured_addresses[..captured].contains(&address)
                || captured >= sample.indirect_addresses.len()
                || !readable_span(address, size_of::<[u64; 12]>())
            {
                continue;
            }
            let indirect = unsafe { ptr::read_unaligned(address as *const [u64; 12]) };
            sample.indirect_addresses[captured].store(address, Ordering::Relaxed);
            for (destination, value) in sample.indirect_data[captured].iter().zip(indirect) {
                destination.store(value, Ordering::Relaxed);
            }
            sample.indirect_valid[captured].store(true, Ordering::Relaxed);
            captured_addresses[captured] = address;
            captured += 1;
        }
    }
    sample.result_ready.store(true, Ordering::Release);
}

#[inline(never)]
extern "C" fn record_world_query_return(result: u64) -> usize {
    let frame = WORLD_QUERY_RETURN_STACK
        .try_with(|stack| {
            stack
                .try_borrow_mut()
                .ok()
                .and_then(|mut stack| stack.pop())
        })
        .ok()
        .flatten();
    let Some(frame) = frame else {
        return 0;
    };
    if frame.sample_index < WORLD_QUERY_SAMPLE_CAPACITY {
        WORLD_QUERY_ACTIVE_SAMPLES.fetch_sub(1, Ordering::Relaxed);
        record_world_query_result(
            frame.sample_index,
            frame.return_address,
            frame.query_data_address,
            result,
        );
    }
    frame.return_address
}

fn record_world_query_caller(return_address: usize) -> Option<usize> {
    if return_address == 0 {
        return None;
    }

    let hash = return_address.wrapping_shr(4) ^ return_address.wrapping_shr(12);
    let start = hash & (WORLD_QUERY_CALLER_CAPACITY - 1);
    for offset in 0..WORLD_QUERY_CALLER_CAPACITY {
        let index = (start + offset) & (WORLD_QUERY_CALLER_CAPACITY - 1);
        let slot = &WORLD_QUERY_CALLERS[index];
        let existing = slot.load(Ordering::Acquire);
        if existing == return_address {
            return Some(index);
        }
        if existing == 0 {
            match slot.compare_exchange(0, return_address, Ordering::AcqRel, Ordering::Acquire) {
                Ok(_) => return Some(index),
                Err(observed) if observed == return_address => return Some(index),
                Err(_) => continue,
            }
        }
    }
    WORLD_QUERY_CALLER_TABLE_FULL.store(true, Ordering::Release);
    None
}

fn capture_ammo_receiver_dispatch(sample: &WorldQuerySample, receiver: usize) {
    let Some(vtable_address) = receiver
        .checked_add(0)
        .filter(|address| readable_span(*address, size_of::<usize>()))
        .map(|address| unsafe { ptr::read_unaligned(address as *const usize) })
        .filter(|address| *address != 0)
    else {
        return;
    };
    let Some(vtable_span) = vtable_address.checked_add(0x60) else {
        return;
    };
    if !readable_span(
        vtable_address,
        vtable_span - vtable_address + size_of::<usize>(),
    ) {
        return;
    }

    sample
        .ammo_receiver_vtable
        .store(vtable_address, Ordering::Relaxed);
    sample
        .ammo_receiver_vtable_valid
        .store(true, Ordering::Relaxed);
    let slot_58 = unsafe { ptr::read_unaligned((vtable_address + 0x58) as *const usize) };
    let slot_60 = unsafe { ptr::read_unaligned((vtable_address + 0x60) as *const usize) };
    sample.ammo_dispatch_58.store(slot_58, Ordering::Relaxed);
    sample.ammo_dispatch_58_valid.store(true, Ordering::Relaxed);
    sample.ammo_dispatch_60.store(slot_60, Ordering::Relaxed);
    sample.ammo_dispatch_60_valid.store(true, Ordering::Relaxed);
}

type WorldQueryCandidateCallback = unsafe extern "system" fn(*mut c_void, *mut c_void, u32) -> f32;
type CRaycastFilterAddRayHit = unsafe extern "system" fn(*mut c_void, *mut c_void, *mut c_void);
type WorldQueryBoundsPredicate = unsafe extern "system" fn(*mut c_void, *const f32, f32) -> u8;
type WorldQueryGroupFilter =
    unsafe extern "system" fn(*mut c_void, *mut u8, *const c_void, *const c_void) -> *mut u8;

static ORIGINAL_WORLD_QUERY_GROUP_FILTER: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

#[derive(Clone, Copy)]
struct WorldQueryCandidateBefore {
    sample_index: usize,
    callback_slot: Option<usize>,
    ray_record: usize,
    collector_state_address: usize,
}

#[derive(Clone, Copy)]
struct RaycastFilterHitCapture {
    sample: &'static RaycastFilterHitSample,
    filter_context: usize,
}

fn is_reticle_query_label_bytes(label: &[u8; WORLD_QUERY_LABEL_CAPACITY]) -> bool {
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

fn is_reticle_query_label(sample: &WorldQuerySample) -> bool {
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

fn register_reticle_query_caller(return_address: usize) -> Option<usize> {
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

fn begin_raycast_filter_hit_capture(
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

fn finish_raycast_filter_hit_capture(capture: RaycastFilterHitCapture) {
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

fn world_query_candidate_capture_slot(callback_ordinal: u32) -> Option<usize> {
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

fn current_world_query_sample_index() -> Option<usize> {
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

unsafe extern "system" fn native_world_query_bounds_predicate_tap(
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

fn pointer_at(address: usize) -> usize {
    if address >= 0x10000 && readable_span(address, size_of::<usize>()) {
        unsafe { ptr::read_unaligned(address as *const usize) }
    } else {
        0
    }
}

fn candidate_virtual_target(object: usize, slot: usize) -> usize {
    pointer_at(pointer_at(object).checked_add(slot).unwrap_or_default())
}

fn begin_world_query_candidate_capture(
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

fn finish_world_query_candidate_capture(capture: WorldQueryCandidateBefore, fraction: f32) {
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

unsafe extern "system" fn native_world_query_candidate_tap(
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

unsafe extern "system" fn raycast_filter_add_hit_tap(
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

unsafe extern "system" fn native_world_query_group_filter_tap(
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
    if let Some((candidate_sample, ray_info, candidate_info)) = capture {
        if !result_pointer.is_null() && readable_span(result_pointer as usize, size_of::<u8>()) {
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
    }
    result_pointer
}

// Only the selected diagnostic calls replace the return address. All calls
// tail-jump into the original trampoline with the caller's stack unchanged.
#[unsafe(naked)]
unsafe extern "system" fn native_world_query_tap() -> ! {
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
unsafe extern "system" fn native_world_query_return_tap() -> ! {
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

fn writable_span(address: usize, length: usize) -> bool {
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

fn input_focus_address() -> Option<usize> {
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

fn synchronize_game_input_focus(overlap_epoch: u64) {
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
    let thread_id = unsafe { GetCurrentThreadId() };
    let first_thread = CALLBACK_OWNER_THREAD_ID.load(Ordering::Acquire);
    if first_thread == 0 {
        let _ = CALLBACK_OWNER_THREAD_ID.compare_exchange(
            0,
            thread_id,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
    }
    let expected_thread = CALLBACK_OWNER_THREAD_ID.load(Ordering::Acquire);
    let overlap_epoch_at_entry = CALLBACK_OVERLAP_EPOCH.load(Ordering::Acquire);
    let entered_while_active = CALLBACK_IN_FLIGHT.fetch_add(1, Ordering::AcqRel) != 0;
    if entered_while_active {
        CALLBACK_OVERLAPS.fetch_add(1, Ordering::AcqRel);
        CALLBACK_OVERLAP_EPOCH.fetch_add(1, Ordering::AcqRel);
    }

    let original: UpdatePostSim = unsafe { std::mem::transmute(original) };
    unsafe { original(context) };
    UPDATE_POST_SIM_CALLS.fetch_add(1, Ordering::AcqRel);

    let thread_mismatch = thread_id != expected_thread;
    if thread_mismatch {
        CALLBACK_THREAD_MISMATCHES.fetch_add(1, Ordering::AcqRel);
    }
    let overlapping_now = CALLBACK_IN_FLIGHT.load(Ordering::Acquire) != 1
        || CALLBACK_OVERLAP_EPOCH.load(Ordering::Acquire) != overlap_epoch_at_entry;
    if entered_while_active || overlapping_now || thread_mismatch {
        CALLBACK_STABLE_STREAK.store(0, Ordering::Release);
        CALLBACK_GATE_OPEN.store(false, Ordering::Release);
    } else {
        let streak = CALLBACK_STABLE_STREAK
            .fetch_add(1, Ordering::AcqRel)
            .saturating_add(1);
        CALLBACK_GATE_OPEN.store(streak >= CALLBACK_STABLE_STREAK_REQUIRED, Ordering::Release);
    }

    if CALLBACK_GATE_OPEN.load(Ordering::Acquire)
        && !entered_while_active
        && !overlapping_now
        && !thread_mismatch
    {
        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            run_bevy_game_frame(thread_id, overlap_epoch_at_entry)
        }));
        if result.is_err() {
            CALLBACK_RUNTIME_PANICS.fetch_add(1, Ordering::AcqRel);
        }
    }
    CALLBACK_IN_FLIGHT.fetch_sub(1, Ordering::AcqRel);
}

fn trace_game_runtime_stage(stage: u32, description: &str) {
    let bit = 1_u32 << stage;
    if GAME_RUNTIME_TRACE_STAGES.fetch_or(bit, Ordering::AcqRel) & bit == 0 {
        append_log(&format!("Game callback diagnostic: {description}"));
    }
}

fn run_bevy_game_frame(thread_id: u32, overlap_epoch: u64) {
    trace_game_runtime_stage(0, "entered runtime callback");
    if CALLBACK_IN_FLIGHT.load(Ordering::Acquire) != 1
        || CALLBACK_OVERLAP_EPOCH.load(Ordering::Acquire) != overlap_epoch
    {
        return;
    }
    synchronize_game_input_focus(overlap_epoch);
    trace_game_runtime_stage(1, "input-focus synchronization returned");
    let now = Instant::now();
    let should_sample = {
        let mut last_sample = LAST_GAME_STATE_SAMPLE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if last_sample
            .is_none_or(|last_sample| now.duration_since(last_sample) >= GAME_STATE_SAMPLE_INTERVAL)
        {
            *last_sample = Some(now);
            true
        } else {
            false
        }
    };
    if should_sample {
        if CALLBACK_IN_FLIGHT.load(Ordering::Acquire) != 1
            || CALLBACK_OVERLAP_EPOCH.load(Ordering::Acquire) != overlap_epoch
        {
            return;
        }
        GAME_SAMPLE_SEQUENCE.fetch_add(1, Ordering::AcqRel);
        probe_game_state();
        trace_game_runtime_stage(2, "game-state probe returned");
    }

    if CALLBACK_IN_FLIGHT.load(Ordering::Acquire) != 1
        || CALLBACK_OVERLAP_EPOCH.load(Ordering::Acquire) != overlap_epoch
    {
        return;
    }

    let snapshot = current_game_snapshot(thread_id);
    trace_game_runtime_stage(3, "game snapshot copied");
    GAME_RUNTIME.with(|runtime_cell| {
        let mut runtime_cell = runtime_cell.borrow_mut();
        if runtime_cell.is_none() {
            if let Some(endpoint) = take_runtime_endpoint() {
                *runtime_cell = Some(GameRuntime::new(endpoint));
                trace_game_runtime_stage(4, "Bevy game runtime initialized");
            }
        }
        let Some(runtime) = runtime_cell.as_mut() else {
            return;
        };

        if let Some(pending) = runtime.prepare_and_run(snapshot) {
            match pending {
                PendingGameIntent::Spawn(pending) => {
                    let outcome = apply_spawn_intent(&pending, overlap_epoch);
                    runtime.complete_spawn_intent(pending, outcome);
                }
                PendingGameIntent::Velocity(pending) => {
                    let result = apply_velocity_intent(pending, overlap_epoch);
                    runtime.complete_intent(pending, result);
                }
            }
        }
        let script_debug_draws = runtime.take_script_debug_draws();
        if !script_debug_draws.is_empty() {
            if let Err(reason) = draw_script_debug_primitives(&script_debug_draws, overlap_epoch) {
                let source = script_debug_draws
                    .first()
                    .map(|effect| effect.origin.id.as_str())
                    .unwrap_or("unknown");
                append_log(&format!(
                    "CDebugRenderer rejected script draw batch from {source}: {reason}."
                ));
            } else if !SCRIPT_DEBUG_DRAW_SUBMISSION_LOGGED.swap(true, Ordering::AcqRel) {
                let source = script_debug_draws
                    .first()
                    .map(|effect| effect.origin.id.as_str())
                    .unwrap_or("unknown");
                append_log(&format!(
                    "CDebugRenderer accepted the first script draw batch from {source} ({} primitives). Visibility still depends on the game's renderer queue.",
                    script_debug_draws.len()
                ));
            }
        }
        trace_game_runtime_stage(5, "Bevy post-simulation schedule returned");
    });
}

fn draw_script_debug_primitives(
    draws: &[crate::scripting::manager::ScriptDebugDrawEffect],
    overlap_epoch: u64,
) -> Result<(), &'static str> {
    if CALLBACK_IN_FLIGHT.load(Ordering::Acquire) != 1
        || CALLBACK_OVERLAP_EPOCH.load(Ordering::Acquire) != overlap_epoch
    {
        return Err("UpdatePostSim callback was not exclusive");
    }
    let module = unsafe { GetModuleHandleW(hudhook::windows::core::PCWSTR::null()) }
        .map_err(|_| "could not read the game image base")?;
    let image_base = module.0 as usize;
    if image_base != STEAM_IMAGE_BASE {
        return Err("game image base did not match the supported Steam build");
    }
    let facade_global = image_base + CDEBUG_RENDERER_INSTANCE_GLOBAL_RVA;
    if !readable_span(facade_global, size_of::<*mut c_void>()) {
        return Err("CDebugRenderer singleton pointer is unreadable");
    }
    let facade = unsafe { ptr::read_volatile(facade_global as *const *mut c_void) };
    if facade.is_null() || !readable_span(facade as usize, 0x18) {
        return Err("CDebugRenderer facade object is unavailable");
    }
    let facade_vtable = unsafe { ptr::read_volatile(facade.cast::<usize>()) };
    if facade_vtable != image_base + CDEBUG_RENDERER_FACADE_VTABLE_RVA
        || !readable_span(
            facade_vtable,
            (CDEBUG_RENDERER_BOUNDS_CONVEX_VERTICES_SLOT + 1) * size_of::<usize>(),
        )
    {
        return Err("CDebugRenderer facade vtable did not match the mapped interface");
    }
    let backend =
        unsafe { ptr::read_volatile((facade as *const u8).add(0x10).cast::<*mut c_void>()) };
    if backend.is_null() || !readable_span(backend as usize, size_of::<usize>()) {
        return Err("CDebugRenderer backend is unavailable");
    }
    let backend_vtable = unsafe { ptr::read_volatile(backend.cast::<usize>()) };
    if !readable_span(
        backend_vtable,
        (CDEBUG_RENDERER_BOUNDS_CONVEX_VERTICES_SLOT + 1) * size_of::<usize>(),
    ) {
        return Err("CDebugRenderer backend vtable is unreadable");
    }
    let backend_line = unsafe {
        ptr::read_volatile((backend_vtable as *const usize).add(CDEBUG_RENDERER_LINE_SLOT))
    };
    let backend_arrow = unsafe {
        ptr::read_volatile((backend_vtable as *const usize).add(CDEBUG_RENDERER_ARROW_SLOT))
    };
    let backend_axes = unsafe {
        ptr::read_volatile((backend_vtable as *const usize).add(CDEBUG_RENDERER_AXES_SLOT))
    };
    if backend_line != image_base + PRIMITIVE_RENDERER_LINE_RVA
        || backend_arrow != image_base + PRIMITIVE_RENDERER_ARROW_RVA
        || backend_axes != image_base + PRIMITIVE_RENDERER_AXES_RVA
        || !is_readable_executable_page(backend_line)
        || !is_readable_executable_page(backend_arrow)
        || !is_readable_executable_page(backend_axes)
    {
        return Err("active backend did not match the mapped NGraphicsEngine primitive renderer");
    }
    let line_target = unsafe {
        ptr::read_volatile((facade_vtable as *const usize).add(CDEBUG_RENDERER_LINE_SLOT))
    };
    let arrow_target = unsafe {
        ptr::read_volatile((facade_vtable as *const usize).add(CDEBUG_RENDERER_ARROW_SLOT))
    };
    let axes_target = unsafe {
        ptr::read_volatile((facade_vtable as *const usize).add(CDEBUG_RENDERER_AXES_SLOT))
    };
    if line_target != image_base + CDEBUG_RENDERER_LINE_METHOD_RVA
        || arrow_target != image_base + CDEBUG_RENDERER_ARROW_METHOD_RVA
        || axes_target != image_base + CDEBUG_RENDERER_AXES_METHOD_RVA
        || !is_readable_executable_page(line_target)
        || !is_readable_executable_page(arrow_target)
        || !is_readable_executable_page(axes_target)
    {
        return Err("CDebugRenderer facade methods did not match the mapped line/arrow/axes slots");
    }

    let mapped_facade_methods = [
        (
            CDEBUG_RENDERER_SPHERE_SLOT,
            CDEBUG_RENDERER_SPHERE_METHOD_RVA,
        ),
        (
            CDEBUG_RENDERER_SPARSE_SPHERE_SLOT,
            CDEBUG_RENDERER_SPARSE_SPHERE_METHOD_RVA,
        ),
        (
            CDEBUG_RENDERER_TESSELLATED_SPHERE_SLOT,
            CDEBUG_RENDERER_TESSELLATED_SPHERE_METHOD_RVA,
        ),
        (CDEBUG_RENDERER_AABB_SLOT, CDEBUG_RENDERER_AABB_METHOD_RVA),
        (
            CDEBUG_RENDERER_TRANSFORMED_AABB_SLOT,
            CDEBUG_RENDERER_TRANSFORMED_AABB_METHOD_RVA,
        ),
        (
            CDEBUG_RENDERER_QUEUED_TRANSFORMED_AABB_SLOT,
            CDEBUG_RENDERER_QUEUED_TRANSFORMED_AABB_METHOD_RVA,
        ),
        (
            CDEBUG_RENDERER_BOUNDS_SPHERE_SLOT,
            CDEBUG_RENDERER_BOUNDS_SPHERE_METHOD_RVA,
        ),
        (
            CDEBUG_RENDERER_BOUNDS_CYLINDER_SLOT,
            CDEBUG_RENDERER_BOUNDS_CYLINDER_METHOD_RVA,
        ),
        (
            CDEBUG_RENDERER_BOUNDS_CAPSULE_SLOT,
            CDEBUG_RENDERER_BOUNDS_CAPSULE_METHOD_RVA,
        ),
        (
            CDEBUG_RENDERER_BOUNDS_CONVEX_VERTICES_SLOT,
            CDEBUG_RENDERER_BOUNDS_CONVEX_VERTICES_METHOD_RVA,
        ),
    ];
    for (slot, rva) in mapped_facade_methods {
        let target = unsafe { ptr::read_volatile((facade_vtable as *const usize).add(slot)) };
        if target != image_base + rva || !is_readable_executable_page(target) {
            return Err("CDebugRenderer facade geometry method did not match the mapped interface");
        }
    }
    let backend_geometry_slots = [
        CDEBUG_RENDERER_SPHERE_SLOT,
        CDEBUG_RENDERER_SPARSE_SPHERE_SLOT,
        CDEBUG_RENDERER_TESSELLATED_SPHERE_SLOT,
        CDEBUG_RENDERER_AABB_SLOT,
        CDEBUG_RENDERER_TRANSFORMED_AABB_SLOT,
        CDEBUG_RENDERER_QUEUED_TRANSFORMED_AABB_SLOT,
        CDEBUG_RENDERER_BOUNDS_SPHERE_SLOT,
        CDEBUG_RENDERER_BOUNDS_CYLINDER_SLOT,
        CDEBUG_RENDERER_BOUNDS_CAPSULE_SLOT,
        CDEBUG_RENDERER_BOUNDS_CONVEX_VERTICES_SLOT,
    ];
    if backend_geometry_slots.into_iter().any(|slot| {
        let target = unsafe { ptr::read_volatile((backend_vtable as *const usize).add(slot)) };
        !is_readable_executable_page(target)
    }) {
        return Err("CDebugRenderer backend geometry slot was not executable");
    }

    let line: CDebugRendererLine = unsafe { std::mem::transmute(line_target) };
    let arrow: CDebugRendererArrow = unsafe { std::mem::transmute(arrow_target) };
    let axes: CDebugRendererAxes = unsafe { std::mem::transmute(axes_target) };
    let sphere: CDebugRendererSphere =
        unsafe { std::mem::transmute(image_base + CDEBUG_RENDERER_SPHERE_METHOD_RVA) };
    let sparse_sphere: CDebugRendererSphere =
        unsafe { std::mem::transmute(image_base + CDEBUG_RENDERER_SPARSE_SPHERE_METHOD_RVA) };
    let tessellated_sphere: CDebugRendererSphere =
        unsafe { std::mem::transmute(image_base + CDEBUG_RENDERER_TESSELLATED_SPHERE_METHOD_RVA) };
    let aabb: CDebugRendererAabb =
        unsafe { std::mem::transmute(image_base + CDEBUG_RENDERER_AABB_METHOD_RVA) };
    let transformed_aabb: CDebugRendererTransformedAabb =
        unsafe { std::mem::transmute(image_base + CDEBUG_RENDERER_TRANSFORMED_AABB_METHOD_RVA) };
    let queued_transformed_aabb: CDebugRendererTransformedAabb = unsafe {
        std::mem::transmute(image_base + CDEBUG_RENDERER_QUEUED_TRANSFORMED_AABB_METHOD_RVA)
    };
    let bounds_sphere: CDebugRendererBoundsShape =
        unsafe { std::mem::transmute(image_base + CDEBUG_RENDERER_BOUNDS_SPHERE_METHOD_RVA) };
    let bounds_cylinder: CDebugRendererBoundsShape =
        unsafe { std::mem::transmute(image_base + CDEBUG_RENDERER_BOUNDS_CYLINDER_METHOD_RVA) };
    let bounds_capsule: CDebugRendererBoundsShape =
        unsafe { std::mem::transmute(image_base + CDEBUG_RENDERER_BOUNDS_CAPSULE_METHOD_RVA) };
    let bounds_convex_vertices: CDebugRendererBoundsShape = unsafe {
        std::mem::transmute(image_base + CDEBUG_RENDERER_BOUNDS_CONVEX_VERTICES_METHOD_RVA)
    };
    for draw in draws {
        match draw.primitive {
            DebugPrimitive::Axes { position, length } => {
                let mut transform = [0.0_f32; 16];
                transform[0] = 1.0;
                transform[5] = 1.0;
                transform[10] = 1.0;
                transform[15] = 1.0;
                transform[12] = position[0];
                transform[13] = position[1];
                transform[14] = position[2];
                unsafe { axes(facade, &transform, length) };
            }
            DebugPrimitive::TransformedAxes { transform, length } => {
                unsafe { axes(facade, &transform, length) };
            }
            DebugPrimitive::Line { start, end, color } => {
                unsafe { line(facade, &start, &end, &color, &color) };
            }
            DebugPrimitive::Arrow { start, end, color } => {
                unsafe { arrow(facade, &start, &end, &color, &color) };
            }
            DebugPrimitive::Sphere {
                center,
                radius,
                color,
            } => unsafe { sphere(facade, &center, radius, &color) },
            DebugPrimitive::SparseSphere {
                center,
                radius,
                color,
            } => unsafe { sparse_sphere(facade, &center, radius, &color) },
            DebugPrimitive::TessellatedSphere {
                center,
                radius,
                color,
            } => unsafe { tessellated_sphere(facade, &center, radius, &color) },
            DebugPrimitive::Aabb {
                minimum,
                maximum,
                color,
            } => unsafe { aabb(facade, &minimum, &maximum, &color) },
            DebugPrimitive::TransformedAabb {
                transform,
                minimum,
                maximum,
                color,
            } => unsafe { transformed_aabb(facade, &transform, &minimum, &maximum, &color) },
            DebugPrimitive::QueuedTransformedAabb {
                transform,
                minimum,
                maximum,
                color,
            } => unsafe { queued_transformed_aabb(facade, &transform, &minimum, &maximum, &color) },
            DebugPrimitive::BoundsSphere {
                transform,
                minimum,
                maximum,
                color,
            } => unsafe { bounds_sphere(facade, &transform, &minimum, &maximum, &color) },
            DebugPrimitive::BoundsCylinder {
                transform,
                minimum,
                maximum,
                color,
            } => unsafe { bounds_cylinder(facade, &transform, &minimum, &maximum, &color) },
            DebugPrimitive::BoundsCapsule {
                transform,
                minimum,
                maximum,
                color,
            } => unsafe { bounds_capsule(facade, &transform, &minimum, &maximum, &color) },
            DebugPrimitive::BoundsConvexVertices {
                transform,
                minimum,
                maximum,
                color,
            } => unsafe { bounds_convex_vertices(facade, &transform, &minimum, &maximum, &color) },
        }
    }
    Ok(())
}

fn current_game_snapshot(thread_id: u32) -> GameSnapshot {
    let player = PLAYER_SAMPLE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let player_fresh = player
        .sampled_at
        .is_some_and(|sampled_at| sampled_at.elapsed() <= PLAYER_SAMPLE_MAX_AGE);
    let player_stale = player.status == 10 && !player_fresh;
    let vehicle = VEHICLE_SAMPLE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let vehicle_fresh = vehicle
        .sampled_at
        .is_some_and(|sampled_at| sampled_at.elapsed() <= PLAYER_SAMPLE_MAX_AGE);
    let latest_sample_at = player.sampled_at.max(vehicle.sampled_at);
    let sample_age_seconds = latest_sample_at.map(|sampled_at| sampled_at.elapsed().as_secs_f32());

    GameSnapshot {
        callback_sequence: UPDATE_POST_SIM_CALLS.load(Ordering::Acquire),
        camera_raycast_hook_installed: CAMERA_RAYCAST_HOOK_INSTALLED.load(Ordering::Acquire),
        camera_raycast_calls: CAMERA_RAYCAST_CALLS.load(Ordering::Acquire),
        world_query_hook_installed: WORLD_QUERY_HOOK_INSTALLED.load(Ordering::Acquire),
        world_query_calls: WORLD_QUERY_CALLS.load(Ordering::Acquire),
        world_query_last_return_address: match WORLD_QUERY_LAST_RETURN_ADDRESS
            .load(Ordering::Acquire)
        {
            0 => None,
            address => Some(address),
        },
        sample_sequence: GAME_SAMPLE_SEQUENCE.load(Ordering::Acquire),
        sample_age_seconds,
        callback_thread_id: Some(thread_id),
        callback_stable_streak: CALLBACK_STABLE_STREAK.load(Ordering::Acquire),
        callback_overlap_count: CALLBACK_OVERLAPS.load(Ordering::Acquire),
        callback_thread_mismatch_count: CALLBACK_THREAD_MISMATCHES.load(Ordering::Acquire),
        callback_gate_open: CALLBACK_GATE_OPEN.load(Ordering::Acquire),
        callback_panics: CALLBACK_RUNTIME_PANICS.load(Ordering::Acquire),
        player_position: (!player_stale).then_some(player.position).flatten(),
        player_forward: (!player_stale).then_some(player.forward).flatten(),
        player_transform: (!player_stale).then_some(player.transform).flatten(),
        player_rotation: (!player_stale).then_some(player.rotation).flatten(),
        view_rotation: (!player_stale).then_some(player.view_rotation).flatten(),
        reticle_ray_origin: (!player_stale)
            .then_some(player.reticle_ray_origin)
            .flatten(),
        reticle_ray_direction: (!player_stale)
            .then_some(player.reticle_ray_direction)
            .flatten(),
        reticle_query_available: !player_stale && player.reticle_query_available,
        reticle_query_state_present: !player_stale && player.reticle_query_state_present,
        reticle_hit_fraction: (!player_stale)
            .then_some(player.reticle_hit_fraction)
            .flatten(),
        reticle_candidate_position: (!player_stale)
            .then_some(player.reticle_candidate_position)
            .flatten(),
        reticle_candidate_shape: (!player_stale)
            .then_some(player.reticle_candidate_shape)
            .flatten(),
        player_aim_query_available: !player_stale && player.player_aim_query_available,
        player_aim_query_state_present: !player_stale && player.player_aim_query_state_present,
        player_aim_hit_fraction: (!player_stale)
            .then_some(player.player_aim_hit_fraction)
            .flatten(),
        player_aim_candidate_position: (!player_stale)
            .then_some(player.player_aim_candidate_position)
            .flatten(),
        player_aim_candidate_shape: (!player_stale)
            .then_some(player.player_aim_candidate_shape)
            .flatten(),
        ignore_characters_vehicles_query_available: !player_stale
            && player.ignore_characters_vehicles_query_available,
        ignore_characters_vehicles_query_state_present: !player_stale
            && player.ignore_characters_vehicles_query_state_present,
        ignore_characters_vehicles_hit_fraction: (!player_stale)
            .then_some(player.ignore_characters_vehicles_hit_fraction)
            .flatten(),
        ignore_characters_vehicles_candidate_position: (!player_stale)
            .then_some(player.ignore_characters_vehicles_candidate_position)
            .flatten(),
        ignore_characters_vehicles_candidate_shape: (!player_stale)
            .then_some(player.ignore_characters_vehicles_candidate_shape)
            .flatten(),
        ignore_player_owner_query_available: !player_stale
            && player.ignore_player_owner_query_available,
        ignore_player_owner_query_state_present: !player_stale
            && player.ignore_player_owner_query_state_present,
        ignore_player_owner_hit_fraction: (!player_stale)
            .then_some(player.ignore_player_owner_fraction)
            .flatten(),
        ignore_player_owner_candidate_position: (!player_stale)
            .then_some(player.ignore_player_owner_candidate_position)
            .flatten(),
        ignore_player_owner_candidate_shape: (!player_stale)
            .then_some(player.ignore_player_owner_candidate_shape)
            .flatten(),
        player_velocity: (!player_stale).then_some(player.velocity).flatten(),
        player_acceleration: (!player_stale).then_some(player.acceleration).flatten(),
        vehicle_position: vehicle_fresh.then_some(vehicle.position).flatten(),
        vehicle_transform: vehicle_fresh.then_some(vehicle.transform).flatten(),
        vehicle_forward: vehicle_fresh.then_some(vehicle.forward).flatten(),
        vehicle_rotation: vehicle_fresh.then_some(vehicle.rotation).flatten(),
        vehicle_velocity: vehicle_fresh.then_some(vehicle.velocity).flatten(),
        vehicle_acceleration: vehicle_fresh.then_some(vehicle.acceleration).flatten(),
        vehicle_angular_velocity: vehicle_fresh.then_some(vehicle.angular_velocity).flatten(),
        vehicle_velocity_last_frame: vehicle_fresh
            .then_some(vehicle.velocity_last_frame)
            .flatten(),
        ..GameSnapshot::default()
    }
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
    let player = unsafe { get_player_character(manager.cast::<CharacterManager>()) };
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
    publish_vehicle_sample(sample_vehicle(player));
}

fn apply_velocity_intent(pending: PendingVelocityIntent, overlap_epoch: u64) -> IntentResult {
    if CALLBACK_IN_FLIGHT.load(Ordering::Acquire) != 1
        || CALLBACK_OVERLAP_EPOCH.load(Ordering::Acquire) != overlap_epoch
    {
        return IntentResult::CallbackNotExclusive;
    }
    if !pending.velocity.iter().all(|value| value.is_finite()) {
        return IntentResult::InvalidValue;
    }
    let Some(vehicle) = current_player_vehicle() else {
        return IntentResult::NoCurrentVehicle;
    };
    let Some(target) = vehicle_virtual_function(vehicle, VEHICLE_SET_VELOCITY_VTABLE_SLOT) else {
        return IntentResult::SetterUnavailable;
    };
    let set_velocity: SetVehicleVector = unsafe { std::mem::transmute(target) };
    unsafe { set_velocity(vehicle, &pending.velocity) };
    IntentResult::Applied
}

const SPAWN_INFO_BYTES: usize = 0x98;
const SPAWN_INFO_ID_OFFSET: usize = 0x00;
const SPAWN_INFO_PATCH_ID_OFFSET: usize = 0x04;
const SPAWN_INFO_DESPAWN_DISTANCE_OFFSET: usize = 0x08;
const SPAWN_INFO_PRIORITY_OFFSET: usize = 0x4C;

struct SpawnInfoGuard {
    pointer: *mut SpawnInstanceInfo,
    destructor: SpawnInfoDestructorFn,
}

impl Drop for SpawnInfoGuard {
    fn drop(&mut self) {
        unsafe { (self.destructor)(self.pointer) };
    }
}

fn apply_spawn_intent(pending: &PendingSpawnIntent, overlap_epoch: u64) -> SpawnOutcome {
    let outcome = |result, native_return| SpawnOutcome {
        request_id: pending.request_id,
        result,
        native_return,
    };
    if CALLBACK_IN_FLIGHT.load(Ordering::Acquire) != 1
        || CALLBACK_OVERLAP_EPOCH.load(Ordering::Acquire) != overlap_epoch
    {
        return outcome(SpawnRequestResult::CallbackNotExclusive, None);
    }
    let Ok(asset_key) = CString::new(format!("{}.ee", pending.asset_key)) else {
        return outcome(SpawnRequestResult::InvalidRequest, None);
    };
    if !pending
        .matrix
        .values
        .iter()
        .flatten()
        .all(|value| value.is_finite())
        || pending.request_id == 0
    {
        return outcome(SpawnRequestResult::InvalidRequest, None);
    }
    if pending.request_id > u64::from(u32::MAX) {
        return outcome(SpawnRequestResult::SpawnIdUnavailable, None);
    }
    if ![
        SPAWN_INFO_CTOR,
        SPAWN_INFO_DTOR,
        SPAWN_SET_ENTITY_FILE,
        SPAWN_SUBMIT,
    ]
    .into_iter()
    .all(is_readable_executable_page)
    {
        return outcome(SpawnRequestResult::NativeSymbolUnavailable, None);
    }

    let constructor: SpawnInfoConstructorFn = unsafe { std::mem::transmute(SPAWN_INFO_CTOR) };
    let destructor: SpawnInfoDestructorFn = unsafe { std::mem::transmute(SPAWN_INFO_DTOR) };
    let set_entity_file: SetEntityFileFn = unsafe { std::mem::transmute(SPAWN_SET_ENTITY_FILE) };
    let spawn: SpawnFn = unsafe { std::mem::transmute(SPAWN_SUBMIT) };
    let mut storage = MaybeUninit::<[u64; SPAWN_INFO_BYTES / size_of::<u64>()]>::uninit();
    let info = storage.as_mut_ptr().cast::<SpawnInstanceInfo>();
    unsafe { constructor(info) };
    let _guard = SpawnInfoGuard {
        pointer: info,
        destructor,
    };
    unsafe { set_entity_file(info, asset_key.as_ptr()) };

    let raw_info = info.cast::<u8>();
    // This first experiment uses a request-derived nonzero scalar ID because
    // the SDK's CHashString reference-returning allocator has an unverified
    // Rust ABI. It is unique within this process until the u32 range is spent.
    let spawn_id = (pending.request_id as u32).max(1);
    unsafe {
        ptr::write_unaligned(raw_info.add(SPAWN_INFO_ID_OFFSET).cast::<u32>(), spawn_id);
        ptr::write_unaligned(
            raw_info.add(SPAWN_INFO_PATCH_ID_OFFSET).cast::<u32>(),
            spawn_id,
        );
        ptr::write_unaligned(
            raw_info
                .add(SPAWN_INFO_DESPAWN_DISTANCE_OFFSET)
                .cast::<f32>(),
            90_000.0,
        );
        ptr::write_unaligned(raw_info.add(SPAWN_INFO_PRIORITY_OFFSET).cast::<i32>(), 2);
    }

    let mut matrix = pending.matrix;
    let native_return = unsafe { spawn(info, &mut matrix) };
    outcome(SpawnRequestResult::NativeCallSubmitted, Some(native_return))
}

fn current_player_vehicle() -> Option<*mut Vehicle> {
    if !readable_span(CHARACTER_MANAGER_SINGLETON, size_of::<*mut c_void>())
        || !is_readable_executable_page(GET_PLAYER_CHARACTER)
        || !is_readable_executable_page(GET_CHARACTER_VEHICLE)
    {
        return None;
    }
    let manager = unsafe { ptr::read_volatile(CHARACTER_MANAGER_SINGLETON as *const *mut c_void) };
    if manager.is_null() || !readable_span(manager as usize, 1) {
        return None;
    }
    let get_player_character: GetPlayerCharacter =
        unsafe { std::mem::transmute(GET_PLAYER_CHARACTER) };
    let player = unsafe { get_player_character(manager.cast::<CharacterManager>()) };
    if player.is_null() || !readable_span(player as usize, 1) {
        return None;
    }
    let get_vehicle: GetCharacterVehicle = unsafe { std::mem::transmute(GET_CHARACTER_VEHICLE) };
    let vehicle = unsafe { get_vehicle(player) };
    (!vehicle.is_null() && readable_span(vehicle as usize, 1)).then_some(vehicle)
}

fn probe_player_transform(player: *mut Character) {
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
    if !unsafe { get_transform(game_object_address as *mut GameObject, &mut transform) } {
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
    let camera_matrix = sample_camera_matrix();
    let view_rotation = camera_matrix.as_ref().and_then(rotation_xyz_degrees);
    let reticle_ray_sample = camera_matrix
        .as_ref()
        .and_then(|matrix| sample_reticle_ray_if_due(matrix, player));
    publish_player_sample(10, Some(position), rotation, view_rotation);
    if let Some(sample) = reticle_ray_sample {
        let mut player_sample = PLAYER_SAMPLE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        player_sample.reticle_ray_origin = sample.origin;
        player_sample.reticle_ray_direction = sample.direction;
        player_sample.reticle_query_available = sample.query_available;
        player_sample.reticle_query_state_present = sample.query_state_present;
        player_sample.reticle_hit_fraction = sample.fraction;
        player_sample.reticle_candidate_position = sample.candidate_position;
        player_sample.reticle_candidate_shape = sample.candidate_shape;
        player_sample.player_aim_query_available = sample.player_aim_query_available;
        player_sample.player_aim_query_state_present = sample.player_aim_query_state_present;
        player_sample.player_aim_hit_fraction = sample.player_aim_fraction;
        player_sample.player_aim_candidate_position = sample.player_aim_candidate_position;
        player_sample.player_aim_candidate_shape = sample.player_aim_candidate_shape;
        player_sample.ignore_characters_vehicles_query_available =
            sample.ignore_characters_vehicles_query_available;
        player_sample.ignore_characters_vehicles_query_state_present =
            sample.ignore_characters_vehicles_query_state_present;
        player_sample.ignore_characters_vehicles_hit_fraction =
            sample.ignore_characters_vehicles_fraction;
        player_sample.ignore_characters_vehicles_candidate_position =
            sample.ignore_characters_vehicles_candidate_position;
        player_sample.ignore_characters_vehicles_candidate_shape =
            sample.ignore_characters_vehicles_candidate_shape;
        player_sample.ignore_player_owner_query_available =
            sample.ignore_player_owner_query_available;
        player_sample.ignore_player_owner_query_state_present =
            sample.ignore_player_owner_query_state_present;
        player_sample.ignore_player_owner_fraction = sample.ignore_player_owner_fraction;
        player_sample.ignore_player_owner_candidate_position =
            sample.ignore_player_owner_candidate_position;
        player_sample.ignore_player_owner_candidate_shape =
            sample.ignore_player_owner_candidate_shape;
    }
    let mut player_sample = PLAYER_SAMPLE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    player_sample.transform = Some(transform);
    player_sample.forward = horizontal_forward(&transform);
}

fn sample_vehicle(
    player: *mut Character,
) -> Option<(
    Option<Matrix4x4>,
    Option<[f32; 3]>,
    Option<[f32; 3]>,
    Option<[f32; 3]>,
    Option<[f32; 3]>,
)> {
    if !is_readable_executable_page(GET_CHARACTER_VEHICLE) {
        return None;
    }

    let get_vehicle: GetCharacterVehicle = unsafe { std::mem::transmute(GET_CHARACTER_VEHICLE) };
    let vehicle = unsafe { get_vehicle(player) };
    if vehicle.is_null() || !readable_span(vehicle as usize, 1) {
        return None;
    }

    let velocity = read_vehicle_vector(vehicle, VEHICLE_GET_VELOCITY_VTABLE_SLOT);
    let acceleration = read_vehicle_vector(vehicle, VEHICLE_GET_ACCELERATION_VTABLE_SLOT);
    let angular_velocity = read_vehicle_vector(vehicle, VEHICLE_GET_ANGULAR_VELOCITY_VTABLE_SLOT);
    let velocity_last_frame =
        read_vehicle_last_frame_vector(vehicle, VEHICLE_GET_VELOCITY_LAST_FRAME_VTABLE_SLOT);

    let Some(game_object_address) = (vehicle as usize).checked_add(VEHICLE_TO_GAME_OBJECT_OFFSET)
    else {
        return Some((
            None,
            velocity,
            acceleration,
            angular_velocity,
            velocity_last_frame,
        ));
    };
    if game_object_address % std::mem::align_of::<*const usize>() != 0
        || !readable_span(game_object_address, size_of::<*const usize>())
    {
        return Some((
            None,
            velocity,
            acceleration,
            angular_velocity,
            velocity_last_frame,
        ));
    }

    let vtable = unsafe { ptr::read_volatile(game_object_address as *const *const usize) };
    if vtable.is_null() || (vtable as usize) % std::mem::align_of::<usize>() != 0 {
        return Some((
            None,
            velocity,
            acceleration,
            angular_velocity,
            velocity_last_frame,
        ));
    }
    let slot_count = GET_TRANSFORM_VTABLE_SLOT.checked_add(1)?;
    let vtable_bytes = slot_count.checked_mul(size_of::<usize>())?;
    if !readable_span(vtable as usize, vtable_bytes) {
        return Some((
            None,
            velocity,
            acceleration,
            angular_velocity,
            velocity_last_frame,
        ));
    }

    let target = unsafe { ptr::read_volatile(vtable.wrapping_add(GET_TRANSFORM_VTABLE_SLOT)) };
    if target == 0 || !is_readable_executable_page(target) {
        return Some((
            None,
            velocity,
            acceleration,
            angular_velocity,
            velocity_last_frame,
        ));
    }

    let get_transform: GetTransform = unsafe { std::mem::transmute(target) };
    let mut transform = Matrix4x4 {
        values: [[0.0; 4]; 4],
    };
    if !unsafe { get_transform(game_object_address as *mut GameObject, &mut transform) } {
        return Some((
            None,
            velocity,
            acceleration,
            angular_velocity,
            velocity_last_frame,
        ));
    }

    Some((
        Some(transform),
        velocity,
        acceleration,
        angular_velocity,
        velocity_last_frame,
    ))
}

fn read_vehicle_vector(vehicle: *mut Vehicle, slot: usize) -> Option<[f32; 3]> {
    let target = vehicle_virtual_function(vehicle, slot)?;
    let get_vector: GetVehicleVector = unsafe { std::mem::transmute(target) };
    let mut vector = [0.0; 3];
    unsafe { get_vector(vehicle, &mut vector) };
    vector
        .iter()
        .all(|component| component.is_finite())
        .then_some(vector)
}

fn read_vehicle_last_frame_vector(vehicle: *mut Vehicle, slot: usize) -> Option<[f32; 3]> {
    let target = vehicle_virtual_function(vehicle, slot)?;
    let get_vector: GetVehicleVectorPointer = unsafe { std::mem::transmute(target) };
    let vector_pointer = unsafe { get_vector(vehicle) };
    if vector_pointer.is_null() || !readable_span(vector_pointer as usize, size_of::<[f32; 3]>()) {
        return None;
    }

    let vector = unsafe { ptr::read_volatile(vector_pointer) };
    vector
        .iter()
        .all(|component| component.is_finite())
        .then_some(vector)
}

fn vehicle_virtual_function(vehicle: *mut Vehicle, slot: usize) -> Option<usize> {
    let vehicle_address = vehicle as usize;
    if vehicle.is_null()
        || vehicle_address % std::mem::align_of::<*const usize>() != 0
        || !readable_span(vehicle_address, size_of::<*const usize>())
    {
        return None;
    }

    let vtable = unsafe { ptr::read_volatile(vehicle as *const *const usize) };
    if vtable.is_null() || (vtable as usize) % std::mem::align_of::<usize>() != 0 {
        return None;
    }
    let slot_count = slot.checked_add(1)?;
    let vtable_bytes = slot_count.checked_mul(size_of::<usize>())?;
    if !readable_span(vtable as usize, vtable_bytes) {
        return None;
    }

    let target = unsafe { ptr::read_volatile(vtable.wrapping_add(slot)) };
    if target == 0 || !is_readable_executable_page(target) {
        return None;
    }
    Some(target)
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

fn horizontal_forward(matrix: &Matrix4x4) -> Option<[f32; 3]> {
    // The live transformed-axes diagnostic showed actor GetTransform matrices
    // use row 0 = right, row 1 = up, and row 2 = backward. This differs from
    // the generic CMatrix helper names in the community SDK. Actor/vehicle
    // forward is therefore the negated horizontal projection of row 2.
    let backward_x = matrix.values[2][0];
    let backward_z = matrix.values[2][2];
    let magnitude = (backward_x * backward_x + backward_z * backward_z).sqrt();
    (backward_x.is_finite()
        && backward_z.is_finite()
        && magnitude.is_finite()
        && magnitude > 1.0e-6)
        .then_some([-backward_x / magnitude, 0.0, -backward_z / magnitude])
}

fn sample_camera_matrix() -> Option<Matrix4x4> {
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
    unsafe { get_camera_matrix(manager.cast::<CameraControlManager>(), &mut camera_matrix) };
    camera_matrix
        .values
        .iter()
        .flatten()
        .all(|value| value.is_finite())
        .then_some(camera_matrix)
}

fn sample_reticle_ray_if_due(
    camera_matrix: &Matrix4x4,
    player: *mut Character,
) -> Option<ReticleRaySample> {
    let now = Instant::now();
    {
        let mut last_sample = LAST_RETICLE_RAY_SAMPLE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if last_sample.is_some_and(|last| now.duration_since(last) < RETICLE_RAY_SAMPLE_INTERVAL) {
            return None;
        }
        *last_sample = Some(now);
    }

    let Some(ray) = WorldQueryRayInput::from_camera_matrix(camera_matrix) else {
        return Some(ReticleRaySample {
            origin: None,
            direction: None,
            query_available: false,
            query_state_present: false,
            fraction: None,
            candidate_position: None,
            candidate_shape: None,
            player_aim_query_available: false,
            player_aim_query_state_present: false,
            player_aim_fraction: None,
            player_aim_candidate_position: None,
            player_aim_candidate_shape: None,
            ignore_characters_vehicles_query_available: false,
            ignore_characters_vehicles_query_state_present: false,
            ignore_characters_vehicles_fraction: None,
            ignore_characters_vehicles_candidate_position: None,
            ignore_characters_vehicles_candidate_shape: None,
            ignore_player_owner_query_available: false,
            ignore_player_owner_query_state_present: false,
            ignore_player_owner_fraction: None,
            ignore_player_owner_candidate_position: None,
            ignore_player_owner_candidate_shape: None,
        });
    };
    let origin = [ray.origin.x, ray.origin.y, ray.origin.z];
    let direction = [ray.direction.x, ray.direction.y, ray.direction.z];
    let mut sample = ReticleRaySample {
        origin: Some(origin),
        direction: Some(direction),
        query_available: false,
        query_state_present: false,
        fraction: None,
        candidate_position: None,
        candidate_shape: None,
        player_aim_query_available: false,
        player_aim_query_state_present: false,
        player_aim_fraction: None,
        player_aim_candidate_position: None,
        player_aim_candidate_shape: None,
        ignore_characters_vehicles_query_available: false,
        ignore_characters_vehicles_query_state_present: false,
        ignore_characters_vehicles_fraction: None,
        ignore_characters_vehicles_candidate_position: None,
        ignore_characters_vehicles_candidate_shape: None,
        ignore_player_owner_query_available: false,
        ignore_player_owner_query_state_present: false,
        ignore_player_owner_fraction: None,
        ignore_player_owner_candidate_position: None,
        ignore_player_owner_candidate_shape: None,
    };

    let module = unsafe { GetModuleHandleW(hudhook::windows::core::PCWSTR::null()) };
    let Ok(module) = module else {
        return Some(sample);
    };
    let image_base = module.0 as usize;
    if image_base != STEAM_IMAGE_BASE {
        return Some(sample);
    }
    let Some(initializer_address) = image_base.checked_add(WORLD_QUERY_CONTEXT_INITIALIZER_RVA)
    else {
        return Some(sample);
    };
    let Some(option_setter_address) = image_base.checked_add(RAYCAST_FILTER_SET_OPTION_RVA) else {
        return Some(sample);
    };
    let Some(query_address) = image_base.checked_add(WORLD_QUERY_HELPER_RVA) else {
        return Some(sample);
    };
    let Some(filter_vtable) = image_base.checked_add(C_STATIC_ONLY_RAYCAST_FILTER_VFTABLE_RVA)
    else {
        return Some(sample);
    };
    if ![initializer_address, option_setter_address, query_address]
        .into_iter()
        .all(is_readable_executable_page)
    {
        return Some(sample);
    }

    let Some(query_base_global) = image_base.checked_add(WORLD_QUERY_BASE_POINTER_RVA) else {
        return Some(sample);
    };
    if !readable_span(query_base_global, size_of::<*mut c_void>()) {
        return Some(sample);
    }
    let query_base = unsafe { ptr::read_volatile(query_base_global as *const *mut c_void) };
    if query_base.is_null() || !readable_span(query_base as usize, 1) {
        return Some(sample);
    }

    let mut context = RaycastFilterContextStorage([0; 0xD0]);
    let context_pointer = context.0.as_mut_ptr().cast::<c_void>();
    let initialize: WorldQueryContextInitializeFn =
        unsafe { std::mem::transmute(initializer_address) };
    let initialized_context = unsafe { initialize(context_pointer, 0, 0, 0) };
    if initialized_context.is_null() {
        return Some(sample);
    }
    unsafe {
        ptr::write_unaligned(context_pointer.cast::<usize>(), filter_vtable);
        ptr::write_unaligned(
            context
                .0
                .as_mut_ptr()
                .add(C_STATIC_ONLY_FILTER_CATEGORY_MASK_OFFSET)
                .cast::<u32>(),
            3,
        );
    }
    let set_option: RaycastFilterSetOptionFn =
        unsafe { std::mem::transmute(option_setter_address) };
    unsafe { set_option(context_pointer, 1) };

    let mut output = WorldQueryOutputRecord {
        qword_00: 0,
        vector_08: Vector3::default(),
        scalar_14: 1.0,
        qword_18: 0,
        qword_20: 0,
        qword_28: 0,
        words_30: [0; 8],
    };
    let query: WorldQueryHelperFn = unsafe { std::mem::transmute(query_address) };
    let label = b"MadMaxModdingReticleRay\0";
    let state_present = unsafe {
        query(
            query_base,
            label.as_ptr().cast(),
            &ray,
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
    sample.query_available = true;
    sample.query_state_present = state_present;
    sample.candidate_shape = read_query_candidate_shape(&context);
    if output.scalar_14.is_finite() && (0.0..=1.0).contains(&output.scalar_14) {
        sample.fraction = Some(output.scalar_14);
        if output.scalar_14 < 1.0 {
            if let Some(point) = ray.point_at(RETICLE_RAY_MAX_DISTANCE * output.scalar_14) {
                sample.candidate_position = Some([point.x, point.y, point.z]);
            }
        }
    }

    // Repeat the same geometric query with the game's distinct player-aim filter.
    // Its candidate predicate explicitly rejects the current player character and
    // does not consume the static-only category mask at context +0x90.
    let mut player_aim_context = RaycastFilterContextStorage([0; 0xD0]);
    let player_aim_context_pointer = player_aim_context.0.as_mut_ptr().cast::<c_void>();
    let initialized_player_aim_context = unsafe { initialize(player_aim_context_pointer, 0, 0, 0) };
    let Some(player_aim_filter_vtable) =
        image_base.checked_add(C_PLAYER_AIM_RAYCAST_FILTER_VFTABLE_RVA)
    else {
        return Some(sample);
    };
    if !initialized_player_aim_context.is_null() {
        unsafe {
            ptr::write_unaligned(
                player_aim_context_pointer.cast::<usize>(),
                player_aim_filter_vtable,
            );
        }
        unsafe { set_option(player_aim_context_pointer, 1) };
        let mut player_aim_output = WorldQueryOutputRecord {
            qword_00: 0,
            vector_08: Vector3::default(),
            scalar_14: 1.0,
            qword_18: 0,
            qword_20: 0,
            qword_28: 0,
            words_30: [0; 8],
        };
        let player_aim_state_present = unsafe {
            query(
                query_base,
                label.as_ptr().cast(),
                &ray,
                0.0,
                RETICLE_RAY_MAX_DISTANCE,
                &mut player_aim_output,
                player_aim_context_pointer,
                0,
                false,
                0,
                ptr::null(),
            )
        };
        sample.player_aim_query_available = true;
        sample.player_aim_query_state_present = player_aim_state_present;
        sample.player_aim_candidate_shape = read_query_candidate_shape(&player_aim_context);
        if player_aim_output.scalar_14.is_finite()
            && (0.0..=1.0).contains(&player_aim_output.scalar_14)
        {
            sample.player_aim_fraction = Some(player_aim_output.scalar_14);
            if player_aim_output.scalar_14 < 1.0 {
                if let Some(point) =
                    ray.point_at(RETICLE_RAY_MAX_DISTANCE * player_aim_output.scalar_14)
                {
                    sample.player_aim_candidate_position = Some([point.x, point.y, point.z]);
                }
            }
        }
    }

    let Some(ignore_characters_vehicles_filter_vtable) =
        image_base.checked_add(C_IGNORE_CHARACTERS_AND_VEHICLES_RAYCAST_FILTER_VFTABLE_RVA)
    else {
        return Some(sample);
    };
    let mut ignore_characters_vehicles_context = RaycastFilterContextStorage([0; 0xD0]);
    let ignore_characters_vehicles_context_pointer = ignore_characters_vehicles_context
        .0
        .as_mut_ptr()
        .cast::<c_void>();
    let initialized_ignore_characters_vehicles_context =
        unsafe { initialize(ignore_characters_vehicles_context_pointer, 0, 0, 0) };
    if !initialized_ignore_characters_vehicles_context.is_null() {
        unsafe {
            ptr::write_unaligned(
                ignore_characters_vehicles_context_pointer.cast::<usize>(),
                ignore_characters_vehicles_filter_vtable,
            );
        }
        unsafe { set_option(ignore_characters_vehicles_context_pointer, 1) };
        let mut ignore_characters_vehicles_output = WorldQueryOutputRecord {
            qword_00: 0,
            vector_08: Vector3::default(),
            scalar_14: 1.0,
            qword_18: 0,
            qword_20: 0,
            qword_28: 0,
            words_30: [0; 8],
        };
        let ignore_characters_vehicles_label = b"MadMaxModdingReticleIgnoreCharsVehicles\0";
        let ignore_characters_vehicles_state_present = unsafe {
            query(
                query_base,
                ignore_characters_vehicles_label.as_ptr().cast(),
                &ray,
                0.0,
                RETICLE_RAY_MAX_DISTANCE,
                &mut ignore_characters_vehicles_output,
                ignore_characters_vehicles_context_pointer,
                0,
                false,
                0,
                ptr::null(),
            )
        };
        sample.ignore_characters_vehicles_query_available = true;
        sample.ignore_characters_vehicles_query_state_present =
            ignore_characters_vehicles_state_present;
        sample.ignore_characters_vehicles_candidate_shape =
            read_query_candidate_shape(&ignore_characters_vehicles_context);
        if ignore_characters_vehicles_output.scalar_14.is_finite()
            && (0.0..=1.0).contains(&ignore_characters_vehicles_output.scalar_14)
        {
            sample.ignore_characters_vehicles_fraction =
                Some(ignore_characters_vehicles_output.scalar_14);
            if ignore_characters_vehicles_output.scalar_14 < 1.0 {
                if let Some(point) = ray.point_at(
                    RETICLE_RAY_MAX_DISTANCE * ignore_characters_vehicles_output.scalar_14,
                ) {
                    sample.ignore_characters_vehicles_candidate_position =
                        Some([point.x, point.y, point.z]);
                }
            }
        }
    }

    // Repeat the same geometric query with a CIgnoreFilter owner list that
    // contains only the current player object. This isolates owner exclusion
    // from the PlayerAim policy and keeps the shared filter-info exclusion
    // and required masks identical.
    let Some(ignore_filter_vtable) = image_base.checked_add(C_IGNORE_FILTER_VFTABLE_RVA) else {
        return Some(sample);
    };
    if !player.is_null() && readable_span(player as usize, size_of::<usize>()) {
        let mut ignore_player_context = RaycastFilterContextStorage([0; 0xD0]);
        let ignore_player_context_pointer = ignore_player_context.0.as_mut_ptr().cast::<c_void>();
        let initialized_ignore_player_context =
            unsafe { initialize(ignore_player_context_pointer, 0, 0, 0) };
        if !initialized_ignore_player_context.is_null() {
            let ignored_owner = player as usize;
            let ignored_owners = [ignored_owner];
            let owner_begin = ignored_owners.as_ptr() as usize;
            let owner_end = owner_begin + size_of::<usize>();
            unsafe {
                ptr::write_unaligned(
                    ignore_player_context_pointer.cast::<usize>(),
                    ignore_filter_vtable,
                );
                ptr::write_unaligned(
                    ignore_player_context
                        .0
                        .as_mut_ptr()
                        .add(C_IGNORE_FILTER_OWNER_VECTOR_BEGIN_OFFSET)
                        .cast::<usize>(),
                    owner_begin,
                );
                ptr::write_unaligned(
                    ignore_player_context
                        .0
                        .as_mut_ptr()
                        .add(C_IGNORE_FILTER_OWNER_VECTOR_END_OFFSET)
                        .cast::<usize>(),
                    owner_end,
                );
                ptr::write_unaligned(
                    ignore_player_context
                        .0
                        .as_mut_ptr()
                        .add(C_IGNORE_FILTER_OWNER_VECTOR_CAPACITY_OFFSET)
                        .cast::<usize>(),
                    owner_end,
                );
            }
            unsafe { set_option(ignore_player_context_pointer, 1) };
            let mut ignore_player_output = WorldQueryOutputRecord {
                qword_00: 0,
                vector_08: Vector3::default(),
                scalar_14: 1.0,
                qword_18: 0,
                qword_20: 0,
                qword_28: 0,
                words_30: [0; 8],
            };
            let ignore_player_label = b"MadMaxModdingReticleIgnorePlayerOwner\0";
            let ignore_player_state_present = unsafe {
                query(
                    query_base,
                    ignore_player_label.as_ptr().cast(),
                    &ray,
                    0.0,
                    RETICLE_RAY_MAX_DISTANCE,
                    &mut ignore_player_output,
                    ignore_player_context_pointer,
                    0,
                    false,
                    0,
                    ptr::null(),
                )
            };
            sample.ignore_player_owner_query_available = true;
            sample.ignore_player_owner_query_state_present = ignore_player_state_present;
            sample.ignore_player_owner_candidate_shape =
                read_query_candidate_shape(&ignore_player_context);
            if ignore_player_output.scalar_14.is_finite()
                && (0.0..=1.0).contains(&ignore_player_output.scalar_14)
            {
                sample.ignore_player_owner_fraction = Some(ignore_player_output.scalar_14);
                if ignore_player_output.scalar_14 < 1.0 {
                    if let Some(point) =
                        ray.point_at(RETICLE_RAY_MAX_DISTANCE * ignore_player_output.scalar_14)
                    {
                        sample.ignore_player_owner_candidate_position =
                            Some([point.x, point.y, point.z]);
                    }
                }
            }
        }
    }

    Some(sample)
}

fn read_query_candidate_shape(
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

#[cfg(test)]
mod actor_transform_tests {
    use super::horizontal_forward;
    use crate::sdk::types::Matrix4x4;

    #[test]
    fn actor_forward_uses_negative_third_basis_not_world_up() {
        let transform = Matrix4x4::from_rows([
            [1.0, 0.0, 0.0, 0.0],  // right
            [0.0, 1.0, 0.0, 0.0],  // up in the observed actor transform
            [0.6, 0.2, -0.8, 0.0], // pitched backward basis
            [10.0, 20.0, 30.0, 1.0],
        ]);

        assert_eq!(horizontal_forward(&transform), Some([-0.6, 0.0, 0.8]));
    }
}
