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

## Validation and remaining in-game check

The regression test checks repeated request submission; the camera-copy tests check independent freshness handling. These tests cannot verify live camera orientation or the game’s rendered aim point. After installing this build, inspect whether the compact hit marker follows the reticle smoothly. If it does not, capture camera origin/direction, fraction, and hit point in one controlled sample before changing matrix math or filter selection.
