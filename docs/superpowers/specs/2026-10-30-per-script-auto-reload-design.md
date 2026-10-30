# Per-Script Automatic Reload Design

## Intent

Let a modder edit a loaded Rhai package in an external IDE and have MadMaxModding prepare and activate the new version automatically. The Scripts tab controls this per loaded script with a compact inline toggle. Manual load/reload remains available.

## Current state

- `ScriptLoader` owns package file access and Rhai compilation on a worker thread. Requests and results use bounded channels.
- `ScriptManager` activates a prepared generation at the game callback boundary. A failed preparation does not replace the active generation.
- The Scripts tab has a loaded-runtime list, and the UI already sends lifecycle requests through the runtime boundary.
- Script packages contain `manifest.toml` and `main.rhai`; source and manifest reads are size-limited and confined to the configured scripts root.
- There is no automatic file watching. Reload is currently explicit.

## Goals

- Add an inline Auto Reload On/Off toggle for each loaded script in the Scripts tab.
- Default the toggle to Off for newly loaded scripts. Keep its value for the current loaded script across successful reloads; unloading removes it.
- Detect edits to the loaded package's manifest or `main.rhai` and request a normal compile/replace through the existing worker and game-boundary activation path.
- Keep file I/O, fingerprinting, debounce, and compilation off the game/render callback.
- A syntax error, invalid manifest, missing file, or transient partial save reports one useful error and leaves the last working generation active. Do not retry the same unchanged bad contents every polling interval.
- Keep UI and worker messaging bounded and avoid flooding console output during an editor's save sequence.

## Non-goals

- Watching unloaded packages or automatically loading newly discovered packages.
- Editing scripts inside the game.
- Watching nested assets; current packages execute only manifest metadata and `main.rhai`.
- Persisting toggle settings between game sessions.
- Changing script state reset behavior on successful reload.
- Adding a new script language or changing capability boundaries.

## Approaches

1. **Worker polling (selected recommendation).** The existing loader worker checks only enabled, loaded package IDs on a modest interval, fingerprints the two bounded package files, and debounces a changed fingerprint before handing it through the existing prepare/replace route. This has no new dependency, tolerates editors that save through rename, and keeps filesystem work on the existing worker. With at most 16 active scripts, bounded metadata/content checks are practical. Polling adds up to one interval of reload latency.
2. **Filesystem notification crate.** A watcher reports changes quickly, but editors commonly replace a file by rename and may emit several events for one save. The host still needs debounce, content fingerprinting, bounded queues, and worker-side validation. This adds a dependency and more Windows notification behavior without removing the hard parts.

The recommendation is worker polling at approximately 500 ms. The interval is a builder choice and may be adjusted if measurement shows avoidable I/O or UI delay.

## UI behavior

Each loaded-runtime row keeps the script ID and status, with a compact inline button such as `Auto Reload: Off`. Clicking it sends a typed request to the game runtime and updates the row to `On` or `Off` after the runtime accepts the change. The button is available only for loaded scripts. It does not insert a synthetic command into the console transcript.

## Lifecycle and failure behavior

- A newly loaded script starts with Auto Reload Off.
- Turning it On registers that script's package with the loader worker. Turning it Off removes its watch state and cancels pending automatic reload notifications that have not begun preparation.
- A change is considered stable after the same file fingerprint is observed in two consecutive polling passes. The fingerprint covers both `manifest.toml` and `main.rhai` and includes file presence, bounded content length, and content hash, so atomic-save and same-size edits are detected.
- At most one automatic preparation per script may be pending. Further edits during preparation coalesce into a later scan.
- A failed prepare reports one error for that attempted content fingerprint; the old active generation keeps running. A different fingerprint may trigger a new attempt.
- A successful prepared generation activates at the normal game callback boundary, and the Auto Reload setting is preserved for that script ID.
- Unload removes the watch; reloading a script manually uses the same failure-safe replacement semantics.
- If a manual reload arrives while an automatic reload for that script is queued or compiling, the manual request supersedes the automatic request; at most one prepared replacement for that script may activate.
- Automatic reload failures and queue saturation appear as one error in the script's loaded-runtime row and one error event in the console transcript. Repeated polling of unchanged invalid contents does not repeat either message.
- If the package or a watched file is moved/deleted while watching, report the stable missing-path state once and keep watching the configured package path. Restoring the package or saving valid files there resumes normal detection; unloading or turning Auto Reload Off removes the watch.

## Boundaries and limits

- Only manifest and entry source are watched, using existing byte limits and root-confinement checks.
- Watch configuration, worker notifications, and reload requests remain bounded. Queue saturation is visible as one status/error and can be recovered by the next stable changed fingerprint.
- File handles are closed after each scan; no script file remains locked for the duration of gameplay.
- No file watching or hashing runs in Rhai callbacks or native game hooks.

## Verification

- Unit-test fingerprint changes for content edits, manifest edits, missing files, same-size edits, and stable-save debounce.
- Test enable/disable, unload cleanup, preservation of Auto Reload through a successful generation swap, and no automatic repeated retries on unchanged invalid source.
- Test that a failed compile leaves the prior generation active and that editing back to valid content reloads it.
- Test bounded queue handling and ensure polling work executes only on the loader worker.
- Verify the inline toggle uses the current loaded-script row and does not submit a console transcript command.
- Build the Windows release and manually verify: load script, enable toggle, save a visible change, observe reload; introduce a syntax error, verify prior generation remains active; save a valid fix, observe recovery; disable toggle, save another edit, verify no reload; unload and verify watch cleanup.

## Decisions to confirm

- This document chooses session-only state and worker polling as implementation defaults. If the user expects settings to persist across game sessions or wants immediate event-driven notification instead of a sub-second polling interval, revise before planning.

## Builder-check resolutions

- Manual reload precedence: a new manual request supersedes pending automatic work for the same script. This keeps the user's explicit command authoritative and prevents duplicate activation.
- Failure/saturation reporting: show a single error status on the script row and one console error event. This makes the issue visible in the UI while preserving the console's existing diagnostics channel.
- Missing or renamed package files: report once and retain the watched package path until re-enabled/unloaded, allowing recovery when files are restored without requiring a toggle cycle.
