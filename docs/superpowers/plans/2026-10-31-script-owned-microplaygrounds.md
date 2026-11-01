# Script-Owned Micro-Playgrounds Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let Rhai packages publish bounded, interactive ImGui micro-playgrounds, ship `reticle_ray` as the first Ray Workbench, and expose a global visibility toggle in the UI Configurator.

**Architecture:** Rhai publishes owned panel descriptions and consumes copied widget changes on a later `game.post_sim` callback. A bounded mailbox transfers descriptions and interaction events between the game and UI worlds; the native ImGui renderer validates and draws them on the render thread. A `UiPreferences` setting in the existing UI Configurator globally shows or hides script panels without unloading scripts or changing individual panel visibility.

**Tech Stack:** Rust 2024, Bevy ECS 0.19, Rhai 1.27, Dear ImGui through Hudhook DX11, TOML package manifests, generated Rhai API documentation.

**Spec:** `docs/superpowers/specs/2026-10-31-script-owned-microplaygrounds-design.md`

## Global Constraints

- Keep Rhai execution out of the DX11/ImGui render callback.
- Scripts publish panel models in `game.post_sim`; native ImGui renders copied, validated models.
- UI events are bounded, typed, and attributed to package, generation, panel, and widget IDs.
- Script panels do not replace `OverlayMode`; they coexist with the compact HUD and other script panels.
- F1 remains the global input-capture toggle; F2/F3 and existing overlay behavior remain unchanged.
- The UI Configurator's `Script panels` toggle defaults to enabled and hides/shows panels only; it does not unload or pause scripts or alter per-panel visibility.
- Reuse the current blue-gray theme, opacity, compact spacing, title-bar styling, and fixed-precision formatting from the approved mockup.
- The UI panel capability is explicit (`ui.panels`); do not grant panel creation to scripts that did not request it.
- Raycast and debug-draw APIs remain separate capabilities; the UI adapter does not execute either one.
- Do not add or run tests unless the user asks for testing or verification; use build and user-directed in-game checks for this pass.

## Review Focus

- Old-generation UI events arriving after reload/unload: ignored before script state changes.
- Full widget-event mailbox: coalesce slider/numeric values, preserve checkbox/button edges within the documented bounded policy.
- Invalid IDs, non-finite widget values, and over-budget panel models: reject that package update without affecting other panels or the render frame.
- Master toggle disabled while a panel is open: hide it without losing per-panel open state; re-enable restores that state.
- Game callback delayed or unavailable: render the latest copied panel model without blocking or executing Rhai in the renderer.

---

## File Map

- `src/api/scripts/mod.rs`: owned panel/widget/event DTOs and queue limits shared across the runtime boundary.
- `src/api/ecs/transport/mod.rs`: bounded panel-model and widget-event channels in `GameEndpoint`/`UiEndpoint`.
- `src/api/ecs/systems/script_dispatch.rs`: collect per-generation panel publications from script dispatch.
- `src/api/ecs/runtime/mod.rs` and `src/api/ecs/runtime/effects.rs`: move published models to the UI and return UI events to the game-side script manager on callback boundaries.
- `src/api/scripts/manager.rs` and `src/api/scripts/manager/invocation.rs`: generation-scoped publication and widget-event access during callback invocation.
- `src/api/scripts/package.rs`: validate the explicit `ui.panels` manifest capability through the API registry.
- `src/api/rhai/bindings.rs` and `src/api/rhai/catalog.rs`: bounded Rhai panel builder/value access, capability checks, and generated API metadata.
- `src/devtools/state/overlay.rs`: global `show_script_panels` preference, default on.
- `src/devtools/state/presentation.rs`, `src/devtools/state/console.rs`, and `src/devtools/state/mod.rs`: UI-owned panel snapshots, visibility state, event staging, and Scripts dashboard reopen state.
- `src/devtools/presentation.rs`: consume newest panel snapshots in the render world without game access.
- `src/devtools/overlay/mod.rs`: draw script panels beside the HUD, apply global configurator visibility toggle, preserve current theme and capture semantics.
- `src/devtools/overlay/panel.rs`: add the Scripts dashboard list/reopen control and shared small widget helpers; avoid duplicating the existing console UI palette rules.
- `examples/scripts/reticle_ray/manifest.toml` and `main.rhai`: declare `ui.panels` and build the first Ray Workbench.
- `docs/scripting.md` and `src/api/rhai/catalog.rs` (generated to `docs/rhai-api.md` by `build.rs`): describe capability, event timing, limits, and example panel; deploy the generated reference as `MadMaxModding/API.md` beside the scripts.

## Task 1: Owned Panel Contract and Cross-World Transport

**Files:**
- Modify: `src/api/scripts/mod.rs`
- Modify: `src/api/ecs/transport/mod.rs`
- Modify: `src/api/ecs/systems/script_dispatch.rs`
- Modify: `src/api/ecs/runtime/mod.rs`
- Modify: `src/api/ecs/runtime/effects.rs`
- Modify: `src/api/scripts/manager.rs`
- Modify: `src/api/scripts/manager/invocation.rs`

**Interfaces:**
- Produce bounded owned `ScriptPanelModel`, `ScriptPanelWidget`, `ScriptPanelWidgetKind`, and `ScriptPanelEvent` records. Every model/event carries `ScriptId`, `ScriptGeneration`, panel ID, and widget ID as applicable.
- `ScriptPanelWidgetKind` supports label/heading, checkbox, bounded slider, bounded numeric input, fixed-option selector, and button.
- Add one bounded publication channel game-to-UI and one bounded widget-event channel UI-to-game. Use `try_send`/nonblocking drain only.
- Invocation exposes current-generation panel input changes as owned values and collects at most the configured model/widget/event budget per callback.
- `ScriptPanelPublication { script_id, generation, panels }` replaces the previous publication for that script; an empty `panels` list removes its windows. Script unload emits an empty publication for its former generation.

- [ ] Define the DTO field types, identifier/text limits, numeric bounds, per-package/per-frame quotas, and mailbox capacities next to the existing script effect limits.
- [ ] Add channels to the endpoint pair; make panel publication replacement-based per script generation so removed widgets do not linger.
- [ ] Drain UI events at the accepted game callback; discard stale generation events before passing values to Rhai.
- [ ] Collect a script's complete panel snapshot after its callback succeeds; publish an empty snapshot on unload and discard old queued publications/events when a generation is replaced.
- [ ] Preserve discrete checkbox/button changes and coalesce continuous slider/numeric values when the UI event queue reaches capacity.
- [ ] Confirm no Rhai values, borrowed game data, closures, ImGui handles, or native pointers cross either channel.

## Task 2: UI-Owned Panel State, Rendering, and Configurator Toggle

**Files:**
- Modify: `src/devtools/state/overlay.rs`
- Modify: `src/devtools/state/presentation.rs`
- Modify: `src/devtools/state/mod.rs`
- Modify: `src/devtools/presentation.rs`
- Modify: `src/devtools/overlay/mod.rs`
- Modify: `src/devtools/overlay/panel.rs` only if a shared rendering helper is needed

**Interfaces:**
- Add `UiPreferences::show_script_panels: bool`, default `true`.
- Add UI-owned resources for latest panel snapshots and per-panel open/closed state. UI state is keyed by `(ScriptId, panel_id)` and retained across matching script generations.
- The renderer sends typed `ScriptPanelEvent` values through the UI endpoint; it never calls the script manager or game adapter.
- `ScriptConsoleUiState` retains the panel index used by the Scripts dashboard's reopen controls.

- [ ] Add a `Script panels` checkbox to `draw_configurator_controls`; it changes only global panel visibility.
- [ ] Render script panels as independent ImGui windows after the HUD, gated by the global toggle and each panel's open state; do not modify `OverlayMode`.
- [ ] Match the approved mockup: compact dark blue-gray body, blue title bar, title left, owner ID right, existing opacity, readable spacing, current number formatting, and no extra lifecycle/status boilerplate.
- [ ] Respect F1 capture: when input capture is off, panel windows are display-only with `NO_INPUTS`; when on, controls interact and existing game-input suppression applies.
- [ ] Preserve visibility for matching IDs on reload; remove state for unloaded packages; allow the Scripts dashboard to reopen a closed panel when global visibility is enabled.
- [ ] Reject malformed or over-budget models individually and keep rendering other panels and the HUD.

## Task 3: Rhai Panel API and Capability Gate

**Files:**
- Modify: `src/api/rhai/catalog.rs`
- Modify: `src/api/rhai/bindings.rs`
- Modify: `src/api/scripts/mod.rs`
- Modify: `src/api/scripts/package.rs`
- Modify: `docs/scripting.md`
- Generated: `docs/rhai-api.md` via `build.rs`

**Interfaces:**
- Add capability ID `ui.panels` to `API_CAPABILITIES`.
- Register a Rhai `ctx.ui` facade only for `ui.panels` packages. It provides `ctx.ui.events(panel_id) -> map`, `ctx.ui.panel(id, title) -> MadMaxPanelBuilder`, and `ctx.ui.publish(panel) -> ()`.
- Builder methods are `panel.heading(widget_id, text)`, `panel.label(widget_id, text)`, `panel.checkbox(widget_id, label, value)`, `panel.slider(widget_id, label, value, min, max)`, `panel.number(widget_id, label, value, min, max, step)`, `panel.select(widget_id, label, value, options)`, and `panel.button(widget_id, label)`; they append one bounded widget and return the builder for chaining.
- Widget event map values are copied Rhai primitives/maps and are read on the next callback; button events are one-callback edges.

- [ ] Implement the `MadMaxPanelBuilder` registration and conversion to owned DTOs; reject duplicate IDs, invalid bounds/defaults, unknown option values, non-finite numbers, and over-limit text/options.
- [ ] Require `ui.panels` for panel builder, event reads, and publication; keep `ctx.raycast` and `ctx.gizmos` capability checks independent.
- [ ] Document delayed event timing, replacement semantics, widget value shapes, limits, and the explicit capability in the catalog and scripting guide.
- [ ] Regenerate `docs/rhai-api.md` through the existing Cargo build script and review its generated diff.

## Task 4: `reticle_ray` Ray Workbench and Compact HUD

**Files:**
- Modify: `examples/scripts/reticle_ray/manifest.toml`
- Modify: `examples/scripts/reticle_ray/main.rhai`
- Modify: `src/devtools/presentation.rs`
- Modify: `docs/scripting.md`

**Interfaces:**
- `reticle_ray` declares `ui.panels` alongside its existing `game.read`, `game.raycast`, and `debug.draw` capabilities.
- The panel exposes one ray's enabled state, independent origin/direction source selection, supported filter, max distance, custom origin/direction values, query diagnostics, and independent origin/ray/hit visualization switches.
- The script remains the only layer that composes raycast requests with gizmo drawing.

- [ ] Process panel events before making the next request; issue at most one raycast per callback using exactly the configuration shown in the panel.
- [ ] Show `pending` after changing ray configuration until the matching ticket completes; clear prior hit data and hit gizmo on miss, unavailable data, disable, or source change.
- [ ] Display origin, normalized direction, range, filter, ticket state, optional hit position, fraction, and distance in the Ray Workbench only.
- [ ] Remove ray query rows from the normal HUD presentation while preserving the compact Player/Vehicle sections and other enabled HUD fields.
- [ ] Keep ray visualization optional and separate from all UI widget methods.

## Task 5: Documentation, Build, and In-Game Acceptance

**Files:**
- Modify: `docs/scripting.md`
- Generated: `docs/rhai-api.md`
- Deploy: `MadMaxModding/API.md` beside installed scripts, following the existing procedure recorded in `docs/research/install-history.md`

- [ ] Ensure the generated API file deployed beside scripts at `MadMaxModding/API.md` includes `ui.panels` and the exact Rhai signatures/limits.
- [ ] Build the release ASI/package and preserve the existing install backup procedure; do not install over a running Mad Max process.
- [ ] Ask the user to launch the installed build and verify the mockup match, simultaneous HUD/workbench visibility, configurator toggle, input-capture behavior, source/filter/range changes, miss clearing, script reload/unload, and panel reopen behavior.
- [ ] Record any in-game-only discrepancy and fix it before calling the feature complete.

## Plan Self-Review

- Spec goals/non-goals are assigned across Tasks 1–4; Task 5 handles generated/deployed documentation and live acceptance.
- Generation ownership, bounds, and queue-full behavior are covered in Tasks 1–2 and the Review Focus list.
- The configurator master visibility toggle is in Task 2, defaults enabled, and is explicitly separate from package lifecycle and per-panel visibility.
- Freecam and camera mutation remain excluded, matching the approved design.
- No unit tests are scheduled because this project task did not request test authoring or execution; use build and user-directed in-game verification as the validation path.
