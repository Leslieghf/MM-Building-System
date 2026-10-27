# Player position probe startup crash — 2026-10-27

## Outcome

The first in-game player-transform probe crashed Mad Max during startup. The log showed the DX11 overlay and `UpdatePostSim` hook installed, but it never logged a player sample. Windows Error Reporting recorded an access violation (`0xC0000005`) at MadMax.exe RVA `0x1B34211`, in the startup code area shortly before the PE entry point. The faulting instruction and precise cause were not recovered because the WER dump was inaccessible.

The prior known-good ASI was restored and its SHA-256 matched the pre-probe backup (`3E9DEAC81E64D5F9F7E8054C48DE83C472E075A929D1E129D57ED0786E2F2D3F`). The game is not currently running.

## Evidence clarified from the pinned SDK source

At `mm_sdk` commit `aab754c7bab016d3f043143eead03669dea3076a`:

- `CMETHODV` expands to a call that passes `this`, so `CCharacterManager::GetPlayerCharacter()` is an instance method. `CAvaSingle<CCharacterManager>::Instance` identifies the intended singleton storage address.
- `DEFHOOK(void, UpdatePostSim, (void* ctx))` establishes the SDK's source-level callback signature as `void(void*)`; the SDK hook calls the original callback and then dispatches queued game work.
- The SDK's own `GetStartupInfoW` hook uses return address `0x14303978D` and installs plugin hooks synchronously at that point. The Rust startup-gate draft did not reproduce that timing; it has been removed from the current candidate.

These declarations clarify the SDK's intended source-level call shapes. They do not establish that its addresses, calling conventions, or layouts match the loaded executable at runtime.

## What remains unproven

- The crash's exact cause. A player-object call is plausible because the callback was installed before the crash but no sample was logged; this is not confirmed by the WER evidence.
- Whether an `UpdatePostSim` detour installed by this ASI is compatible with the exact runtime state and any other installed hooks.
- The C++ subobject adjustment from `CCharacter*` to `CGameObject*`, the `GetTransform` vtable slot, and the exact Rust-callable virtual ABI.
- Player/manager validity and lifetime during menu, loading, death, or other transitions.
- Whether the callback's observed thread is safe for the intended game-object access.

## Current source candidate

The current source removes all manager/player/vtable/transform operations while preserving the prior probe's hook timing: after the DX11 overlay reports installed, a worker waits for the `UpdatePostSim` page to be executable and installs the hook. The callback calls the original and increments an atomic counter; the overlay displays the counter. This candidate is prepared for source review only. It has not been built or run.

If this candidate survives startup and its callback counter advances, that narrows the earlier crash toward the removed player-object operations. It does not identify which removed operation was unsafe or prove that any game-object read is safe. If it crashes, investigate hook installation, callback/trampoline behavior, or interaction with the overlay before restoring player access.

## Run boundary

Do not replace the restored overlay-only ASI yet. Show the complete changed Rust source to the user and obtain their review and approval before building or installing the callback-only candidate.

## Callback-only run update — 2026-10-27

The user-reviewed callback-only candidate built successfully, was installed with a verified SHA-256, and launched. The actual `MadMax.exe` process remained responsive about 25 seconds after startup, and no matching recent Application crash events were found. The log confirms the overlay and callback hook installed. The user confirmed callback invocation in menu/gameplay and no calls during loading screens. The previous overlay-only plugin is backed up and can be restored after the observation.

