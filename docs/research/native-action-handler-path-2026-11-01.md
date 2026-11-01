# Native action-handler registration path

Date: 2026-11-01

Static program: `/MadMax/MadMax.exe`

Decoded functions: saved historical snapshot `/runtime_snapshots/MadMax_xpdata_runtime_slice_141C00000.bin`

## Confidence and boundary

This note maps a decoded registration/subscription path, not a complete input API. The snapshot is historical, not the current game process. No action was invoked, no input event was synthesized, and no keybind UI or key-to-action mapping was recovered. Function labels are analyst labels. Keep the native action registry separate from `NEvent::CEventHandler`: `InitializeSpecialCameras` initializes both structures, but that alone does not prove the two systems share dispatch.

## Special-camera action keys

`InitializeSpecialCameras` (`0x1422A5930`) initializes a special-camera object, then registers the following exact string keys through `RegisterActionByHash` (`0x1429922A0`). The strings are referenced from the static pointer table at `0x1417B95C0`–`0x1417B9620`; their decoded values are below. `handler slot` is the pointer passed as the first registration argument, expressed as an offset from the special-camera object. `raw mask` is the third registration argument; its meaning is not decoded.

| Action string | Handler slot | Raw mask |
|---|---:|---:|
| `__specialcamera_cancel` | `+0xF` | `0xFFFFFFFFFF` |
| `__specialcamera_freefly` | `+0x2` | `0xFFFFFFFFFF` |
| `__specialcamera_movie` | `+0x3` | `0xFFFFFFFFFF` |
| `__specialcamera_movie_reset` | `+0x9` | `0xFFFFFFFFFF` |
| `__specialcamera_soakfly` | `+0x4` | `0xFFFFFFFFFF` |
| `__specialcamera_debug` | `+0x5` | `0xFFFFFFFFFF` |
| `__specialcamera_debug_south` | `+0x6` | `0xFFFFFFFFFF` |
| `__specialcamera_debug_east` | `+0x7` | `0xFFFFFFFFFF` |
| `__specialcamera_set_transform` | `+0xE` | `0xFFFFFFFF04` |
| `__specialcamera_input_enable` | `+0xC` | `0xFFFFFFFFFF` |
| `__specialcamera_input_disable` | `+0xD` | `0xFFFFFFFFFF` |
| `__specialcamera_toggle_follow_terrain` | `+0x8` | `0xFFFFFFFFFF` |

Two additional action registrations in the same initializer use hard-coded hashes `0x6527BBC9` at slot `+0x10` and `0x2382A6C0` at slot `+0xB`; their string names were not recovered. The nearby string `Total` at `0x1412E3A30` is not passed through the registrations observed in this function.

## Registration and subscription flow

`RegisterActionByHash` takes a handler pointer, a 32-bit action hash, a raw mask, and three byte-sized options. It converts the hash into a 16-byte local descriptor through `0x1429B68D0`, then calls `RegisterActionHandlerKeys` with a count of one. The descriptor-conversion helper and option semantics remain unresolved.

`RegisterActionHandlerKeys` caps the requested key count at `0x80`, asks `FindOrCreateActionRegistryEntry` for each key, collects the resulting 16-bit registry-slot indices, and appends those indices to the handler's local key list. The shared registry is reached through global `0x1417F4ED8`; a slot record is addressed as `global + 0x80 + slot_index * 0x18` in the decoded subscription paths.

`FindOrCreateActionRegistryEntry` shows that the global contains a bucketed hash registry. It indexes a 16-bit bucket array at `global + 0x60` using the key modulo a bucket count stored at `global + 0x70`; bucket entries chain through 16-byte records starting at the pointer stored at `global + 0x68`. A matching entry carries a 16-bit slot ID at offset `+0xA`; this selects the corresponding 0x18-byte subscriber record at `global + 0x80 + slot_id * 0x18`. The helper increments a field in that record when reusing an existing key. On an allowed miss, it takes a slot from a free-list-like structure, initializes the record, and inserts a hash entry. The helper refuses a new entry when the count at `global + 0x78` exceeds `0xFBFF`. The table/record domain fields and hash algorithm are not fully resolved, but the bucket, chain, slot, and stride arithmetic are directly visible in the saved decompilation.

`RefreshActionHandlerSubscriptions` first removes the handler's old subscriptions. It then walks its recorded key/slot IDs, finds the corresponding registry records, adds the handler as a subscriber, and appends the active slot indices. `RemoveActionHandlerSubscriptions` performs the reverse operation. `AddActionSubscriber` stores handler pointers in a per-slot subscriber list; the implementation includes a capacity-growth path. Some subscription functions end at unreadable/bad instruction data in this snapshot, so the entire dispatch/cleanup contract is not recovered.

## What this establishes

- Mad Max has a native hashed action registration path that is distinct from raw key polling.
- A complex native system (`InitializeSpecialCameras`) registers multiple named actions and refreshes/removes subscriptions through reusable helpers.
- The engine maintains action-slot records and per-slot subscriber lists; handlers can refresh and remove their subscriptions.
- This gives a focused next reverse-engineering path for input routing: resolve the hash/key descriptor, trace a known action from registry through its subscriber to the event delivery callback, and identify how bindings map physical controls into action IDs.

It does **not** establish that mods can add actions, that actions are exposed in Mad Max's keybind menu, or that a registered action can suppress native gameplay input. Those require tracing the physical binding lookup and dispatch/consumption phase. No function in this map is safe to call from Rust yet.
