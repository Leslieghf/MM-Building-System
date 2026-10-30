//! Opaque game/Havok physics types recovered from Mad Max's MSVC RTTI.
//!
//! Most declarations record class identity and inheritance only. The
//! `CIgnoreFilter` declaration includes one observed vector field. These
//! declarations do not imply ownership rules or safe APIs.

/// Havok broad-phase handle passed to a ray-caster candidate callback.
#[repr(C)]
pub struct HkpBroadPhaseHandle {
    _opaque: [u8; 0],
}

/// Havok broad-phase query interface. Mad Max calls a ray-query virtual on
/// this interface from its world-query dispatcher; no instance layout is
/// modeled here.
#[repr(C)]
pub struct HkpBroadPhase {
    _opaque: [u8; 0],
}

/// Havok broad-phase cast callback base class.
#[repr(C)]
pub struct HkpBroadPhaseCastCollector {
    _opaque: [u8; 0],
}

/// Input record passed to `hkpBroadPhase::castRay`.
///
/// The 0x30-byte field layout is recovered from the game's dispatcher stack
/// initialization and cross-checked against a public Havok header. Mad Max's
/// exact Havok revision and cache-pointer semantics remain unverified.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct HkpCastRayInput {
    /// `hkVector4` start point, including the fourth float supplied by the game.
    pub from: crate::sdk::types::Vector4,
    pub num_casts: i32,
    pub _padding_14: [u8; 4],
    /// Base of the end-point array; elements use `to_stride` bytes of spacing.
    pub to_base: *const crate::sdk::types::Vector4,
    pub to_stride: i32,
    pub _padding_24: [u8; 4],
    /// Optional broad-phase cache pointer; meaning and lifetime are unknown.
    pub aabb_cache_info: *const core::ffi::c_void,
}

/// Havok ray-hit callback base class.
#[repr(C)]
pub struct HkpRayHitCollector {
    _opaque: [u8; 0],
}

/// Havok closest-hit collector, derived from `HkpRayHitCollector` in RTTI.
#[repr(C)]
pub struct HkpClosestRayHitCollector {
    _opaque: [u8; 0],
}

/// Havok collision body passed to ray-hit collector callbacks.
#[repr(C)]
pub struct HkpCdBody {
    _opaque: [u8; 0],
}

/// Havok per-shape ray-cast hit data passed to collector callbacks.
#[repr(C)]
pub struct HkpShapeRayCastCollectorOutput {
    _opaque: [u8; 0],
}

/// Havok collision shape interface. Its Mad Max field layout and exact vtable
/// are not modeled.
#[repr(C)]
pub struct HkpShape {
    _opaque: [u8; 0],
}

/// Havok shape base identified in the RTTI hierarchy of
/// `CCustomSampledHeightFieldShape`. This marker carries no native layout.
#[repr(C)]
pub struct HkpShapeBase {
    _opaque: [u8; 0],
}

/// Havok collision-data shape base identified in the RTTI hierarchy of
/// `CCustomSampledHeightFieldShape`. This marker carries no native layout.
#[repr(C)]
pub struct HkcdShape {
    _opaque: [u8; 0],
}

/// Havok height-field shape base identified by Mad Max RTTI.
#[repr(C)]
pub struct HkpHeightFieldShape {
    _opaque: [u8; 0],
}

/// Havok reference-counted object base identified in the RTTI hierarchy of
/// `CCustomSampledHeightFieldShape`. No ownership semantics are modeled.
#[repr(C)]
pub struct HkpReferencedObject {
    _opaque: [u8; 0],
}

/// Havok root object base identified in the RTTI hierarchy of
/// `CCustomSampledHeightFieldShape`. This marker carries no native layout.
#[repr(C)]
pub struct HkBaseObject {
    _opaque: [u8; 0],
}

/// Game-specific Havok shape class identified by RTTI on a world-query
/// broadphase candidate. RTTI bases are `HkpHeightFieldShape`, `HkpShape`,
/// `HkpShapeBase`, `HkcdShape`, `HkpReferencedObject`, and `HkBaseObject`.
/// Runtime vtable slot +0x30 is a separate wrapper into the sampled-height-field
/// ray path. The generic shape-cast call uses slot +0x28 (see world_query.rs).
/// slot +0x80 currently resolves to a four-byte function returning -1. The
/// +0x30 caller leaves RDX, R8, and a candidate-relative R9 pointer live at the
/// virtual call; exact argument meanings and the caller's `EAX == 3` check are
/// unresolved. The +0x80 slot's callsite semantics and this class's layout
/// remain unknown.
#[repr(C)]
pub struct CCustomSampledHeightFieldShape {
    _opaque: [u8; 0],
}

/// Per-shape local-space ray input. The game writes a 0x30-byte scratch block
/// for the fallback collector path, but individual field semantics have not
/// yet been confirmed against Mad Max's embedded Havok revision.
#[repr(C)]
pub struct HkpShapeRayCastInput {
    _opaque: [u8; 0],
}

/// Havok world ray caster; RTTI reports `hkpBroadPhaseCastCollector` as its base.
#[repr(C)]
pub struct HkpWorldRayCaster {
    _opaque: [u8; 0],
}

/// Havok proxy ray caster; RTTI reports `hkpWorldRayCaster` as its base.
/// Mad Max's world-query dispatcher initializes its stack callback object's
/// vptr to this class's vtable, but the object's field layout is not modeled.
#[repr(C)]
pub struct HkProxyRayCaster {
    _opaque: [u8; 0],
}

/// Havok's alternate simple world ray caster; RTTI reports
/// `HkpBroadPhaseCastCollector` as its base.
#[repr(C)]
pub struct HkpSimpleWorldRayCaster {
    _opaque: [u8; 0],
}

/// Havok CPU world-ray collector; RTTI reports
/// `HkpBroadPhaseCastCollector` as its base.
#[repr(C)]
pub struct HkCpuWorldRayCastCollector {
    _opaque: [u8; 0],
}

/// Havok CPU collector for storing a fixed number of ray hits.
#[repr(C)]
pub struct HkpFixedBufferRayHitCollector {
    _opaque: [u8; 0],
}

/// Async ray-result base nested in `CPhysicsSystem`.
#[repr(C)]
pub struct CPhysicsSystemAsyncRaycastResults {
    _opaque: [u8; 0],
}

/// Game ray query result object. RTTI bases, in order, are
/// `CPhysicsSystem::asyncRaycastResults`, `hkReferencedObject`, and
/// `hkBaseObject`.
#[repr(C)]
pub struct CRayQueryData {
    _opaque: [u8; 0],
}

/// Game ray-caster object; RTTI bases are `CGameObject` and `CRtti`.
#[repr(C)]
pub struct CRayCasterObject {
    _opaque: [u8; 0],
}

/// Ray-hit collector implemented by the game; RTTI reports
/// `HkpRayHitCollector` as its base.
#[repr(C)]
pub struct CRayCasterRay {
    _opaque: [u8; 0],
}

/// Base class for game raycast filters.
#[repr(C)]
pub struct CRaycastFilter {
    _opaque: [u8; 0],
}

/// Filter that additionally ignores holey geometry.
#[repr(C)]
pub struct CIgnoreHoleyFilter {
    _opaque: [u8; 0],
}

/// Raw 64-bit MSVC vector header: begin, end, and capacity-end pointers.
/// The header does not own or free its storage when represented in Rust.
#[repr(C)]
pub struct MsvcVectorHeader<T> {
    pub begin: *mut T,
    pub end: *mut T,
    pub capacity_end: *mut T,
}

/// RTTI-confirmed game query filter that rejects candidates whose object
/// pointer is present in its ignored-object vector. The first 0x90 bytes are
/// still opaque; the three-pointer vector header at +0x90 is observed on both
/// the ammo collision and wind-occlusion call paths. This is a raw layout only:
/// Rust must not construct, call, own, or drop the game's filter instance.
#[repr(C)]
pub struct CIgnoreFilter {
    _opaque_prefix: [u8; 0x90],
    pub ignored_objects: MsvcVectorHeader<*mut core::ffi::c_void>,
}

/// RTTI-named game query filter/collector for static geometry. Its vtable uses
/// the shared `CRaycastFilter::addRayHit` at slot +0; do not call the former
/// slot +0x08 label as `addRayHit` because it is a deleting destructor.
#[repr(C)]
pub struct CStaticOnlyRayHitCollector {
    _opaque: [u8; 0],
}

/// RTTI-named game query filter/collector for closest-hit material selection.
/// Its slot +0x08 target is a deleting destructor; the class-specific hit
/// behavior is not yet mapped to a callable Rust ABI.
#[repr(C)]
pub struct CDefaultClosestMaterialRayHitCollector {
    _opaque: [u8; 0],
}

/// RTTI-named game query filter/collector restricted to static and key-framed
/// geometry. Its slot +0x08 target is a deleting destructor; class-specific
/// hit behavior is not yet mapped to a callable Rust ABI.
#[repr(C)]
pub struct CStaticAndKeyFramedOnlyRayHitCollector {
    _opaque: [u8; 0],
}

/// Player-aim ray filter. RTTI bases are `CIgnoreHoleyFilter`,
/// `CRaycastFilter`, `hkpClosestRayHitCollector`, and `hkpRayHitCollector`.
/// Its runtime candidate predicate rejects the current player entity, then
/// applies additional candidate category and tag checks. It is not a generic
/// "ignore only the player" filter; use its results as player-aim-target
/// semantics rather than assuming it preserves every other world hit.
#[repr(C)]
pub struct CPlayerAimRaycastFilter {
    _opaque: [u8; 0],
}

/// Character collision filter; RTTI places it in the
/// `CIgnoreHoleyFilter`/`CRaycastFilter` collector hierarchy.
#[repr(C)]
pub struct CCharacterRaycastFilter {
    _opaque: [u8; 0],
}

/// Camera-collision raycast filter derived from `CRaycastFilter`.
#[repr(C)]
pub struct CCCameraCollisionRaycastFilter {
    _opaque: [u8; 0],
}

/// Filter that ignores character entities, derived from `CIgnoreHoleyFilter`.
#[repr(C)]
pub struct CIgnoreCharactersRaycastFilter {
    _opaque: [u8; 0],
}

/// Static-geometry filter derived from `CIgnoreHoleyFilter`.
#[repr(C)]
pub struct CStaticOnlyRaycastFilter {
    _opaque: [u8; 0],
}

/// Character-only static-geometry filter derived from `CCharacterRaycastFilter`.
#[repr(C)]
pub struct CCharacterStaticOnlyRaycastFilter {
    _opaque: [u8; 0],
}

/// Filter that ignores both characters and vehicles.
#[repr(C)]
pub struct CIgnoreCharactersAndVehiclesRaycastFilter {
    _opaque: [u8; 0],
}

/// Camera-collision filter used by the video-mode camera path.
#[repr(C)]
pub struct CVideoModeCameraCollisionRaycastFilter {
    _opaque: [u8; 0],
}

/// Ray-cast scheduler registered as an application system.
#[repr(C)]
pub struct CRayCastScheduler {
    _opaque: [u8; 0],
}

/// Base class for animation-system conditions.
#[repr(C)]
pub struct CAnimationSystemCondition {
    _opaque: [u8; 0],
}

/// Animation condition driven by a ray cast.
#[repr(C)]
pub struct CRayCastCondition {
    _opaque: [u8; 0],
}

/// Animation condition that evaluates a fan of ray casts.
#[repr(C)]
pub struct CRayCastFanCondition {
    _opaque: [u8; 0],
}

/// Forward ray-fan animation condition.
#[repr(C)]
pub struct CForwardRayCastFanCondition {
    _opaque: [u8; 0],
}

/// Camera-to-player sphere-cast animation condition.
#[repr(C)]
pub struct CCamToPlayerSphereCastCondition {
    _opaque: [u8; 0],
}

/// Game ammunition object; RTTI reports `CDamageInflictor` as its base.
#[repr(C)]
pub struct CAmmunition {
    _opaque: [u8; 0],
}

/// RVA of the runtime-labeled `CAmmunition::CheckCollisions` method.
///
/// This method is a stateful projectile-collision routine, not a general ray
/// query API. It builds a caller-specific filter and output record, invokes
/// `WorldQueryHelper`, writes the resulting endpoint into the ammunition
/// object, and may dispatch collision callbacks. Keep this as an unsafe raw
/// binding; arbitrary mod callers should use a separately reconstructed query
/// adapter instead of manufacturing a `CAmmunition` receiver.
pub const C_AMMUNITION_CHECK_COLLISIONS_RVA: usize = 0x01DA_56C0;

/// Runtime-observed ABI of `CAmmunition::CheckCollisions`.
///
/// The receiver is game-owned. `mode` is the byte argument observed in `DL`;
/// zero derives a segment length from ammunition fields, while nonzero reads
/// the stored length at receiver offset `+0xD0` and clears bit 0 at `+0x11E`.
/// This function mutates the receiver and can invoke native collision
/// callbacks, so it is not suitable as a standalone placement raycast.
pub type CAmmunitionCheckCollisionsFn =
    unsafe extern "system" fn(ammunition: *mut CAmmunition, mode: u8);

/// Game force-pulse object with multiple game, PFX, RTTI, and event-handler
/// bases. The native layout and base offsets are deliberately not modeled.
#[repr(C)]
pub struct CForcePulse {
    _opaque: [u8; 0],
}
