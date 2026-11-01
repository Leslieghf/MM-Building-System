# Game object transforms and camera follow

**Date:** 2026-11-01  
**Executable:** supported Steam Mad Max build, SHA-256 `CD719A721DB92575AA32FF01E07A590F47DD34B702B1BCD98EC698A07358BB0D`  
**Analysis source:** Ghidra runtime snapshot `MadMax_xpdata_runtime_slice_141C00000.bin`; the encrypted on-disk `.xpdata` bytes are not the basis for the function-body conclusions below.

## Confirmed object-transform behavior

The SDK's `CGameObject::GetTransform(CMatrix*)` virtual is slot 22 (byte offset `0xB0`) for the inspected vtables. In the supported runtime code:

- The base `CGameObject` implementation returns `false` without filling the output. A base pointer therefore does not guarantee a transform.
- `CTransformObject` slot 22 copies 64 bytes from `this + 0x110` to the caller's output and returns `true`.
- `CTransformLinker` slot 22 copies 64 bytes from `this + 0x128` to the caller's output and returns `true`.
- `CTransformLinker` slot 23 returns `this + 0xD0`. This is a pointer accessor, but its field semantics are not established yet; do not call it a parent getter in an API based only on this evidence.

This establishes a stored-matrix contract for these overrides. It does **not** establish when the matrices are recomputed, whether they are local or world transforms, or how a linked hierarchy propagates updates. In particular, the two matrix offsets differ, so they are not evidence for a common Rust-side object layout.

## Confirmed camera matrix selection

The matrix-selection routine begins at VA `0x14222DA60` (RVA `0x0222DA60`). The current runtime snapshot has a MinHook `E9` jump in its first five bytes, so Ghidra sees a detour there; the original leading instruction is not restored in the current snapshot. The valid instruction stream after that patch begins at `0x14222DA65` and contains the state check, buffer selection, and 64-byte copy described below. A read-only live-memory check confirmed the entry jump and the trampoline route. Conclusions here are based on that intact body, not on decompiling the jump stub.

The function selects one of two 64-byte matrix buffers:

1. It reads a 32-bit state code from `(manager + 0x480) + 0x400`, i.e. manager offset `0x880`.
2. If that code equals `3`, an obfuscated pointer resolver at `0x1421A5010` resolves to `0x142265EF0`. The resolved accessor returns its input plus `0x260`; called with `manager + 0x480`, the source buffer is `manager + 0x6E0`.
3. Otherwise, the source buffer is `manager + 0x270`.
4. The selected 64-byte matrix is copied into the caller-provided output buffer through the memory-copy dispatch thunk at `0x1430394E0`.

The resolver's odd arithmetic is not an arbitrary matrix operation: with the captured runtime value at `0x141D80ED4` and the byte sequence it hashes, it resolves to the small accessor at `0x142265EF0`, whose body is `return this + 0x260`. Ghidra labels in the saved runtime snapshot now identify that state getter and matrix-buffer accessor.

## Confirmed special-camera subobject

RTTI identifies the embedded object at `CCameraControlManager + 0x480` as `CSpecialCameras` (`.?AVCSpecialCameras@@`). The camera-manager constructor at `0x14225E020` constructs it inline at that offset; this is not a separately allocated camera object.

The `CSpecialCameras` constructor at `0x1422A5930` initializes four transform-like records with the same `0x1422365C0` initializer. Their offsets relative to `CSpecialCameras` are `0xC0`, `0x190`, `0x260`, and `0x330` (manager-relative: `0x540`, `0x610`, `0x6E0`, and `0x7B0`). The camera-matrix getter's state-`3` path selects the record at `CSpecialCameras + 0x260`; other modes use the manager's separate buffer at `+0x270`.

The constructor initializes a 32-bit camera-state field at `CSpecialCameras + 0x400` to `0xFFFFFFFF` and a neighboring 16-bit field at `+0x404` to `1`. The state field is the same one read by `GetCameraMatrix`; `0xFFFFFFFF` is therefore the constructor's inactive/sentinel state. A single live read also observed this sentinel, but does not establish every transition or state meaning.

The constructor registers these built-in special-camera action names with the game-side command/action machinery:

- `__specialcamera_freefly`
- `__specialcamera_cancel`
- `__specialcamera_movie`
- `__specialcamera_movie_reset`
- `__specialcamera_soakfly`
- `__specialcamera_debug`
- `__specialcamera_debug_south`
- `__specialcamera_debug_east`
- `__specialcamera_set_transform`
- `__specialcamera_input_enable`
- `__specialcamera_input_disable`
- `__specialcamera_toggle_follow_terrain`

The same constructor associates those names with embedded handler records inside `CSpecialCameras` (offsets below are bytes from the subobject start):

| Action name | Embedded handler record |
|---|---:|
| `__specialcamera_freefly` | `+0x10` |
| `__specialcamera_cancel` | `+0x78` |
| `__specialcamera_movie` | `+0x18` |
| `__specialcamera_movie_reset` | `+0x48` |
| `__specialcamera_soakfly` | `+0x20` |
| `__specialcamera_debug` | `+0x28` |
| `__specialcamera_debug_south` | `+0x30` |
| `__specialcamera_debug_east` | `+0x38` |
| `__specialcamera_set_transform` | `+0x70` (uses a distinct initializer/binder pair) |
| `__specialcamera_input_enable` | `+0x60` |
| `__specialcamera_input_disable` | `+0x68` |
| `__specialcamera_toggle_follow_terrain` | `+0x40` |

Two additional handler records at `+0x80` and `+0x88` are initialized and registered using literal hashes `0x6527BBC9` and `0x2382A6C0`; their action names are not yet identified. These are embedded registration records, not a safe ABI for direct mod calls.

### Shared action registry path (runtime-confirmed)

The special-camera registrations use a shared action-key registry, rather than a camera-specific direct state setter. The relevant code was captured read-only from the live process into the separate runtime-snapshot Ghidra program:

- `RegisterActionByHash` (`0x1429922A0`) builds an 8-byte key from the supplied 32-bit hash and a zero secondary word, then delegates with a one-key list.
- `RegisterActionHandlerKeys` (`0x1429904A0`) resolves each key through `FindOrCreateActionRegistryEntry` and converts the resulting registry record address into a 16-bit slot index (`(entry - registryRecordsBase) / 0x18`). It then appends those indices to the handler record through `AppendActionSlotIndices`.
- `FindOrCreateActionRegistryEntry` (`0x14298AA50`) hashes the 64-bit key into the shared table, returns the matching 0x18-byte registry record when present, and increments a 16-bit reference/count field. On a miss, its create-enabled path allocates a record from the registry free list, stores the key, inserts the bucket mapping, and initializes that count.
- `AppendActionSlotIndices` (`0x1429900D0`) grows the handler's compact index list and appends the resolved slot IDs.
- `RemoveActionSlotId` (`0x1429909E0`) removes a matching 16-bit ID from a handler-owned list by replacing it with the last entry and shrinking the list.

The registration lifecycle also maintains reverse subscriber lists:

- `FindActionSlotIndex` (`0x142982180`) searches an owner's 16-bit action-slot list and returns the matching index or `-1`.
- `AddActionSubscriber` (`0x14298FED0`) increments the registry record's 16-bit reference count and appends the owner pointer to its subscriber array, growing that array when needed.
- `RemoveActionSubscriber` (`0x14298FE40`) clears a matching owner pointer from the subscriber array and decrements the reference count; the zero-reference path returns the record to the shared registry free list.
- `RemoveActionHandlerSubscriptions` (`0x141BAB4D0`) walks a handler's action slots, removes the owner from the corresponding registry records, and removes those slot IDs from its owner list.
- `RefreshActionHandlerSubscriptions` (`0x141BBF680`) first performs that cleanup, then reattaches the owner for handler slots not already present and appends the missing IDs to the owner's list.

In `InitializeSpecialCameras`, this cleanup/register/refresh sequence brackets each named camera-action registration. That is direct evidence for a shared, hashed **action subscription system**: keys resolve to registry slots, and records hold owner subscribers. It is plausibly part of the engine's broader event/input machinery; the dispatch contract is mapped below, while the scope of the system beyond action events remains unknown.

### Action dispatch and payload contract (runtime-confirmed)

The live callback path is now mapped far enough to establish the outward behavior:

1. `HashActionNameTokens` (`0x142982080`) normalizes/tokenizes a name and hashes its token(s) into 64-bit action keys. `BuildActionKeyList` (`0x142990830`) packages key pairs into the action-key list consumed by dispatch.
2. `QueueOrDispatchActionEvent` (`0x142992320`) either dispatches immediately when the manager flag at `+0x51` is set, or posts the event through a ring buffer. The ring uses a monotonically incremented atomic counter at `+0x1C`, capacity at `+0x20`, queue base at `+0x10`, and `0x38`-byte entries.
3. `DispatchActionPayload` (`0x142990D60`) resolves each action key to its registry record, or consumes pre-resolved slot IDs, then calls `InvokeActionSubscribers` (`0x142990040`).
4. `InvokeActionSubscribers` walks the registry record's owner-pointer list and calls each owner's vtable slot 1 with `(owner, registry record, payload pointer)`. It marks the record as dispatching during iteration and defers/compacts removals through its pending-cleanup flag.

This confirms a real game-owned publish/queue/dispatch/subscribe mechanism. The interface is action-name keyed and callback based, but the callback receives a game-defined payload whose meaning depends on the action. It is not yet a safe general-purpose Rust API: object lifetime, thread/timing rules, payload schemas, and callback reentrancy constraints remain unknown.

The camera state machine uses this same mechanism. State 1 and state 4 converge on `PublishNamedActionWithTransform` (`0x14228C8D0`) with the literal action name `__SetFreeFlyTransform` and the 64-byte transform at `CSpecialCameras + 0xC0`. The publisher copies the transform into a game-owned allocation, builds the named action key list, creates a transform-bearing event payload, and passes it to `QueueOrDispatchActionEvent`. This is a proven bridge from those camera update branches into the shared action system.

The `CSpecialCameras` vtable's slot 1 points to `SpecialCameraActionHandlerThunk` (`0x1422AD350`), so the registered camera actions reach a camera-owned handler through the callback mechanism. The thunk jumps through additional obfuscated code outside the captured runtime slice; that handler body has not been semantically recovered. Therefore, the exact mapping from `__specialcamera_freefly`, `__specialcamera_movie`, and the other registered actions to the state field at `+0x400` is still unresolved. This is now a narrow callback-decoding problem rather than an unknown engine mechanism.

### Broader use of the action/event layer

The registry and queue are shared beyond `CSpecialCameras`. In the runtime snapshot, `RegisterActionByHash` has 37 direct callers and `RegisterActionHandlerKeys` has 23; `QueueOrDispatchActionEvent` has 22 direct callers. These counts include wrappers and registration sites, so they are not counts of unique gameplay systems, but they establish multiple independent call paths into the same machinery.

Two generic publisher shapes are now identified from their decompilation:

- `DispatchNamedActionEvent` (`0x141D82930`) takes an action-name string, passes it through `HashActionNameTokens`, builds the 64-bit key list, and queues/dispatches an event payload.
- `QueueNamedActionWithU32Data` (`0x141E4E970`) takes an action-name string plus a pointer to a 32-bit value, copies that value into event-owned storage, builds the key list, and queues/dispatches the payload.

This is stronger evidence for a general name-keyed action/event facility than the camera registrations alone. It still does not prove that arbitrary external strings are accepted safely, that every registered key is user-triggerable, or that the named actions are equivalent to input bindings. Most callers remain unnamed and payload formats vary; each action needs its own subscriber and payload mapping before an API is exposed.

### Runtime-only bytes at the callback dispatch boundary

The callback thunk's branch path was followed using read-only live-memory captures. Its jump stubs converge through `0x1448161C3 -> 0x1445EF1DF -> 0x1448E3A0D -> 0x1447F9423 -> 0x144A60A5B -> 0x14239E344 -> 0x142305FE7`, then continue to `0x142048ECC` and `0x1421D06C8`. At `0x142305FE7` and `0x142233EA0`, the live bytes differ from the static `MadMax.exe` view: live bytes begin with coherent short register-save/jump stubs, while the file view decodes as unrelated instructions. The live-only bytes are captured as named overlays/blocks in the runtime-snapshot Ghidra program; the original executable program was not modified.

Those captures support a runtime-materialization or runtime-patching distinction at these addresses. They do **not**, by themselves, identify whether the cause is decryption, a code virtualization layer, hotpatching, or another loader/runtime transformation. Following the observed live branch path gives `0x142305FE7 -> 0x141F92E4F -> 0x142048ECC -> 0x1421D06C8 -> 0x14226C918 -> 0x142305967 -> 0x1423A1A73 -> 0x14208771A`. The path repeatedly alternates register-save/stack operations, short branch stubs, and instruction sequences that do not resemble an ordinary compiler-generated C++ handler. The next hop has not been semantically decoded, and this trace has not yet reached a camera-state write. The neighboring bytes contain many alternate branch stubs, so a linear sweep is not a valid model of the active path.

Additional live-only ranges on this path are captured as named overlays in the runtime-snapshot Ghidra program (`LiveCode_141F92E4F`, `LiveCode_142048ECC`, `LiveCode_1421D06C8`, `LiveCode_14226C918`, `LiveCode_142305967`, and `LiveCode_1423A1A73`). The first addresses at the path's entry and at `0x142233EA0` have equivalent live captures. This preserves the bytes and branch targets for later analysis without changing the game or replacing the static executable view.

The static RTTI/class inventory also contains native types named `CEventRouter`, `CGameEventHandler`, `CReceiveEventsTrigger`, `CInputActionOverride`, `CEventStealthRouter`, `CFreeEventDataQueue`, and `EventSystem`. Their names make them useful next investigation targets for the game's authored event and input-routing architecture. No direct reference from these types to the hashed action registry has been established yet, so they remain related leads rather than confirmed parts of the same implementation.

`SetSpecialCameraStatusMessage` (`0x1422AE090`) is a separate helper: it copies text into the buffer at `CSpecialCameras + 0x978` and sets the countdown at `+0x9A0`. `UpdateSpecialCameraState` decrements that counter. Its 60-tick call sites are transient status text, not action dispatches or state changes.

This confirms the **registration/indexing layer** and the basic handler-list cleanup operation. It does not yet reveal the callback signature, prove that these action keys are public input bindings, or show which handler changes `CSpecialCameras + 0x400`. In particular, the registration path is not itself evidence that an external mod can activate a camera mode by calling the registry. The `CSpecialCameras` action-to-state transition remains the critical missing link.

The runtime snapshot names above describe internal implementation behavior, not an exposed ABI. Their bodies were captured from the running process because corresponding on-disk `.xpdata` ranges decompile as invalid code in the static `MadMax.exe` project.

Nearby internal labels also include `__SetFreeFlyEnable`, `__SetFreeFlyDisable`, `__SetFreeFlyTransform`, and `__TeleportToCamera`. These names make the existing game-owned free-fly camera a strong path to investigate before attempting to detach the camera from Max. They do **not** yet prove how to invoke it from a mod, which object owns the handlers, or whether any action name is accepted by a public console.

The RTTI and constructor establish the class and inline layout. The destructor call chain is present, but Ghidra's recovered body contains bad control-flow bytes; no destructor semantics beyond member cleanup should be inferred from its partial decompilation. The special-camera dispatch table also has a small number of ordinary function entries followed by adjacent metadata/obfuscated code, so its remaining entries have not been treated as a straightforward Rust-callable vtable.

The special-camera update routine at `0x1422C8740` copies a 64-byte transform from its update context into `CSpecialCameras + 0x80`, snapshots the 0xD0-byte record at `+0xC0` into `+0x190`, then dispatches a state handler using `state + 1` as an index into a nine-entry relative jump table. For state code `3`, the table selects `0x1422C8838`. That handler reads a record at `CSpecialCameras + 0x410`, calls the large update routine at `0x1422B8690`, checks a game/input-state predicate at `0x14227A040`, and conditionally copies a 64-byte transform into the record at `+0xC0`.

This makes state `3` a concrete camera-update branch with a per-frame context and an input-related gate. It is consistent with the named free-fly feature, but the link from `__specialcamera_freefly` to state `3` is not yet traced. Nor have we shown how the state-3 output record at `+0xC0` reaches the getter-selected record at `+0x260`. Those two links are the next reverse-engineering targets before exposing an invocation API.

The jump table currently resolves these state-code paths:

| State | Observed path | Current interpretation |
|---:|---|---|
| `-1` | Copies `0xD0` bytes from a local transform record into `CSpecialCameras + 0xC0`. | Initialization/reset-like path; exact trigger unknown. |
| `0` | Returns through the common update epilogue. | No transform update observed. |
| `1` | Its table target is `0x1422C880F`, one byte after a linear-sweep instruction start. Decoded from that target, it tests the flag at `+0x404`; may call `ApplySpecialCameraMovement` (`0x142279490`), then publishes `__SetFreeFlyTransform` with the record at `+0xC0` through the shared action-event path. | Named transform handoff is confirmed; mapping of incoming public actions to state 1 remains hidden in the obfuscated callback. The overlapping instruction boundary explains why ordinary linear disassembly misreads this path. |
| `2` | Calls the large routine at `0x1422C5C90`, which is over 6 KB and reads a camera configuration/resource, updates several camera-owned transforms and state fields, and uses native settings/hash lookups. | A substantial settings-driven camera mode, but the specific state name and action mapping are not established. It is not yet safe to classify as free-fly or expose as a direct state setter. |
| `3` | Updates from `+0x410`, gated by `CanUpdateSpecialCameraControls` (`0x14227A040`), and may replace the record at `+0xC0` via `UpdateSpecialCameraControl` (`0x1422B8690`). | Input-driven special-camera path; likely relevant to free-fly, but not yet proven to be that command. |
| `4` | Calls `GetRandomSpecialCameraTarget` (`0x1422ADB60`), then shares the `__SetFreeFlyTransform` publish path. | Alternate transform-producing path; helper selects from a 14-entry resource collection, exact mode unknown. |
| `5` | Calls `UpdateSpecialCameraFromSettings` (`0x142279750`), then copies a 64-byte transform into `+0xC0`. | Parameter-driven transform path; command mapping unknown. |
| `6`, `7` | Call `ApplySpecialCameraAxisOffset` (`0x142279910`) with a mode flag derived from the state code. | Paired directional/control variants; command mapping unknown. |


Ghidra's runtime-snapshot program also names the verified construction/cleanup path: `InitializeCameraControlManager` (`0x14225E020`) and `DestroyCameraControlManager` (`0x14225BED0`) own the manager lifetime; they call `InitializeSpecialCameras` (`0x1422A5930`) and `DestroySpecialCameras` (`0x1422A6020`) for the inline subobject. Its repeated transform initializer is named `InitializeCameraTransformRecord` (`0x1422365C0`). The shared registry helpers and subscription lifecycle above were also named in this runtime-snapshot program. These are analysis labels, not exported game APIs. The runtime-snapshot Ghidra program was saved with the names; the main executable program was not changed.

`UpdateCameraControlManager` also exposes useful ordering: during a timed camera transition it selects either the special-camera record or the manager buffer and feeds that transform into its interpolation state; it then updates the camera stack, invokes a list of camera objects through virtual slot 3, and finally calls `UpdateSpecialCameraState`. This establishes the order of those native update calls. It does not yet establish which rendered frame consumes each buffer, so a one-frame handoff delay remains possible but unconfirmed.

## Confirmed follow-modifier behavior

RTTI identifies `CCameraFollowModifier` and `CCameraSpringFollowModifier` as subclasses of `CCameraModifier`. Their runtime vtable slot 4 functions are:

- `CCameraFollowModifier`, `0x14224D0B0` (`CCameraFollowModifier` vtable slot 4): reads camera context and time-step data, keeps positional state on the modifier, applies velocity-like smoothing, and writes the resulting translation into the output transform at `+0x30`, `+0x34`, and `+0x38`; it sets `+0x3C` to `1.0`.
- `CCameraSpringFollowModifier`, `0x14224DBD0` (`CCameraSpringFollowModifier` vtable slot 4): reads the target and camera-context translations, limits follow distance using a modifier field, updates spring/damping state, and writes the output translation in the same matrix positions.

The `CCameraFollowModifier` vtable slot 6 routine at `0x14224D410` reads two typed serialized properties and stores their 32-bit values at modifier offsets `+0x10` and `+0x14`. Those fields are consumed by the slot-4 update as smoothing and damping coefficients. Their serialized identifiers are `0x7EABEA30` and `0xD7123E57`; their original names and units are not yet recovered. This is a much narrower next target than guessing arbitrary camera-manager offsets: map these two property hashes to their authored camera-resource names, then trace where the modifier instance receives them.

The wrapper at `0x14222F540` calls virtual slot 3 on a modifier, copies a 64-byte context transform into the context buffer at `context + 0x80`, then calls virtual slot 4. This supports the interpretation that camera modifiers are processed against a working transform/context. Exact ownership of that wrapper and the complete modifier-stack iteration are still open.

## What this means for camera control

The camera is not simply derived from Max's or the vehicle's `GetTransform` result. The manager getter selects manager-owned camera matrix storage, while the follow modifiers operate on a working transform and retain smoothing state. These pieces strongly suggest a manager-owned camera pipeline, but we have not yet traced the complete call chain proving that the exact buffers selected by `GetCameraMatrix` are the buffers written by those modifiers. A reliable follow-behavior change should target a verified modifier parameter or a traced stage in the modifier pipeline, rather than assuming that changing the player transform or camera getter source changes the follow target.

The existing getter hook is still a useful experimental output boundary, but these findings do not prove that every renderer consumer uses that getter or that overriding its returned copy safely changes the camera's internal state. It should remain described as a bounded output override, not as a detached/free camera.

## Remaining mapping questions

- What does `CTransformLinker::GetTransform` return semantically: local, parent-composed, or world-space matrix?
- What exactly is the `+0xD0` value returned by `CTransformLinker` slot 23, and which code updates/consumes it?
- Which function owns the modifier chain, how are modifiers ordered, and where are the remaining follow/spring parameters initialized from camera resources?
- Which camera matrix buffer is used for gameplay, cutscenes, vehicles, and transitions, and does the manager switch the state code during those modes?
- What is the safe game-thread/lifetime window for changing a modifier parameter?

No game code, installed ASI, or process memory was modified for this mapping. The Ghidra runtime-snapshot program received evidence-backed names for the camera-state getter, selected matrix-buffer accessor, and two follow-modifier update routines.
