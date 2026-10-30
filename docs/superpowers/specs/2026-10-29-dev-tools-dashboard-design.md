# Mad Max Dev Tools Dashboard and Script Discovery

**Date:** 2026-10-29  
**Status:** Approved for implementation by the user's request to proceed on 2026-10-29
**Scope:** Replace the script-console window's mixed layout with Console/Scripts tabs and make package discovery visible without auto-running scripts.

## Intent

Give mod authors a clear place to discover, inspect, and manage scripts while keeping command output and entry separate from script status. The in-game window becomes **Mad Max Dev Tools**, with **Console** and **Scripts** tabs.

## Goals

- Preserve the existing console's prompt, bounded scrollback, history, ANSI colors, and command behavior in the Console tab.
- Give Scripts its own scrollable dashboard for discovered packages and loaded/active generations.
- Discover packages from the configured direct-child package layout under `MadMaxModding/scripts`, so users do not need to know IDs in advance.
- Show each package's ID, manifest/API validity, declared callbacks/capabilities, and current loaded status/generation when available.
- Provide explicit lifecycle actions for the selected package: Load, Reload, Run (when `on_run` exists), and Unload (when loaded).
- Keep file enumeration, reads, and manifest validation on the existing loader worker; cross threads only with bounded owned DTOs.
- Keep discovery separate from execution: discovery never compiles or runs Rhai code, and a user explicitly chooses Load.
- Make `script list` return discovered package inventory and current runtime status rather than only reporting loaded scripts.

## Non-goals

- Automatic script loading or execution on game startup.
- Automatic reload-on-save, recursive package search, script editing, arbitrary new lifecycle phases, or a general plugin marketplace.
- Changing the F1/F2/F3 bindings, input suppression model, or game input mapping.
- Reintroducing the mod-owned `InputFrame` build until its startup crash is isolated and a safe candidate is available.

## Approaches considered

1. **Keep the current single console layout and improve `script list`.** Lowest implementation cost, but command output and runtime status remain mixed together and there is no dedicated dashboard.
2. **Two-tab Dev Tools window with asynchronous discovery and explicit load (selected).** Console output stays in Console; Scripts shows discovered and loaded packages in a scrollable list with actions. This directly matches the requested layout and avoids executing files merely because they exist.
3. **Two tabs with auto-load of every discovered package.** Convenient, but any package copied into the directory would execute during startup. This conflicts with deliberate lifecycle control and is not selected.

## Package discovery and safety

- Scan direct child directories only; a package consists of `manifest.toml` and `main.rhai` as in the current loader format.
- Canonicalize the configured root and package paths, retain the current root-confinement checks, and reject symlink/junction escapes.
- Bound discovered package count, manifest bytes, names, and returned DTO size. Discovery reports invalid/missing manifests as per-package issues rather than aborting the whole scan.
- Parse and validate manifest metadata, but do not compile Rhai source or execute callbacks until Load is requested.
- Trigger an initial scan when the Scripts tab is first opened, allow an explicit Rescan, and route `script list` through the same discovery operation. A missing scripts directory is reported as an expected-path/empty state and can be rescanned after creation.
- Merge discovered metadata with the manager's current status by stable script ID. A deleted source package remains visible as loaded until explicitly unloaded; status should make the missing source apparent.

## UI behavior

- Keep F3 as the hardcoded toggle for the Dev Tools window; F1 remains input/cursor capture and F2 remains UI configuration. Escape closes Dev Tools with the existing input-release behavior.
- Tabs are visible at the top of one resizable window. The Console tab contains only console controls/output/prompt; the loaded-script list is removed from it.
- The Scripts tab has a scrollable inventory of available and loaded packages. Selecting a row shows details and only the lifecycle actions valid for that row's current status.
- Scan/load/reload/run/unload progress and errors appear as concise status in the Scripts tab; command history and general diagnostic output remain in Console.
- The Scripts tab and Console retain independent selection/scroll state while switching tabs.

## Runtime flow

```text
Scripts tab opens / Rescan / `script list`
  -> bounded discovery request
  -> loader worker scans and validates package metadata only
  -> bounded owned discovery result reaches the UI
  -> UI merges inventory with copied runtime status

User chooses Load / Reload / Run / Unload
  -> existing bounded lifecycle command path
  -> existing game callback and ScriptManager semantics
  -> status/error events reach the dashboard and console output as appropriate
```

## Constraints and acceptance

- No filesystem traversal or Rhai compile work runs on the render or game callback thread.
- Existing console commands and script lifecycle behavior continue to work from Console and from dashboard buttons.
- An empty/missing scripts root has a useful empty state and can recover after the directory is created.
- A directory with a malformed manifest appears as an invalid discoverable entry with a reason; valid neighboring packages remain listed.
- A discovered package remains unloaded until explicit Load. Loading reports success only after the existing asynchronous preparation/activation completes.
- The input-enabled ASI is separately under crash investigation and stays rolled back during this work unless a corrected input candidate is verified.
