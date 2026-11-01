# Static Prop Placement and Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver a one-object, reversible test of whether Mad Max can instantiate a shipped static-prop entity through its game-owned spawn system, with script-owned controls and safe cleanup only if its runtime identity is proven.

**Architecture:** First classify a concrete asset and verify the spawn lifecycle using the local game build, SDK source, and Ghidra. Then extend the existing bounded ECS request/observation path and Rhai script-panel API; native work remains inside the exclusive post-simulation callback. If cleanup identity cannot be proven, ship placement observation only and keep removal disabled rather than guessing at a handle.

**Tech Stack:** Rust, Bevy ECS, Rhai, ImGui script panels, Mad Max Steam x64 native SDK adapter, Ghidra.

**Spec:** `docs/superpowers/specs/2026-10-31-static-prop-lifecycle-design.md`

## Global Constraints

- Spawn at most one outstanding request and one live test instance.
- Use only a fixed shipped asset confirmed to be a standalone prop entity; do not accept arbitrary script-provided asset paths.
- Route native work through the existing exclusive post-simulation callback; never call Mad Max from Rhai or the render callback.
- Do not treat `SpawnSystemSpawn` submission or its opaque return value as proof of creation or as an instance handle.
- Cleanup remains unavailable until a native-generated identity and matching game-owned removal path are confirmed.
- Do not remove map-authored objects, persist placements, edit save files, or allocate Havok objects directly in this slice.
- Preserve existing user changes in the working tree; do not commit, push, install, or close the running game without a safe window.

## Review Focus

- **Map-baked scenery versus standalone `.ee` entities:** only a shipped entity asset that the spawn system can load independently qualifies for the first test.
- **Native identity and ownership:** request IDs are host-local; a failed or ambiguous identity must never reach cleanup.
- **Async completion and callback timing:** queued/submitted/active/failed must remain distinct through ECS and script UI.
- **Entity behavior:** “static” must be verified as a non-vehicle prop and checked for collision; appearance alone is insufficient.
- **Game state and streaming:** no spawn/despawn calls during loading, missing world context, or stale callback overlap.

---

### Task 1: Classify a shipped test entity

**Files:**
- Modify: `docs/superpowers/specs/2026-10-31-static-prop-lifecycle-design.md`
- Create: `docs/research/static-prop-asset-candidates-2026-10-31.md`

**Interfaces:**
- Consumes: the community spawner's known entity keys and the installed game archives.
- Produces: one pinned candidate key plus a classification as standalone entity, encounter bundle, map/chunk-only asset, or unverified.

- [ ] Inspect public spawner catalog entries and local archive/file-list tooling; identify a small non-vehicle candidate rather than infer from its name alone.
- [ ] Record the exact candidate key, source, asset type evidence, and any unresolved dependencies.
- [ ] If no standalone static prop can be verified, stop native placement work and revise the experiment to an explicitly labeled spawnable entity candidate.

### Task 2: Establish spawn observation and cleanup identity

**Files:**
- Modify: `docs/research/static-prop-asset-candidates-2026-10-31.md`
- Modify: `src/api/game/abi/game/spawn.rs` only after verified facts exist.
- Modify: `src/api/game/native/effects.rs` only after verified facts exist.

**Interfaces:**
- Consumes: Task 1's pinned asset candidate and existing `SpawnSystemSpawn` submission path.
- Produces: a verified native-generated spawn identity, positive completion signal, and cleanup signature/ownership rule; otherwise an explicit “placement only” gate.

- [ ] Compare the community SDK declaration and caller with current Rust ABI assumptions; inspect any available runtime snapshots through Ghidra without modifying or restarting Mad Max.
- [ ] Determine whether spawned/despawned/failed handler arrays, native IDs, or another game-owned record can be safely copied at the callback boundary.
- [ ] Do not infer an identity from host request IDs or raw return values.
- [ ] If evidence is insufficient, document the stopping point and omit cleanup implementation.

### Task 3: Add a bounded, script-attributed prop lifecycle request

**Files:**
- Modify: `src/api/ecs/intents.rs`
- Modify: `src/api/ecs/transport/mod.rs`
- Modify: `src/api/ecs/runtime/effects.rs`
- Modify: `src/api/ecs/runtime/mod.rs`
- Modify: `src/api/scripts/manager.rs`
- Modify: `src/api/scripts/manager/invocation.rs`
- Modify: `src/api/rhai/bindings.rs`

**Interfaces:**
- Consumes: verified Task 1 asset key and Task 2 lifecycle observation contract.
- Produces: one typed script effect carrying a request correlation ID and generation-safe script origin; the UI receives queued, submitted, observed-active, failed, removing, or observed-removed status.

- [ ] Add a test proving an invalid asset key or non-finite placement cannot enter the queue.
- [ ] Add a test proving request submission and native completion are separate lifecycle states.
- [ ] Implement only the transition model and adapter fields supported by Task 2 evidence.
- [ ] Ensure unloading/reloading a script cancels stale UI attribution without losing native ownership tracking.

### Task 4: Add the focused `static_prop_lab` package

**Files:**
- Create: `examples/scripts/static_prop_lab/manifest.toml`
- Create: `examples/scripts/static_prop_lab/main.rhai`
- Modify: `src/api/rhai/catalog.rs`
- Modify: `docs/rhai-api.md`
- Modify: `docs/scripting.md`
- Modify: deployed `MadMaxModding/API.md` only when installing a verified build.

**Interfaces:**
- Consumes: Task 3's capability and lifecycle event model, plus existing `ctx.ui` and `ctx.raycast` interfaces.
- Produces: one focused panel with Place and Remove controls, one-instance guard, clear unavailable/no-hit/pending/active/removing/failure states, and optional debug visualization composed from existing script APIs.

- [ ] Add a Rhai-level test that the script registers the panel and does not request placement without a valid hit.
- [ ] Add a Rhai-level test that removal is unavailable unless the backend reports an owned active instance and Task 2 verified removal.
- [ ] Keep all engine gizmos/raycast calls orchestrated by the script rather than embedding them in the native spawn API.

### Task 5: Build and manual in-game validation

**Files:**
- Modify: `docs/research/static-prop-asset-candidates-2026-10-31.md`
- Modify: `docs/research/install-history.md` only if an installation occurs.

**Interfaces:**
- Consumes: the compiled candidate and packaged `static_prop_lab` script.
- Produces: confirmed runtime result for spawn visibility, placement, world-query collision, lifecycle state, and cleanup if enabled.

- [ ] Build the plugin and inspect the exact deployment files before installing.
- [ ] Wait for Mad Max to be closed before replacing the ASI or deployed files; do not close it automatically.
- [ ] In game, place one prop on a clear surface, observe it, raycast it, and remove only that mod-owned instance if the cleanup path passed Task 2.
- [ ] Report outcomes separately: asset loaded, instance observed, collision confirmed, cleanup confirmed.
