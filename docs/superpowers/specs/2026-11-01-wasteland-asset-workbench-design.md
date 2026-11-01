# Wasteland Asset Workbench Design

## Intent

Expand the GMod-inspired spawn browser into a broad, searchable Mad Max content workbench. The game overlay should remain responsive and should use its existing native spawn path; an offline companion prepares asset metadata, readable text, audio previews, and baked visual thumbnails. The user should be able to discover complete encounters and individual candidate assets without confusing “present in the files” with “safe to spawn.”

## Current state

- The in-game browser currently has four content tabs, a folder tree, spawn tiles, and a Tools/Settings sidebar. It is populated from curated and community JSON catalogs.
- Q is intercepted in the Windows message hook while `debug_spawn_menu` is active. The hook cannot inspect the current ImGui item under the cursor, so it currently captures every Q press while that script is enabled.
- The installed path census contains 27,501 named paths; the current SARC inventory covers 467 entity packages and 10,944 unique compiled member paths. The package index already links entity definitions to candidate model, collision, script, graph, animation, effects, and texture resources.
- Known inventory families include 2,479 `.ddsc` texture paths, 12,919 animation paths, 576 `.xvmc` compiled scripts, 428 `.gsrc` compiled graphs, 30 `.fsb` sound banks, 11 `.fev` audio event banks, 1,127 `effects/` paths, and 38 `.mdic` world/decorative-index paths. These are path/family counts, not confirmed viewer support.
- `.xpy` and similar readable-looking source paths often correspond to compiled resources inside bundles. Shaders have not yet been identified as a trustworthy standalone family. Format discovery must precede claims of plaintext source or shader viewing.

## Goals

1. Keep the in-game browser responsive and make its Q hold behavior respect UI intent.
2. Present spawning and inspection as related workbench functions: Props, Vehicles, NPCs/Characters, Encounters/Compositions, Textures, Models, Scripts/Graphs, Audio, Animation, Effects/VFX, and World/Streaming.
3. Preview/read resources without requiring a game restart: textures, readable text, audio, models, and animation/skeleton data as format support becomes available.
4. Generate stable thumbnails offline and let the game overlay display only cached metadata/images; no archive extraction, decoding, or asset rendering on the game render thread.
5. Preserve exact source paths, archive/member provenance, entity relationships, uncertainty, and the green/yellow/red semantics. Archive existence alone never promotes an entry to “verified.”
6. Support complete encounter browsing alongside single-object candidates, NPCs, props, effects, and resources; keep spawn capability independent of viewer capability.

## Non-goals for the first tranche

- Decode every game format before exposing any useful view.
- Interpret compiled XVM as editable original source without a demonstrated decoder/decompiler.
- Claim an asset is a shader, standalone NPC, physical prop, or spawnable solely from its filename or extension.
- Extract all game archives or ship copyrighted game payloads, textures, audio, or model data in the mod repository.
- Add per-frame file scanning, asset decoding, or thumbnail rendering to Mad Max's injected render callback.
- Implement animation authoring or overwrite game content.

## Approaches considered

1. **Hybrid offline preparation plus lightweight in-game browser (chosen).** A local offline asset tool indexes/reads selected resources and prepares safe previews. The initial extraction/index utilities reuse the existing Python research tooling; an interactive desktop workbench can be built in Rust once its formats and needs are grounded. The injected overlay consumes a small generated manifest and cached previews, while its existing game-native spawn path remains authoritative. This separates costly/format-specific work from the game UI and fits the existing path census and catalog.
2. **Decode and preview inside the injected overlay.** This would make every asset immediately interactive, but increases crash, latency, and memory risk in the game process and mixes archive/format parsers into the render callback.
3. **External-only viewer with the existing spawn menu unchanged.** This avoids game-process risk but forces users to jump between tools and loses the requested integrated browsing/discovery flow.

The hybrid was selected by the user. It costs an offline tool and a manifest/cache boundary, but keeps the live game thin and lets viewers ship incrementally.

## Product structure

The workbench is split into a source inventory, offline preview pipeline, and in-game projection:

- **Source inventory:** exact logical path, type/family, archive or bundle location, content relationships, structural class, evidence, and spawn/viewer status. It includes whole entity compositions and single-resource entries.
- **Offline workbench:** searchable local index, selected-resource extraction into a mod-owned cache, read-only text display, audio audition, texture preview, model preview/render, and animation/skeleton inspection as each codec is understood. Heavy processing is on demand and cached.
- **In-game browser:** compact tabs/folders/search, preview thumbnail, metadata/status, and an explicit Spawn action where a safe game entity key exists. Viewing a resource does not imply spawning it. The overlay does not parse game archives.
- **Generated deliverables:** stable resource manifest plus content-addressed preview cache. The game-side manifest contains relative paths and metadata only; source payloads remain local to the installed game or its mod-owned cache.

## Q-toggle interaction contract

- Q remains a hold-to-open menu key and stays suppressed from the game whenever it is consumed by the overlay.
- A Q key-down opens the menu unless the last rendered UI frame says an excluded UI control owns the interaction. Top-level spawn tabs and individual spawn tiles are explicitly allowed and do not lock Q; text input, folders, tools, settings, and other buttons do.
- If Q was consumed and the menu is already open, its key-up is always consumed and closes the menu, regardless of current pointer target. This prevents a stuck menu when the cursor moves over a text field, settings control, or side panel.
- Other UI controls remain usable while the menu is open. The gate concerns opening/toggling by Q, not ordinary mouse interaction.
- Focus loss clears the held/consumed state. UI target classification is published as a small atomic snapshot from ImGui after each frame; the WndProc hook does not call ImGui APIs.

## Resource view behavior

- **Textures:** identify `.ddsc` and other image candidates; begin with a format probe and one representative decode. Show source path, dimensions/format if decoded, and cached image. Unsupported variants remain listed with format status.
- **Scripts and graphs:** display known text resources read-only with plain monospace text. For compiled `.xvmc`/`.gsrc`, show strings/hex or decoded structure only when justified by a format reader; label representation clearly. Shader status remains unknown until identified.
- **Models:** resolve `.model_xml` references to compiled `.lod`/`.rbs` candidates and dependencies. Render in the offline viewer; generate compact transparent thumbnails for the in-game grid. Keep collision and render-model previews distinct.
- **Audio:** index `.fsb`/`.fev` and entity-referenced audio. Audition only through the offline tool, with duration and source identity; no autoplay in game.
- **Animation suite:** connect actors/models, skeleton/rig resources, clips/animation banks, and animation references. First milestone is browse/inspect; playback follows once the asset contract is understood.
- **Effects/VFX:** expose named effect candidates and parent-entity links. Preview only when a safe standalone offline renderer is available.
- **Spawned content:** encounter packages remain browseable as compositions; character and rigid-object root candidates get their own filtered views, without claiming every class record is a world-spawnable instance.

## Performance and data rules

- The in-game overlay receives immutable, prebuilt manifest records and thumbnail paths; it never scans the 27,501 path list, reads archive members, or decodes media while drawing.
- The offline tool indexes once per game build and updates only changed metadata/cache entries.
- Viewer work is selection-driven, bounded, cancellable, and cached by content hash. No background sweep decodes every resource by default.
- Every row retains provenance and separate `exists`, `decoded`, `preview_ready`, `spawn_route_known`, and `spawn_observed` states.

## Delivery phases

1. **Input safety and inventory contract:** implement the Q hit-target gate; define an offline manifest schema capable of representing all resource families and relationships.
2. **Offline asset index and readable text:** use existing path/SARC indexes, provide safe selected-resource extraction, and open genuinely readable text resources; compiled formats are identified rather than mislabeled as source.
3. **Texture and audio viewers:** decode representative `.ddsc` and `.fsb/.fev` resources, then make cached, explicit-selection viewers.
4. **Model viewer and thumbnail baker:** resolve geometry/material dependencies, render offline, and publish thumbnails for the game browser.
5. **Character/animation suite:** connect model/skeleton/animation resources, inspect rigs/clips, and then prototype playback.
6. **Broader effects/world views:** expose effects, VFX, encounter structures, and world-streaming data as separate asset views with evidence-backed capability labels.

Each phase should deliver a usable slice and keep unknowns visible. The user requested autonomous progress; implementation decisions within these bounds may be made without another requirements round.

## Acceptance criteria

- Q cannot open the menu while a text field or excluded control owns the interaction; Q opens normally when no excluded control is active, including over the game view, and remains usable over top-level browser tabs and spawn tiles. A consumed Q release over an excluded control pins the menu until a fresh eligible Q cycle; Escape and focus loss clear that state.
- The offline inventory can represent and search named resources, bundle members, entity references, whole compositions, and single-object candidates without conflating them.
- Read-only text, texture, audio, model, and animation support each reports whether it is a real decoded preview, a structural view, or an unsupported candidate.
- Thumbnail generation is offline, cached, and atomically published for the in-game browser.
- The game overlay remains a consumer of prepared data and continues to use its existing allowlisted spawn adapter.
- Local copyrighted game data and generated previews stay outside version control.
