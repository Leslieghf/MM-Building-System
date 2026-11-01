# Selected object inheritance maps from executable RTTI

Date: 2026-11-01  
Program: `/MadMax/MadMax.exe`, image base `0x140000000`  
Method: read-only Ghidra script traversed MSVC `RTTICompleteObjectLocator`, `RTTIClassHierarchyDescriptor`, base-class arrays, and `RTTIBaseClassDescriptor` records for selected TypeDescriptors.

## Findings

These are static RTTI hierarchy records. They establish the executable's declared base-class graph and subobject metadata, not a safe construction/destruction ABI, runtime instance presence, or complete ECS/component model. Repeated complete-object locators for a class share a hierarchy descriptor and likely represent multiple vftables/subobjects; the count below is the base entries in that hierarchy, not an object count.

| Concrete type | RTTI base entries | Hierarchy (derived first) |
|---|---:|---|
| `CRigidObject` | 9 | `CRigidObject` → `CPhysicsGameObject` → `IPfxGameObject` → `CGameObject` → `CRtti` → `NEvent::CEventHandler` → `CPfxRigidBody::IPfxActivationListener` → `hkpEntityActivationListener` → `IPhysicsGameObjectListenable` |
| `CAnimatedRigidObject` | 10 | `CAnimatedRigidObject` → `CRigidObject` → `CPhysicsGameObject` → `IPfxGameObject` → `CGameObject` → `CRtti` → `NEvent::CEventHandler` → `CPfxRigidBody::IPfxActivationListener` → `hkpEntityActivationListener` → `IPhysicsGameObjectListenable` |
| `CCharacter` | 11 | `CCharacter` → `CDamageable` → `CPhysicsGameObject` → `IPfxGameObject` → `CGameObject` → `CRtti` → `NEvent::CEventHandler` → `NCharacterSystem::CCharacterBase` → `IPfxCharacter` → second `CRtti` subobject → `IPhysicsGameObjectListenable` |
| `CCar` | 10 | `CCar` → `CLandVehicle` → `CVehicle` → `CDamageable` → `CPhysicsGameObject` → `IPfxGameObject` → `CGameObject` → `CRtti` → `NEvent::CEventHandler` → `IPathfindingStepDoneListener` |
| `CWorldDecorationCache` | 3 | `CWorldDecorationCache` → `IAppSystem@Base` → `CAvaSingle<CWorldDecorationCache>` |

## Address anchors

- `CRigidObject` TypeDescriptor: `0x14179E600`; shared hierarchy descriptor: `0x1414B9468`; 5 complete-object locators were observed.
- `CAnimatedRigidObject` TypeDescriptor: `0x1417B07E8`; hierarchy descriptor: `0x1414D60E8`; 5 complete-object locators were observed.
- `CCharacter` TypeDescriptor: `0x141797910`; hierarchy descriptor: `0x1414B1020`; 6 complete-object locators were observed.
- `CCar` TypeDescriptor: `0x1417B7A30`; hierarchy descriptor: `0x1414DAC70`; 4 complete-object locators were observed.
- `CWorldDecorationCache` TypeDescriptor: `0x1417AC798`; hierarchy descriptor: `0x1414D5490`; one complete-object locator was observed.

## Why this matters

The object model is not just a flat list of spawnable classes. Rigid props and vehicles share a `CPhysicsGameObject` / `CGameObject` layer, while cars add vehicle and land-vehicle layers, and characters add damage and character-system bases. RTTI also shows Havok-facing listener interfaces on rigid objects and characters, which are useful leads for investigating physics callbacks and registration. Those interfaces do not by themselves show how a body is created, inserted into a world, or removed.

For next steps, trace the resource/entity creator path for `CRigidObject` and one concrete rigid prop, then separately map the native physics-world association and lifetime. Keep RTTI inheritance, parsed entity assets, and runtime-confirmed object behavior as distinct evidence categories.

The saved decoded-code snapshot provides a concrete layout cross-check for the event-handler hierarchy entry: `InitializeCRigidObjectInstance` calls a helper at byte offset `+0xD8` that writes vptr `0x141347040`. The vptr's COL resolves through TypeDescriptor `0x141792908` to `.?AVCEventHandler@NEvent@@`. The enclosing initializer then installs its `CRigidObject` vptr at that same offset; that vtable's COL identifies complete-object offset `0xD8`. This confirms base initialization followed by the concrete object's vtable at that subobject location, but it does not map event dispatch or registration semantics.

The helper has 64 direct callers in the Ghidra saved snapshot, though the caller list is partly truncated and the callers remain unnamed. This supports prioritizing the shared event-handler base as a common construction seam; it is not a count of events or registrations.

The static `NEvent::CEventHandler` vftable at `0x141347040` has two observed pointer-sized entries, `0x1429918A0` and `0x143039550`, both inside non-executable `.xpdata`. The first is a one-byte bad-data stub; the second is not defined as a function. These protected targets are leads only; their slot meanings, callable bodies, and event contracts remain unknown.

The separately mapped native action-handler registration path is documented in [native-action-handler-path-2026-11-01.md](native-action-handler-path-2026-11-01.md). Special-camera initialization uses both the `NEvent::CEventHandler` subobject and action subscriptions; do not collapse them into one event model without tracing the dispatch edge.

## Construction evidence from the saved runtime slice

Static RTTI declares `CAnimatedRigidObject` as a `CRigidObject` subclass. The saved decoded-code snapshot confirms the construction sequence: its creator entry calls an initializer that first invokes `InitializeCRigidObjectInstance`, then installs the animated class's five vtable address points at subobject offsets `0`, `8`, `0xD8`, `0xE8`, and `0xF0`, and initializes additional fields. The concrete initializer reaches at least offset `0x414`; the associated creator's third slot returns `0x4A0`.

The same pattern appears for `CScaledRigidObject`: its initializer calls `InitializeCRigidObjectInstance`, writes a separate vtable set, and sets additional defaults. Its creator's third slot returns `0x2B0`; `CRigidObject`'s returns `0x2A0`. This supports a shared rigid-base construction path with fixed creator-reported sizes. The scaled type's RTTI ancestry was not recovered in this slice, so the initializer call alone is not recorded as a complete RTTI hierarchy.

The creation entries also construct a shared-pointer-like result around the object. The exact allocation helper, control-block type contract, required resource state, world insertion, and safe destruction/removal order remain unknown. These constructor paths are not exposed through the mod API.

## Creator dispatch lead: `CGameObjectCreator<CRigidObject>`

The creator's MSVC TypeDescriptor string is at `0x1417A1128` (TypeDescriptor base `0x1417A1118`). Its RTTI complete-object locator is at `0x1414BFDD0`, and the referenced vtable address point begins at `0x14128C840`. The next complete-object locator appears at `0x14128C858` and identifies `boost::detail::sp_counted_impl_p<CRigidObject>`, bounding the creator vtable to three entries: `0x141F789B0`, `0x141FB4120`, and `0x141F69250`. The first two are exact PE unwind-function starts (`0x141F789B0..0x141F789D9` and `0x141FB4120..0x141FB41CD`); the third has no exact unwind entry and remains a one-byte Ghidra stub. A raw read at `0x141F69250` yields non-instruction-looking bytes (`4A E6 02 A3 7E 76 66 33 ...`).

The same data region contains a distinct adjacent three-entry `CGameObjectCreator<CScaledRigidObject>` table at `0x14128C890`, bounded by its own RTTI records. Its first two targets also have exact PE unwind ranges; its third remains unresolved. Earlier broad pointer counting merged these adjacent table groups, so the addresses `0x141F69290`, `0x141F692B0`, `0x141F69320`, `0x141F69360`, and `0x141F69380` are not documented as methods of the `CRigidObject` creator.

This ties a concrete rigid-object creator type to a three-entry dispatch table, with two entry boundaries confirmed by unwind metadata and all actual method semantics still unknown. The `.xpdata` mapping, runtime decode path, and callable ABI remain unresolved. Do not call these pointers or rename them as constructor methods based on this evidence alone.

