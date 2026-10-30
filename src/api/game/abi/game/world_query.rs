//! Raw binary-derived declarations for Mad Max's world ray-query path.
//!
//! These declarations describe the observed Steam 1.0.3.0 x64 ABI. They are
//! not safe game APIs: do not call the function pointers or construct native
//! query state until object ownership, thread phase, and caller-specific
//! semantics are established. The source SDK does not declare these symbols.

use core::ffi::{c_char, c_void};

use crate::api::game::abi::types::{Matrix4x4, Vector3};

/// RVA of `WorldQueryHelper` in the inspected Steam 1.0.3.0 executable image.
pub const WORLD_QUERY_HELPER_RVA: usize = 0x0252_EDF0;
/// RVA of the initialized runtime global that provides the world-query base pointer.
/// The on-disk image contains zero here; read it only after game initialization.
pub const WORLD_QUERY_BASE_POINTER_RVA: usize = 0x017F_4F08;
/// Runtime-observed base query-context initializer used by camera and terrain callers.
pub const WORLD_QUERY_CONTEXT_INITIALIZER_RVA: usize = 0x024E_5AA0;
/// Runtime-observed filter option setter called by camera static-only and
/// player-aim query contexts. It writes the shared filter-info exclusion mask
/// at context+0x70, using the global DWORD and fixed 0x100 bit when enabled.
pub const RAYCAST_FILTER_SET_OPTION_RVA: usize = 0x0202_A360;
/// Global filter-info exclusion mask read by SetRaycastFilterOption before
/// writing context+0x70.
pub const RAYCAST_FILTER_OPTION_GLOBAL_MASK_RVA: usize = 0x017A_ABDC;
/// On-disk DWORD initializer at the mask global. Runtime value has not been checked.
pub const RAYCAST_FILTER_OPTION_GLOBAL_MASK_ON_DISK: u32 = 0x20;
/// SetRaycastFilterOption ORs this exclusion bit into the global mask when
/// enabled; when disabled it writes this exclusion bit alone.
pub const RAYCAST_FILTER_OPTION_ENABLE_BIT: u32 = 0x100;
/// RTTI-confirmed `CStaticOnlyRaycastFilter` vftable used by the camera query path.
/// Its candidate predicate is at slot +0x10; context+0x90 stores a category mask.
pub const C_STATIC_ONLY_RAYCAST_FILTER_VFTABLE_RVA: usize = 0x0123_D6A8;
/// RTTI-confirmed `CCharacterRaycastFilter` vftable installed by HeightOverGround.
/// Its +0x08 entry is a deleting destructor; candidate testing is at +0x10.
pub const C_CHARACTER_RAYCAST_FILTER_VFTABLE_RVA: usize = 0x0123_8388;
/// The base initializer writes the first 0x90 bytes of query context storage.
/// HeightOverGround additionally places its owner pointer at offset 0x90 in the
/// adjacent stack slot; this is not a generic context constructor contract.
pub const OBSERVED_WORLD_QUERY_CONTEXT_INITIALIZED_SIZE: usize = 0x90;
/// HeightOverGround local storage including the owner pointer consumed at +0x90.
pub const HEIGHT_OVER_GROUND_FILTER_STORAGE_SIZE: usize = 0x98;
pub const HEIGHT_OVER_GROUND_FILTER_OWNER_POINTER_OFFSET: usize = 0x90;
/// RVA of the endpoint bounds predicate called by `WorldQueryHelper`.
pub const CHECK_RAY_BOUNDS_RVA: usize = 0x0252_67E0;
/// RVA of the dispatcher called after both endpoints pass bounds checks.
pub const DISPATCH_RAY_CAST_QUERY_RVA: usize = 0x02B0_DDE0;
/// RTTI-confirmed hkp3AxisSweep vftable address point in the executable.
pub const HKP_3_AXIS_SWEEP_VFTABLE_RVA: usize = 0x013E_0A18;
/// Runtime-captured hkp3AxisSweep virtual at vftable slot +0xC8, the broadphase
/// ray-cast traversal. It enumerates candidate handles and calls the supplied
/// collector; this internal method is not a mod-callable raycast API.
pub const HKP_3_AXIS_SWEEP_BROADPHASE_CAST_RAY_RVA: usize = 0x02B2_CD40;
/// Captured per-candidate narrowphase callback reached from the broadphase
/// collector. It runs a game candidate predicate, transforms ray endpoints to
/// candidate-local coordinates, and dispatches candidate shape testing.
pub const PROCESS_GENERIC_RAY_CANDIDATE_RVA: usize = 0x02B0_DC60;
/// Captured segment-versus-local-sphere early test used during candidate
/// traversal. Its candidate radius is read at +0x20; not a standalone API.
pub const RAY_TEST_CANDIDATE_LOCAL_SPHERE_RVA: usize = 0x02B0_E7D0;
/// Captured shape adapter that invokes shape virtual +0x28, transforms a hit
/// into world space, and calls the supplied collector's slot +0.
pub const TEST_RAY_AGAINST_SHAPE_AND_REPORT_HIT_RVA: usize = 0x02B1_3810;
/// RTTI Complete Object Locator for the hkp3AxisSweep vftable; its type
/// descriptor names .?AVhkp3AxisSweep@@.
pub const HKP_3_AXIS_SWEEP_VFTABLE_COL_RVA: usize = 0x0150_48B0;
/// Observed `hkpBroadPhase::hkpCastRayInput` field offsets from the stack
/// block assembled immediately before the hkp3AxisSweep virtual call.
pub const HKP_CAST_RAY_INPUT_FROM_OFFSET: usize = 0x00;
pub const HKP_CAST_RAY_INPUT_NUM_CASTS_OFFSET: usize = 0x10;
pub const HKP_CAST_RAY_INPUT_TO_BASE_OFFSET: usize = 0x18;
pub const HKP_CAST_RAY_INPUT_TO_STRIDE_OFFSET: usize = 0x20;
pub const HKP_CAST_RAY_INPUT_AABB_CACHE_OFFSET: usize = 0x28;
pub const HKP_CAST_RAY_INPUT_SIZE: usize = 0x30;
/// RVA of the RTTI-identified `hkProxyRayCaster` vftable in this executable.
pub const HK_PROXY_RAY_CASTER_VFTABLE_RVA: usize = 0x0132_BCE0;
/// RVA of the RTTI-confirmed `CCustomSampledHeightFieldShape` vftable observed
/// on a world-query broadphase candidate. Generic shape ray dispatch invokes
/// virtual slot +0x28; the separate +0x30 wrapper is also captured below.
pub const C_CUSTOM_SAMPLED_HEIGHT_FIELD_SHAPE_VFTABLE_RVA: usize = 0x0132_5C18;
/// Runtime target of sampled-height-field vtable slot +0x28, called by the
/// generic `CheckRayShapeIntersection` path. This wrapper forwards to
/// `SAMPLED_HEIGHT_FIELD_RAYCAST_DISPATCH_RVA`.
pub const SAMPLED_HEIGHT_FIELD_RAYCAST_SLOT_RVA: usize = 0x024F_6800;
/// Runtime target of the separate vtable slot +0x30 wrapper. It also reaches
/// the heightfield helper, but its argument setup differs from the generic
/// +0x28 cast entry and its caller-specific ABI remains unresolved.
pub const SAMPLED_HEIGHT_FIELD_SECONDARY_RAYCAST_WRAPPER_RVA: usize = 0x024F_6880;
/// Runtime target called by both sampled-height-field wrappers. It copies ray
/// vectors, calls the segment kernel, and forwards successful hit data to a
/// ray-hit collector.
pub const SAMPLED_HEIGHT_FIELD_RAYCAST_DISPATCH_RVA: usize = 0x024E_E900;
/// Runtime target called by the dispatch wrapper. It tests a segment against
/// sampled height-field data and writes seven raw float words to its output.
pub const SAMPLED_HEIGHT_FIELD_RAYCAST_KERNEL_RVA: usize = 0x0246_7FD0;
/// Internal helper reached by the segment kernel. A live code snapshot shows
/// it constructing traversal state from the two endpoints, clipping/limiting
/// the segment, mapping X/Z to grid coordinates, and initializing grid steps.
/// Its scratch layout is not recovered; this is not a callable mod API.
pub const BUILD_HEIGHT_TRAVERSAL_STATE_RVA: usize = 0x0245_CDA0;
/// Internal helper reached by the segment kernel to look up sampled
/// height-field tiles and calculate raw point-query outputs. Its output
/// meanings remain partial; this is not a standalone raycast API.
pub const CALCULATE_SAMPLED_HEIGHT_FIELD_POINT_RVA: usize = 0x0245_C900;
/// Internal interpolation routine used by the sampled-height-field point
/// evaluator. It calculates point sample values and derivative-like outputs;
/// field semantics and native state are not exposed here.
pub const CALCULATE_HEIGHT_FIELD_POINT_SAMPLE_RVA: usize = 0x029E_00F0;
/// Internal helper that maintains a tile-indexed cache of sampled-height-field
/// data before calculating point outputs. Its cache layout is only partially
/// understood; this is not a callable mod API.
pub const GET_HEIGHT_FIELD_CACHED_TILE_SAMPLE_RVA: usize = 0x029E_03B0;
/// Internal initializer for the sampled-height-field grid walk. It derives
/// per-axis stepping from the segment; the scratch-state layout is unknown.
pub const INITIALIZE_HEIGHT_FIELD_GRID_TRAVERSAL_RVA: usize = 0x0244_AFF0;
/// Internal one-cell advancement routine for the grid traversal state.
pub const ADVANCE_HEIGHT_FIELD_GRID_TRAVERSAL_RVA: usize = 0x0245_D5C0;
/// Internal tile-key/data and vertical-range check used by the grid walk.
pub const CHECK_HEIGHT_FIELD_TRAVERSAL_TILE_RVA: usize = 0x0245_D890;
/// Internal neighboring-height sampler used to locate a layer overlapping
/// the current ray interval.
pub const FIND_HEIGHT_FIELD_TRAVERSAL_LAYER_RVA: usize = 0x0245_D960;
/// Internal sampled-height-field grid traversal loop; not a world-query API.
pub const TRACE_HEIGHT_FIELD_GRID_SEGMENT_RVA: usize = 0x0245_DCB0;
/// Internal initialization of ray samples and interpolation bounds.
pub const INITIALIZE_HEIGHT_FIELD_RAY_SAMPLE_RVA: usize = 0x0244_B590;
/// Internal helper that derives grid-cell bounds for the ray traversal state.
pub const CALCULATE_HEIGHT_FIELD_TRAVERSAL_BOUNDS_RVA: usize = 0x0244_B420;
/// Internal candidate-intersection check that updates hit-distance and
/// surface-gradient-like state on success.
pub const CHECK_HEIGHT_FIELD_SEGMENT_INTERSECTION_RVA: usize = 0x0244_B7F0;
/// Vector helper used to normalize the sampled-height-field surface gradient.
pub const CALCULATE_NORMALIZED_VECTOR3_RVA: usize = 0x01BB_0770;
/// Runtime target of sampled-height-field vtable slot +0x80; the captured
/// function body returns 0xFFFFFFFF and then returns. Meaning at its callsite
/// remains unresolved.
pub const SAMPLED_HEIGHT_FIELD_PROXY_QUERY_SLOT_RVA: usize = 0x02B1_B0A0;
/// RVA of the RTTI-confirmed `CIgnoreFilter` primary vftable observed in the
/// `wind_occlusion` caller's query-context area.
pub const C_IGNORE_FILTER_VFTABLE_RVA: usize = 0x0125_3B20;
/// Runtime-confirmed candidate predicate at CIgnoreFilter vtable slot +0x10.
pub const C_IGNORE_FILTER_CANDIDATE_TEST_RVA: usize = 0x0205_9ED0;
/// CIgnoreFilter's owner-pointer vector begins after the 0x90-byte
/// CRaycastFilter base. Predicate accepts candidates whose owner is absent.
pub const C_IGNORE_FILTER_OWNER_VECTOR_BEGIN_OFFSET: usize = 0x90;
pub const C_IGNORE_FILTER_OWNER_VECTOR_END_OFFSET: usize = 0x98;
pub const C_IGNORE_FILTER_OWNER_VECTOR_CAPACITY_OFFSET: usize = 0xA0;
/// Observed CIgnoreFilter stack storage size in GetRayWorldPosition callers.
pub const C_IGNORE_FILTER_OBSERVED_STORAGE_SIZE: usize = 0xA8;
/// RVA of the game-owned owner-context segment query. This is a weapon/actor
/// helper with caller-specific range and exclusions, not a generic reticle API.
pub const GET_RAY_WORLD_POSITION_RVA: usize = 0x01E1_BBF0;
/// GetRayWorldPosition reads max distance from *(owner + 0x3E0) + 0x270.
pub const GET_RAY_WORLD_POSITION_RANGE_OWNER_OFFSET: usize = 0x3E0;
pub const GET_RAY_WORLD_POSITION_RANGE_VALUE_OFFSET: usize = 0x270;
/// GetRayWorldPosition's CIgnoreFilter source range is [owner+0x2C8, owner+0x2D0);
/// it also appends owner+0x478 when that pointer is non-null.
pub const GET_RAY_WORLD_POSITION_IGNORE_RANGE_BEGIN_OFFSET: usize = 0x2C8;
pub const GET_RAY_WORLD_POSITION_IGNORE_RANGE_END_OFFSET: usize = 0x2D0;
pub const GET_RAY_WORLD_POSITION_OPTIONAL_IGNORE_OWNER_OFFSET: usize = 0x478;
/// Owner flag byte read from *(owner+0x3E0)+0x2DC by GetRayWorldPosition.
pub const GET_RAY_WORLD_POSITION_FILTER_FLAGS_OFFSET: usize = 0x2DC;
pub const GET_RAY_WORLD_POSITION_FILTER_ENABLE_FLAG: u8 = 0x40;
/// The helper shifts the owner flag left by two to form the CRaycastFilter
/// exclusion mask at context+0x70 (0 or 0x100).
pub const GET_RAY_WORLD_POSITION_FILTER_MASK_SHIFT: u32 = 2;
/// Observed byte offsets used to retrieve query dependencies. These names
/// intentionally remain structural until the owning game object is identified.
pub const WORLD_QUERY_DEPENDENCY_PTR_OFFSET: usize = 0x38;
pub const WORLD_QUERY_BROAD_PHASE_PTR_FROM_DEPENDENCY_OFFSET: usize = 0x88;
pub const WORLD_QUERY_FILTER_PTR_FROM_DEPENDENCY_OFFSET: usize = 0xC0;
/// Byte offset of `hkpBroadPhase::castRay` in the broadphase object's virtual
/// table on Steam Mad Max 1.0.3.0. The observed call passes an input record,
/// a `hkpBroadPhaseCastCollector`, and zero collector stride. Do not call this
/// virtual directly until world ownership, phase, and locking are established.
pub const HKP_BROAD_PHASE_CAST_RAY_VTABLE_OFFSET: usize = 0xC8;
/// The same `castRay` slot expressed as a zero-based pointer-sized vtable index.
pub const HKP_BROAD_PHASE_CAST_RAY_VTABLE_INDEX: usize =
    HKP_BROAD_PHASE_CAST_RAY_VTABLE_OFFSET / core::mem::size_of::<usize>();
/// Observed offsets within the stack-allocated `hkProxyRayCaster` callback
/// state. They are not a complete or aligned native layout.
pub const HK_PROXY_RAY_CASTER_ENDPOINTS_OFFSET: usize = 0x08;
pub const HK_PROXY_RAY_CASTER_FILTER_DATA_OFFSET: usize = 0x10;
pub const HK_PROXY_RAY_CASTER_QUERY_CONTEXT_OFFSET: usize = 0x18;
pub const HK_PROXY_RAY_CASTER_ZEROED_FIELD_OFFSET: usize = 0x20;
pub const HK_PROXY_RAY_CASTER_SECONDARY_FILTER_DATA_OFFSET: usize = 0x58;
/// RVA of `hkProxyRayCaster::addBroadPhaseHandle`, identified through the
/// `hkProxyRayCaster` RTTI vtable's second slot (after its destructor slot).
pub const HK_PROXY_RAY_CASTER_ADD_BROAD_PHASE_HANDLE_RVA: usize = 0x0252_6AA0;
/// RVA of `hkpWorldRayCaster::addBroadPhaseHandle`, identified through the
/// `hkpWorldRayCaster` RTTI vtable's second slot (after its destructor slot).
pub const HKP_WORLD_RAY_CASTER_ADD_BROAD_PHASE_HANDLE_RVA: usize = 0x02B0_DC60;
/// RVA of `hkpSimpleWorldRayCaster::addBroadPhaseHandle`, identified through
/// the class RTTI vtable's second slot.
pub const HKP_SIMPLE_WORLD_RAY_CASTER_ADD_BROAD_PHASE_HANDLE_RVA: usize = 0x0311_E480;
/// RVA of `hkCpuWorldRayCastCollector::addBroadPhaseHandle`, identified
/// through the class RTTI vtable's second slot.
pub const HK_CPU_WORLD_RAY_CAST_COLLECTOR_ADD_BROAD_PHASE_HANDLE_RVA: usize = 0x0312_E950;
/// RVA of `CRayCasterRay::addRayHit`, identified through the class RTTI vtable.
pub const C_RAY_CASTER_RAY_ADD_RAY_HIT_RVA: usize = 0x0207_7540;
/// RTTI-confirmed `CPlayerAimRaycastFilter` vftable; its +0x10 predicate is
/// `C_PLAYER_AIM_RAYCAST_FILTER_CANDIDATE_TEST_RVA`.
pub const C_PLAYER_AIM_RAYCAST_FILTER_VFTABLE_RVA: usize = 0x0123_83C8;
/// Runtime-confirmed deleting destructor at CPlayerAimRaycastFilter vtable +0x08.
pub const C_PLAYER_AIM_RAYCAST_FILTER_SCALAR_DELETING_DTOR_RVA: usize = 0x01C1_25E0;
/// Runtime candidate predicate at CPlayerAimRaycastFilter vtable +0x10.
pub const C_PLAYER_AIM_RAYCAST_FILTER_CANDIDATE_TEST_RVA: usize = 0x0203_9E90;
/// RTTI-confirmed `CStaticOnlyRayHitCollector` vftable. Slot +0 uses the
/// shared `CRaycastFilter::addRayHit`; slot +0x08 is the deleting destructor.
pub const C_STATIC_ONLY_RAY_HIT_COLLECTOR_VFTABLE_RVA: usize = 0x0125_A168;
pub const C_STATIC_ONLY_RAY_HIT_COLLECTOR_SCALAR_DELETING_DTOR_RVA: usize = 0x01D8_3480;
/// RTTI-confirmed `CStaticAndKeyFramedOnlyRayHitCollector` vftable. Slot +0
/// uses the shared `CRaycastFilter::addRayHit`; slot +0x08 is the deleting destructor.
pub const C_STATIC_AND_KEY_FRAMED_ONLY_RAY_HIT_COLLECTOR_VFTABLE_RVA: usize = 0x0127_50F8;
pub const C_STATIC_AND_KEY_FRAMED_ONLY_RAY_HIT_COLLECTOR_SCALAR_DELETING_DTOR_RVA: usize =
    0x01EA_7F70;
/// RTTI-confirmed `CDefaultClosestMaterialRayHitCollector` vftable. Slot +0
/// uses the shared `CRaycastFilter::addRayHit`; slot +0x08 is the deleting destructor.
pub const C_DEFAULT_CLOSEST_MATERIAL_RAY_HIT_COLLECTOR_VFTABLE_RVA: usize = 0x0128_75B0;
pub const C_DEFAULT_CLOSEST_MATERIAL_RAY_HIT_COLLECTOR_SCALAR_DELETING_DTOR_RVA: usize =
    0x01F6_1B00;
/// Runtime-confirmed scalar deleting destructor (vtable +0x08), not a hit callback.
pub const C_CHARACTER_RAYCAST_FILTER_SCALAR_DELETING_DTOR_RVA: usize = 0x01C1_2540;
/// Runtime-confirmed candidate predicate at CCharacterRaycastFilter vtable +0x10.
/// Candidate predicate requires flag 1 or 3, owner != context+0x90, and helper 0x1425232A0 returning zero.
/// Runtime-confirmed deleting destructor at CCCameraCollisionRaycastFilter vtable +0x08.
pub const CC_CAMERA_COLLISION_RAYCAST_FILTER_SCALAR_DELETING_DTOR_RVA: usize = 0x01C1_2590;
/// Runtime candidate predicate at CCCameraCollisionRaycastFilter vtable +0x10.
pub const CC_CAMERA_COLLISION_RAYCAST_FILTER_CANDIDATE_TEST_RVA: usize = 0x0206_6D10;
/// The following older filter callback labels have not been checked against live vtables.
/// Do not use them as callable ABI declarations until their slots are audited.
/// Unverified legacy static label; do not call until its runtime vtable slot is audited.
pub const C_IGNORE_CHARACTERS_RAYCAST_FILTER_ADD_RAY_HIT_RVA: usize = 0x01C7_FBC0;
/// Runtime-confirmed deleting destructor at CStaticOnlyRaycastFilter vtable +0x08.
pub const C_STATIC_ONLY_RAYCAST_FILTER_SCALAR_DELETING_DTOR_RVA: usize = 0x01C7_FC10;
/// Runtime-confirmed candidate predicate at CStaticOnlyRaycastFilter vtable +0x10.
pub const C_STATIC_ONLY_RAYCAST_FILTER_CANDIDATE_TEST_RVA: usize = 0x0203_9DB0;
pub const C_STATIC_ONLY_FILTER_CATEGORY_MASK_OFFSET: usize = 0x90;
/// Base collector callback shared at vtable slot +0x00 by RTTI-identified game
/// raycast filters. The live body applies context masks and updates closest-hit
/// state; its full callback record layout remains partial.
pub const C_RAYCAST_FILTER_ADD_RAY_HIT_RVA: usize = 0x024F_6380;
/// RTTI-confirmed CRaycastFilter vftable; +0x08 is the scalar deleting destructor.
pub const C_RAYCAST_FILTER_VFTABLE_RVA: usize = 0x0132_4960;
pub const C_RAYCAST_FILTER_SCALAR_DELETING_DTOR_RVA: usize = 0x024E_DB70;
/// RTTI-confirmed CIgnoreCharactersRaycastFilter vftable and +0x10 predicate.
pub const C_IGNORE_CHARACTERS_RAYCAST_FILTER_VFTABLE_RVA: usize = 0x0123_D688;
pub const C_IGNORE_CHARACTERS_RAYCAST_FILTER_SCALAR_DELETING_DTOR_RVA: usize = 0x01C7_FBC0;
/// Candidate predicate needs flag 1 or 3 and may then reject by owner-associated tag/interface; numeric flag names remain unknown.
/// RTTI-confirmed CIgnoreCharactersAndVehiclesRaycastFilter vftable and +0x10 predicate.
pub const C_IGNORE_CHARACTERS_AND_VEHICLES_RAYCAST_FILTER_VFTABLE_RVA: usize = 0x012D_38C0;
pub const C_IGNORE_CHARACTERS_AND_VEHICLES_RAYCAST_FILTER_SCALAR_DELETING_DTOR_RVA: usize =
    0x021A_DC20;
/// Candidate predicate rejects category flags 2 or 4; meanings inferred from class name, not independently mapped.
/// RTTI-confirmed CCharacterStaticOnlyRaycastFilter vftable and +0x10 predicate.
pub const C_CHARACTER_STATIC_ONLY_RAYCAST_FILTER_VFTABLE_RVA: usize = 0x012A_73A0;
pub const C_CHARACTER_STATIC_ONLY_RAYCAST_FILTER_SCALAR_DELETING_DTOR_RVA: usize = 0x0203_9BE0;
/// Candidate predicate requires flag 1, owner != context+0x90, and helper 0x1425232A0 returning zero.
/// RTTI-confirmed CVideoModeCameraCollisionRaycastFilter vftable and +0x10 predicate.
pub const C_VIDEO_MODE_CAMERA_COLLISION_RAYCAST_FILTER_VFTABLE_RVA: usize = 0x012E_6430;
pub const C_VIDEO_MODE_CAMERA_COLLISION_RAYCAST_FILTER_SCALAR_DELETING_DTOR_RVA: usize =
    0x0227_A110;
pub const C_VIDEO_MODE_CAMERA_COLLISION_RAYCAST_FILTER_CANDIDATE_TEST_RVA: usize = 0x029D_2C0;
/// RTTI-confirmed foot-placement filter vftable and candidate predicate.
pub const FOOT_PLACEMENT_RAYCAST_FILTER_VFTABLE_RVA: usize = 0x0132_07C8;
pub const FOOT_PLACEMENT_RAYCAST_FILTER_CANDIDATE_TEST_RVA: usize = 0x0249_3670;
/// RVA of Havok's base `hkpRayHitCollector::addRayHit` virtual slot target.
pub const HKP_RAY_HIT_COLLECTOR_ADD_RAY_HIT_RVA: usize = 0x01BF_BBB0;
/// RTTI-confirmed `hkpClosestRayHitCollector` vftable.
pub const HKP_CLOSEST_RAY_HIT_COLLECTOR_VFTABLE_RVA: usize = 0x0132_0790;
/// `hkpClosestRayHitCollector::addRayHit`, called by the shared game filter
/// callback and present at vtable slot +0.
pub const HKP_CLOSEST_RAY_HIT_COLLECTOR_ADD_RAY_HIT_RVA: usize = 0x02B1_4540;
/// Its slot +0x08 target; the captured body is a deleting destructor.
pub const HKP_CLOSEST_RAY_HIT_COLLECTOR_SCALAR_DELETING_DTOR_RVA: usize = 0x0249_F5A0;
/// Registered script-function entry for `RaycastStatic`.
pub const SCRIPT_RAYCAST_STATIC_ENTRY_RVA: usize = 0x01EA_C3F0;
/// Internal body reached by the registered `RaycastStatic` entry.
pub const SCRIPT_RAYCAST_STATIC_BODY_RVA: usize = 0x01EA_C0C0;
/// Return RVA of the body's call into `WorldQueryHelper`.
pub const SCRIPT_RAYCAST_STATIC_QUERY_RETURN_RVA: usize = 0x01EA_C351;
/// Registered `RaycastStaticFromCamera` script callback. The recovered body
/// reads camera-manager coordinates and forwards into the script static-ray
/// query path. This is not an always-running player-aim/view ray hook or a
/// general raycast API.
pub const SCRIPT_RAYCAST_STATIC_FROM_CAMERA_ENTRY_RVA: usize = 0x01EA_C4C0;
/// Camera caller's postprocessor; reconstructs a query point, then checks
/// object bounds. This analysis label is not a general raycast API.
pub const CAMERA_RAY_CANDIDATE_BOUNDS_HANDLER_RVA: usize = 0x01EA_8D60;
/// Analysis label for the camera object's min-inclusive/max-exclusive 3D bounds test.
/// This is a post-query candidate filter, not a world-raycast hit test.
pub const CAMERA_POINT_IN_BOUNDS_TEST_RVA: usize = 0x029A_DE20;

/// ABI input consumed by WorldQueryHelper as `origin + direction * distance`.
///
/// This also represents the geometric camera ray used by a reticle query. The
/// direction is normalized by `from_camera_matrix`; native calls still require
/// a valid game-owned query context and execution phase.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct WorldQueryRayInput {
    pub origin: Vector3,
    pub direction: Vector3,
}

impl WorldQueryRayInput {
    /// Builds a normalized candidate view ray from the camera transform. The
    /// current sign convention negates the SDK's `Forward()` row; gameplay
    /// validation is still needed to confirm it matches the visible view.
    /// Returns `None` for invalid matrices or a degenerate forward vector.
    pub fn from_camera_matrix(matrix: &Matrix4x4) -> Option<Self> {
        let origin = matrix.translation();
        let forward = matrix.forward();
        let magnitude = forward.magnitude();
        if ![
            origin.x, origin.y, origin.z, forward.x, forward.y, forward.z,
        ]
        .into_iter()
        .all(f32::is_finite)
            || !magnitude.is_finite()
            || magnitude <= f32::EPSILON
        {
            return None;
        }

        Some(Self {
            origin,
            direction: forward * (-1.0 / magnitude),
        })
    }

    /// Returns a point along this ray for a finite, nonnegative distance.
    pub fn point_at(self, distance: f32) -> Option<Vector3> {
        (distance.is_finite() && distance >= 0.0).then(|| self.origin + self.direction * distance)
    }
}
/// Seven floats written by the sampled-height-field segment kernel on its hit
/// path. The shape wrapper forwards the first three as the collector normal
/// and the fourth as its hit fraction. The trailing values are not consumed by
/// that wrapper and remain unknown.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SampledHeightFieldRaycastOutput {
    pub surface_normal: Vector3,
    pub hit_fraction: f32,
    pub unknown_trailing_values: [f32; 3],
}

/// Runtime-observed ABI of `SAMPLED_HEIGHT_FIELD_RAYCAST_KERNEL_RVA`.
///
/// This is a raw binding only. The function depends on game-owned sampled
/// height-field state and a caller-specific execution context; do not call it
/// directly until its ownership, phase, and thread requirements are mapped.
pub type SampledHeightFieldRaycastKernelFn = unsafe extern "system" fn(
    from: *const Vector3,
    to: *const Vector3,
    output: *mut SampledHeightFieldRaycastOutput,
    mode: u8,
) -> u8;

/// Raw 0x50-byte output written by `WorldQueryHelper`.
///
/// Field names describe observed storage, not universal gameplay semantics.
/// `scalar_14` is consumed as a segment fraction by the sampled
/// `CAmmunition::CheckCollisions` and `GraphScriptRaycastStatic` callers: values
/// below 1 select caller-specific result handling, and captured values match
/// the minimum candidate fraction. Other callers may assign it different
/// semantics. Pointer fields have no recovered Rust-side pointee types.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WorldQueryOutputRecord {
    pub qword_00: u64,
    pub vector_08: Vector3,
    pub scalar_14: f32,
    pub qword_18: u64,
    pub qword_20: u64,
    pub qword_28: u64,
    pub words_30: [u32; 8],
}

/// Captured stack window length used by current caller investigations.
/// On the ammo `CIgnoreFilter` path, 0xA8 covers its opaque 0x90-byte prefix
/// and observed 0x18-byte vector header. For other callers this remains only
/// a capture length, not a native `sizeof`.
pub const OBSERVED_QUERY_CONTEXT_CAPTURE_BYTES: usize = 0xA8;
/// Context pointer-presence field tested by WorldQueryHelper for its Boolean return.
/// The pointee type and universal physical-hit meaning are unresolved.
pub const WORLD_QUERY_CONTEXT_RESULT_STATE_POINTER_OFFSET: usize = 0x60;
/// Float initialized to 1.0 by InitializeWorldQueryContext and read before/after
/// the shared CRaycastFilter hit-update helper. A strictly smaller value marks
/// a closer accepted candidate and gates updating the accepted-shape pointer.
/// This is the query's closest-hit bound/current fraction, not the output field
/// copied by WorldQueryHelper.
pub const WORLD_QUERY_CONTEXT_CLOSEST_HIT_FRACTION_OFFSET: usize = 0x08;
/// Float initialized to 1.0 and copied by WorldQueryHelper to
/// WorldQueryOutputRecord::scalar_14. Its precise writer and relationship to
/// the closest-hit bound at +0x08 remain unresolved.
pub const WORLD_QUERY_CONTEXT_EXPORTED_FRACTION_OFFSET: usize = 0x20;
/// Accepted leaf-shape pointer written by the shared CRaycastFilter hit
/// callback after a closer accepted candidate is found.
pub const WORLD_QUERY_CONTEXT_ACCEPTED_SHAPE_POINTER_OFFSET: usize = 0x80;

/// Game-owned path that filters a world-ray request and forwards accepted
/// requests to `EnqueueWorldRayQueryRequest`. This is an asynchronous request
/// path candidate, not a direct reticle query; its queue consumer is unresolved.
pub const FILTER_AND_QUEUE_WORLD_RAY_QUERY_RVA: usize = 0x01BB_2C20;
/// Copies one request into the query-system's bounded queue. The consumer and
/// the meaning of each query type are not recovered; do not call as a mod API.
pub const ENQUEUE_WORLD_RAY_QUERY_REQUEST_RVA: usize = 0x01BA_22D0;
/// Observed queue capacity in the enqueue routine's 0x80-entry bound.
pub const WORLD_RAY_QUERY_REQUEST_CAPACITY: usize = 0x80;
/// Observed queue record stride and field offsets in bytes.
pub const WORLD_RAY_QUERY_REQUEST_RECORD_SIZE: usize = 0x20;
pub const WORLD_RAY_QUERY_REQUEST_QUEUE_OFFSET: usize = 0x1040;
pub const WORLD_RAY_QUERY_REQUEST_COUNT_OFFSET: usize = 0x2040;

/// Raw 32-byte record written by `EnqueueWorldRayQueryRequest`.
///
/// `query_type`, `max_distance`, and the copied vector are direct arguments.
/// The two trailing words are copied from unrelated system/global fields; their
/// semantics and the queued vector's role (origin, direction, or another value)
/// remain unresolved. This layout is descriptive, not a supported API.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct WorldRayQueryRequestRecord {
    pub query_type: u32,
    pub max_distance: f32,
    pub result_flags: u32,
    pub unknown_system_counter_plus_one: u32,
    pub vector_10: Vector3,
    pub unknown_global_value_154: u32,
}

/// Raw ABI of the game-owned request filter/queue function. Boolean flag
/// meanings and query-context requirements are unresolved.
pub type FilterAndQueueWorldRayQueryFn = unsafe extern "system" fn(
    query_system: *mut c_void,
    query_type: u32,
    max_distance: f32,
    query_context: *mut c_void,
    filter_flag_a: bool,
    filter_flag_b: bool,
    vector: *const Vector3,
);

/// Raw ABI of the bounded queue writer. `result_flags` is computed by the
/// filtering caller; no result consumer or completion contract is recovered.
pub type EnqueueWorldRayQueryRequestFn = unsafe extern "system" fn(
    query_system: *mut c_void,
    query_type: u32,
    max_distance: f32,
    result_flags: u32,
    vector: *const Vector3,
);

/// Runtime-observed base query-context initializer ABI. Its callers pass zero
/// values, then install a filter-specific vftable; do not treat it as a safe
/// constructor for arbitrary Rust-created contexts.

pub type WorldQueryContextInitializeFn = unsafe extern "system" fn(
    context: *mut c_void,
    exclude_filter_info_mask: u32,
    required_filter_info_mask: u32,
    opaque_78: u64,
) -> *mut c_void;

/// Runtime-observed filter option setter. It writes context+0x70 as the
/// global exclusion mask OR 0x100 when enabled, and 0x100 when disabled. The
/// on-disk global initializer is 0x20; its runtime value/mutability is unverified.
pub type RaycastFilterSetOptionFn = unsafe extern "system" fn(context: *mut c_void, enabled: u8);
/// Runtime ABI of `WorldQueryHelper` recovered from its captured live body and
/// the native camera-script call site. The camera caller uses start distance
/// zero and a caller-derived finite end distance; it passes `option8=0`,
/// `mode_flag=false`, `query_group=0`, and `filter_override=null`.

/// The helper forms a dispatch word as `query_group * 0x10000 + 0x10` when
/// `mode_flag` is true, or `query_group * 0x10000 + 0x12` otherwise; a
/// non-null explicit override replaces that word. `option8` is present in the
/// ABI but is not consumed in the captured helper.

/// Its raw status is true exactly when `query_context + 0x60` is non-null
/// after the call. The pointee meaning is unresolved, so this is not a
/// universal hit flag. The camera caller ignores the Boolean and derives its
/// script result from later fraction and bounds logic. Treat pointers as
/// borrowed with caller-specific lifetimes.
pub type WorldQueryHelperFn = unsafe extern "system" fn(
    query_base: *mut c_void,
    caller_label: *const c_char,
    ray: *const WorldQueryRayInput,
    start_distance: f32,
    end_distance: f32,
    output: *mut WorldQueryOutputRecord,
    query_context: *mut c_void,
    option8: i64,
    mode_flag: bool,
    query_group: i32,
    filter_override: *const i32,
) -> bool;

/// ABI of the helper's endpoint bounds predicate.
pub type CheckRayBoundsFn =
    unsafe extern "system" fn(query_base: *mut c_void, point: *const Vector3, margin: f32) -> bool;

/// ABI of the internal dispatcher. Parameter meanings remain opaque.
pub type DispatchRayCastQueryFn = unsafe extern "system" fn(
    callback_state: *mut c_void,
    broadphase: *mut c_void,
    ray_endpoints: *const Vector3,
    filter_config: *mut c_void,
    option5: i64,
    query_context: *mut c_void,
);

/// ABI shape for the broad-phase virtual used by `DispatchRayCastQuery`.
///
/// The binary snapshot constructs `HkpCastRayInput` with one cast, a 0x10-byte
/// endpoint stride, and the caller's `option5` as the cache-info pointer, then
/// invokes vtable byte offset `0xC8` with collector stride zero. The function
/// is represented for documentation and future validated adapters only; it
/// is not currently safe to call from a mod thread.
pub type HkpBroadPhaseCastRayFn = unsafe extern "system" fn(
    this: *const crate::api::game::abi::game::physics::HkpBroadPhase,
    input: *const crate::api::game::abi::game::physics::HkpCastRayInput,
    collector: *mut crate::api::game::abi::game::physics::HkpBroadPhaseCastCollector,
    collector_stride: i32,
);

/// ABI shape of Havok's shape-level callback ray implementation.
///
/// In the fallback branch of `hkProxyRayCaster::addBroadPhaseHandle`, the
/// binary loads the shape pointer from the candidate's collision-body view
/// and makes an indirect call through shape vtable offset `0x30` with four
/// ABI arguments: shape, a locally transformed ray-input block, the collision
/// body, and the per-ray collector. That argument shape matches the public
/// `hkpShape::castRayWithCollectorImpl` declaration. The game vtable target at
/// this slot has not yet been resolved to a standalone RVA, so this alias is
/// descriptive and must not be used as a fixed-address call.
pub type HkpShapeCastRayWithCollectorImplFn = unsafe extern "system" fn(
    this: *const crate::api::game::abi::game::physics::HkpShape,
    input: *const crate::api::game::abi::game::physics::HkpShapeRayCastInput,
    cd_body: *const crate::api::game::abi::game::physics::HkpCdBody,
    collector: *mut crate::api::game::abi::game::physics::HkpRayHitCollector,
);

/// ABI of Havok's `hkpWorldRayCaster::addBroadPhaseHandle` virtual method.
/// The game binary's RTTI vtable and captured body identify the function; the
/// method signature is cross-checked against the matching Havok SDK API shape.
pub type HkpWorldRayCasterAddBroadPhaseHandleFn = unsafe extern "system" fn(
    this: *mut crate::api::game::abi::game::physics::HkpWorldRayCaster,
    broad_phase_handle: *const crate::api::game::abi::game::physics::HkpBroadPhaseHandle,
    cast_index: i32,
) -> f32;

/// ABI of `hkProxyRayCaster`'s override of
/// `hkpWorldRayCaster::addBroadPhaseHandle`.
pub type HkProxyRayCasterAddBroadPhaseHandleFn = unsafe extern "system" fn(
    this: *mut crate::api::game::abi::game::physics::HkProxyRayCaster,
    broad_phase_handle: *const crate::api::game::abi::game::physics::HkpBroadPhaseHandle,
    cast_index: i32,
) -> f32;

/// ABI shared by the game's other RTTI-identified broad-phase caster callbacks.
pub type HkpSimpleWorldRayCasterAddBroadPhaseHandleFn = unsafe extern "system" fn(
    this: *mut crate::api::game::abi::game::physics::HkpSimpleWorldRayCaster,
    broad_phase_handle: *const crate::api::game::abi::game::physics::HkpBroadPhaseHandle,
    cast_index: i32,
) -> f32;

pub type HkCpuWorldRayCastCollectorAddBroadPhaseHandleFn = unsafe extern "system" fn(
    this: *mut crate::api::game::abi::game::physics::HkCpuWorldRayCastCollector,
    broad_phase_handle: *const crate::api::game::abi::game::physics::HkpBroadPhaseHandle,
    cast_index: i32,
) -> f32;

/// ABI of the game/Havok ray-hit collector callback. The argument shape is
/// cross-checked against the public Havok `hkpClosestRayHitCollector` header;
/// the Mad Max binary vtables identify the class-specific implementations.
pub type CRayCasterRayAddRayHitFn = unsafe extern "system" fn(
    this: *mut crate::api::game::abi::game::physics::CRayCasterRay,
    cd_body: *const crate::api::game::abi::game::physics::HkpCdBody,
    hit_info: *const crate::api::game::abi::game::physics::HkpShapeRayCastCollectorOutput,
);

/// Common ABI for RTTI-identified overrides of Havok's `addRayHit` callback.
pub type RayHitCollectorCallbackFn<T> = unsafe extern "system" fn(
    this: *mut T,
    cd_body: *const crate::api::game::abi::game::physics::HkpCdBody,
    hit_info: *const crate::api::game::abi::game::physics::HkpShapeRayCastCollectorOutput,
);

/// ABI of the observed CCharacterRaycastFilter vtable +0x10 candidate predicate.
/// The candidate object layout is unresolved, so it remains opaque.
pub type CCharacterRaycastFilterCandidateTestFn = unsafe extern "system" fn(
    this: *mut crate::api::game::abi::game::physics::CCharacterRaycastFilter,
    candidate: *mut c_void,
) -> bool;

/// The final two arguments are forwarded to a candidate-classification query;
/// their meanings are unresolved.
pub type CIgnoreCharactersRaycastFilterCandidateTestFn = unsafe extern "system" fn(
    this: *mut crate::api::game::abi::game::physics::CIgnoreCharactersRaycastFilter,
    candidate: *mut c_void,
    unknown_3: usize,
    unknown_4: usize,
) -> bool;

pub type CIgnoreCharactersAndVehiclesCandidateTestFn = unsafe extern "system" fn(
    this: *mut crate::api::game::abi::game::physics::CIgnoreCharactersAndVehiclesRaycastFilter,
    candidate: *mut c_void,
) -> bool;

/// This predicate reads an owner pointer from this+0x90.
pub type CCharacterStaticOnlyRaycastFilterCandidateTestFn = unsafe extern "system" fn(
    this: *mut crate::api::game::abi::game::physics::CCharacterStaticOnlyRaycastFilter,
    candidate: *mut c_void,
) -> bool;

/// Candidate predicate ABI observed for camera-collision and player-aim filters.
pub type CCCameraCollisionRaycastFilterCandidateTestFn = unsafe extern "system" fn(
    this: *mut crate::api::game::abi::game::physics::CCCameraCollisionRaycastFilter,
    candidate: *mut c_void,
) -> bool;
pub type CPlayerAimRaycastFilterCandidateTestFn = unsafe extern "system" fn(
    this: *mut crate::api::game::abi::game::physics::CPlayerAimRaycastFilter,
    candidate: *mut c_void,
) -> bool;

/// ABI of the observed CStaticOnlyRaycastFilter candidate predicate. The last two
/// words are forwarded to candidate classification; their meaning is unresolved.
pub type CStaticOnlyRaycastFilterCandidateTestFn = unsafe extern "system" fn(
    this: *mut crate::api::game::abi::game::physics::CStaticOnlyRaycastFilter,
    candidate: *mut c_void,
    unknown_3: usize,
    unknown_4: usize,
) -> bool;

/// ABI of the base Havok ray-hit callback, with borrowed callback arguments.
pub type HkpRayHitCollectorAddRayHitFn = unsafe extern "system" fn(
    this: *mut crate::api::game::abi::game::physics::HkpRayHitCollector,
    cd_body: *const crate::api::game::abi::game::physics::HkpCdBody,
    hit_info: *const crate::api::game::abi::game::physics::HkpShapeRayCastCollectorOutput,
);

/// ABI of Havok's closest-hit collector callback override.
pub type HkpClosestRayHitCollectorAddRayHitFn = unsafe extern "system" fn(
    this: *mut crate::api::game::abi::game::physics::HkpClosestRayHitCollector,
    cd_body: *const crate::api::game::abi::game::physics::HkpCdBody,
    hit_info: *const crate::api::game::abi::game::physics::HkpShapeRayCastCollectorOutput,
);
