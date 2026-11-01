# Raycast hit-marker stability check — 2026-10-31

## Result

The reticle script was not requesting a new ray after its first callback. It compared its stored value against `ctx.frame.callback_sequence`, but the Rhai frame map exposes the monotonic callback counter as `ctx.frame.sequence`. Rhai resolved the absent property as unit (`()`), stored that value, and then saw no change on later callbacks. The visible hit marker therefore remained at an old world-space point while the camera moved, which looked like a spatially unstable/jiggling hit.

The script now compares `state.callback_sequence` with `ctx.frame.sequence` and submits once on each accepted post-simulation callback. A regression test invokes two callbacks with callback sequence 1 then 2 while keeping telemetry sample sequence at 1; both must queue rays.

## Sampling changes

Camera pose was previously read only inside `probe_player_transform`, which is gated by `GAME_STATE_SAMPLE_INTERVAL` (50 ms). That cadence is appropriate for heavier player/vehicle telemetry but too slow for a camera-driven ray. Camera position, direction, and view rotation are now sampled independently once per accepted game callback. Player/vehicle telemetry remains at the 50 ms cadence. A failed camera sample clears the copied pose, and snapshots omit camera data when its sample is older than 100 ms.

## Ghidra-backed boundary facts

- The live snapshot of `WorldQueryHelper` at `0x14252EDF5` uses the six-float ray as origin XYZ followed by direction XYZ and forms both segment endpoints as `origin + direction * distance`, then subtracts the world-query origin at `queryBase + 0x40` before dispatch.
- The live snapshot of `ProcessStaticCameraCollisionRay` at `0x141EAC0C5` reads a camera transform through virtual slot `+0xB0`, uses its translation as camera position, and forms a *focus-to-camera obstruction segment*. This is a distinct query purpose; it does not establish the direction/sign of a reticle ray.
- The live snapshot of `GetCameraMatrix` at `0x14222DA60` is partial (98 bytes, bad instruction tail). It reads manager state at `+0x480`, checks state `3`, and calls an update routine. It is insufficient to independently prove the matrix basis/sign.
- The ray hit position remains reconstructed as `origin + normalized_direction * (max_distance * hit_fraction)`. The world-query helper’s endpoint construction supports this segment interpretation. We have not established that the candidate camera basis is aligned with the visible reticle or that `static_only` is the ideal filter for every scene.

## Live request/result cross-check

The current script-labelled query tap reads the six floats passed to `WorldQueryHelper` as origin XYZ followed by direction XYZ. Its endpoint samples at distances 0 and 500 agree with `origin + direction * distance`, including the query-base translation. This independently confirms the Rust request layout and the native helper's segment arithmetic for the current path; the remaining discrepancy is upstream of that boundary or in the returned scene hit, not a mismatch in our point-add/multiply implementation.

The helper's output hit fraction at offset `0x14` agrees with the closest candidate fraction captured during traversal. In the current live samples, fractions around `0.0031–0.0042` on a 500-unit segment mean the hit is only about `1.5–2.1` world units from the supplied origin. The helper's boolean return is separate from hit presence: a hit is represented by an output fraction below the no-hit sentinel `1.0`.

These observations do **not** prove that the camera matrix row/sign chosen by `from_camera_matrix` points along the rendered view. The logged submitted directions in these samples are steeply downward; compare the script's green direction arrow with the visible center-screen aim before changing the native endpoint or hit-fraction math again.

## Validation and remaining in-game check

The regression test checks repeated request submission; the camera-copy tests check independent freshness handling. These tests cannot verify live camera orientation or the game’s rendered aim point. After installing this build, inspect whether the compact hit marker follows the reticle smoothly. If it does not, capture camera origin/direction, fraction, and hit point in one controlled sample before changing matrix math or filter selection.

## Camera direction row/sign hypothesis (2026-10-31)

The user observes in-game that a camera-origin ray works when its direction is manually set downward, but the ray disappears when using camera orientation; the camera-orientation ray appeared to point downward consistently. Player-forward with camera origin is approximately useful, and vehicle-forward with camera origin also looks plausible. After trying negative row 2, the user reported that it points backward when it should point toward the aim. This directly supports using positive row 2 for the camera ray; it also shows the camera matrix's basis convention differs from the actor/vehicle transform convention previously observed.

The first candidate changed camera direction from negated row 1 to negated row 2, based on the actor-transform convention. In-game observation showed that negated row 2 points backward. The installed correction now uses positive row 2. This is based on direct visual feedback and awaits the next normal in-game check for alignment with visible aim.

The positive-row-2 fix is installed. Next check is the normal in-game reticle-ray use: confirm whether camera orientation now follows the visible aim and hits nearby terrain/objects. The native ray adapter, origin, filter behavior, and hit-position arithmetic were left unchanged.
