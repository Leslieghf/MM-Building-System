# Spawn Capability: First Construction Vertical Slice

> **For agentic workers:** Treat this as an evidence-gated implementation plan. Do not guess native layouts, execute unverified calls, launch the game, or add broad SDK bindings before the gates below pass.

**Date:** 2026-10-27  
**Goal:** Establish a reversible, development-only spawn → observe → despawn slice for one existing Mad Max asset, then make it available as typed Bevy ECS intents/results.  
**Architecture:** Existing Rust ASI and Bevy schedules remain the host. A private native adapter owns all pointers, native strings, descriptor construction/destruction, and engine calls. ECS carries only owned, validated request/result data.  
**Reference audit:** [`docs/research/2026-10-27-mm-sdk-rust-port-audit.md`](../../research/2026-10-27-mm-sdk-rust-port-audit.md)

## Constraints

- Keep the user's Rust review preference: expose each change as a small diff with rationale and evidence; do not bury substantial implementation in generated code.
- Keep the game closed during build/deploy and never launch it. In-game observation is user-controlled.
- No save/persistence work until the spawn lifecycle and stable asset identity are demonstrated.
- No raw pointers, native C++ strings/containers, or ABI structs in Bevy components/resources.
- Do not call the SDK's direct handler singleton path; it contains distribution-specific/underspecified addresses.
- Fail closed for unsupported executable builds and missing signature/ABI evidence.
- Do not add or run tests unless the user asks. Use formatting/build checks when code is ready.

## Gate 0 — Establish a valid spawn recipe

Before adding a native call, identify one harmless existing asset and gather evidence for:

- its exact spawn/entity-file key and required resource set(s);
- all required `SSpawnInstanceInfo` fields and their valid defaults;
- correct Steam constructor/destructor and `SetEntityFile` ABI for the supported executable;
- `SpawnSystemSpawn` calling convention, argument layout, return meaning, and game-thread requirements;
- a reliable completion signal and stable identity usable for despawn;
- a safe cleanup path if the object is created but observation or UI reporting fails.

Use game data, existing verified community source, and executable-specific disassembly/inspection as appropriate. The SDK headers alone do not pass this gate. If a requirement remains unknown, report the evidence gap and continue source research rather than guessing in live memory.

## Task 1 — Add a typed ECS request/result model

**Files:** `src/runtime.rs`, `src/modding.rs` (or a dedicated spawn model module after inspecting current structure).

- Define owned types for a request ID, validated asset key, transform, stable placement ID, lifecycle state, and outcome/error.
- Keep request states explicit: queued, rejected, dispatching, pending observation, spawned, despawn requested, despawned, and failed.
- Bound the number of accepted requests and the work performed per host callback; overflow returns a visible result instead of blocking.
- Keep the first capability inaccessible to normal users; provide only a development trigger after the native adapter is ready.

## Task 2 — Encapsulate native descriptor lifetime

**Files:** new narrow native adapter module and the existing build-specific address profile.

- Add a supported-build profile entry only after confirming exact ABI/address/signature evidence.
- Allocate `SSpawnInstanceInfo` through the game's native constructor and release through the matching native destructor; do not reproduce its C++ layout in ordinary Rust code unless proven unavoidable.
- Marshal strings and initialize all required fields inside the adapter; make ownership and cleanup explicit on every error path.
- Keep construction/destruction and calls on the verified game callback thread.
- Add signature/target validation consistent with existing project policy; do not confuse executable-memory validation with ABI proof.

## Task 3 — Dispatch and observe one spawn

**Files:** native adapter and game schedule systems.

- Consume at most one validated spawn request per safe post-simulation callback.
- Record the native return value as opaque diagnostic data until its semantics are proven.
- Poll or consume the verified completion mechanism without retaining unowned engine pointers.
- Publish an owned result event only after positive evidence that the intended object exists.
- On timeout/error, preserve enough state to clean up safely; do not report success from a nonzero return alone unless its semantics are established.

## Task 4 — Despawn and prove repeatability

- Resolve the spawned object through the verified identity path and request despawn on the same safe callback phase.
- Wait for positive despawn observation before marking the request complete.
- Confirm one request produces one object, cleanup removes it, and a second request does not duplicate stale state.
- Add a bounded development status surface for request ID, asset key, lifecycle state, and native error/return diagnostics.

## Task 5 — Build and prepare user-controlled game verification

- Review the diff for ABI assumptions, pointer escape, callback blocking, unbounded queues, and incomplete cleanup.
- Run `cargo fmt --check` and a Windows x64 release build; do not run tests unless asked.
- Do not launch or deploy automatically. Present the artifact and a concise in-game procedure for the user to run when ready.
- Capture the observed spawn, cleanup, and repeatability result before planning persistence.

## Deferred until the vertical slice passes

- Mod-owned per-playthrough construction file and atomic save/load.
- Area/camp/instance readiness and reconciliation.
- Placement UI, previews, snapping, collision checks, recipes, and arbitrary object selection.
- Custom assets, asset injection, independent Rust plugin ABI, or broad SDK parity.
- Replacing native systems or general-purpose cancellation/override mechanisms.

## Completion criteria

The slice is complete only when the supported Steam build can create one identified existing object at a requested transform, positively report it as present, despawn it through a verified path, positively report removal, and repeat without duplicates or leaked descriptor/object ownership. Build success alone is not proof of runtime safety or construction functionality.
