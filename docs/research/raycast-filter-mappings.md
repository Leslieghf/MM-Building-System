# Ray query filter mappings

**Executable:** Steam Mad Max 1.0.3.0 x64  
**Updated:** 2026-10-29  
**Scope:** Recovered filter behavior and caller contracts. This is an API map, not a chronological capture log. Runtime snapshots are used where the on-disk code section is scrambled.

## Shared raycast candidate filtering

The live CRaycastFilter::AddRayHit path applies three stages to a candidate:

1. The active filter's virtual predicate at vtable slot +0x10 must accept the candidate.
2. The candidate filter-info word at candidate+0x18 must have no bits in common with the filter's exclusion word at +0x70, and must contain every bit in the required word at +0x74.
3. The closest-hit collector receives the candidate. When the candidate strictly improves the fraction, the filter stores the candidate node pointer at filter+0x80.

The output path copies the accepted candidate pointer separately from the hit fraction/position. Do not treat the candidate pointer as a hit position.

## Candidate owner pointer

For the observed candidate record, candidate+0x28 is the owner/object pointer used by multiple predicates. Candidate category flags come from its virtual method at slot +0x48; the shape/subobject pointer comes from slot +0x10. These are observed interfaces, not yet complete Rust ABI types.

## CIgnoreFilter

RTTI identifies the vtable at RVA 0x01253B20. Its candidate predicate is vtable slot +0x10 at RVA 0x02059ED0.

- The filter stores a pointer-vector range: begin at +0x90, end at +0x98, capacity at +0xA0.
- It accepts a candidate exactly when the candidate's owner pointer (candidate+0x28) is absent from that range.
- An empty range accepts every candidate.
- The game-owned GetRayWorldPosition caller populates the range from [owner+0x2C8, owner+0x2D0) and appends owner+0x478 when non-null.

This provides a direct, caller-independent owner-exclusion rule when a correctly owned context is available. It does not establish that arbitrary objects are safe to insert or that the query may run on arbitrary threads.

## CIgnoreHoleyFilter

RTTI identifies a concrete base class between CIgnoreFilter and CRaycastFilter. Its vtable at address point 0x141238368 has the shared AddRayHitToCRaycastFilter entry at slot +0x00, a class-specific destructor at +0x08, and the engine pure-virtual handler at candidate-predicate slot +0x10. This confirms CIgnoreHoleyFilter is abstract: it supplies no usable candidate predicate of its own, and a concrete subclass must override that slot. Do not install this base vtable as a query filter.

Its RTTI hierarchy is CIgnoreHoleyFilter -> CRaycastFilter -> hkpClosestRayHitCollector -> hkpRayHitCollector. The class name alone does not establish what its “holes” represent; that remains unresolved.
## IRayCastRequest and CAiSimpleRayCastRequest

RTTI reveals a game-owned request class that is also a raycast collector/filter. CAiSimpleRayCastRequest has a primary IRayCastRequest subobject at object offset 0 and a secondary CRaycastFilter subobject at +0x10. Its RTTI hierarchy continues through hkpClosestRayHitCollector and hkpRayHitCollector.

- Primary vtable address point: 0x141238460. Virtual entries point to 0x141C417D0, 0x141BFEEC0, and 0x141BFEF10; their names/contracts are not yet recovered.
- Secondary filter vtable address point: 0x141238440. It uses AddRayHitToCRaycastFilter at slot +0x00 and a concrete candidate predicate at slot +0x10 (0x141BFF030). That predicate remains unmapped.
- The standalone IRayCastRequest vtable is at 0x1412383E8. Its slot +0x08 is pure virtual; the remaining method contracts are unresolved.

This is a promising lead because request and collector/filter state coexist in one game type. It is specifically named for AI, and no constructor, callsite, execution phase, lifetime, result contract, or connection to the player reticle has been recovered. Treat it as a research candidate, not a callable mod API.
## CPlayerAimRaycastFilter

RTTI identifies the vtable at RVA 0x012383C8; the predicate is RVA 0x02039E90.

- It first resolves the active player object and rejects a candidate whose owner (candidate+0x28) is that object.
- For other candidates, it accepts category flag 2 immediately.
- Otherwise, it accepts when an owner-associated interface/tag query succeeds, or when any of category flags 1, 3, or 0x15 is present.
- The exact names and meanings of those category flags and the owner-associated interface are unresolved.

This is a game-specific aim policy, distinct from a general ignore list. It does not alter the ray origin or direction.

## CStaticOnlyRaycastFilter

RTTI identifies the vtable at RVA 0x0123D6A8; its predicate is RVA 0x02039DB0. The observed camera-obstruction caller sets its category mask at +0x90 to 3.

- Candidate flag 1 maps to predicate bit 2; candidate flag 3 maps to predicate bit 1.
- The predicate accepts when one of those mapped bits overlaps the filter mask.
- If the candidate's shape/subobject exists, its byte at +0x160 must equal 5.
- A successful owner-associated interface/tag query rejects the candidate.
- The meanings of the shape byte and the interface/tag are not yet named.

Despite the class name, the recovered logic should not be summarized as terrain only. The caller's mask and additional shape/owner conditions matter.

## CCCameraCollisionRaycastFilter

The runtime predicate is at RVA 0x02066D10; the recovered object checks candidate category flags 1, 3, and 0x14, metadata keys 0x1B and 0x13, and caller-supplied object pointers at filter offsets +0xA0 and +0xA8. Additional hierarchy/tag checks are present. Several helper meanings and the owning constructor/callsite remain unresolved, so this is not yet a constructible API contract.

## Known caller contracts

### GetRayWorldPosition (RVA 0x01E1BBF0)

The game-owned helper takes an owner/context pointer, an output Vector3 pointer, and a six-float ray record (origin xyz followed by direction xyz). It reads range from *(owner+0x3E0)+0x270, initializes a CIgnoreFilter-derived query context, and calls WorldQueryHelper. The output defaults to the segment endpoint; for a fraction below 1, it writes origin + direction * range * fraction.

Known callers build and normalize the six-float input before calling. This is a weapon/actor-context segment query, not a generic reticle function.

### ProcessStaticCameraCollisionRay (RVA 0x01EAC0C5)

The camera-obstruction caller constructs a segment from a supplied focus point toward the current camera position, shortens it by a near-plane distance, and invokes WorldQueryHelper with a CStaticOnlyRaycastFilter-derived context. Its camera collision filter and ray geometry are not evidence that it is the player's reticle ray.

## WorldQueryHelper query construction

The restored WorldQueryHelper body (0x14252EDF0) does not pass the supplied ray record straight to the broadphase. It builds segment endpoints from the origin/direction and start/end distances, then subtracts the vector at queryBase+0x40 from both endpoints. Each endpoint is checked by helper 0x1425267E0 before dispatch.

On the accepted path it prepares a stack query/filter configuration. Its observed 32-bit selector word is `(queryGroup << 16) | 0x10` when modeFlag is true and `(queryGroup << 16) | 0x12` otherwise; a non-null filterOverride replaces the complete word. DispatchRayCastQuery receives the query base's broadphase from `*(queryBase+0x38)+0x88` and filter configuration from `*(queryBase+0x38)+0xC0`. The meanings of the low selector values, queryGroup, and validation helper are unresolved.

This is evidence that a reusable query depends on more than ray origin, direction, range, and a type predicate: it also needs the correct query base, local coordinate origin, and filter configuration. The recovered body does not yet establish how to safely construct or retain those objects from a mod.

### Broadphase ray traversal

`DispatchRayCastQuery` calls the broadphase virtual at slot `+0xC8`, passing a stack cast-input block and a collector. The vtable address point at RVA `0x013E0A18` is RTTI-confirmed as `hkp3AxisSweep`: its preceding MSVC Complete Object Locator has type descriptor RVA `0x017DBA28`, whose name is `.?AVhkp3AxisSweep@@`. The target at slot `+0xC8` is runtime-captured at RVA `0x02B2CD40`.

The `DispatchRayCastQuery` stack block matches the public `hkpCastRayInput` layout: a 16-byte `from` vector at `+0x00`, `numCasts=1` at `+0x10`, a pointer to the endpoint vector at `+0x18`, `toStriding=0x10` at `+0x20`, and the optional cache pointer at `+0x28` (total size `0x30`). This makes the input record and its vector-pointer/stride relationship concrete for future captures. It does not make the broadphase function safe to call independently. The captured body maps the cast input into quantized grid coordinates, traverses the spatial index along three axes, and calls the supplied collector's virtual slot `+0x08` for candidate handles while maintaining a closest fraction. This is the game's broadphase candidate traversal and establishes that `WorldQueryHelper` reaches a real Havok-style broadphase cast. It does not establish how to create the owning world/query base, which cast-input fields are safe to populate, or which filter configuration yields a reticle hit. The on-disk code section at this function address is scrambled; this mapping comes from the live runtime snapshot plus executable RTTI, so do not use the main executable's static decompilation for the body.

The captured per-candidate callback at RVA `0x02B0DC60` invokes a candidate predicate before narrowphase work, transforms the segment endpoints through the candidate transform into local coordinates, then dispatches a candidate shape virtual at `+0x30`. A separate captured shape adapter at RVA `0x02B13810` calls shape virtual `+0x28`; when the shape reports a hit, it transforms the returned hit vector back to world coordinates and invokes collector virtual slot `+0`. The adapter therefore confirms the world-space hit handoff into the same collector family, though a direct caller edge between these two snapshots has not been proven.

The captured early local-sphere routine at RVA `0x02B0E7D0` computes closest approach of a segment to a candidate-centered sphere using a radius at candidate-shape `+0x20`, and writes hit data/fraction only when the segment passes the radius/range test. This is a candidate-level geometric stage, not a terrain query result by itself. These mappings suggest a clean debugging split: query-input coordinate setup, candidate filter acceptance, candidate-local shape intersection, world-space transform, and final collector update. They do not explain the observed near-player result without sampling those boundaries for the reticle query.

A public, third-party-vendored Havok header provides an architectural cross-check: hkpWorldRayCaster is described as the utility connecting hkpBroadPhase::castRay to hkpShape::castRay, and the broadphase cast reports candidate AABB handles through a collector. That matches the binary's broadphase virtual, candidate callback, shape virtual, and collector handoff. These headers are from an external vendored source tree, not Mad Max's exact SDK version; they corroborate the architecture, not field offsets or ABI details. See [hkpWorldRayCaster.h](https://github.com/nitaigao/engine-game-factions/blob/master/etc/vendor/havok/Source/Physics/Collide/Query/CastUtil/hkpWorldRayCaster.h) and [hkpBroadPhase.h](https://github.com/nitaigao/engine-showcase/blob/master/etc/vendor/havok/Source/Physics/Internal/Collide/BroadPhase/hkpBroadPhase.h).

## Queued world-ray request path

`FilterAndQueueWorldRayQuery` (RVA 0x01BB2C20) rejects requests whose max-distance argument is not positive. When given a non-null context, it checks the runtime object's type against `CGameObject`, `CCharacter`, and `CWeapon`, extracts a 32-bit result-flags value through an interface call, and applies two caller-provided boolean gates before forwarding to `EnqueueWorldRayQueryRequest` (RVA 0x01BA22D0). The type filters and boolean meanings are not fully named. The observed forwarding path is a separate queue subsystem; no direct call to `WorldQueryHelper` or link to the player's reticle has been established.

The enqueue routine bounds the queue to 0x80 entries, atomically increments a counter at query-system+0x2040, and writes a 0x20-byte record beginning at query-system+0x1040. It copies query type at +0x00, max distance at +0x04, result flags at +0x08, `*(query-system+0x10)+1` at +0x0C, three floats from the supplied vector at +0x10, and a DWORD from `*(global+0x154)` at +0x1C. This yields a complete observed writer layout, but not the record's consumer, lifetime, completion signal, or meaning of the copied vector and two opaque words. It is a promising lead for a game-owned deferred query model, not yet a usable asynchronous raycast API.
## Mod API status

The recovered pieces are enough to describe filter predicates and two game-owned caller shapes. They are not yet enough to expose a safe reusable Rust Raycast API: world ownership, context lifetime, game-thread phase, Havok locking, native object identity, and arbitrary-context construction still need to be established.

The current experimental reticle probe submits the same origin, direction, and range through four filter variants: CStaticOnlyRaycastFilter, CPlayerAimRaycastFilter, CIgnoreCharactersAndVehiclesRaycastFilter, and CIgnoreFilter with a one-entry owner list containing the current player-character pointer. They share the same base filter-info masks; the game-specific predicates and optional owner exclusion differ. The bounded callback observer records candidate filter-info, owner identity, active filter masks, and selected-shape changes only for sampled reticle queries. These comparisons help isolate self-hit and category-filter hypotheses; they do not yet establish a general-purpose reticle API. Raycast diagnostics live in a separately selectable Raycast section.


## Collision masks are separate from type-specific predicates

The shared filter words are separate from type-specific predicate fields such as CStaticOnlyRaycastFilter +0x90 and CIgnoreFilter owner-vector fields:

- `filter+0x70` is an exclusion mask: reject when `(filter+0x70 & candidate+0x18) != 0`.
- `filter+0x74` is a required-bits mask: accept this stage only when `(filter+0x74 & candidate+0x18) == filter+0x74`.
- Therefore, zero at +0x70 excludes no filter-info categories; zero at +0x74 imposes no required bits.

The base context initializer receives the +0x70 exclusion mask in EDX, the +0x74 required-bits mask in R8D, and an opaque value in R9, then stores them at the corresponding offsets. This ABI is mapped from the runtime initializer snapshot; do not infer that the rest of the 0x90-byte state is safe to construct independently.

GetRayWorldPosition sets its shared exclusion mask at context+0x70 to either 0 or 0x100, based on bit 0x40 of the byte at *(owner+0x3E0)+0x2DC. The option setter writes `(global DWORD | 0x100)` when enabled and `0x100` when disabled. With the on-disk global DWORD of `0x20`, this would be `0x120` or `0x100`; the runtime value has not been checked. This is separate from the CIgnoreFilter owner vector. Neither branch means “reject all” merely by virtue of the shared mask: candidates are excluded only when their +0x18 word overlaps it.


## Additional named filter predicates

### CIgnoreCharactersRaycastFilter

Its vtable predicate checks candidate category flags 1 and 3, requiring at least one. It may then reject through an owner-associated interface/tag query using the owner pointer at candidate+0x28. The numeric category names and tag meaning remain unresolved.

### CIgnoreCharactersAndVehiclesRaycastFilter

Its vtable predicate rejects a candidate when category flag 2 or 4 is set and accepts it otherwise. The class name suggests these flags correspond to character/vehicle classes, but that interpretation has not been independently proven.

### CCharacterRaycastFilter and CCharacterStaticOnlyRaycastFilter

The captured predicates require respectively flags 1 or 3, and flag 1; both reject the owner equal to filter+0x90 and call helper 0x1425232A0, requiring a zero return. The helper's semantic name and the category meanings remain unresolved.

