# UpdatePostSim hook isolation diagnostic

**Date:** 2026-10-27  
**Status:** Built, installed, and confirmed in Mad Max by the user.

## Question

Can this ASI survive the same `UpdatePostSim` hook installation path used by the failed transform probe when all player-manager and transform operations are removed?

The earlier run installed `UpdatePostSim` and crashed during startup before logging a player sample. The WER data does not identify whether the detour, the first callback, or a later game-object operation caused the fault.

## Candidate behavior

- Validate the same Steam executable timestamp, file size, and loaded image base.
- After the DX11 overlay reports installed, start the probe worker as in the previous candidate.
- Wait for the `UpdatePostSim` target page to be committed executable memory, then install the detour.
- The detour calls the original `void(void*)` callback with the unchanged context, then increments an atomic counter.
- The DX11 overlay displays the callback count. The callback performs no logging, allocation, locks, pointer reads, or game API calls after the original callback returns.
- Do not install the `GetStartupInfoW` gate. Do not read `CCharacterManager`, call `GetPlayerCharacter`, inspect object/vtable memory, or call `GetTransform`.

This keeps the same broad hook timing as the failed run and changes the callback body by removing player-object operations. It still cannot isolate every sub-operation or prove what the crash's instruction was executing.

## Run record

- Build: `cargo build --release --target x86_64-pc-windows-msvc` completed successfully with exit code 0.
- Artifact: `target/x86_64-pc-windows-msvc/release/madmax_bootstrap.dll`; PE machine `0x8664`, PE32+ optional header `0x020B`, export `InitializeASI`; SHA-256 `B04875264DA21B8A66A59578427304D99AEB5642233D4EB14D79BC4F8AAEAE8D`.
- Installed as `Mad Max/scripts/MadMaxModding.asi` with the same SHA-256. The previous overlay-only plugin is backed up as `MadMaxModding.asi.before-callback-isolation-20261027`; its SHA-256 is `3E9DEAC81E64D5F9F7E8054C48DE83C472E075A929D1E129D57ED0786E2F2D3F`.
- The real `MadMax.exe` process (PID 16612) was still responding after about 25 seconds. No matching Application log event 1000/1001 appeared in the inspected recent window.
- The bootstrap log confirms the DX11 overlay and callback hook installed at `0x141FA5A10`. It does not record callback counts; that count is visible only in the overlay, so whether it advanced is not yet confirmed here.

## Interpretation

- If the game remains stable and the overlay count advances, the removed player-object path becomes the leading explanation for the earlier crash. This does not identify the unsafe call or validate the object layout.
- If the game crashes, the hook installation, callback/trampoline, callback ABI, early timing, or overlay interaction remains implicated; do not restore player reads.
- If the game remains stable but the overlay shows zero, the target may not have been reached or the callback may not be active; do not add transform calls to compensate.

## Run boundary

The user approved building and running this callback-only candidate. Keep the known-good backup in place until the observation is complete. Do not add or run automated tests unless requested.

