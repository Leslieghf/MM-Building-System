//! Build-specific native hooks and copied game telemetry.
//! Hook callbacks capture observations; runtime systems consume owned snapshots.
mod config;
mod memory;
mod runtime_loop;
mod sampling;
mod sampling_hooks;
mod telemetry;
mod world_query_capture;
mod world_query_hooks;

use config::*;
use memory::*;
use runtime_loop::*;
use sampling::*;
use sampling_hooks::*;
use telemetry::*;
use world_query_capture::*;
use world_query_hooks::*;

use crate::native::logging::append_log;
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
