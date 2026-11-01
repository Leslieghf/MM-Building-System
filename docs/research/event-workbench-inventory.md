# Event Workbench Inventory

This is the authored inventory shown by `examples/scripts/event_workbench`. It records mapped outward behavior and unresolved boundaries; it is not a claim that arbitrary events can be called safely.

## Confirmed action registrations

`CSpecialCameras` registers twelve literal names: `__specialcamera_freefly`, `__specialcamera_cancel`, `__specialcamera_movie`, `__specialcamera_movie_reset`, `__specialcamera_soakfly`, `__specialcamera_debug`, `__specialcamera_debug_south`, `__specialcamera_debug_east`, `__specialcamera_set_transform`, `__specialcamera_input_enable`, `__specialcamera_input_disable`, and `__specialcamera_toggle_follow_terrain`. Two additional handler records are registered by hashes `0x6527BBC9` and `0x2382A6C0`; their names are unknown.

## Confirmed shared action machinery

Mapped functions include `RegisterActionByHash`, `RegisterActionHandlerKeys`, `FindOrCreateActionRegistryEntry`, `AddActionSubscriber`, `RemoveActionSubscriber`, `RemoveActionHandlerSubscriptions`, and `RefreshActionHandlerSubscriptions`. The dispatch path includes `QueueOrDispatchActionEvent`, `DispatchActionPayload`, and `InvokeActionSubscribers`. The last calls subscriber vtable slot 1 with the owner, registry record, and game payload pointer.

Three publisher shapes are known: `DispatchNamedActionEvent` (name-keyed payload), `QueueNamedActionWithU32Data` (name plus copied 32-bit payload), and `PublishNamedActionWithTransform` (transform payload). Registration/dispatch semantics are mapped at a useful structural level, but payload schemas, object lifetime, thread timing, and callback re-entrancy are not sufficient for a safe script-facing event API.

## Unresolved RTTI leads

The executable contains `CEventRouter`, `CGameEventHandler`, `CReceiveEventsTrigger`, `CEventStealthRouter`, `CFreeEventDataQueue`, `CInputActionOverride`, and `EventSystem`. Their names make them useful research leads. No direct link from these RTTI types to the shared hashed action registry is established here.

## Provenance

The inventory is derived from [gameobject-transform-and-camera-follow-2026-11-01.md](gameobject-transform-and-camera-follow-2026-11-01.md), particularly its special-camera registrations, shared action registry, and dispatch/publisher sections. The workbench package intentionally shows no observation or dispatch controls.
