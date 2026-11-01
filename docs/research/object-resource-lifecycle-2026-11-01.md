# Object and resource lifecycle map (current evidence)

Date: 2026-11-01  
Scope: Mad Max Steam executable and the current Rust bridge. This is a working map, not an API contract.

## Evidence labels

- **SDK-declared**: in the pinned public mm_sdk headers.
- **Static RTTI/data**: present in the supported executable's class metadata or data tables.
- **Runtime-confirmed**: observed by the working Rust bridge or a manual in-game result.
- **Candidate/inference**: consistent with current data, but not yet established end-to-end.
- **Unknown**: no safe conclusion.

## Existing-entity path

    asset key / .ee bundle
      -> SARC package
      -> .epe property definition + referenced resources
      -> object type (for example CRigidObject) + transform + behavior/resource references
      -> SpawnSystem request (SDK-declared; working bridge has submitted known entities)
      -> CGameObjectCreator<T> / game-owned construction path (metadata is static; a historical snapshot decodes a sampled creation protocol, but it has not been called and its full ABI remains unresolved)
      -> game-world and physics registration (not yet mapped for general props)
      -> spawned-handler observation (SDK-declared; current runtime correlates owned requests to native IDs)
      -> SpawnSystemDespawn(native ID) for a positively observed mod-owned spawn

For the small stockpile, archive analysis finds one CRigidObject definition, a model resource reference, a PFX collision reference, graph/fade components, and a Havok packfile. Manual in-game testing has confirmed this entity can be spawned as a working physical object. Its exact motion type and whether the object is fixed/static remain unknown. See [static prop candidates and lifecycle evidence](static-prop-asset-candidates-2026-10-31.md).

The Rust-owned request ID is not a native object ID. The current path retains a request-derived spawn identity because the SDK allocator return ABI has not been validated for this build. The Rust removal adapter is restricted to positively observed mod-owned IDs. Arbitrary map-object deletion has no mapped route.

## Authored decoration/map path

    settings/worlddecorations.bin
      -> six parsed property nodes (two CRigidObject, CDynamicLightObject, CStaticDecalObject)
      -> ? no direct xref or proven link into the runtime cache path
      -> placement, chunk readiness, object/Havok registration (unknown)
      -> renderer instance vs live CGameObject vs Havok body (unknown)
      -> unload/destruction/persistence (unknown)

    CResourceCreator<CWorldDecorationResource>
      -> resource handle builder
      -> CWorldDecorationCache singleton
      -> keyed cache lookup; supplied resource interface read on miss
      -> shared resource handle
      -> ? link to settings/worlddecorations.bin and map placement (unproven)

A 1,871-byte settings/worlddecorations.bin sample parses as six property nodes containing a pair of barrel CRigidObject definitions, a dynamic light, and a static decal. RTTI establishes CWorldDecorationCache as an app-system singleton with three hierarchy entries (CWorldDecorationCache, IAppSystem@Base, CAvaSingle<CWorldDecorationCache>). The exact path string still has no direct code/data xrefs in the inspected Ghidra program, so the sample is not yet linked to the separately mapped cache path. The cache class's other candidate virtual targets lead into .xpdata and are not statically callable under current analysis.

The separate stream-patch family includes map and terrain patch types. Existing research identifies shared RTTI/vtable patterns but has not connected them to this composition file or proven they construct Havok-backed game objects. See the streaming and world-decoration sections in [static prop asset candidates](static-prop-asset-candidates-2026-10-31.md).

### World-decoration resource creator: static boundary

Ghidra RTTI references identify `CResourceCreator<CWorldDecorationResource>` at TypeDescriptor `0x1417B0740`. One associated complete-object locator is at `0x1414D5F40`; its vtable metadata reference is at `0x1412BC2E0`, followed by a first slot at `0x1412BC2E8` targeting `.xpdata` address `0x1420BD6D0`. Ghidra has only a one-byte bad-data stub at that target, with no usable body or call references. A second TypeDescriptor reference at `0x1414D5F98` remains structurally unclassified.

In the static executable, this establishes that a resource-creator vtable slot is routed into `.xpdata`; that view still has no usable body. The saved historical runtime slice decodes this same target as `CreateCWorldDecorationResource` and exposes a separate cache/handle construction path described below. This does not show that the creator is invoked by the `worlddecorations.bin` loader. No callable ABI or map-object lifecycle has been established, and no function has been called in game.

### `CGameObjectCreator<CRigidObject>` table boundary

The RTTI complete-object locator `0x1414BFDD0` points to TypeDescriptor `0x1417A1118`; its vftable metadata reference is `0x14128C838`, so the address point begins at `0x14128C840`. The next COL pointer is at `0x14128C858` and identifies `boost::detail::sp_counted_impl_p<CRigidObject>`, which bounds the creator's table to **three entries**. Those three target values are `0x141F789B0`, `0x141FB4120`, and `0x141F69250`. The first two are exact `.pdata` entries (`0x141F789B0..0x141F789D9` and `0x141FB4120..0x141FB41CD`); the third is still only a one-byte Ghidra stub and has no exact unwind entry in the checked table.

This prevents treating the adjacent RTTI records as more methods of `CGameObjectCreator<CRigidObject>`. The next address point is the Boost control-block vtable, followed later by a separate three-entry `CGameObjectCreator<CScaledRigidObject>` table. Its first two targets likewise have exact `.pdata` ranges, while its third is a one-byte static stub. Table boundaries remain structural evidence; the saved runtime slice later supplies behavior for three creator slots, but signatures and invocation context remain unvalidated.

The related `CAppSystemCreator<CWorldDecorationCache>` has TypeDescriptor `0x1417A0300`, a locator at `0x1414BE650`, and a vtable reference at `0x14128C028`. Its first three slots point into `.xpdata` (`0x141F67B50`, `0x141F7F980`, `0x141F7F9F0`); all three also decompile as one-byte bad-data stubs. This supports that the cache is wired through the engine's app-system creator machinery, but static analysis still cannot establish its registration order or callable methods. The `settings/worlddecorations.bin` string at `0x1412B1A58` has no Ghidra code/data xrefs in this program, so the content sample remains unconnected to this registration evidence.

The adjacent streaming layer shows the same static boundary. `CMapStreamer` RTTI (`0x14179C7B0`) has a primary locator at `0x1414B5388` and vtable start `0x141263928`; its first five observed slots land in `.xpdata` (`0x141DEFDD0`, `0x141DE0EB0`, `0x141DE0ED0`, `0x141DE0EE0`, `0x141B9FCB0`). `CStreamPatchManager` RTTI (`0x1417C7DE8`) has locator `0x1414ED140`, vtable start `0x14131C960`, and observed entries in `.xpdata` (`0x142447A60`, `0x142491110`, `0x142490C20`). The first mapped targets are one-byte bad-data stubs or have no valid static function body. This confirms that the streamer and patch manager are also represented by protected virtual tables, but yields no verified transition/callback names or area readiness contract.

The PE exception directory adds useful boundaries without decoding these bodies. Exact `.pdata` entries begin at the `CWorldDecorationResource` creator target (`0x1420BD6D0..0x1420BD71A`, 0x4A bytes), all three `CAppSystemCreator<CWorldDecorationCache>` targets (`0x141F67B50..0x141F67B97`, `0x141F7F980..0x141F7F9E4`, `0x141F7F9F0..0x141F7FA27`), the first `CMapStreamer` target (`0x141DEFDD0..0x141DEFDFF`), and `CStreamPatchManager`'s second observed target (`0x142491110..0x142491130`). These are structural unwind ranges, not method names, signatures, or proof that the encrypted file bytes decode as code. The other inspected streamer/manager slots only appeared inside a single very large overlapping range (`RVA 0x01B6D128..0x03039B50`); earlier exception-table work flagged this record as malformed-looking and discarded it. I therefore exclude it rather than treating it as evidence that those slots share one function.

### `.pdata_a` records associated with protected virtual targets

A separate Ghidra memory block named `.pdata_a` (`0x141A0C000..0x141B315FF`) contains repeated 12-byte records that point at several of the `.xpdata` virtual targets above. The checked records have three 32-bit values: a value resolving into `.data1_a`, followed by values resolving to the start and end of a protected target range. This is an observed record shape only; it is **not** identified as the PE exception directory's `RUNTIME_FUNCTION` format, and the role/encoding of the `.data1_a` bytes is unknown.

| RTTI/vtable target | `.pdata_a` record | First value resolves to | Protected range values | Cross-check |
|---|---:|---:|---:|---|
| `CGameObjectCreator<CRigidObject>` slot 0 (`0x141F789B0`) | `0x141A427D0` | `0x1415B2848` | `0x01F789B0..0x01F789D9` | Range matches the previously parsed PE exception extent. |
| `CAppSystemCreator<CWorldDecorationCache>` slot 0 (`0x141F67B50`) | `0x141A4184C` | `0x1415B10AC` | `0x01F67B50..0x01F67B97` | Range matches the previously parsed PE exception extent. |
| `CAppSystemCreator<CWorldDecorationCache>` slot 1 (`0x141F7F980`) | `0x141A439DC` | `0x1415B409C` | `0x01F7F980..0x01F7F9E4` | Range matches the previously parsed PE exception extent. |
| `CAppSystemCreator<CWorldDecorationCache>` slot 2 (`0x141F7F9F0`) | `0x141A439E8` | `0x1415B40AC` | `0x01F7F9F0..0x01F7FA27` | Range matches the previously parsed PE exception extent. |
| `CWorldDecorationResource` creator slot (`0x1420BD6D0`) | `0x141A58418` | `0x1415F176C` | `0x020BD6D0..0x020BD71A` | Range matches the previously parsed PE exception extent. |
| `CMapStreamer` first observed slot (`0x141DEFDD0`) | `0x141A2DCBC` | `0x1415799E4` | `0x01DEFDD0..0x01DEFDFF` | Range matches the previously parsed PE exception extent. |
| `CStreamPatchManager` observed slot (`0x142491110`) | `0x141A8FB1C` | `0x1416820D8` | `0x02491110..0x02491130` | Range matches the previously parsed PE exception extent. |

For example, the bytes at `0x141A4184C` decode as `0x015B10AC, 0x01F67B50, 0x01F67B97`; the first address points into `.data1_a` and the latter two reproduce the range of the vtable target. Similar triples were read for the decoration resource, cache, streamer, and stream-patch manager. The referenced `.data1_a` bytes are initialized but have not been decoded or assigned semantics. A Ghidra reference pass over the seven companion addresses found one `DATA` reference each, from its matching `.pdata_a` record; the seven record starts themselves had no references. This only rules out direct references recorded at these exact addresses, not indirect/base-relative access. Thus the table adds a reproducible address relation between protected dispatch targets and companion data, but does not reveal how code is decoded, what the companion bytes mean, or how to invoke a method safely.

The additional cache-factory-like target at `0x141F66F90` has the same record shape: static data at `0x141A4166C` reads `0x015B0E7C, 0x01F66F90, 0x01F66FDF`, with a data reference from the record's second word to the target. The first value resolves into `.data1_a`. The historical runtime slice decodes an allocation-and-initialization routine at this target, but the static target remains a one-byte bad-data stub. This extends the observed address relation by one example; it still does not establish the record format's purpose or a caller/dispatch edge.

Next static step: identify code references to this record table or to its `.data1_a` companions, then compare the same relationship for a target whose runtime behavior is independently known. Do not call the records function descriptors or decryption metadata until that consumer path is established.

## Type model from executable RTTI

The static executable contains 286 distinct CGameObjectCreator<T> RTTI names. Selected hierarchies show a layered inheritance system:

- CRigidObject -> CPhysicsGameObject -> CGameObject, plus PFX, event-handler, and Havok activation-listener interfaces.
- CAnimatedRigidObject extends CRigidObject.
- CCar -> CLandVehicle -> CVehicle -> CDamageable -> CPhysicsGameObject -> CGameObject.
- CCharacter similarly shares damageable/physics/game-object bases and adds character-system interfaces.

The RTTI inventory and exact hierarchy records are [here](object-creator-rtti-inventory-2026-11-01.md) and [here](object-type-hierarchies-2026-11-01.md). This confirms an inheritance-and-interface model; it does not rule out component-style composition inside entity definitions. RTTI alone does not establish creator call contracts or safe object lifetime.

## Current blockers, separated by layer

| Layer | Known | Still unknown |
|---|---|---|
| Asset package | Some .ee bundles and resource dependencies are parsed; one stockpile works in game. | Which entity keys are atomic objects versus encounter/project compositions; model/material resource decoding. |
| Object identity | SDK declares spawn/despawn and spawned-handler records; bridge has a host/native ID correlation path. | Allocator ABI and vector layout validation for this build; whether native IDs survive long enough for safe removal. |
| Creation | SDK SpawnSystemSpawn accepts a descriptor plus transform; creator RTTI exists for 286 types. | The callable CGameObjectCreator<T> virtuals and resource-loading preconditions. Static targets for the rigid creator land in .xpdata. |
| Physics | Stockpile bundle contains a Havok collision packfile; rigid RTTI has Havok/PFX listener bases. | Motion type, material/filter setup, world insertion/removal, and collision callback routing for a newly spawned body. |
| World streaming | CMapStreamer, stream-patch classes, decoration cache/resource, and map-placement tables are present. | Which data source owns each live object, chunk readiness, streaming destruction, and whether scenery is a live game object or renderer-only instance. |
| Persistence | No mod-owned construction persistence adapter exists in this tranche. | Save format interaction and whether authored map-object removal/addition can persist safely. |

## Next low-risk research order

1. Trace one CGameObjectCreator<CRigidObject> vtable from RTTI through its .xpdata decode boundary; do not call any slot yet.
2. Connect the stockpile's .epe node and resource references to the creator/loader path used by SpawnSystemSpawn.
3. Independently trace settings/worlddecorations.bin consumption and map/terrain patch registration; distinguish renderer instances from live Havok bodies.
4. Identify object-ready, area/chunk-ready, despawn, and unload signals before designing arbitrary selection/removal.
5. Only after these contracts are mapped, choose whether procedural props should use game-native creation, an offline packaged entity, or a separate Rust-owned physics/render path.

## Runtime snapshot: rigid-object creator dispatch

The saved runtime slice `/runtime_snapshots/MadMax_xpdata_runtime_slice_141C00000.bin` contains decoded x64 bytes at the same RVAs where the static executable's `.xpdata` bytes look invalid. This is a historical capture, not a live process. In that snapshot, three creator vtables have COL-bounded three-slot layouts. Combining their RTTI-bounded tables with recovered instructions gives this interpretation:

| Creator table | Address point | Slot 0 | Slot 1 | Slot 2 |
|---|---:|---|---|---|
| `CGameObjectCreator<CRigidObject>` | `0x14128C840` | `0x141F789B0`: installs the `IGameObjectCreator` base vptr and conditionally deletes the creator | `0x141FB4120`: initializes a `CRigidObject` and forms a shared-pointer-like result | `0x141F69250`: returns `0x2A0` |
| `CGameObjectCreator<CScaledRigidObject>` | `0x14128C890` | `0x141F789F0`: same base-vptr/conditional-delete pattern | `0x141FB41E0`: initializes a `CScaledRigidObject` and forms a shared-pointer-like result | `0x141F69320`: returns `0x2B0` |
| `CGameObjectCreator<CAnimatedRigidObject>` | `0x14128C8E0` | `0x141F78A20`: same base-vptr/conditional-delete pattern | `0x141FB42A0`: initializes a `CAnimatedRigidObject` and forms a shared-pointer-like result | `0x141F69400`: returns `0x4A0` |

The slot-0 ranges match checked unwind bounds. The slot-2 bodies in the runtime snapshot are exactly `MOV EAX, immediate; RET`. Given their placement as the third entry in each creator table and the returned values' correspondence to object layout size, they are documented as creator allocation-size callbacks. This is an interpretation of the slot contract; the recovered callback itself simply returns a constant.

The create-entry bodies share a repeatable shape: calculate a value, pass it to a helper at `0x141B6D150`, initialize the concrete object, build a two-word pointer/control-block result, increment a control-block count, and pass that result to `0x1422E03E0`. Across 26 checked creator types, the value passed to this helper exactly equals the type's slot-2 constant. This strongly supports reading slot 2 as an instance-size callback and the helper as a size-driven allocation/acquisition path. The helper's body is not decoded in this snapshot, so its exact role and ABI remain unresolved. Creator inputs, ownership rules, and world/Havok registration are also not established. No creator was called.

The shared follow-up at `0x1422E03E0` is now labeled `AssignSharedPointerAtOffsetC0` in Ghidra. Its decoded body writes the first word of the pair to `param_1 + 0xC0`, compares the control-block word with the existing field at `param_1 + 0xC8`, increments the incoming count when needed, releases the old count, stores the new control-block word, then releases the temporary pair's count. This is strong evidence that the pair is reference-counted shared ownership and that this helper assigns such a member at offset `0xC0`; it does not identify the member's domain meaning or prove that every creator's output is assigned to the same logical field outside these call sites.

The 26 cross-checks span rigid/destructible props, character/vehicle classes and components, spawn/encounter objects, a raycaster, skeletal animation, flame, map UI, and resource/system objects; the full address/value table is [here](object-creator-slot2-return-map-2026-11-01.md). The returned object pointer can be the allocation base or use a creator-specific adjustment: checked examples include `+0`, `+0x8`, and `+0x10`. Fifteen of the original 19 have inspectable type-specific initializer bodies. The seven added paths include four inspectable initializer bodies, one inline creator initialization path, and two unavailable/unreadable initializer targets. `CSpawnObjectCollection` and `CResourceObject` call a shared constructor at `0x14233E580`, outside this captured code, before applying their own vptrs. `CItemPropObject` and `CAiRoadFlagSetter` initializer targets are unreadable in this capture, while the keyframed destructible target is outside it; `CStateContainerStateHarpoonShoot` bytes are not meaningful decoded code.

The initializer at `0x142077B90` calls `InitializeCRigidObjectInstance`, replaces its vptrs with the scaled-object vptr set, and writes additional defaults. The initializer at `0x1420C20B0` also calls `InitializeCRigidObjectInstance`, installs the animated-object vptr set, and initializes a much larger field region. Its assignments reach at least offset `0x414`; its creator size callback returns `0x4A0`. RTTI confirms the same five subobject offsets (`0`, `8`, `0xD8`, `0xE8`, `0xF0`) across the `CRigidObject` base and `CAnimatedRigidObject` derived construction sequence. This is construction/layout evidence, not a full field map or complete constructor ABI.

One subobject at that `+0xD8` offset is now identified directly. `InitializeCRigidObjectInstance` calls `InitializeCEventHandlerSubobject` (`0x142982160`) with `param_1 + 0x1B` qwords, i.e. byte offset `+0xD8`. That helper writes vptr `0x141347040` and clears the following qword. The vptr's COL at `0x1414FAA28` resolves to TypeDescriptor `0x141792908`, named `.?AVCEventHandler@NEvent@@`. The enclosing rigid initializer later writes its own `CRigidObject` vptr `0x1412ABC08` at the same `+0xD8` location; the static COL for that derived vftable address point reports complete-object offset `0xD8`. This is consistent with base-subobject initialization followed by installation of the complete type's override table. It corroborates the RTTI hierarchy's `NEvent::CEventHandler` base/interface in the rigid-object construction layout, but does not identify event IDs, registration behavior, or callable dispatch methods.

Ghidra records 64 direct callers of `InitializeCEventHandlerSubobject` in the saved snapshot. The caller list is partly truncated and all entries currently retain generated names; the CRigidObject path is one verified concrete example. Treat this as evidence that the initializer is shared broadly across construction paths, not as evidence that 64 event types or registrations exist.

The RTTI-confirmed `NEvent::CEventHandler` vftable address point has two observed pointer-sized entries: `0x1429918A0` and `0x143039550`. Both addresses fall in the static executable's non-executable `.xpdata` section; the first is represented as a one-byte bad-data stub and the second has no defined function body. This is a direct lead from rigid-object construction into protected event-handler virtual code, but it does not establish which slot handles events, the method signatures, or a callable dispatch API. Ghidra bookmarks preserve the table and second target for follow-up.

The game also has a distinct native action-handler registration/subscription path; its special-camera actions and current evidence boundary are mapped separately in [the native action-handler note](native-action-handler-path-2026-11-01.md). The shared `NEvent::CEventHandler` base and action subscription helpers co-occur in special-camera initialization, but there is no evidence yet that they are the same dispatch system.

All 286 third-slot targets are now named `Get<Type>InstanceSize` in the saved runtime snapshot; each target's address and observed return value are listed in [the full creator slot-2 map](object-creator-slot2-return-map-2026-11-01.md). Behavior-derived names now also cover 26 checked creator entries and their inspectable initializer targets, including `CThunderStickLauncher`, `CVehicleInstantRepairRefuelComponent`, `CAiRoadFlagSetter`, `CStateContainerStateHarpoonShoot`, `CMoveToGround`, `CMapIcon`, and `CUpgradeSystem`. The common shared-member assignment helper is labeled `AssignSharedPointerAtOffsetC0`. These are analyst labels, not recovered original symbols. The snapshot does not currently contain a reliably decoded `CScaledRigidObject` RTTI hierarchy, so do not extend its inheritance claim beyond the initializer's explicit call to the CRigid initializer.

This advances the boundary from “three opaque creator slots” to a repeatable creator protocol whose size value is corroborated across 26 types. It still does not connect the stockpile `.epe` resource to these slots, decode the helper implementation, establish how an object is registered with the world/Havok, or reveal the full descriptor/resource prerequisites. Those remain separate steps.

### World-decoration resource path in the saved runtime slice

The saved capture also contains the target of the static `CResourceCreator<CWorldDecorationResource>` vtable slot. `CreateCWorldDecorationResource` at `0x1420BD6D0` is a short wrapper that forwards its arguments to `0x14209DCE0` and returns the output pair. The latter is now labeled `BuildWorldDecorationResourceSharedHandle`; its visible path constructs key/config data, calls `FindOrCreateWorldDecorationCacheEntry` at `0x14209A480`, creates an object-like record, and returns a two-word reference-counted handle.

The cache itself is a confirmed `CWorldDecorationCache` singleton. `CreateWorldDecorationCacheSingleton` checks global `0x1417FDDF8`, allocates `0xB0` bytes at `0x10` alignment when empty, invokes `InitializeCWorldDecorationCache` (`0x14209CE50`), then stores the pointer. That initializer writes the `CWorldDecorationCache` vptr `0x1412B16B0`; its COL at `0x1414D5468` resolves to TypeDescriptor string `.?AVCWorldDecorationCache@@`. It zeros the vector begin/end/capacity fields at cache offsets `0x10`, `0x18`, and `0x20`, matching the fields read by the cache lookup. It also initializes a separate sentinel-list-like allocation at cache offset `0x50` through `InitializeWorldDecorationCacheSentinelList`; the helper allocates `0x48` bytes and self-links three list pointers, though the element type and flag meanings remain unknown. The paired `DestroyWorldDecorationCacheSingleton` invokes a virtual destructor, frees the cache through `0x14298C7F0`, and clears the global. This establishes singleton create/destroy mechanics for the cache, but not when the app-system framework calls them or what world/level transition triggers that lifecycle.

A second factory-like function, now labeled `AllocateInitializedCWorldDecorationCache` at `0x141F66F90`, makes the same `0xB0`/`0x10` allocator request and calls the same initializer, but returns the initialized pointer without writing global `0x1417FDDF8`. No callers are recovered in this partial snapshot. This behavior is compatible with a framework-owned instance factory alongside the explicit singleton path, but its connection to the `CAppSystemCreator<CWorldDecorationCache>` table is not proven; retain both construction paths as separate mappings until the dispatch edge is recovered.

`FindOrCreateWorldDecorationCacheEntry` compares the supplied key with entries in a global vector based at `0x1417FDDF8` (begin/end fields at offsets `+0x10/+0x18`) using a `0x50`-byte record stride. A key match exits without the cache-miss work. On a miss, the function uses a supplied virtual interface through several methods, handles a pointer-like result and associated data, then appends a cache record. The control flow supports the cache/lookup interpretation; the full record schema, source-interface type, and meaning of the byte payload remain unknown.

The saved decompilation narrows the record shape without resolving its domain meaning. The lookup compares a string-like key by its stored length and character bytes. On a cache miss, the supplied interface is called through vtable offsets `+0x48`, `+0x30`, `+0x38`, and `+0x18`; these are preserved as raw dispatch offsets because the method contracts are not recovered. The append path copies the key plus fields beginning at record offsets `+0x28` and `+0x30`, then advances the vector end by `0x50`. The capacity helper now has behavior-based labels `EnsureWorldDecorationCacheEntryCapacity` (`0x142095B10`) and `ReserveWorldDecorationCacheEntryVector` (`0x142095870`); the first grows the count/capacity by roughly 1.5x when needed, and the second reallocates/copies entries using the fixed `0x50` stride. This remains a resource-cache data structure, not evidence of a live `CGameObject` or Havok-body record.

The resource handle's control-block construction is particularly well anchored. `InitializeBoostControlBlockForDecorationResource` at `0x14209DB10` allocates `0x18` bytes, initializes two counters to one, writes vptr `0x1412A8230`, and stores the managed pointer. The vptr's preceding COL at `0x1414D1BF8` points to TypeDescriptor `0x1417AB210`, whose name is `boost::detail::sp_counted_impl_p<CWorldDecorationResource>`. This confirms Boost shared ownership for this resource path. It does **not** make this resource a `CGameObject`, prove linkage to `settings/worlddecorations.bin`, or reveal how cached data becomes a streamed world decoration or Havok body. The static path string still has no direct code xref in the inspected executable.

This is a separate resource/cache pipeline from `CGameObjectCreator<T>` allocation: the former keys and caches resource bytes before returning a shared resource handle; the latter's sampled creator entries request an instance-sized allocation and initialize game-object instances. No verified edge currently connects a cached world-decoration record to a live `CGameObject`, `CMapStreamer` transition, or physics-world insertion.
