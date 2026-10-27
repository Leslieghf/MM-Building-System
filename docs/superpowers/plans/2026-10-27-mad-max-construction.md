# Mad Max Construction Mod Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Mad Max-specific ScriptHook-style foundation and prove a repeatable place → persist → reload → restore loop for one existing object. The first implementation checkpoint proves only that a Rust ASI plugin loads and fail-closes on an unsupported build.

**Architecture:** A Rust ASI plugin is the in-process bootstrap and gradually grows a narrow, build-gated Mad Max API. First it only fingerprints/logs the game; later, verified hooks provide safe copied capabilities. A headless Rust/Bevy sidecar and versioned IPC are deferred until one useful game capability is proven. Mod-owned data remains outside the native save. Work stops at evidence gates if safe spawn, area readiness, or playthrough identity cannot be established.

**Tech Stack:** Rust Windows `cdylib` and a compatible ASI loading path for the first checkpoint; later, optionally Bevy App/ECS without rendering plus versioned local IPC. Exact loader proxy, hook backend, IPC transport, serialization, and dependency versions are selected only after evidence supports them. Do not install tooling/dependencies before user review or add dependencies before they are needed.

**Spec:** `docs/superpowers/specs/2026-10-27-mad-max-construction-design.md`

## Global Constraints

- Keep construction records outside the native Mad Max save file.
- Scope each placement set to one playthrough so switching saves cannot silently mix or lose buildings.
- Build the in-process plugin as a Mad Max-specific ScriptHook layer, not as a claim that Ultimate ASI Loader provides game APIs.
- Defer the sidecar/Bevy runtime until the in-process plugin exposes at least one useful, verified capability.
- Keep authored project code in Rust where practical. The user writes it, or reviews it line by line before accepting it.
- The sidecar and bridge communicate over a versioned IPC protocol, not a shared Rust ABI.
- Do not generate or accept a broad runtime API before the first capability is proven.
- Native save editing, camps/instances, custom assets, and polished placement UI are out of scope for this milestone.

## Review Focus

- Unknown or mismatched game executable: hook install must fail closed; pin this in the build-detection task.
- Bridge/runtime disconnect or stale queued command: no command may execute after disconnect or against a later game session; pin this in IPC/lifecycle tests.
- Malformed, unsupported-version, or out-of-range message data: reject before game calls; pin this in protocol tests.
- Duplicate area-ready events and repeated restore: reconcile by stable placement ID; pin this in persistence/runtime tests and in-game re-entry verification.
- Missing or ambiguous playthrough identity: require explicit profile selection or stop; pin this in identity/storage tests.

---

## File and Component Map

The project folder currently contains design/research documentation only. Preserve it. The first Rust workspace files may be added only after the revised design and bootstrap plan are reviewed, and only when the user is ready to install the toolchain and loader.

- `docs/superpowers/specs/2026-10-27-mad-max-construction-design.md` — approved design and capability boundaries.
- `docs/research/feasibility.md` — executable fingerprint, loader/runtime evidence, spawn/thread/world-ready findings, playthrough identity evidence, and gate decision.
- `Cargo.toml` — workspace membership and shared lint/profile settings after crate targets are known.
- `crates/mm-protocol/` — versioned message types and validation shared conceptually by the bridge and sidecar; no game pointers or engine objects.
- `crates/mm-runtime/` — headless Bevy schedule, IPC session, mod-owned construction storage, reconciliation and logs.
- `crates/mm-bridge/` — Rust `cdylib` ASI entry point, build gate/logging, and later narrow game-specific hook/call adapters.
- `crates/mm-launch/` — optional command-line development launcher only if needed to pair sidecar and game reliably; do not create it unless the lifecycle investigation requires it.
- `docs/protocol.md` — wire version, message schemas, validation rules, connection lifecycle and compatibility behavior.
- `docs/manual-test.md` — exact manual install, rollback, test-save, area re-entry, and log collection procedure.

## Task 1: Establish the Feasibility Gate

**Files:**
- Create: `docs/research/feasibility.md`
- Read-only inspect: installed Mad Max executable and existing project/community tools; do not patch game files or modify saves.

- [x] Record the available store/distribution, executable path, file version, and cryptographic fingerprint. If there is no single identifiable installation, stop build-specific work and ask the user to select the target.
- [x] Inventory the existing loader and mod installation path. Record loader entry-point constraints, supported executable evidence, and proxy/ASI conflicts without installing anything. Note: Steam PE has no .text section (.xpdata/.xtext instead), matching recent scanner failures; MadMaxFix uses the matching timestamp, known addresses and post-decryption byte checks.
- [ ] Trace evidence for one candidate existing static structure: asset/object identifier, any known creation/destruction path, ownership/lifetime, and what is still unverified. Cite source files/addresses and distinguish observation from inference.
- [ ] Determine whether a usable area-ready callback and safe game thread are evidenced for the target build. If no evidence supports both, record the blocker and stop before implementation.
- [x] Investigate available active-save/playthrough identity. Header offset `0x18` provides a playthrough creation timestamp and `0x48` a slot number; local read-only inspection found distinct timestamps in slots 1, 2, 3, and 10 and matching timestamps in both available backup copies. Use explicit profile selection because automatic active-slot discovery is not established; do not guess based on newest-file time.
- [ ] Record a concrete rollback-safe manual experiment and a safe test location. No placement or hook experiment is run until bridge behavior and rollback are reviewed.
- [ ] Write a gate result: **pass** only if loader/build identification, a plausible safe spawn path, an area-ready/thread path, and a safe playthrough profile strategy are supported; otherwise **stop** with the specific evidence gap.

**Exit check:** Review the evidence and choose whether to stop or proceed only with the reversible ASI bootstrap checkpoint. The bootstrap can proceed without spawn/thread proof, but it does not pass the construction feasibility gate or authorize gameplay hooks.

## Task 2: Prove a Minimal Rust ASI Bootstrap

**Files:**
- Create: `Cargo.toml`
- Create: `crates/mm-bridge/Cargo.toml`
- Create: `crates/mm-bridge/src/lib.rs`
- Create: `crates/mm-bridge/src/build_id.rs`
- Create: `docs/manual-test.md`

- [ ] Before installation, show the user the selected loader/proxy, file hashes, install path, possible conflicts, rollback steps, and exact Rust toolchain needed. The user must be told to keep the game closed during installation.
- [ ] Build a minimal Rust Windows `cdylib` exporting only the ASI loader's required entrypoint; write a startup log and check the exact supported executable fingerprint.
- [ ] Perform no hooks, memory patches, save writes, or gameplay operations. Unknown builds must log refusal and stop.
- [ ] Ask the user to review every Rust line before accepting or running the plugin.
- [ ] Load it on the target Steam build, verify the expected startup log, then remove it and the proxy/restore any renamed original file.
- [ ] Verify the unmodified game still starts; record exact rollback and any loader conflict.

**Exit check:** User reviews the complete Rust diff and startup/rollback evidence. This proves plugin loading only; it does not authorize game hooks or count as a construction capability.

## Task 3: Establish the First Useful Game Capability

**Files:**
- Modify: `docs/research/feasibility.md`
- Modify: `crates/mm-bridge/src/` only after evidence identifies a candidate capability

**Interfaces:**
- First prefer a read-only world/area signal with a clear callback/thread contract.
- A spawn operation is not considered a usable capability until object identity, ownership/lifetime, position/transform conventions, and safe cleanup are understood.

- [ ] Inspect community source/data and the target build to identify a specific hook/function and its initialization/decryption timing.
- [ ] Record exact signatures/addresses, expected bytes, calling convention, thread, lifecycle, and evidence source. Avoid generic `.text` scans.
- [ ] Draft a tiny read-only hook spike first; fail closed on unknown executable/signature and include logs.
- [ ] Explain every Rust line and unsafe/FFI/hook boundary; user reviews before accepting or running it.
- [ ] If no safe callback is supported by evidence, stop and record the blocker rather than broadening the API.

### First meaningful read-only game signal: player position

- The user chose a live player-position readout as the next milestone; the display-mode-count hook is not accepted and remains uninstalled.
- Candidate callback: `CPlayer::UpdateController(void* this, float dt)` from Wasteland Storms source. It is game-specific and runs regularly, but its published GOG-derived signature is not proven on the target Steam build and the source does not expose player coordinates.
- Community lead: M4 author `u/VespucciBoy` publicly reported locating Max's host position/transform and later achieving stable guest tracking. Their public posts do not provide source, offsets, or build details, so this is evidence that the capability is feasible, not an implementation contract.
- Research gate: establish the Steam callback match and the player-transform field/coordinate convention before implementing a readout. The implementation and user workflow must not depend on Cheat Engine; use public mod source and our own read-only inspection of the game build.
- The current Wasteland Storms source scans only `.text`, even when it defers its scan until after Steam startup. Since this Steam executable has no `.text`, do not treat that source's GOG signature as a Steam-ready hook.
- Prototype behavior, once supported: copy only validated XYZ values to the existing overlay while the open world is confirmed active; hide stale values in menus/loading/unknown states. No memory writes, teleportation, or save access.
- Fail closed on unsupported executable, ambiguous/missing callback match, unreadable player data, or unknown world state. The user reviews every Rust line before installation. Tell the user to launch a loaded save and walk in the open world only after the approved build is installed.

**Exit check:** At least one read-only capability is demonstrated with a known game-thread boundary before adding IPC or Bevy. If it cannot be established, the project pauses at the documented bootstrap/API finding.

## Task 4: Freeze the Runtime Contract After a Capability Is Proven

**Files:**
- Create: `docs/protocol.md`
- Modify: `docs/research/feasibility.md`

- [ ] Decide whether the proven capability benefits from a separate sidecar; keep logic in-process if IPC adds no value yet.
- [ ] If a sidecar is warranted, select one local IPC transport and document process pairing, endpoint discovery, version handshake, disconnect, and shutdown.
- [ ] Define only messages required for construction: `Hello`, `WorldReady`, `PlaceExistingObject`, and `PlacementResult`, with session IDs and correlation IDs.
- [ ] Specify serialization, limits, coordinate units/orientation, world bounds, error behavior, and stale-session rejection.
- [ ] Have the user review the protocol and every proposed Rust dependency before scaffolding.

**Exit check:** The user reviews the bridge/sidecar boundary and any new dependencies before protocol or Bevy code begins.

## Task 5: Create the Protocol and Optional Sidecar Runtime

**Files:**
- Create: `crates/mm-protocol/Cargo.toml`
- Create: `crates/mm-protocol/src/lib.rs`
- Create: `crates/mm-protocol/src/message.rs`
- Create: `crates/mm-protocol/src/validation.rs`
- Create: `crates/mm-runtime/Cargo.toml`
- Create: `crates/mm-runtime/src/main.rs`
- Create: `crates/mm-runtime/src/app.rs`
- Create: `crates/mm-runtime/src/session.rs`
- Create: `crates/mm-runtime/src/construction.rs`
- Create: `crates/mm-runtime/src/store.rs`

**Interfaces:**
- Consumes Task 4 protocol messages.
- `ConstructionStore::load(profile: &WorldIdentity) -> Result<ConstructionSet, StoreError>`.
- `ConstructionStore::upsert(profile: &WorldIdentity, placement: Placement) -> Result<(), StoreError>`.
- `ConstructionStore::remove(profile: &WorldIdentity, id: PlacementId) -> Result<bool, StoreError>`.
- The Bevy schedule receives validated protocol events and emits only validated protocol commands; no Bevy renderer/window plugins are installed.

- [ ] Review profile separation, missing/corrupt/version-mismatched file handling, atomic replacement recovery, upsert/removal idempotence, and duplicate `WorldReady` reconciliation. Do not add or run automated tests unless the user asks.
- [ ] Implement a versioned mod-owned construction file with atomic write/replace. Never write to the native save path.
- [ ] Implement a headless Bevy app update loop and connection state. On bridge disconnect, stop issuing commands and discard commands associated with the expired session.
- [ ] Present the complete Rust diff, explain every changed line, and wait for the user’s line-by-line review before accepting it.

## Task 6: Implement One Narrow Game Operation

**Files:**
- Create: `crates/mm-bridge/Cargo.toml`
- Create: `crates/mm-bridge/src/game_thread.rs`
- Create: `crates/mm-bridge/src/object_spawn.rs`
- Create: `crates/mm-bridge/src/ipc_client.rs`

**Interfaces:**
- Consumes only `docs/protocol.md` messages and evidence-backed addresses/signatures recorded in Task 1.
- `detect_supported_build(fingerprint: &ExecutableFingerprint) -> Result<SupportedBuild, UnsupportedBuild>`, where `ExecutableFingerprint` contains the executable SHA-256 and observed file version.
- `GameContext { session_id: SessionId, world_identity: WorldIdentity, area_id: AreaId }` is the copied, current context received from the bridge; `ValidatedSpawn { session_id: SessionId, placement_id: PlacementId, object_id: ExistingObjectId, transform: Transform }` is the only game-thread spawn request.
- `validate_command(command: &PlaceExistingObject, context: &GameContext) -> Result<ValidatedSpawn, BridgeError>`.
- Game-specific unsafe calls remain private to the narrow adapter module; safe functions receive validated copied data.

- [ ] Implement the smallest loader entry point and IPC connection supported by the approved backend. Do not add generalized hooks, reflection, or broad game APIs.
- [ ] Queue validated commands to the verified game-thread callback; never call game functions from the IPC worker thread.
- [ ] Implement one evidence-backed existing-object spawn and safe removal/cleanup path. If cleanup or ownership is not understood, fail the feasibility gate rather than ship an unremovable object.
- [ ] Present the complete Rust diff with a line-by-line unsafe/FFI/hook explanation. The user explicitly reviews and accepts every line before any in-game experiment.

## Task 7: End-to-End Placement, Persistence, and Manual Verification

**Files:**
- Modify: `docs/manual-test.md`
- Modify only the runtime/bridge files needed to complete the approved path.
- Create: sample logs under `artifacts/` only if the user approves keeping test artifacts in the project.

- [ ] Start the sidecar and game using the documented pairing method; verify handshake reports matching protocol and game fingerprint.
- [ ] Trigger one existing structure at the documented safe location; verify the bridge reports success and the sidecar writes one placement record to the selected profile.
- [ ] Leave and re-enter the area. Deliver duplicate `WorldReady` events in a controlled test and verify exactly one structure exists at the saved transform.
- [ ] Remove the placement, reload the area, and verify it does not return.
- [ ] Switch to a distinct playthrough/profile and verify the first profile’s placement is not visible.
- [ ] Compare native save file hashes/metadata before and after the mod operation; confirm the mod did not modify the native save.
- [ ] Exercise bridge exit, sidecar exit, and game exit. Confirm stale commands are not replayed and logs identify the failed session.
- [ ] If all gates pass, record exact executable fingerprint, test steps, logs, rollback, and known limits in `docs/manual-test.md`. If any gate fails, stop and record the concrete technical finding; do not claim the feature works.

## Task 8: Review and Handoff

- [ ] Review all Rust diffs against the design’s authorship rule; list every unsafe block, hook, FFI boundary, and persistence write path.
- [ ] Capture the manual bootstrap/build/game evidence recorded by the tasks; do not claim success without output. Automated tests are not added or run unless the user asks.
- [ ] Check that no native save changes, unsupported build hooks, camp support, custom assets, or unrelated APIs slipped into scope.
- [ ] Deliver the implementation status, evidence, unresolved risks, and next smallest step. No release or community outreach is implied by this plan.
