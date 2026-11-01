# Catalog Stabilization and Freecam Implementation Plan

## Phase 1 — Catalog-wide spawn attempts

1. Treat green as verified in-game, yellow as an unverified attempt that may fail or destabilize the game, and red as having no usable spawn path.
2. Generate a catalog-backed ID-to-asset-path table from all curated and community entries; never accept arbitrary script-provided asset paths.
3. Make all safe-relative path-backed entries yellow and actionable; keep `spawn_readiness` as confidence metadata rather than a dispatch gate.
4. Route any spawn-menu tile through one generic script event; perform one camera ray and submit the selected catalog ID only after a hit.
5. Keep the exact Roadkill asset path and give it the same yellow experimental status as other unverified entries; it follows the same generic attempt route.

## Phase 2 — Camera-control investigation and workbench

1. Verify the actual `GetCameraMatrix` target in the supported build and identify existing hook installation patterns.
2. Add a bounded, atomic camera override state owned by the host adapter. No engine pointers cross into ECS/Rhai.
3. Install an optional getter hook that calls the original function first and modifies only the returned matrix when the override is enabled.
4. Add a script-facing API to request/clear the override, and ensure script unload/failure clears its contribution.
5. Add a separate `freecam_workbench` package with enable, world offset, relative yaw/pitch/roll, and status controls. Begin with a fixed pose adjustment; do not enable continuous flight controls yet.
6. Build and stage the candidate without launching Mad Max. Once the user closes the game, install and request a single startup check; then visually verify framing and enable/disable restoration.

## Phase 3 — Broader catalog audit

1. Review all curated and imported rows against available archive/definition evidence.
2. Keep names-only candidates yellow if they have a safe-relative asset key; their uncertainty belongs in their evidence and notes.
3. Promote rows only after a spawn lifecycle observation and user-visible behavior check.
4. Make every green or yellow catalog row actionable through its exact catalog key; red remains reserved for rows without a usable safe-relative asset path. Yellow attempts may fail or destabilize the game.

## Verification constraints

- Do not launch, close, or write into the live Mad Max process.
- Do not install a replacement ASI while Mad Max is running.
- Build the Rust target and regenerate catalog/API artifacts; do not treat compilation or native-call submission as in-game validation.
- Preserve the user’s existing working-tree changes and do not create commits unless asked.
