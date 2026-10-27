# Mad Max Player Transform Probe Design

**Date:** 2026-10-27  
**Status:** Draft for user approval; no transform implementation authorized yet.

## Intent

Extend the existing Rust ASI diagnostic to show Max's world position in the overlay. This is the first useful game-state read and a foundation for later construction tools, not a construction system itself.

## Evidence and constraints

- The user observed `player character found` in the main menu. During the initial loading menu the overlay remained `waiting`, matching the known behavior that `UpdatePostSim` callbacks pause during loading.
- The pinned SDK declares `CCharacterManager::GetPlayerCharacter()` at Steam VA `0x141C91250`, and the returned pointer is a `CCharacter*`.
- `CCharacter` uses multiple inheritance. `GetTransform(CMatrix*)` is a virtual method on its `CGameObject` base, so a Rust call needs the correct C++ base adjustment and vtable dispatch.
- The SDK's `CMatrix::Position()` returns `_41`, `_42`, `_43`. Honor `GetTransform`'s boolean result; read no matrix values when it returns false.
- The existing callback runs in the menu and gameplay and pauses during loading. All game-object access stays on that callback, after the original callback returns.

## Goals

- Display a fresh position sample in the overlay at a low rate, at most once per 120 `UpdatePostSim` callbacks.
- Keep hook lifecycle, executable checks, scheduling, status handling, and overlay rendering in Rust.
- Use one narrow C ABI C++ helper for C++ inheritance adjustment, virtual `GetTransform` dispatch, and copying three floats to caller-owned outputs.
- Fetch the character for each scheduled sample; do not retain game-owned manager, character, matrix, or other pointers between callbacks.
- Return an explicit unavailable result for missing manager/character, unavailable getter or bridge, failed transform, or invalid/non-finite coordinates.
- Keep the probe read-only. Show every changed or added Rust and C++ source line before building or installing it.

## Non-goals

- No construction, object spawning, game-state writes, save changes, custom scripts, or general-purpose mod API.
- No direct Rust vtable call, guessed object offset, or transform call from the render hook or worker thread.
- No Cheat Engine requirement and no external process-memory access.
- No vendoring or redistribution of the community SDK's implementation. The bridge should be a small local adapter based on reviewed declarations; pause if exact class layout or licensing cannot be handled responsibly.
- Do not add or run automated tests unless the user asks.

## Approaches considered

1. **Small C++ ABI bridge, selected by the user.** Rust remains the runtime and calls one C ABI function with a fresh opaque character pointer. C++ performs the class adjustment and virtual transform call, checks success, and copies out only three floats. This limits C++ to the part that depends on C++ object layout.
2. **Rust-only virtual dispatch.** Determine the exact subobject adjustment and vtable slot, then reproduce them in Rust. This avoids a C++ compiler dependency but adds fragile reverse-engineering assumptions to every transform read.

## Behavior

After the existing callback invokes the game's original function, every 120th callback checks the manager and obtains the current character. If available, Rust calls the C++ adapter immediately. The adapter invokes `GetTransform` and returns a success flag plus position scalars; Rust stores only those scalars and displays them. A failed or invalid sample updates a readable status and does not expose a game pointer. Loading pauses samples naturally because the callback does not run there.

## Safety and compatibility boundaries

- Continue to require the exact supported Steam executable and expected loaded image base.
- Validate known singleton/getter addresses and the character pointer's readable page before use; these checks reduce risk but cannot prove object lifetime or layout correctness.
- Call the adapter only on the observed `UpdatePostSim` path and never cache the character pointer.
- Verify the SDK class declarations and C++ ABI against the pinned SDK revision before implementation. Build the bridge for the same x64 MSVC ABI as the ASI. Check whether the current Rust build environment already has a usable C++ compiler; tell the user before installing any missing tooling.
- If the exact bridge declarations, ABI, or compiler setup cannot be verified, stop and report the blocker instead of falling back to guessed Rust vtable access.

## Success criteria

- The overlay shows plausible coordinates that change when Max moves and remain stable when he is stationary, while never writing game state.
- Failed transform reads show an explicit status rather than stale or uninitialized coordinates.
- The plugin remains stable in a user-launched Steam session; the known-good callback-only ASI remains available for rollback.
- The user reviews every changed Rust and C++ source line before build/install, and starts the game through Steam for the run.

## References

- [`CGameObject::GetTransform`](https://github.com/gigaHours/mm_sdk/blob/aab754c7bab016d3f043143eead03669dea3076a/game/go/go.h)
- [`CCharacter` inheritance](https://github.com/gigaHours/mm_sdk/blob/aab754c7bab016d3f043143eead03669dea3076a/game/go/character.h)
- [`CPhysicsGameObject` inheritance](https://github.com/gigaHours/mm_sdk/blob/aab754c7bab016d3f043143eead03669dea3076a/game/go/physgo.h)
- [`CMatrix::Position`](https://github.com/gigaHours/mm_sdk/blob/aab754c7bab016d3f043143eead03669dea3076a/math3d.h)
