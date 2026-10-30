# World Query Caller Activity Design

## Intent

Identify which of the game's observed world-query callsites are active most often during play. The current bounded capture records detailed snapshots for selected calls and reports each unique caller once, but it gives no per-caller activity rate.

## Goals

- Count every call by its return address using a fixed atomic counter indexed by the existing caller table.
- On the background logger thread, report non-empty activity windows every five seconds, with the top twelve callsites by count.
- Keep the hook path allocation-free; do not add per-call log writes or expand the detailed sample budget.
- Keep counters and output bounded to the existing 256 caller slots and one compact line per active window.

## Non-goals

- Do not infer hit/miss from the shared helper Boolean.
- Do not count return values for unsampled calls or add a detour on every return.
- Do not resolve caller names in the hook; report RVAs for correlation with static/runtime labels.
- Do not interact with, close, or relaunch the running game.

## Approaches

- **Log every call:** rejected because a hot query site can produce unbounded log volume and hook-path I/O.
- **Count calls and summarize on the existing worker:** selected. It adds one relaxed atomic increment to each call and moves aggregation/formatting to the existing background thread.
- **Hook every return and count result states:** deferred. It expands the return interception surface and is unnecessary for the immediate question of which callsites are hot.

## Constraints and behavior

- Preserve existing ammo, height, GraphScript, and first-call-per-other-site sample caps.
- Counters are process-local `u64` values. Each five-second window reports the count delta per caller; empty windows produce no log line.
- The summary reports caller RVAs and counts only. It does not imply that a caller is a physical raycast or that a result is a collision.
- The currently running game and its installed plugin remain untouched; the new build is for a later launch.

## Acceptance evidence

- Release build succeeds and formatting/diff checks are clean.
- Static review confirms the hook only increments an atomic and all allocation, sorting, and formatting stay on the background worker.
- Runtime verification is deferred until the candidate can be installed in a later game session.
