# Scripted Spawn Workbench and Package Dependencies

## Intent

Grow the GMod-inspired spawn menu into a modular in-game workbench for Mad Max modders. Its tabs should be independently loadable script packages, while a small Rust shell keeps layout, catalog rendering, focus, and large lists fast and predictable. This establishes a home for asset browsing, event research, and later object-model/class inspection without making the shell itself a monolithic script.

## Current state

- `src/devtools/overlay/spawn_menu.rs` owns the current native browser and its compact three-pane UI; catalog data is generated into `spawn_menu_catalog.rs`.
- `examples/scripts/debug_spawn_menu` is a single-file Rhai package for placement ray configuration and spawning, not the spawn browser itself.
- Script packages declare an API version, string capability IDs, and callbacks. The loader reads only `manifest.toml` and `main.rhai`; Rhai's module and filesystem features are disabled.
- Scripts load/reload/unload independently at callback boundaries, but the manager has no package dependency graph, versions, or conflict rules. Existing `capabilities` are permissions and must remain distinct from dependencies.
- Ghidra maps a shared hashed action registry, registration/subscriber handling, and generic publishers. The callback path for special-camera actions remains obfuscated. `game/events.h` has only opaque Rust markers; no safe general subscription/dispatch ABI exists yet.

## Approaches

1. **Separate tab packages over a Rust workbench shell (selected).** Each tool tab has its own script package and lifecycle. The shell owns high-volume drawing, focus, shared navigation, and bounded data presentation. Packages contribute typed tab metadata and domain behavior through a versioned host API. This supports independent iteration and a useful dependency model while protecting rendering performance.
2. **One package with multiple source files.** This makes the initial package easier to deploy, but tabs cannot be loaded, unloaded, conflicted, or versioned independently. It does not meet the desired package-level extensibility.
3. **Fully script-drawn workbench.** This gives scripts unrestricted layout ownership but requires a broad immediate-mode UI API, transfers large catalogs into script evaluation, and increases render-time and performance risks. Keep this as a possible future extension, not the first architecture.

For source organization, packages use an explicit ordered `sources` list in their manifest. The loader reads each listed file inside the package root and compiles the combined bounded source as one AST. Compile diagnostics map combined line positions back to the originating file and line. This is a safe include mechanism without enabling Rhai filesystem modules, globs, or arbitrary file access.

## Goals

- Keep a single compact, native-rendered spawn workbench shell with the established visual language, focus behavior, and efficient catalog grid.
- Let independent script packages contribute tabs such as Spawn/Assets, Events, and later Object Model, each discoverable and loadable on request.
- Support multiple Rhai source files per package through explicit, safe manifest declarations and include all sources in auto-reload fingerprints.
- Add typed package and host-API dependency declarations, optional requirements, semantic versions, conflict declarations, and actionable dependency status in package discovery and loading.
- Keep dependency resolution deterministic and local-only: load required installed packages in dependency order as one activation transaction; reject cycles, missing/incompatible requirements, and conflicts without leaving a partial activation.
- Prevent unloading a required package while active dependents remain; report the dependents so users can unload them deliberately.
- Keep permission capabilities (for example `game.spawn`, `debug.draw`, and UI access) as a separate allowlist that is validated and enforced at callback boundaries.
- Establish an Events tab with an evidence-labeled inventory of known action names, event-related classes, payload knowledge, and observation/dispatch status.
- Represent event observation and dispatch as separate, versioned, capability-gated host APIs. Do not expose arbitrary event injection or general callbacks until their ownership, payload, timing, and re-entrancy contracts are mapped.
- Keep the future roadmap open for asset/resource families and class/inheritance/composition views, without requiring every game system to be fully understood before the shell is useful.
- Keep API/rustdoc updates concise: canonical detailed contracts live in Rust docs and generated `API.md`; user docs explain package authoring and link to the generated reference.

## Non-goals for the first delivery

- A network package manager, remote repositories, package signing, or automatic downloads.
- A fully generic script-authored docking/layout engine or arbitrary ImGui calls from Rhai.
- A complete event inventory, safe subscription to every game event, or unrestricted action dispatch.
- Replacing the spawn native ABI, asserting that every catalog candidate spawns, or adding all asset viewers in this tranche.
- A complete class browser or full inheritance/component map; those are later workbench packages built from separate evidence-backed inventories.

## Package contract

The manifest keeps the current `id`, `api_version`, `capabilities`, and `callbacks`. It adds a package version and an ordered list of confined source files, with `main.rhai` required as the entry source. For compatibility, omitted version defaults to `0.0.0` and omitted `sources` defaults to `main.rhai`; omitted dependency/conflict lists are empty. New packages declare a real package version before another package can depend on them with a non-wildcard requirement. It also adds typed requirements and conflicts:

- A requirement is a typed record with `kind = "package" | "host_api"`, an ID, a semantic-version requirement, and `optional` status.
- A conflict is a typed package record with an ID and semantic-version requirement. It blocks simultaneous activation, not installation.
- Package dependencies describe availability/compatibility; capability declarations describe permission to perform effects. Neither substitutes for the other.
- An optional dependency never prevents package activation. When an optional package/API is absent, the dependent tab must show the unavailable feature state instead of calling it.
- Dependency resolution uses installed local packages only. A required package is staged before its dependent; all requested generations activate at the normal callback boundary only after the whole graph compiles and validates.
- Dependency cycles, unsatisfied hard requirements, and active conflicts reject the requested graph with a specific reason. A failed graph leaves the currently active generations unchanged.
- Unload is refused while another active package requires the target. The error names those dependents; there is no implicit cascade unload.
- Auto-reload fingerprints the manifest and every declared source file. A failed replacement retains the last active generation under the existing behavior.

The shell's tab API is a versioned host extension contract, not a permission capability. A tab package declares that API requirement and static tab metadata in its manifest. The shell owns tab registration/removal. A package publishes a bounded, copied tab view model from its game callback; the shell renders that model and returns copied input events at a later callback boundary. The injected render callback does not execute Rhai or access script filesystem state.

## Event workbench behavior

The first Events package is a read-oriented explorer. Its catalog records stable ID, observed/display name or hash, kind (named action, event type, router/handler class, or publisher), owner/handler when known, payload description/status, evidence source, and whether observation or dispatch is currently supported. Unknown fields remain explicitly unknown.

Initial inventory may include the mapped special-camera action names, the shared action-registry/publisher shapes, and RTTI leads such as `CEventRouter`, `CGameEventHandler`, `CReceiveEventsTrigger`, `CEventStealthRouter`, `CFreeEventDataQueue`, and `CInputActionOverride`. Related RTTI names are not labeled as members of the action registry without a direct link.

Live observation is a later, read-only API with bounded copied records and a documented thread/timing contract. Dispatch is a separate experimental API and only accepts mapped action IDs with a validated payload schema. The UI may show unavailable/experimental states but must not imply that an unmapped action is safe to send.

## Initial delivery phases

1. **Package foundation:** explicit multi-file sources, typed package/host-API requirements and conflicts, deterministic dependency resolution, unload protection, and discovery status. Preserve existing one-file manifests as valid packages.
2. **Workbench extension contract:** native shell discovers independently loadable tab packages, applies metadata/order/status, and routes bounded input/results without Rhai on the render callback. Existing spawn browser remains the dependable base tab during this migration.
3. **Events inventory tab:** curate the current Ghidra findings and class/event leads into a generated or validated catalog; expose current support states without sending events.
4. **Event observation spike:** map a bounded read-only observer contract and add live records only after callback re-entrancy, timing, payload copying, and teardown are understood.
5. **Dispatch and object-model tabs:** later, after exact payload routes and object/class inventories are mapped. Asset-browser and object-model packages can use the same shell and dependency APIs.

Each phase should work on its own and preserve the existing spawn menu if a tab package is missing or fails to load.

## Constraints and failure behavior

- Package paths are explicit relative files, confined to the discovered package directory, UTF-8, and bounded by per-file and combined source limits.
- Rhai remains without filesystem, process, network, dynamic-module, or FFI access.
- Load, reload, dependency graph changes, and unload remain callback-boundary operations.
- A tab package cannot call native game functions by declaring a dependency; it must separately declare granted capabilities, and the native adapter still validates each effect.
- Missing or failed optional tabs do not hide the base spawn browser.
- The event catalog is mod-owned research data. Runtime display does not scan Ghidra, archives, or game files.
- The project should not grow duplicate API prose. Detailed signatures/contracts stay in Rustdoc and generated `API.md`, with short authoring guidance in `docs/scripting.md`.

## Acceptance criteria

- Existing one-file packages still discover, compile, load, reload, and unload unchanged.
- A package with several explicitly listed source files compiles as one package; missing, oversized, duplicate, invalid, or escaping source paths are rejected with a useful error.
- Required dependencies load in deterministic order; cycles, incompatible versions, and conflicts fail atomically and retain any active generations.
- Optional dependencies do not block loading; blocked unloads list active dependents.
- A test tab package can declare its tab metadata and versioned workbench API requirement, appear in the native shell, receive bounded input, and disappear on unload without affecting the base spawn tab.
- The Events tab accurately distinguishes mapped action registrations/publishers from unresolved event classes, and exposes no arbitrary dispatch before a payload is mapped.
- Rendering a tab does not execute Rhai, rescan the package filesystem, or rebuild the entire asset catalog each frame.
- Rustdoc and generated `API.md` describe the new author-facing contract without duplicating the full internal design.
