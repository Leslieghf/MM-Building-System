# Mad Max Rust Player Transform Feasibility Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Determine whether the community SDK's player-transform path can be implemented as a small, original Rust binding on the user's exact Steam build, and identify the safe callback needed before writing runtime code.

**Architecture:** This is a gated, research-only first plan. It inspects public SDK bindings, the local executable identity, and the current Rust ASI lifecycle, then records evidence and a go/no-go decision. It creates no runtime code; if the address, startup, ABI, or safe-thread evidence remains insufficient, stop and prepare a narrowly scoped follow-on probe plan for user review.

**Tech Stack:** Existing Rust `cdylib` ASI source, PE metadata tools/PowerShell, public GitHub source. No new compiler, dependency, external process-memory tool, test suite, or game installation is part of this plan.

**Spec:** `docs/superpowers/specs/2026-10-27-mad-max-rust-bindings-design.md`

**Result:** `NEEDS_USER_REVIEWED_PROBE_DESIGN` — evidence is recorded in `docs/research/rust-transform-feasibility.md` and reviewed with no material issues.

## Global Constraints

- Keep the runtime Rust-native; do not introduce a C++ bridge or compiler.
- Do not copy, line-translate, vendor, or redistribute `mm_sdk` source/assets; use it as technical reference and independently validate behavior/ABI facts.
- First capability is read-only copied player-transform data; no game-state writes, spawn calls, save writes, Cheat Engine, or external memory reader.
- Do not expose or retain game-owned pointers outside a localized unsafe adapter.
- Do not build or install any candidate until the user has reviewed every Rust source line and explicitly agrees to proceed.
- Preserve the known-working installed overlay/hotkey plugin; do not install a probe in this research plan.
- Do not add or run tests unless the user asks.
- Tell the user before any later game installation or runtime probe; close the game before replacing the plugin.

## Review Focus

- **Target executable mismatch:** Treat only the recorded Steam executable identity as in scope; any other hash/timestamp must end in unsupported/no-go.
- **Encrypted/deferred code:** Static file bytes may not represent executable runtime bytes; do not call a signature confirmed unless the relevant bytes are verified at the correct lifecycle point.
- **Thread affinity:** DX11 rendering and the existing worker do not prove game-thread safety; no transform read may be scheduled there without evidence.
- **ABI/layout drift:** C++ virtual dispatch, object layout, calling convention, and transform representation must be independently supported before Rust calls are proposed.
- **Provenance/reuse:** Keep source references and separate behavioral facts from copied implementation; if implementing requires direct source reuse, stop for a permission decision.

---

## File Map

- Create `docs/research/rust-transform-feasibility.md` — evidence record, source references, exact-build assessment, unresolved risks, and go/no-go decision.
- Read only `src/lib.rs`, `src/overlay.rs`, and `src/game_probe.rs` — understand current entry point and keep the unapproved display-mode source candidate out of any runtime build.
- Read only the local `MadMax.exe` metadata and public source files — no game process access.
- Do not modify any Rust source, Cargo files, installed game files, or save data.

## Task 1: Record the SDK Player-Transform Call Chain

**Files:**
- Read: `https://github.com/gigaHours/mm_sdk/blob/main/game/charactermanager.h`
- Read: `https://github.com/gigaHours/mm_sdk/blob/main/game/go/go.h`
- Read: `https://github.com/gigaHours/mm_sdk/blob/main/game/go/character.h`
- Read: `https://github.com/gigaHours/mm_sdk/blob/main/hookmgr.h`
- Read: `https://github.com/gigaHours/mm_sdk/blob/main/hookmgr.cpp`
- Read: `https://github.com/gigaHours/MadMaxSpawner/blob/main/spawnermenu.cpp`
- Create: `docs/research/rust-transform-feasibility.md`

**Interfaces:**
- Consumes: approved design and cited public source.
- Produces: a short call-chain record: manager lookup/accessor, game-object transform method, address-selection/startup logic, data layout/calling convention evidence, and which claims are only the SDK author's assumptions.

- [x] **Step 1: Inspect the public source paths and record only the minimal transform call chain**

Record exact source links and relevant symbols. Do not copy source bodies into the project. Explicitly flag unknown `this` ABI, vtable slot, manager singleton, pointer lifetime, matrix layout, and startup timing.

- [x] **Step 2: Compare the spawner's usage against the SDK declarations**

Record what the example demonstrates (obtaining Max and reading a transform to place an existing entity) and what it does not demonstrate (safe thread, exact local binary compatibility, transform validity across loading, persistence).

- [x] **Step 3: Record SDK license/provenance status**

Note that no license file was found and keep this plan limited to independently authored behavior. If a later implementation would require literal source reuse, stop and ask the user before using it.

**Deliverable check:** The research note distinguishes verified source facts, SDK claims, and unknowns without copying SDK implementation.

## Task 2: Check the Exact Local Executable and Address Assumptions

**Files:**
- Read: local `C:\Program Files (x86)\Steam\steamapps\common\Mad Max\MadMax.exe`
- Read: recorded target inventory in `docs/research/feasibility.md`
- Update: `docs/research/rust-transform-feasibility.md`

**Interfaces:**
- Consumes: Task 1 call chain and the recorded executable metadata.
- Produces: exact target identity, PE section/timestamp observations, address interpretation questions, and whether static data is sufficient to validate the SDK's address set.

- [x] **Step 1: Reconfirm the target identity from disk**

Record SHA-256, file size, PE timestamp, image base, and section names; compare against the existing inventory (`0x5628A448`, 76,814,792 bytes, recorded SHA-256). If it differs, mark the SDK path unsupported pending investigation.

- [x] **Step 2: Map the SDK Steam address assumptions to the target image**

Determine whether SDK constants are absolute virtual addresses or RVAs and whether the PE image base/relocations make those assumptions plausible. Do not infer compatibility from timestamp alone.

- [x] **Step 3: Decide whether static evidence verifies executable code**

Because the local PE lacks `.text` and public mods report deferred/decrypted code behavior, static bytes alone may be insufficient. Mark runtime signatures as unverified unless evidence proves the on-disk representation is the code that executes.

**Deliverable check:** The note gives a bounded conclusion: exact build supported by evidence, unsupported, or unresolved pending user-reviewed runtime probe. It must not claim hook compatibility merely from a matching timestamp.

## Task 3: Establish Whether a Safe Read Callback Is Known

**Files:**
- Read: `src/lib.rs`
- Read: `src/overlay.rs`
- Read: `src/game_probe.rs`
- Read: public SDK and MadMaxSpawner sources from Task 1
- Update: `docs/research/rust-transform-feasibility.md`

**Interfaces:**
- Consumes: Task 1 call chain and Task 2 target assessment.
- Produces: documented candidate callback(s), expected thread/lifecycle, evidence, and explicit go/no-go for a transform read without an additional probe.

- [x] **Step 1: Map the current Rust callback and worker lifecycle**

Record that `InitializeASI` starts the overlay worker, the DX11 overlay callback runs in render context, and the existing display-mode candidate hooks a function from a separate worker. None is safe for game-object reads without independent evidence.

- [x] **Step 2: Inspect existing public hooks for a documented game-safe callback**

Check whether `mm_sdk` or its dependent sample establishes a game update callback, object lifetime, and thread affinity for `GetTransform`. A function call in a menu action is not sufficient evidence unless the event dispatch thread is established.

- [x] **Step 3: Choose the research outcome**

If exact-build support, transform ABI, and a safe callback are all established, record the evidence and mark ready for a separate implementation plan. Otherwise record precisely which requirement is missing and mark no-go for implementation until a new, user-reviewed probe design is approved.

**Deliverable check:** No candidate game call is proposed from the DX11 callback or worker by default; the report states the evidence needed for any next probe.

## Task 4: Finalize the Feasibility Decision

**Files:**
- Update: `docs/research/rust-transform-feasibility.md`
- Read: approved design and this plan

**Interfaces:**
- Consumes: Tasks 1–3 findings.
- Produces: one of `READY_FOR_IMPLEMENTATION_PLAN`, `NEEDS_USER_REVIEWED_PROBE_DESIGN`, or `NO_GO_WITH_CURRENT_SDK_FINDINGS` and the next smallest action.

- [x] **Step 1: Check the design's go/no-go requirements against the evidence**

The report must answer whether the exact local build is addressed, when the code is available, how the player transform is reached, and what evidence establishes safe thread access.

- [x] **Step 2: State the next action without implementing it**

If ready, propose the follow-on Rust implementation plan. If not ready, bring the specific missing evidence to the user and request approval only for a concrete, source-reviewed probe plan. Do not write or build the probe in this plan.

**Deliverable check:** The feasibility report is complete, all critical unknowns are visible, and no source/runtime/game files were modified.


