# Mad Max Construction Mod — Design

**Date:** 2026-10-27  
**Status:** Revised design draft for user review; the first implementation step is a reversible ASI bootstrap proof. No toolchain, loader, or plugin has been installed or created.

## Intent

Build a community-facing construction mod for Mad Max PC. Players should eventually be able to place structures and other existing game objects, keep them where they put them across saves and restarts, and build in the open wasteland as well as camps and other instances. Over time, the project may support custom assets and mod-defined systems.

The first product milestone remains deliberately narrow: place one existing structure in the open world, save its placement in mod-owned data associated with a selected playthrough, and restore it after loading. Before that, a small bootstrap checkpoint will prove only that a Rust `.asi` plugin can load safely in the user's exact Steam build and identify/log that build. The bootstrap checkpoint does not claim that a Mad Max scripting API or construction capability exists.

The user wants AI to accelerate reverse engineering and implementation, while remaining the author of Rust code or reviewing every line of Rust generated for them. AI assistance must expose its reasoning and evidence; no opaque, unchecked code generation is acceptable.

## Goals

- Prove a repeatable place → persist → reload → restore loop using one existing Mad Max object.
- Build a Mad Max-specific ScriptHook-style layer that gradually exposes small, reviewed game capabilities to Rust mod logic.
- Keep construction records outside the native Mad Max save file.
- Scope each placement set to one playthrough so switching saves cannot silently mix or lose buildings.
- Load a small Rust `cdylib` plugin through a compatible ASI loader. The loader is only the bootstrap; our project must implement its own build checks, hooks, safe game-thread dispatch, lifecycle signals, and high-level adapters.
- Keep the in-process layer narrow and game-specific. Add a separate headless Rust/Bevy sidecar only after at least one useful game capability is verified; it can then own mod logic and construction persistence over versioned IPC.
- Keep authored project code in Rust where practical. The user writes it, or reviews it line by line before accepting it.
- Use findings and code from the active community tools and save research as evidence, with credit and license review before reuse.
- Grow toward camps/instances, multiple object types, and eventually custom assets only after the preceding capabilities are demonstrated.

## Non-goals for the first milestone

- A polished construction UI, snapping/grid placement, collision-aware previews, recipes, resource costs, or multiplayer.
- Arbitrary custom meshes, textures, animated assets, or new engine-level entity types.
- Building in camps, interiors, or other streamed/instanced areas.
- Editing, patching, or replacing records in the native game save.
- Supporting every distribution/build of Mad Max from day one.
- Replacing Gibbed.MadMax, Mad Manager, or the community save editor.
- A Bevy renderer, window, or game loop taking over Mad Max.
- A broad, promised ScriptHook API before individual game operations are verified.
- A sidecar process or Bevy dependencies in the first ASI loading proof.

## Constraints and known evidence

- Gibbed.MadMax provides C# tools for archive and game-data inspection, including XVM decompile/compile support.
- A community save-editor project reports in-game verified restoration of destroyed static world objects and cleared convoys. Its documented method links saved objects to game data, but does not demonstrate arbitrary player-placed object persistence. It also reports that some save data and camp state remain unexplained.
- September 2026 community mods now demonstrate native storm invocation, weather/world-context reads, convoy respawning, in-game status text, and persistent INI settings. Their pages describe ASI loading and targeted hooks, but Steam compatibility is still uncertain for some mods. The installed Steam PE has no `.text` section; recent scanners report failure to find it. MadMaxFix supports the matching Steam timestamp through known addresses and byte checks after code decryption. The target is viable for focused reverse engineering, but a generic `.text` scanner is the wrong assumption.
- A separate process cannot directly call Mad Max's internal object-spawn functions. A small in-process bridge must perform verified game operations at safe game-thread points and exchange copied events/commands with the sidecar over local IPC. The sidecar may run Bevy ECS headlessly; it does not own rendering, physics, or the authoritative game world.
- The sidecar and bridge communicate over a versioned IPC protocol, not a shared Rust ABI. Use a stable C-compatible ABI only for any actual in-process loader/module boundary; Rust's native ABI is not a stable plugin contract.
- The user's game has no built-in developer console setting. A community console injector is closed source and is not the foundation for the runtime design.
- The installed executable imports `DINPUT8.dll`, `dxgi.dll`, and `XINPUT9_1_0.dll`; these are possible ASI-loader proxy entry points, not yet-tested choices. The game root already contains `bink2w64.dll`, which makes a Bink proxy a less attractive option because its original must be preserved/renamed.
- The Rust Reference supports Windows `cdylib` output and explicit external ABIs. A Rust ASI entrypoint is technically plausible because the loader calls an exported native function, but a minimal plugin must prove the exact export/load behavior before it becomes a foundation.
- The project is a single-player hobby/community effort. The prototype targets the Windows Mad Max installation available for in-game verification; the first feasibility stage records the store and exact executable fingerprint. If no installation is available or multiple candidates cannot be distinguished, pause for user input before build-specific hook work. Supporting other builds is a later compatibility task.

## Evidence and references

- [Gibbed.MadMax](https://github.com/gigaHours/Gibbed.MadMax) — C# archive, data conversion, and XVM tooling.
- [Mad Max save editor project](https://github.com/Rick-Thompson/MadMax_Savegame_Editor_Project), especially its [save format notes](https://github.com/Rick-Thompson/MadMax_Savegame_Editor_Project/blob/main/docs/FORMAT.md) and [game-file findings](https://github.com/Rick-Thompson/MadMax_Savegame_Editor_Project/blob/main/docs/GAME-FILES.md) — documents verified restoration of existing world objects and convoys, while noting remaining save and camp-state gaps.
- [MadMaxFix](https://github.com/Alexxlml/MadMaxFix) — its source (https://raw.githubusercontent.com/Alexxlml/MadMaxFix/main/src/fix.c) documents Steam PE timestamp 0x5628A448, byte-checked hook signatures, delayed installation after Steam decrypts game code, and an XInput9_1_0.dll proxy; [Wasteland Storms](https://www.nexusmods.com/madmax/mods/212), [Enhanced Convoy](https://www.nexusmods.com/madmax/mods/210), and [Enhanced Archangels](https://www.nexusmods.com/madmax/mods/213) demonstrate current runtime/data mod patterns and build compatibility limits.
- [Ultimate ASI Loader](https://github.com/ThirteenAG/Ultimate-ASI-Loader) — supplies a proxy-DLL plugin-loading mechanism and calls an exported ASI entrypoint; it does not provide Mad Max game APIs.
- [Rust Reference: `cdylib` linkage](https://doc.rust-lang.org/reference/linkage.html) and [external ABIs](https://doc.rust-lang.org/reference/items/external-blocks.html) — support the feasibility of a native Rust DLL/FFI boundary; they do not prove compatibility with Mad Max or a particular ASI loader.
- [Rust Reference: external block ABIs](https://doc.rust-lang.org/reference/items/external-blocks.html) — Rust ABI stability constraint for future component boundaries.

## Approaches considered

1. **Edit native save data.** This would align placement data with the game’s own save lifecycle, but arbitrary new placements are not demonstrated, save semantics are still partly reverse-engineered, and mistakes could corrupt player saves. Rejected for the prototype.
2. **Keep a mod-owned construction file and re-create placements at runtime.** This isolates mod data from native saves and supports a reversible experiment. It requires reliable object spawning and world-load detection inside the game. Selected for the prototype because it offers a safer path while directly testing the core capability.
3. **Only add static placements through game data files.** This may support curated world edits, but does not establish player-driven building or per-playthrough placement persistence. It can complement the project later, but does not satisfy the initial proof.

## Proposed architecture

- **ASI bootstrap:** Ultimate ASI Loader or another justified loader supplies a proxy DLL and loads the `.asi` plugin. First prove startup, exact executable identification, refusal on an unknown build, and log output. This stage performs no game-memory patch, hook, save write, or world operation. The proxy name must be chosen from the executable's imports and installed-mod inventory, with a written rollback path.
- **Mad Max ScriptHook layer:** the Rust plugin becomes our own game-specific runtime API. It adds capabilities one at a time: first a read-only, useful world/area signal if a safe callback can be established, then a spawn operation with validated inputs and a proven cleanup path. Each adapter copies stable values; raw pointers never leave the narrow unsafe module. Hook install waits for verified post-decryption signatures on the supported Steam build and fails closed on any mismatch.
- **Separate Rust mod runtime (deferred):** after a useful bridge capability exists, a normal Windows process may run a headless Bevy `App`/ECS for higher-level mod logic and persistent state. It receives versioned events over local IPC and returns validated commands for execution at safe game-thread points. This isolates most mod rules from the game process, but adds lifecycle and IPC complexity; it is not required to prove that the ASI bridge works.
- **Mod-owned construction data:** a versioned file outside the native save stores stable placement IDs, existing game asset/object identifiers, transforms, and world identity. The sidecar runtime owns atomic writes and playthrough scoping. Area-ready events drive reconciliation by stable ID so restore does not duplicate placements.
- **Bootstrap proof trigger:** none. A startup log is sufficient; avoid introducing a hotkey, overlay, or console before the game's input/world lifecycle is understood.
- **Placement trigger after capability proof:** use a temporary, documented development command/hotkey or sidecar command, selected after the bridge callback is known. The bridge validates and executes a request only at a verified safe game-thread point.
- **Scope control:** the first code artifact proves only ASI loading/build detection. The first game capability is then established separately. Do not map every engine system or define a general mod API before one read-only event and one safe operation work. Bevy/IPC are added only when they simplify a demonstrated use case.

Save-format research documents a playthrough-creation timestamp at header offset 0x18 and a slot number at 0x48. Local read-only inspection found distinct timestamps in occupied slots 1, 2, 3, and 10, and matching values in both available backup copies. This is promising for profile identity but does not reveal the active slot to a runtime. Require explicit profile selection until active-slot identification is demonstrated; never choose the newest save as a guess. The first prototype is independent but should document its formats and interfaces for possible future interoperability with community tools. This does not assume an upstream home or authorize external outreach.

## Bootstrap checkpoint (before construction capability work)

1. Review the exact loader, proxy name, file hashes, install/rollback steps, and Rust toolchain needed. Tell the user before installing either the toolchain or base loader. The game must be closed during installation.
2. Build a minimal Rust Windows `cdylib` exposing only the loader-required ASI entrypoint and a log path. The user reviews every Rust line before accepting it.
3. Load it on the exact supported Steam executable. It records the executable fingerprint and loader/plugin startup; it installs no hooks and makes no memory changes. An unknown build logs a refusal and performs no game operation.
4. Remove the plugin and proxy, restore any original file if one was renamed, and verify the unmodded game still starts.

Passing this checkpoint proves only loading and rollback. It does not pass the construction feasibility gate, establish game API safety, or authorize a broader runtime.

## Construction milestone acceptance (after the bootstrap checkpoint and capability gates)

On one verified Windows PC executable build:

1. A documented launch path identifies the exact supported executable. An unknown build is disabled without installing hooks.
2. The sidecar starts its headless Bevy schedule and reports a clear disconnected state if the bridge is unavailable. A bridge or sidecar exit fails closed; no queued command is replayed against a later game session.
3. A development trigger produces one validated command from the sidecar; the bridge executes it only at the verified safe game-thread point and spawns one existing, non-destructive structure at the intended open-world transform.
4. A successful placement is recorded by the sidecar in a mod-owned file associated with the active playthrough; the native save file is not modified by the mod.
5. After leaving and re-entering the area, an area-ready event reaches the sidecar and the structure returns at the saved transform exactly once. Full game restart/reload is a subsequent verification step if the same persistence profile can be identified reliably.
6. Loading a different playthrough does not show the first playthrough’s placement.
7. Removing the prototype placement removes its sidecar record and prevents it from reappearing after reload.
8. Logs identify build detection, IPC connection/version, placement identity, persistence profile, and restore result well enough to diagnose a failed experiment.
Before target-specific runtime implementation, a feasibility investigation must establish whether an existing structure can be safely spawned and removed, which world-ready signal is usable and on which thread, and how the active playthrough can be distinguished. These are evidence gates, not assumptions. If reliable spawning, area-load detection, or per-playthrough identification cannot be established, the milestone stops at a documented technical finding rather than pretending persistence works.

## Expansion path

0. Prove the Rust ASI bootstrap loads, fingerprints the target build, fails closed, and rolls back cleanly.
1. Verify a read-only world/area signal and safe game-thread boundary; then prove one existing structure can be spawned and cleaned up.
2. Add the separate headless Bevy sidecar and versioned IPC only after the first useful bridge capability is verified; complete one existing structure spawn and open-world restore with mod-owned persistence.
3. Support multiple existing object types, rotation/placement edits, and clean deletion without duplicates.
4. Discover stable identities and load/unload events for one camp or instance; persist instance-local placements without leaking them into other contexts.
5. Add a practical in-game placement workflow and a companion developer tool if community testing shows a need.
6. Investigate custom assets and mod-defined systems as separate design stages, each backed by evidence that the engine exposes the needed object lifecycle and save/load hooks.

## Risks and unresolved questions

- Does the game expose a safe runtime path to spawn an existing static structure, or must the mod reconstruct a game object from lower-level engine types?
- Which events reliably mark an open-world area or instance as ready for restoration, and what threading rules apply?
- Is there a stable save/playthrough identifier that can key sidecar data across slots, stores, and restarts?
- Can existing objects be spawned with their expected collision, scripts, destruction behavior, and cleanup, or will the first prototype need to limit object classes?
- How do overlapping runtime loaders and mods sharing proxy DLL names affect installation and compatibility?
- What development launch path pairs the sidecar with the correct game instance, and how should it recover from either process exiting?
- What exact interaction should mature construction use: in-game selection/placement, a command interface, or a companion-assisted workflow?
- What portions of the work should be offered upstream or coordinated with the maintainers of the existing save and format tools?

## Decisions left to the implementation plan

- Exact Rust targets, libraries, and hook backend after a read-only feasibility pass against the chosen game build.
- A prototype trigger and safe test location.
- The concrete asset/object identifier for the first structure.
- Storage encoding and exact profile identifier, after evidence is gathered.
- Which ASI proxy name is conflict-free for the installed game and which specific game capability can be exposed safely. The prototype is treated as an independent, reusable project; future community integration remains open.
- Whether adding the separate Bevy sidecar/IPC boundary materially helps after the first bridge capability is proven.
- Manual in-game verification procedure and rollback instructions.

## AI and authorship workflow

- AI may search and summarize decompiled code, compare save/file snapshots, propose hook targets, and draft focused code with evidence and source references.
- The user remains the Rust author. If AI drafts Rust, each changed line must be reviewable, with an explanation of unsafe blocks, FFI boundaries, hook signatures, and persistence behavior before the user accepts it.
- Do not generate or accept a broad runtime API before the first capability is proven.
- Keep experiments reproducible: record game build, source save/profile, exact test steps, logs, and before/after observations.


