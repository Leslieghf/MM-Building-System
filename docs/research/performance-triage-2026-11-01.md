# Runtime performance triage — 2026-11-01

## Findings

- The native raycast adapter calls the mapped game world-query helper directly. The passive world-query detour is reverse-engineering instrumentation, not a dependency of `ctx.raycast`.
- The normal startup path was installing the camera-ray callback observer, `CWeapon::GetRayWorldPosition`, the shared world-query helper, and the player-aim filter candidate observer. The world-query detour also installed candidate and ray-filter observers. The world-query detour saves a large register/context snapshot and performs caller/sample accounting for game-owned queries, not only mod queries.
- The research capture buffer is bounded at 596 samples per run. `bootstrap.log` is cumulative across runs; its current 61.8 MB size and 5-second activity reports do not establish that the current session wrote that whole volume.
- The spawn catalog uses an ImGui list clipper. Its 466 entries are not all drawn each frame, so the catalog count alone does not explain accumulating lag.
- `debug_spawn_menu` and the two ray workbench examples previously defaulted to continuous ray requests. A frame-local result map coalesces only exactly identical requests in one accepted game callback; it does not coalesce different filters or requests across callbacks.
- Ray tickets were retained for up to 240 callback sequences (a 120-callback result expiry plus a 120-callback cleanup grace). Consuming a completed/failed/expired result now removes its ticket immediately; expiry remains the fallback for abandoned tickets.

## Performance changes

- Added the opt-in Cargo feature `research-taps`. Ordinary builds no longer install the camera-ray, weapon-ray, or world-query observer hooks. Build with `--features research-taps` only when collecting reverse-engineering captures.
- Changed the spawn browser to issue one ray query per catalog selection, not a continuous query while its script is active.
- Changed continuous ray workbench examples to start disabled. Enable them explicitly when inspecting a ray.
- Removed consumed terminal tickets immediately.
- Cached the validated native ray-query addresses after their first successful resolution. The live world-query base pointer and its readability are still checked for each request, so this does not cache world/session state.
- Avoided allocating a frame-local hash map for the common single-ray request; identical requests are still shared within a callback batch when there are multiple requests.
- Reduced per-frame UI work in the spawn browser by precomputing folder paths/labels and clipping the folder list as well as the catalog grid. Visible tile IDs and shortened labels no longer require temporary strings.
- Removed per-frame deep clones of script-panel publications and replaced the cloned open-state map with a temporary move out and return after drawing.
- Enabled ThinLTO for release builds. This can optimize Rust-side wrapper/rendering code, but it cannot optimize the game’s native ray-query implementation or Havok code.

## Remaining cost boundary

- `ctx.raycast` still invokes the game-owned world-query helper synchronously. Each non-identical ray request that reaches the callback still performs a real native query, and frame-local sharing only removes exact duplicate requests in that callback. If one ray per frame remains slow after this pass, the dominant cost is likely inside that native query or its setup, and needs in-game timing to confirm; ThinLTO cannot remove it.
- This source inspection identifies avoidable allocations and repeated validation, but does not quantify frame time. Compare the same scene and UI state in-game before attributing a remaining slowdown to a specific subsystem.

## UI-versus-runtime timing probe

- The DX11 render callback synchronizes the UI Bevy world and runs its presentation schedule on every rendered frame, then rebuilds visible ImGui windows. Runtime toggles do not currently stop that UI-world work.
- Added opt-in readouts under the existing Runtime debug section. They report previous-frame CPU time for overlay sync/presentation, input handling, debug HUD, script panels, console/configurator, spawn browser, and total overlay callback.
- Added latest-mod-game-callback and latest native world-query CPU samples. These help separate Dear ImGui/UI CPU layout from script/ECS work and the specific synchronous raycast helper. The probe excludes the game's actual GPU presentation time and the original native game callback.
- The UI profiler itself is only drawn when the Runtime section is enabled. Compare readings with windows opened/closed to identify which stage tracks the slowdown.
- `cargo tree --locked -e features` confirms the project requests only `bevy_app/std` and `bevy_ecs/std`. No Bevy renderer, 2D/3D, UI, audio, or Bevy physics subsystem appears in the resolved feature graph.
- Inference: trimming Bevy rendering/physics features further is unlikely to be the next frame-time win because they are already absent. The existing Dear ImGui overlay and its render callback are separate from Bevy and remain the main UI-side measurement target.

## Validation boundary

The release build compiled successfully with ThinLTO. Compilation verifies the code shape but not in-game frame time. The current live Mad Max process is still running the previous installed build, so its performance cannot validate these changes until the new build is installed and compared in the same gameplay scene. No log was deleted or truncated.
