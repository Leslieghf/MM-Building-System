# Player position probe startup crash — 2026-10-27

## Outcome

The first in-game player-transform probe crashed Mad Max during startup. The log showed the DX11 overlay and `UpdatePostSim` hook installed, but it never logged a player sample. Windows Error Reporting recorded an access violation (`0xC0000005`) at MadMax.exe RVA `0x1B34211`, in the startup code area shortly before the PE entry point. The faulting instruction and precise cause were not recovered because the WER dump was inaccessible.

The prior known-good ASI was restored and its SHA-256 matched the pre-probe backup. The game is not currently running.

## Current hypothesis and follow-up

The SDK delays plugin hook installation until `GetStartupInfoW` returns to a particular `_tmainCRTStartup` address. The probe installed as soon as the candidate callback page appeared executable, so an early hook is a plausible cause, but this is not established by the crash evidence. A startup gate modeled on that SDK trigger has since been added to the source; it is unbuilt and untested.

Independent source review found additional unresolved hazards in the direct player read:

- `GetPlayerCharacter` may be static; the current Rust call supplies a manager instance without definitive signature evidence.
- The `UpdatePostSim` callback ABI is not confirmed from a declaration.
- `CCharacter` multiple inheritance does not establish that `CGameObject` is at `player + 8`.
- The `GetTransform` vtable slot and `bool` return ABI have not been verified against the exact MSVC object layout.
- The startup gate's timing does not guarantee game initialization or object lifetime safety.

Do not run the current player-position path again until these points and the startup failure are resolved. The overlay-only ASI remains the recovery baseline.
