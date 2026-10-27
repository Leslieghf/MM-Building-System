# Mad Max Rust-Only Player Transform Probe Design

**Date:** 2026-10-27  
**Status:** Revised draft for user approval; no transform implementation authorized yet.

## Intent

Extend the existing Rust ASI diagnostic to show Max's world position in the overlay. This is the first useful game-state read and a foundation for later construction tools, not a construction system itself.

## Evidence and constraints

- The user observed `player character found` in the main menu. During the initial loading menu the overlay remained `waiting`, matching the known behavior that `UpdatePostSim` callbacks pause during loading.
- The pinned SDK declares `CCharacterManager::GetPlayerCharacter()` at Steam VA `0x141C91250`, and the returned pointer is a `CCharacter*`.
- `CCharacter` uses multiple inheritance. `GetTransform(CMatrix*)` is a virtual method on its `CGameObject` base, so a Rust call needs the correct C++ base adjustment and vtable dispatch.
- In the pinned SDK declarations, `CPhysicsGameObject` lists `IPfxGameObject` first and `CGameObject` second. With the x64 MSVC ABI, this suggests a `CGameObject` subobject at byte offset `0x8`; this is a candidate layout to verify, not yet runtime proof.
- `CRtti` has a virtual destructor inherited by `CGameObject`, followed by 22 `CGameObject` virtual methods ending in `GetTransform`. This suggests zero-based vtable slot 22 if the target uses the expected single MSVC destructor entry; verify this before a call.
- The SDK's `CMatrix::Position()` returns `_41`, `_42`, `_43`. Honor `GetTransform`'s boolean result; read no matrix values when it returns false.
- The existing callback runs in the menu and gameplay and pauses during loading. All game-object access stays on that callback, after the original callback returns.

## Goals

- Display a fresh position sample in the overlay at a low rate, at most once per 120 `UpdatePostSim` callbacks.
- Keep all code in Rust, including the inheritance adjustment, virtual dispatch, checks, scheduling, status handling, and overlay rendering.
- Represent the transform as a 16-float matrix and copy `_41`, `_42`, and `_43` into Rust-owned scalars only after `GetTransform` returns success.
- Fetch the character for each scheduled sample; do not retain game-owned manager, character, matrix, or other pointers between callbacks.
- Return an explicit unavailable result for missing manager/character, invalid candidate vtable, failed transform, or invalid/non-finite coordinates.
- Keep the probe read-only. Show every changed or added Rust source line before building or installing it.

## Non-goals

- No construction, object spawning, game-state writes, save changes, custom scripts, or general-purpose mod API.
- No C++, C++ shim, direct function-address guess, or transform call from the render hook or worker thread.
- No Cheat Engine requirement and no external process-memory access.
- No vendoring or redistribution of the community SDK's implementation. The adapter should be a small local Rust binding based on reviewed declarations; pause if exact class layout or licensing cannot be handled responsibly.
- Do not add or run automated tests unless the user asks.

## Approaches considered

1. **Rust-only vtable adapter with a non-calling preflight.** Derive the candidate subobject offset and slot from the pinned SDK declarations and x64 MSVC ABI. First validate the candidate subobject/vtable pages and slot target without invoking it; only call after that evidence is reviewed. This matches the user's Rust-only preference but leaves more ABI responsibility in our adapter.
2. **Stop at player availability.** Keep the current getter-only capability and defer position until a stronger transform call contract is available. This is safest but does not advance the requested position capability.

**Selected direction:** Rust-only adapter with a separate non-calling preflight stage, per the user's instruction to avoid C++.

## Behavior

The first candidate is preflight-only. After the original callback, every 120th callback obtains the current character and checks the candidate `CGameObject` subobject at `+0x8`, its vtable, and zero-based slot 22. It reports only whether these candidate addresses are readable/executable; it does not call `GetTransform`. The user reviews that observation before we prepare a second candidate that performs the virtual call. In that later candidate, Rust passes a caller-owned 16-float matrix and copies `_41`, `_42`, `_43` only when `GetTransform` returns true and the coordinates are finite. Failed samples show a status, never a game pointer. Loading pauses samples naturally because the callback does not run there.

## Safety and compatibility boundaries

- Continue to require the exact supported Steam executable and expected loaded image base.
- Validate known singleton/getter addresses and the character pointer's readable page before use; these checks reduce risk but cannot prove object lifetime or layout correctness.
- Perform all object/vtable reads and any eventual virtual call only on the observed `UpdatePostSim` path; never cache the character pointer.
- Verify the candidate base offset, inherited destructor/vtable layout, method slot, function signature, and matrix layout against the pinned SDK declarations and x64 MSVC ABI before implementation.
- The runtime preflight checks memory state/protection for the player object, candidate subobject, vtable, and method address. These checks show only that the candidate layout is readable and executable; they cannot prove that the slot is semantically `GetTransform` or that the object remains alive. The no-call boundary is deliberate: any transform invocation is a separate reviewed/run step after observing the preflight.
- If the source and runtime evidence cannot establish the candidate slot well enough for a reviewed call, stop and report the blocker. Do not add a C++ fallback.

## Success criteria

- The overlay shows plausible coordinates that change when Max moves and remain stable when he is stationary, while never writing game state.
- Failed transform reads show an explicit status rather than stale or uninitialized coordinates.
- The plugin remains stable in a user-launched Steam session; the known-good callback-only ASI remains available for rollback.
- The user reviews every changed Rust source line before build/install, and starts the game through Steam for the run.

## References

- [`CGameObject::GetTransform`](https://github.com/gigaHours/mm_sdk/blob/aab754c7bab016d3f043143eead03669dea3076a/game/go/go.h)
- [`CCharacter` inheritance](https://github.com/gigaHours/mm_sdk/blob/aab754c7bab016d3f043143eead03669dea3076a/game/go/character.h)
- [`CPhysicsGameObject` inheritance](https://github.com/gigaHours/mm_sdk/blob/aab754c7bab016d3f043143eead03669dea3076a/game/go/physgo.h)
- [`CMatrix::Position`](https://github.com/gigaHours/mm_sdk/blob/aab754c7bab016d3f043143eead03669dea3076a/math3d.h)
