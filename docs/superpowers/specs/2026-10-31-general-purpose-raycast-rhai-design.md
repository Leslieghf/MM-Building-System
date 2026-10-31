# General-Purpose Rhai Raycast API Design

## Intent

Give Rhai scripts a reusable way to submit world-ray queries with caller-chosen geometry, retrieve copied results asynchronously, and build gameplay tools such as a camera reticle query. The current reticle probe is a diagnostic consumer, not the API's fixed shared ray.

## Goals

- Allow each script callback to request rays with an origin, direction, maximum distance, and a named supported filter profile.
- Return results on a later callback through owned ticket IDs and copied result values; never expose game pointers or synchronously call the engine from Rhai.
- Use the actual camera matrix translation and camera-forward basis for the reticle example.
- Keep the Raycast debug section to one optional `Hit position` row. The `reticle_ray` script may separately use the debug-draw capability to visualize the camera-ray direction and make a reported hit easy to see.
- Make the query API usable for other script-created origins and directions, not just camera rays.
- Keep query results data-only; scripts separately compose `game.raycast` with `debug.draw` when they want visualization.

## Non-goals

- Expose raw filter masks, native addresses, filter vtables, or arbitrary native context construction.
- Promise all game collision categories until filter behavior is mapped.
- Make a synchronous query call from Rhai.
- Add the previously proposed `configure` operation for one shared script-selected probe.

## Baseline before implementation

- `ctx.raycast` currently contains only a copied result from a hard-coded player-relative native probe.
- The installed diagnostic was player-relative: it cast sideways through Max and was not a crosshair hit marker. This was the behavior being replaced.
- `sample_camera_matrix()` reads a validated camera-control matrix, but today it is used only to derive `view_rotation`. `WorldQueryRayInput::from_camera_matrix` builds a candidate camera origin and direction, but its direction sign has not been validated against the in-game crosshair. The implementation must switch the query to the camera matrix and verify the marker against the visible aim before calling it working.
- Native query execution must happen after script dispatch through a typed effect handoff; results are copied into script-visible state for a later callback.
- The query helper requires a native query base, filter context, option and caller-specific range. The adapter owns that setup.
- The `CStaticOnlyRaycastFilter` category configuration and `CPlayerAimRaycastFilter` predicate are mapped. Their names are exposed as the initial named profiles; raw masks remain private.

## Approaches

- **Deferred request/result (selected):** Rhai submits owned parameters; the callback adapter executes accepted requests and scripts poll ticket results on later callbacks. This matches the copied-snapshot and native-boundary design, with one-or-more-frame latency.
- **Synchronous native query inside Rhai:** would make results immediate, but crosses the engine boundary during script evaluation and couples script errors and scheduling directly to native calls. Rejected.
- **Game-owned query scheduler:** potentially ideal if a reusable scheduler exists, but none is mapped or available as a callable API today. Keep this as a future backend option, not a blocker.

## Proposed script contract

Scripts call `ctx.raycast.request(origin, direction, max_distance, filter_name)` and receive a numeric ticket. Direction is normalized by the host. Inputs must be finite; coordinates are limited to ±100000 world units; distance is in `(0, 500]`; direction must be nonzero. The first profiles are `static_only` and `player_aim`; unknown profiles fail with a script error.

Scripts call `ctx.raycast.result(ticket)` in a later callback. A not-yet-completed or expired ticket returns a small result map with an explicit state; a completed result includes `has_hit`, optional `hit_position`, optional `fraction`, and `state_present`. Results are scoped to the requesting script generation so scripts cannot read one another's tickets. Tickets expire after 120 accepted game callbacks. Each script may submit at most one query per callback; the host accepts at most 8 requests per game callback in deterministic dispatch order. Requests from failed or obsolete script generations are discarded.

The script context exposes copied `ctx.camera.position` and `ctx.camera.direction` from a camera matrix sampled on each accepted game callback, independently of the slower player/vehicle telemetry cadence. A missing/stale matrix produces `()`. The context values are observations; they do not grant native access.

## Reticle example and debug display

`reticle_ray` requests a `static_only` ray from the copied camera position along the camera's candidate view direction at a 500-unit maximum distance, once per accepted game callback with fresh camera pose. On later callbacks it reads the ticket result. Its direction must be checked in game; if it points opposite or away from the crosshair, correct the camera basis/sign from observed behavior before treating the feature as complete. The script composes `game.raycast` with `debug.draw`: it draws the camera origin and both direction signs as a visual discriminator, then draws one compact marker at a reported hit. These are separate capabilities; the raycast API itself returns data only. The Raycast debug section displays exactly one row, `Hit position`, showing coordinates only when a hit position exists and `--` otherwise.

The existing direct reticle probe is removed once the general API can supply the display result, avoiding duplicate world queries. Ray cadence becomes bounded by the accepted callback and request limits rather than the old timer. The initial implementation must preserve enough request identity to publish the current `reticle_ray` result to the single debug row without exposing unrelated scripts' results.

## Safety and validation

- All Rhai inputs are copied and validated before leaving the callback.
- Native query construction and execution stay inside the game-native adapter.
- Request queues, result storage, and per-callback work are bounded by the limits above.
- Every result position is reconstructed from the exact accepted ray and range used for its request; missing hits remain optional.
- Errors, unloads, reloads, and callback failures cannot leave executable native work or stale-generation result access.

## Acceptance checks

- A script can submit two different geometries over separate callbacks and receive distinct ticketed results.
- Both named filters use their mapped native setup; unknown filter names and invalid vectors/ranges fail safely.
- Pending, hit, miss, unavailable, expired, failed-callback, and stale-generation cases have explicit tests.
- `reticle_ray` uses camera position/direction and composes raycast results with the separate debug-draw API for visual diagnosis.
- The debug Raycast section has one `Hit position` row and no other raycast telemetry rows.
- Generated API documentation describes the new methods and camera values.
