# Managed Mad Max Scripts and Runtime Console

**Date:** 2026-10-29  
**Status:** Draft for user review  
**Scope:** IDE-authored Rhai scripts managed by the Rust/Bevy runtime in the Mad Max ASI

## Intent

Let mod authors write scripts in their normal IDE, load them into a running Mad Max session, attach them to known Mad Max lifecycle phases, and reload or unload them without rebuilding the ASI or restarting the game. The in-game console manages scripts and shows status/output/errors; it is not a code editor.

Bevy is the bridge's internal runtime: it owns state, schedules, queues, and the callback registry. Modders interact with Mad Max concepts and a versioned scripting API, not Bevy `World`, `Entity`, `SystemParam`, or component internals. Native Rust remains the place to add new low-level game bindings, FFI, and host capabilities; scripts compose those capabilities at the correct phase.

Rhai's script-owned object maps are the first way to create structured script data/state. These are dynamic maps rather than nominal, Rust-style structs. The first version does not require script-defined values to become host Bevy components.

## Goals

- Edit `.rhai` script packages in an existing IDE; load, list, run, reload, and unload named scripts from the game console while Mad Max remains open.
- Dynamically register each script's declared callbacks with named, host-approved lifecycle phases and remove those registrations on unload/reload.
- Expose a stable, discoverable, versioned Mad Max API describing each callback's available context, methods, reads/writes, timing, and unavailable-value semantics.
- Give scripts phase-scoped access to copied Mad Max data and approved mutation operations without exposing native pointers or the Bevy world.
- Keep script-owned state available across callbacks for that active script generation using Rhai maps/arrays and values.
- Apply game mutations only through registered Rust host APIs and the existing native adapter at valid callback boundaries.
- Keep callback work, file loading, queues, output, and scrollback bounded; report errors with script ID, generation, callback, and source location where available.

## Non-goals

- Exposing arbitrary Bevy `Query<T>`, `ResMut<T>`, `World`, `Entity`, unregistered Rust types, raw game pointers, unrestricted FFI, process/network access, or memory patching to scripts.
- Making Rhai script maps behave as nominal Rust types or requiring script-created state to be a Bevy component.
- Allowing script code to add new native Rust game bindings. New host/game capabilities require Rust implementation and a rebuild of the ASI; once exposed, scripts can use them without changing native code.
- Making already-applied Mad Max effects reversible on unload. Unload stops future callbacks and cancels pending intents; reversing an already-applied spawn or game mutation requires a separately implemented owner/undo API.
- Automatic reload-on-save in the first slice. Initial reload is an explicit console command; a watcher may follow after lifecycle semantics are stable.
- Creating schedules for arbitrary script-provided strings. Scripts may register only with host-declared lifecycle IDs that correspond to known callbacks.
- Resuming raycast research or advertising capture controls that are not present in the active ASI.

## Existing architecture and constraints

The project pins `bevy_ecs`/`bevy_app` 0.19.1 and currently runs separate thread-local worlds from two manually invoked host callbacks: `MadMaxPostSim` on Mad Max's `UpdatePostSim` hook and `MadMaxRender` on the DX11 callback. The game callback imports a copied `GameSnapshot`, runs the game schedule, and then lets a native adapter dispatch supported typed intents. The UI callback drains copied snapshots/results and draws through Hudhook's ImGui 0.12 binding. Only bounded owned DTOs cross between callback threads.

The user-facing extension contract should not depend on Bevy system parameters. A stable scripting API context is a better fit: the Rust bridge constructs a phase-scoped context from the approved Rust/Bevy parameters and registers its named methods/types with Rhai. Reads are copied or lifetime-bound to one callback; writes use validated host functions or typed intents. No `World`, pointer, or mutable game object reference enters Rhai.

Rhai has maps and arrays, but its language intentionally has no user-defined struct/record/class declarations. Script-created state can still be structured as nested maps and can use functions as constructors/helpers. If nominal user-defined types or Rust-authored guest code become a requirement, that is a separate runtime/language decision. WASM would still need a defined host ABI for its values and effects; guest Rust types do not automatically become host Bevy/Rust types.

## Approaches considered

1. **Rhai with a Bevy-facing API.** Expose Bevy entities, resources, and parameter names to mod scripts. Rejected: makes an internal implementation framework the modding contract and couples scripts to ECS internals.
2. **Rhai with Mad Max phase contexts and host capabilities (selected).** A script declares lifecycle callbacks. The host associates them dynamically with known phases and invokes them with a phase-specific Mad Max context and per-script map state. Rust owns the context types and API metadata; Bevy schedules and systems are internal. This keeps reloadable scripting flexible while preserving the Rust/FFI boundary.
3. **Rust-authored WASM modules.** Could give script authors Rust syntax and guest-defined Rust types, with Wasmtime providing a sandboxed execution environment and fuel/memory controls. It adds a wasm target/toolchain, runtime, ABI/component design, packaging, and guest-host type mapping. Guest types still need conversion at the boundary and are not automatically Rust types in the host ECS. Defer unless Rhai's dynamic language or maps prove insufficient.

## Script package and public API

Each script package has a stable ID, source file, API version, callback declarations, and optional per-script state. A package may export recurring lifecycle callbacks (such as `on_post_sim(ctx)`) and an optional one-shot `on_run(ctx)` entry point. The exact human-editable manifest format is a plan decision. The script API version is independent of the Bevy version and uses stable semantic IDs.

A schedule/callback descriptor exposed by `api.help()` or `api.describe()` includes:

- stable callback ID and API version;
- which host phase invokes it and its cadence;
- allowed context object(s), properties, methods, and their input/output shapes;
- which operations only read copied state and which request effects;
- when references/values are valid, and how missing/stale state is represented;
- ordering/failure behavior and any capability requirements.

Scripts use phase contexts such as `game.post_sim(ctx)` or `game.command(ctx)` rather than requesting Bevy parameters. A context provides only that callback's capabilities. For example, a player-reading context cannot call a vehicle setter unless that callback's API explicitly includes the setter capability. The API catalog is authored in Rust alongside the adapter, not reflected from the entire Bevy world.

Scripts may keep persistent state for their loaded generation using maps, arrays, strings, numbers, and booleans, for example `state = #{ inventory: [], flags: #{} }`. This is script-owned state and cannot be queried as a native component by other code unless a future host API explicitly exports it. Rhai does not enforce a declared map schema; validation helpers/constructors can be provided where useful. Reload creates a new generation and resets that script's state in v1. Optional migration is deferred.

Example shape (illustrative, not final syntax):

```text
script id: vehicle_watch
api: madmax-script@1
callbacks:
  game.post_sim: on_post_sim
state:
  recent_speeds: []
```

```rhai
fn on_post_sim(ctx) {
    let vehicle = ctx.vehicle.current();
    if vehicle.is_some() {
        state.recent_speeds.push(vehicle.velocity_magnitude());
        ctx.log.info(`speed ${vehicle.velocity_magnitude()}`);
    }
}
```

The sample is conceptual; exact Rhai syntax and API names are implementation decisions.

## Lifecycle and schedule registration

- The host owns a catalog of real Mad Max callback phases. The first game callback is the verified `game.post_sim`; the UI render callback is a separate UI-only context and cannot make game mutations. Additional phases appear only after their hooks and timing are understood.
- Script load validates the package ID, API version, declared callbacks, required capabilities, source limits, and Rhai source. Only registered callback IDs are accepted.
- Internally, Bevy runs a small dispatcher system in each supported schedule. The script manager maintains dynamic per-schedule registrations and invokes each callback in deterministic order (priority then script ID). This provides dynamic script schedule membership without exposing Bevy schedules or making the Rhai function itself pretend to be a native Rust `System`.
- `script run <id>` invokes only that loaded script’s optional `on_run(ctx)` once, on the next `game.post_sim` callback after the current snapshot has been imported and before recurring `on_post_sim` callbacks. It receives the `game.post_sim` context and the same declared capabilities. If `on_run` is absent, the console reports that the script has no one-shot entry point; it does not rerun every registered callback. The request crosses the bounded queue and never invokes native/game operations from ImGui.
- A new schedule/callback requires a Rust-owned context adapter and API descriptor. Script packages cannot create arbitrary lifecycle phases.
- A future optimization may use Bevy's dynamic `SystemParamBuilder`/`DynSystemParam` internally, but the public script contract remains Mad Max contexts. The first implementation should use the simplest internal runner that preserves phase correctness and bounded execution; do not introduce unsafe generic world reflection to make registration appear more dynamic.

## Managed load, reload, and unload

Each active script has a stable ID, generation number, compiled Rhai AST, isolated scope/state, callback list, and bounded error/status information.

- **Load:** Read source only from the configured scripts root; validate and compile; register callbacks at a safe game callback boundary. Invalid packages never partially activate.
- **Reload:** Compile/validate the new generation before touching the old one. If parsing, API validation, or limits fail, keep the old generation active. At the next safe schedule boundary, remove the old registrations, cancel its pending unconsumed intents, reset its state, and activate the new generation. Report the generation transition.
- **Unload:** Remove future callback registrations, cancel pending unconsumed intents tagged with that script generation, and drop its AST/scope/state. Optional lifecycle cleanup hooks are deferred until their context and failure semantics are explicit. Already-applied Mad Max effects are not automatically undone.
- **Delete:** `script unload <id>` works even if the source file has been deleted. External deletion does not silently unload a live script in v1; a future watcher can turn delete/save events into explicit lifecycle requests.
- **Failure isolation:** Rhai errors and host API errors are attributed to script/generation/callback. A callback error does not unwind across the game hook. The v1 policy for repeated failures may disable that callback and report it; the threshold and re-enable behavior are implementation-plan decisions.

## IDE and in-game console UX

Use a normal IDE (for example, VS Code) for editing. Script packages live under one configured root. The loader canonicalizes paths and refuses traversal outside that root. Rhai scripts themselves get no direct filesystem API.

The in-game console is a compact runtime manager, not an editor. It uses the existing ImGui input-capture path and blue-grey visual style. Current controls: `F1` toggles mouse/input capture, `F2` toggles the UI configurator, and `F3` toggles the console. While a panel owns input it captures keyboard/mouse/raw input and shows the cursor; Escape closes it and returns input to Mad Max.

```text
┌ Mad Max Script Console ──────────────────────────┐
│ Runtime: active   Script API: 1   Loaded: 2       │
│ vehicle_watch   active  gen 4  game.post_sim       │
│ terrain_tools   unloaded                            │
│ > script reload vehicle_watch                      │
│ vehicle_watch: active gen 5                         │
│ ── output / errors ────────────────────────────── │
│ [game.post_sim] vehicle_watch: vehicle unavailable │
│ > _                                                 │
└───────────────────────────────────────────────────┘
```

Initial commands: `help`, local `clear`, `api list [phase]`, `script list`, `script load <id>`, `script reload <id>`, `script run <id>`, `script unload <id>`, and `script status <id>`. Scrollback shows bounded output and diagnostics with package, generation, phase, and source location where available. Initial hot reload is explicit (save in IDE, run `script reload`); automatic reload-on-save follows only after lifecycle behavior is stable.

### Mod-owned input and key bindings

The mod runtime is authoritative for script input. It samples the existing Hudhook/ImGui keyboard and mouse-button event stream into a bounded `InputFrame` Bevy resource with down, just-pressed, and just-released states, then carries an owned snapshot through a nonblocking mailbox to the game callback. Rhai scripts may read `ctx.input` only when declaring `input.read`; key names come from the Rust runtime catalog (`show_keys`). This is not a Mad Max keymap, XInput, DirectInput, or gameplay-polling API.

Local console commands `bind <key> script run <id>`, `unbind <key>`, and `show_binds` manage only mod callbacks. Bindings trigger on the mod-owned just-pressed edge and enqueue the existing bounded `script run` command. F1/F2/F3 remain reserved hardcoded controls. When global input capture is off, a mod binding may coincide with a native Mad Max action because this runtime intentionally does not suppress individual native keys. Global capture remains the existing all-input suppression mechanism while the mod UI owns input.

## Runtime flow

```text
IDE saves a package under the configured scripts root
  -> in-game console requests load/reload/unload/run
  -> render callback sends bounded owned ScriptRequest DTO

Mad Max UpdatePostSim callback
  -> import lifecycle requests before running the game schedule
  -> validate/compile candidate and stage manager changes
  -> run the host-owned Bevy schedule
  -> internal script dispatcher invokes registered callbacks with phase-specific Mad Max contexts
  -> scripts read context, update script-owned map state, and request typed host effects
  -> native adapter handles only supported intents at its approved boundary
  -> publish bounded status/output/action results to the UI queue

DX11 render callback
  -> drain results and draw the console
```

Both callback queues remain bounded and nonblocking. Script execution, request count, output, package bytes, loaded scripts, callbacks, and scrollback have finite named limits. Do not claim a hard wall-clock timeout unless a safe cancellation mechanism is available; Rhai operation/recursion/data-size limits are primary. No script thread directly accesses Mad Max or another Bevy world.

## First host API surface

Start with a small actual API catalog and extend it only when a Rust adapter exists:

- `ctx.frame`: callback sequence, delta, freshness, and phase metadata.
- `ctx.player`: read-only, copied values currently supported by `GameSnapshot`.
- `ctx.vehicle`: copied current-vehicle values when available; `set_velocity(#{x,y,z})` with `game.vehicle.write` and `game.read` requests a bounded owned intent, applied only by the existing native game callback adapter.
- `ctx.log`: bounded console output.
- `ctx.capture`: status/start/stop only for capture toggles compiled into the active ASI.
- Future Rust-implemented contexts may add world queries, more vehicle setters, spawning, and other Mad Max features after their callback timing, types, and failure results are defined.

Keep a clear distinction between script state (dynamic Rhai maps) and host game state (typed Rust adapters and validated effects). Native APIs can register custom Rhai types/methods when appropriate, but those types are versioned host capabilities, not script-created native Rust types.

## Limits and failure behavior

- Rhai is built with safety checks enabled and without `unchecked`; configure finite operation, call-depth, expression-depth, variable, string, array, and map limits.
- Set per-callback and aggregate per-schedule work budgets; report limit exhaustion in the console.
- Cap source/manifest bytes, active scripts, callbacks per script, work per callback, queue depth, output bytes, and scrollback entries/bytes.
- Omit external module loading and host escape hatches. Only the root-confined host loader reads files.
- Context methods return explicit unavailable/stale/rejected outcomes; they do not fabricate default game values.
- No background async callbacks or host/game calls from the render callback in v1.

## Verification and acceptance

Before building the full manager, make a focused spike that proves:

- a Rhai callback can be registered to the existing `MadMaxPostSim` runner and receive a phase-scoped typed host context;
- the manager can replace/unregister callbacks at a safe callback boundary without affecting the currently executing callback;
- a failed reload leaves the previous generation active;
- script state survives callbacks and resets on reload;
- operation/depth/data limits stop runaway scripts; and
- output and result transport remain bounded and nonblocking.

Then implement the smallest end-to-end slice: one verified game phase, read-only player/vehicle context, script-owned map state, logging, explicit console load/reload/run/unload/list, line-aware errors, and input capture. Add a native game mutation capability only as a separate typed Rust adapter with its own validated result path. Build the Windows release after source review. Do not deploy while Mad Max remains open; the user decides when to close and verify the integrated ASI.

## Decisions left to the implementation plan

- Choose a human-editable package/manifest layout and configured scripts-root location.
- Define exact Rhai callback signature, state binding syntax, execution ordering, and repeated-error disable/re-enable policy.
- Choose named conservative limits for package bytes, scripts, callbacks, work, output, and queue sizes.
- Map v1 `ctx.player`, `ctx.vehicle`, and `ctx.capture` to existing copied data and real runtime controls; exclude anything not implemented.

- Keep internal Bevy dispatch hidden and simple; revisit dynamic native system construction only if a concrete extension requires it.


## Approved follow-up: focus controls and vehicle jump

Current controls supersede earlier hotkey notes: F1 toggles input capture, F2 toggles the UI configurator, and F3 toggles the script console. Panel modes capture keyboard, mouse and WM_INPUT by default; F1 can release or recapture input. Escape closes panels and returns input to Mad Max. UI opacity is configurable from 0.25 to 1.0 (default 0.96). Native hover behavior and Escape filtering still need live verification. Every finite float metric uses a fixed three-decimal truncating display. Source-style `bind`/`unbind` commands are deferred until one selected key can be intercepted across normal and raw input without triggering the native Mad Max action; native action suppression is required.

The `game.vehicle.write` capability enables `ctx.vehicle.set_velocity(#{x: number, y: number, z: number})` alongside `game.read`. Values must have exactly three finite numeric components with absolute values no greater than 250.000 units/s. The handle expires with its invocation; only a successful callback with valid retained state publishes its owned effect. There is one successful setter per invocation, at most 32 effects accepted in a game callback, and at most 32 outstanding script velocity intents. Native dispatch resolves the current occupied vehicle using the existing callback gate and reports the result with script/generation/request attribution. Pending effects are cancelled on unload/replacement/disable; applied effects are not undone. Uncaught validation/budget errors disable the generation under the existing runtime error policy.

`vehicle_jump` declares only `on_run`, reads copied velocity and adds +18.000 to Y (world up), preserving X/Z. Missing velocity logs a no-write result. A queued setter is not an applied result; the native adapter reports completion separately. The adapter currently consumes one intent per callback and targets the occupied vehicle at consumption; multiple absolute writes from a copied frame are not additive impulses. There is no Space key polling in this slice. Source editing/load/run/reload remains the live iteration path.
