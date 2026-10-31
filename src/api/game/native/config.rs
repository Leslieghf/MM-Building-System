//! Supported executable fingerprint, native addresses, and sampling budgets.
//!
//! Address constants are RVAs relative to the validated Steam image base unless
//! named as a vtable slot. This module centralizes build-specific metadata and
//! sampling intervals; it does not itself validate addresses or authorize
//! native calls. Validation belongs to the adapter and hook installers.
use std::sync::atomic::AtomicBool;
use std::time::Duration;

pub(super) const STEAM_TIMESTAMP: u32 = 0x5628_A448;
pub(super) const STEAM_FILE_SIZE: u64 = 76_814_792;
pub(super) use crate::api::game::abi::build::steam::IMAGE_BASE as STEAM_IMAGE_BASE;
pub(super) const UPDATE_POST_SIM_RVA: usize = 0x01FA_5A10;
// CDebugRenderer facade, its line and auto-width arrow slots, verified in the
// Ghidra project from the captured Steam executable overlays.
pub(super) const CDEBUG_RENDERER_INSTANCE_GLOBAL_RVA: usize = 0x017C_F0E8;
pub(super) const CDEBUG_RENDERER_FACADE_VTABLE_RVA: usize = 0x0134_6938;
pub(super) const CDEBUG_RENDERER_LINE_SLOT: usize = 1;
pub(super) const CDEBUG_RENDERER_ARROW_SLOT: usize = 9;
pub(super) const CDEBUG_RENDERER_AXES_SLOT: usize = 25;
pub(super) const CDEBUG_RENDERER_SPHERE_SLOT: usize = 26;
pub(super) const CDEBUG_RENDERER_SPARSE_SPHERE_SLOT: usize = 27;
pub(super) const CDEBUG_RENDERER_TESSELLATED_SPHERE_SLOT: usize = 28;
pub(super) const CDEBUG_RENDERER_AABB_SLOT: usize = 31;
pub(super) const CDEBUG_RENDERER_TRANSFORMED_AABB_SLOT: usize = 34;
pub(super) const CDEBUG_RENDERER_QUEUED_TRANSFORMED_AABB_SLOT: usize = 35;
// The game renders slot 36 as a solid cube and slot 38 as a solid 3D arrow.
// Slots 37 and 39 are visually redundant with these and are intentionally not exposed.
pub(super) const CDEBUG_RENDERER_SOLID_CUBE_SLOT: usize = 36;
pub(super) const CDEBUG_RENDERER_SOLID_ARROW_SLOT: usize = 38;
pub(super) const CDEBUG_RENDERER_LINE_METHOD_RVA: usize = 0x0297_A1B0;
pub(super) const CDEBUG_RENDERER_ARROW_METHOD_RVA: usize = 0x0297_A420;
pub(super) const CDEBUG_RENDERER_AXES_METHOD_RVA: usize = 0x0297_A940;
pub(super) const CDEBUG_RENDERER_SPHERE_METHOD_RVA: usize = 0x0297_A960;
pub(super) const CDEBUG_RENDERER_SPARSE_SPHERE_METHOD_RVA: usize = 0x0297_A990;
pub(super) const CDEBUG_RENDERER_TESSELLATED_SPHERE_METHOD_RVA: usize = 0x0297_A9D0;
pub(super) const CDEBUG_RENDERER_AABB_METHOD_RVA: usize = 0x0297_AA80;
pub(super) const CDEBUG_RENDERER_TRANSFORMED_AABB_METHOD_RVA: usize = 0x0297_AB30;
pub(super) const CDEBUG_RENDERER_QUEUED_TRANSFORMED_AABB_METHOD_RVA: usize = 0x0297_AB70;
pub(super) const CDEBUG_RENDERER_SOLID_CUBE_METHOD_RVA: usize = 0x0297_ABA0;
pub(super) const CDEBUG_RENDERER_SOLID_ARROW_METHOD_RVA: usize = 0x0297_AC20;
pub(super) const PRIMITIVE_RENDERER_LINE_RVA: usize = 0x028B_BA40;
pub(super) const PRIMITIVE_RENDERER_ARROW_RVA: usize = 0x028B_DF60;
pub(super) const PRIMITIVE_RENDERER_AXES_RVA: usize = 0x028B_79C0;
pub(super) static SCRIPT_DEBUG_DRAW_SUBMISSION_LOGGED: AtomicBool = AtomicBool::new(false);
// Binary-derived callback registered as `RaycastStaticFromCamera` in the
// Steam 1.0.3.0 executable. The tap below only counts and forwards calls.
pub(super) const RAYCAST_STATIC_FROM_CAMERA_RVA: usize = 0x01EA_C4C0;
// Shared lower-level world-query helper reached by the script trace wrapper
// and many gameplay call sites in the supported Steam executable.
pub(super) const WORLD_QUERY_HELPER_RVA: usize = 0x0252_EDF0;
// Point-in-world-bounds predicate called twice by the shared query helper.
pub(super) const WORLD_QUERY_BOUNDS_PREDICATE_RVA: usize = 0x0252_67E0;
pub(super) const WORLD_QUERY_BOUNDS_CHECKS_PER_QUERY: usize = 2;
// hkProxyRayCaster::addBroadPhaseHandle in the supported Steam executable.
pub(super) const WORLD_QUERY_CANDIDATE_CALLBACK_RVA: usize = 0x0252_6AA0;
pub(super) const WORLD_QUERY_CANDIDATE_CAPTURE_CAPACITY: usize = 7;
// RTTI-resolved CCustomSampledHeightFieldShape vtable targets. Their bodies
// are absent from the on-disk .xpdata import, so capture runtime bytes once.
pub(super) const SAMPLED_HEIGHT_FIELD_RAY_CAST_RVA: usize = 0x024F_6880;
pub(super) const SAMPLED_HEIGHT_FIELD_PROXY_QUERY_RVA: usize = 0x02B1_B0A0;
pub(super) const SHAPE_VIRTUAL_CODE_CAPTURE_LENGTH: usize = 96;
// hkpCustomGroupFilter ray-query virtual observed through the world filter's
// subobject vtable at +0x08.
pub(super) const WORLD_QUERY_GROUP_FILTER_RVA: usize = 0x024E_7580;
pub(super) const WORLD_QUERY_RETURN_STACK_CAPACITY: usize = 256;
pub(super) const WORLD_QUERY_NO_SAMPLE: usize = usize::MAX;
pub(super) const WORLD_QUERY_CANDIDATE_STACK_CAPACITY: usize = 16;
pub(super) const RAYCAST_FILTER_HIT_CAPTURE_CAPACITY: usize = 4;
pub(super) const RAYCAST_FILTER_CAPTURE_BROADPHASE_SLOTS: usize = 2;
// RTTI-confirmed CPlayerAimRaycastFilter vtable slot +0x10 candidate test.
// The observer only counts calls and always forwards to the original method.
pub(super) const PLAYER_AIM_FILTER_CANDIDATE_TEST_RVA: usize = 0x0203_9E90;
pub(super) const PLAYER_AIM_FILTER_CANDIDATE_TEST_ENTRY: [u8; 16] = [
    0x57, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0x0D, 0x9C, 0xB4, 0x7B, 0xFF, 0x48, 0x89, 0xD7, 0xE8,
];
// CWeapon::GetRayWorldPosition in the observed Steam executable. The entry
// signature is checked before installing the passive observer.
pub(super) const WEAPON_RAY_POSITION_RVA: usize = 0x01E1_BBF0;
// SDK-derived layout candidates from the pinned mm_sdk declarations; neither
// value is runtime-proven. CPhysicsGameObject lists IPfxGameObject before
// CGameObject, and GetTransform is the final CGameObject virtual.
pub(super) const CHARACTER_TO_GAME_OBJECT_OFFSET: usize = 0x8;
// CVehicle follows the same multiple-inheritance path through CPhysicsGameObject.
pub(super) const VEHICLE_TO_GAME_OBJECT_OFFSET: usize = 0x8;
pub(super) const HOOK_TIMEOUT: Duration = Duration::from_secs(120);
pub(super) const GAME_STATE_SAMPLE_INTERVAL: Duration = Duration::from_millis(50);
pub(super) const RETICLE_RAY_SAMPLE_INTERVAL: Duration = Duration::from_millis(250);
pub(super) const RETICLE_RAY_MAX_DISTANCE: f32 = 500.0;
pub(super) const PLAYER_SAMPLE_MAX_AGE: Duration = Duration::from_secs(1);
pub(super) const CALLBACK_STABLE_STREAK_REQUIRED: u32 = 60;
// The observed helper has dozens of call sites. Keep a fixed, allocation-free
// set on the hook path; a background worker writes newly seen entries to disk.
pub(super) const WORLD_QUERY_CALLER_CAPACITY: usize = 256;
pub(super) const WORLD_QUERY_CALLER_REPORT_INTERVAL: Duration = Duration::from_secs(5);
pub(super) const WORLD_QUERY_CALLER_REPORT_LIMIT: usize = 12;
// Reserve a dedicated region for the camera-script samples: high-rate
// unrelated callers can otherwise consume the shared array before the player
// aims. The remaining region covers the first 320 ammo calls, 8
// HeightOverGround calls, three samples for each of ten important callers,
// three spaced samples for each of two occlusion callers, and one for each
// of the remaining 40 direct sites (404 expected samples), leaving 12 slots.
pub(super) const WORLD_QUERY_CAMERA_SAMPLE_CAPACITY: usize = 160;
pub(super) const WORLD_QUERY_NON_CAMERA_SAMPLE_BASE: usize = WORLD_QUERY_CAMERA_SAMPLE_CAPACITY;
pub(super) const WORLD_QUERY_NON_CAMERA_SAMPLE_CAPACITY: usize = 416;
pub(super) const RETICLE_QUERY_CALLER_CAPACITY: usize = 4;
// The reticle probe runs every 250 ms; these samples span about 18 seconds.
pub(super) const RETICLE_QUERY_SAMPLE_ORDINALS: [u64; 5] = [0, 4, 16, 40, 72];
pub(super) const RETICLE_QUERY_SAMPLE_BASE: usize =
    WORLD_QUERY_NON_CAMERA_SAMPLE_BASE + WORLD_QUERY_NON_CAMERA_SAMPLE_CAPACITY;
pub(super) const WORLD_QUERY_SAMPLE_CAPACITY: usize =
    RETICLE_QUERY_SAMPLE_BASE + RETICLE_QUERY_CALLER_CAPACITY * RETICLE_QUERY_SAMPLE_ORDINALS.len();
pub(super) const WORLD_QUERY_LABEL_CAPACITY: usize = 64;
pub(super) const WEAPON_RAY_SAMPLE_CAPACITY: usize = 64;
pub(super) const AMMO_WORLD_QUERY_CALLER_RVA: usize = 0x01DA_5892;
pub(super) const HEIGHT_OVER_GROUND_WORLD_QUERY_CALLER_RVA: usize = 0x01CA_3731;
pub(super) const SCRIPT_RAYCAST_WORLD_QUERY_CALLER_RVA: usize = 0x01EA_C351;
pub(super) const WORLD_QUERY_REPEAT_CALLERS: [usize; 10] = [
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
pub(super) const WORLD_QUERY_REPEAT_SAMPLE_LIMIT: u32 = 3;
pub(super) const WORLD_QUERY_SPACED_REPEAT_CALLERS: [usize; 2] = [
    0x01FC_0E0E, // wind_occlusion
    0x021D_C95A, // vehicle_sound_occlusion
];
pub(super) const WORLD_QUERY_SPACED_REPEAT_CALL_ORDINALS: [u64; 3] = [0, 256, 2048];

pub(super) const MEM_COMMIT: u32 = 0x1000;
pub(super) const PAGE_READONLY: u32 = 0x02;
pub(super) const PAGE_READWRITE: u32 = 0x04;
pub(super) const PAGE_WRITECOPY: u32 = 0x08;
pub(super) const PAGE_EXECUTE_READ: u32 = 0x20;
pub(super) const PAGE_EXECUTE_READWRITE: u32 = 0x40;
pub(super) const PAGE_EXECUTE_WRITECOPY: u32 = 0x80;
pub(super) const PAGE_GUARD: u32 = 0x100;
pub(super) const PAGE_WRITABLE_MASK: u32 =
    PAGE_READWRITE | PAGE_WRITECOPY | PAGE_EXECUTE_READWRITE | PAGE_EXECUTE_WRITECOPY;
