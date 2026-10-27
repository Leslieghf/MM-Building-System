# Rust Native Mad Max Binding Layer Design

**Date:** 2026-10-27  
**Status:** Draft for user approval; no implementation authorized yet.

## Intent

Build on the community's `gigaHours/mm_sdk` findings while keeping the mod runtime Rust-native. Instead of shipping a C++ bridge or translating the entire SDK, implement a small Rust binding layer for one verified Mad Max capability at a time. The first capability is a read-only snapshot of Max's world transform. The existing Rust ASI remains the loader-facing shell and overlay.

## Why

The SDK demonstrates that Mad Max game systems can be reached through build-specific addresses and game object interfaces. `CCharacterManager::GetPlayerCharacter()` and `CGameObject::GetTransform()` provide a concrete starting point; MadMaxSpawner uses those to position spawned entities. A minimal Rust layer lets this project learn from that work without shipping a second-language runtime. It also preserves the user's ability to review every Rust line.

## Goals

- Keep the installed Rust ASI/overlay as the mod-facing entry and preserve it as a recoverable baseline.
- Create a narrow Rust game API, with unsafe code localized behind small checked functions.
- First capability: return a copied player-transform snapshot with explicit unavailable/error state; never expose game-owned pointers to callers.
- Identify supported game builds and validate relevant addresses/signatures before calls; fail closed on mismatch.
- Establish and document a verified callback/thread before sampling game state. The existing DX11 hook and worker do not prove game-thread safety.
- Add later capabilities only as separate reviewed slices. The next likely capability is a bounded request to spawn an existing game entity; building, removal, persistence, and custom assets remain later goals.
- The user reviews all Rust source changes before any candidate build or installation, and decides before running it in game.

## Non-goals for the first slice

- No separate C++ DLL or C++ compiler requirement.
- No port of the complete SDK, ImGui, MinHook, or third-party dependencies.
- No Bevy/ECS runtime, general mod scripting VM, generated bindings, or broad engine API.
- No writes to game state or saves in the transform slice; no Cheat Engine or external process-memory tool.
- No spawn, delete, persistence, custom asset, profile-selection, or save-slot detection work in the transform slice.
- No replacement of the known-working installed plugin until a separately reviewed candidate is accepted.

## Prior work and reuse boundary

The public [mm_sdk repository](https://github.com/gigaHours/mm_sdk) contains hook management, game bindings, math helpers, ImGui, and MinHook. Its `hookmgr.h` selects fixed GOG/Steam addresses and its Steam startup path uses a hard-coded startup return address. The [MadMaxSpawner example](https://github.com/gigaHours/MadMaxSpawner/blob/main/spawnermenu.cpp) calls the player character accessor, reads a transform, and passes it to the game spawn system. These are valuable research leads, not proof that those addresses or callbacks are safe on this installation.

The repository has no discoverable license file. This design assumes an original Rust implementation of independently validated behavior and ABI facts; do not copy, translate line-by-line, vendor, or redistribute SDK source/assets. Keep links and notes about research provenance. If direct source reuse becomes important, stop and resolve permission first. Keep this project private and do not distribute mod binaries/source unless reuse and distribution terms are reviewed. Private use does not itself grant permission to reproduce or create derivative works from unlicensed source; this design avoids relying on that assumption.

## Constraints and current evidence

- Target: user's Steam `MadMax.exe`, timestamp 2015-10-22 (`0x5628A448`), SHA-256 `CD719A721DB92575AA32FF01E07A590F47DD34B702B1BCD98EC698A07358BB0D`; PE has no `.text` section.
- The installed Rust ASI and DX11 overlay/hotkey were confirmed working. They do not read Mad Max game state.
- `mm_sdk` labels its addresses for Mad Max 1.0.3.0 and selects Steam/GOG fixed absolute addresses. Its startup and transform path have not been validated against the exact local executable or lifecycle.
- `GetPlayerCharacter` is a thin function wrapper in the SDK, but the object method relies on C++ virtual dispatch and correct game object layout. Rust can represent these calls, but ABI, vtable slot, pointer validity, and thread affinity must be independently confirmed.
- Rust tooling is already installed. No C++ toolchain is needed for this design. Do not install dependencies or alter the installed plugin before the user has reviewed the implementation source.
- Do not add or run tests unless the user asks. Planned verification is build diagnostics if authorized, exact-build/address evidence, and user-controlled in-game review of the approved candidate.

## Approaches considered

1. **Separate C++ bridge DLL linked to `mm_sdk`.** Closest to the SDK's C++ object model, but adds a C++ build and a second binary, plus unresolved source reuse terms. Previously selected, then reconsidered by the user.
2. **Port the whole SDK to Rust.** Avoids C++, but needlessly recreates broad engine layouts, helpers, GUI and hook dependencies before we know which capabilities matter. Too large and risky as the first step.
3. **Rust-native minimal binding layer informed by the SDK.** Reimplement only a narrow, independently verified path and add capabilities incrementally. **Selected by the user.** It minimizes runtime language/tooling while keeping the address, ABI, and thread risks explicit.

## Proposed first behavior

A Rust module identifies the supported executable, then exposes a read-only function that can copy Max's transform into a caller-owned scalar snapshot. The function is callable only from a verified game-safe callback or queue. Before that callback is known, the overlay reports that player data is unavailable; it must not poll guessed addresses from the DX11 render callback or worker.

Address/build identification, startup timing, C++ virtual-call ABI, transform representation, and callback/thread choice are implementation details for a later plan, but must be backed by exact-build evidence and fail-closed checks. No game pointers escape the unsafe adapter. Failed preconditions return an explicit unavailable/error result.

## Success criteria

- The exact supported executable and relevant code bytes are identified and validated; an unsupported build causes a clear no-op.
- The implementation explains the verified callback/thread used for the read and the evidence for that choice.
- Rust returns only copied transform scalars and status; callers cannot retain game-owned pointers.
- Every Rust source change is shown to the user before build/install; the installed overlay baseline is preserved and recoverable.
- The approved candidate is only run in game after the user has reviewed it and chooses to proceed.

## Go/no-go checks before implementation planning

1. Confirm the SDK's player-transform path and relevant address set match the exact local Steam executable and can initialize at the required startup point.
2. Identify a verified game-safe callback/thread for reading the transform.
3. Determine whether the behavior can be implemented as an original Rust binding without copying SDK implementation. If exact source reuse is necessary, pause and resolve permission before proceeding.

If any check fails, do not build or install a candidate. Return the evidence and the smallest choice needed to continue. Tell the user before any eventual in-game installation/testing; close the game before replacing the plugin.

## Remaining decisions for planning

- Which exact callback and hook path, if any, safely exposes the player transform on the local build.
- Which transform representation and coordinate conventions are validated by game behavior.
- What small address/signature manifest format best supports the first build without prematurely designing a general compatibility framework.
