# Rust In-Process Address Readiness Probe Design

**Date:** 2026-10-27  
**Status:** Draft for user approval; no implementation authorized yet.

## Intent

Build a temporary, read-only diagnostic into the existing Rust ASI to learn whether the SDK's two candidate player-transform code addresses are executable and readable in the live Mad Max process after startup. This closes the evidence gap before any Rust function pointer, hook, game manager, or player-object access is attempted.

## Why this first

The SDK identifies `UpdatePostSim` at Steam VA `0x141FA5A10` and `GetPlayerCharacter` at Steam VA `0x141C91250`. Both map into the local `.xpdata` PE section, whose on-disk characteristics (`0x40000040`) do not mark it executable. The game may decrypt code or change protections at runtime, but we have not observed that. Reading the target pages from inside the plugin after startup is the least invasive way to learn whether the addresses are live executable code on this exact build.

## Goals

- Keep the diagnostic in Rust and inside the existing ASI; no C++ bridge/toolchain.
- Confirm the loaded main-module base and Steam PE timestamp/size before evaluating any candidate address.
- Poll only the two SDK code candidates (`UpdatePostSim` and `GetPlayerCharacter`) from the expected Steam address set.
- Query each target page's current state and protection using the Windows API; read a short byte window only when the page is committed, readable, and executable. Require `MEM_COMMIT`, reject `PAGE_GUARD` and `PAGE_NOACCESS`, and accept only readable executable protection modes (for example, `PAGE_EXECUTE_READ` or `PAGE_EXECUTE_READWRITE`). Keep every read wholly within the queried page; if the requested window would cross a page boundary, shorten it to the current page rather than reading into an unchecked page.
- Copy observed values and bytes into a local diagnostic log with elapsed time, so a later offline review can compare startup states and decide whether the addresses are credible.
- Fail closed on build mismatch, unexpected image base, address outside the image, uncommitted page, guard/no-access page, failed read, timeout, or unexpected state.
- Preserve the currently installed hotkey/overlay plugin. Show every Rust source line changed or added before building or installing this diagnostic.

## Non-goals

- Do not install a hook or patch any game code.
- Do not call `GetPlayerCharacter`, `GetTransform`, `UpdatePostSim`, or any other game function.
- Do not read or dereference a manager/player pointer or any game object.
- Do not scan arbitrary memory or inspect another process; reads are limited to the current process and the two explicit addresses.
- Do not use Cheat Engine or another external memory tool.
- Do not change saves, game files, the installed plugin, or dependencies as part of this design.
- Do not add or run tests unless the user asks.

## Approaches considered

1. **Passive in-process address readiness probe.** Check module identity, page state/protection, and a short byte window at the two known addresses. It cannot prove function semantics or thread affinity, but it provides the missing runtime mapping evidence without executing game functions. **Selected for this spike.**
2. **Install `UpdatePostSim` hook and read Max's transform immediately.** Delivers the desired signal sooner, but assumes the target page is executable, the callback ABI is correct, the callback runs on a safe thread, and player objects are valid. The static evidence does not support those assumptions yet.
3. **Poll from the DX11 render hook or use an external process-memory tool.** The render thread has no demonstrated game-object safety, and the user has rejected Cheat Engine as a modding requirement. Not selected.

## Behavior

After ASI startup, the diagnostic worker validates the local executable metadata and loaded image base. It computes the two target virtual addresses from the loaded base plus their SDK RVAs, confirms that each lies within the executable image, and periodically queries page metadata for a bounded startup window. When a page is committed and has an allowed readable-executable protection, it copies a small byte window from that exact address using a current-process read API. The window is capped at the end of the checked page so a read never crosses into an unvalidated page. It logs the bytes plus protection/state. It never invokes or patches the address.

The log reports `supported metadata`, `loaded base`, target address, page state/protection, timestamped byte snapshots, read errors, and timeout. It does not call the bytes a verified signature or claim that either function was identified. If a page never becomes executable, the diagnostic records that fact and stops without attempting a hook.

The current source tree contains an unapproved display-mode probe candidate not present in the installed hotkey build. The implementation plan must remove or replace that candidate as part of the reviewed source diff; it must not accidentally build it alongside this diagnostic. The existing overlay renderer must not read the candidate bytes or call the game functions.

## Success criteria

- The exact Steam executable identity and loaded base are validated before any candidate address is used.
- Each attempted read is range-limited to the two addresses, checks current page state/protection, and reads only the current process.
- The diagnostic performs no hook, patch, game-function call, pointer dereference, or save write.
- Every failure produces a clear log reason and no fallback scan.
- The user reviews the complete Rust diff before build/install, then decides whether to run the diagnostic in game.

## Follow-up gate

If the live bytes and page protections look consistent across a user-approved run, inspect the evidence offline and make a separate design decision for the next capability. Any `UpdatePostSim` hook must be separately reviewed for runtime byte validation, calling convention, thread/lifecycle safety, and player validity before it reads a transform. If the target address is not executable/readable or the observations are inconsistent, stop and investigate the SDK/build mismatch rather than guessing another address.

## Constraints

No C++ tooling, new dependency, external memory tool, automated test, or installation is required until the user approves the design and reviews the complete Rust source. Tell the user to close the game before replacing the installed plugin; only run the candidate after source review and explicit user approval.
