# Mad Max Player Transform Feasibility Research

**Date:** 2026-10-27  
**Plan:** `docs/superpowers/plans/2026-10-27-mad-max-rust-transform-research.md`  
**Outcome:** `NEEDS_USER_REVIEWED_PROBE_DESIGN`

## Summary

Existing community code gives us a much better starting point than a generic function scan: it hooks a Steam-specific `UpdatePostSim` callback, then dispatches queued work after the original callback returns. MadMaxSpawner uses that route to retrieve Max's character and call its transform virtual method. This is a credible design lead for a Rust-native first capability.

It is not yet safe to implement as a direct call. The local Steam executable has the matching timestamp and preferred image base, but the SDK relies on hard-coded absolute addresses, a runtime-initialized build discriminator, and executable data in the unusual `.xpdata` section. Static file bytes cannot confirm what those addresses contain after the game's startup/decryption path. The public callback chain also does not establish thread identity, player lifetime, or safe behavior during loading/menu transitions. The next step should be a narrow source-reviewed in-process probe proposal; do not write/build/install it until the user reviews all Rust lines and explicitly approves proceeding.

## 1. Public call chain and evidence

The [MadMaxSpawner repository](https://github.com/gigaHours/MadMaxSpawner) pins the `mm` submodule to [`mm_sdk` commit aab754c7](https://github.com/gigaHours/mm_sdk/tree/aab754c7bab016d3f043143eead03669dea3076a).

- [`CCharacterManager::GetPlayerCharacter`](https://github.com/gigaHours/mm_sdk/blob/aab754c7bab016d3f043143eead03669dea3076a/game/charactermanager.h) is declared as a thin call through a Steam/GOG address pair.
- [`CGameObject`](https://github.com/gigaHours/mm_sdk/blob/aab754c7bab016d3f043143eead03669dea3076a/game/go/go.h) declares `GetTransform(CMatrix*)` as a virtual method and asserts a `0xD0` base-object size.
- [`CCharacter`](https://github.com/gigaHours/mm_sdk/blob/aab754c7bab016d3f043143eead03669dea3076a/game/go/character.h) has multiple inheritance through damageable, character-base, RTTI, and physics interfaces. A Rust implementation cannot assume a simple one-base object layout or guess a vtable slot from the declaration alone.
- [`ImGuiRenderer::UpdatePostSim`](https://github.com/gigaHours/mm_sdk/blob/aab754c7bab016d3f043143eead03669dea3076a/imgui/imgui_renderer.cpp) calls the original callback, then `ProcessGame()`. `ProcessGame()` drains queued game events and then calls each renderer's `Game()`.
- [`MadMaxSpawner`'s queued spawn action](https://github.com/gigaHours/MadMaxSpawner/blob/b4a08cc39728e5050cff3bbb06fb5d16fb03a265/spawnermenu.cpp#L597) null-checks `GetPlayerCharacter()`, calls it a second time, and invokes `GetTransform(&Mat)`. It ignores the transform method's boolean result.
- The menu/state guard inside `Game()` runs **after** queued events are drained that frame. The example has no thread-ID assertion, object-destruction synchronization, manager-null check, or documented lifetime guarantee.

**What this demonstrates:** another modder has a concrete path to Max's character and transform and uses an `UpdatePostSim` hook as a game-work dispatch point.

**What remains unproven:** the target executable compatibility, actual callback thread, character/manager lifetime during transitions, Rust ABI/vtable representation, transform success semantics, and coordinate conventions.

## 2. Local executable and SDK address assumptions

Read-only audit of `C:\Program Files (x86)\Steam\steamapps\common\Mad Max\MadMax.exe` confirmed:

- SHA-256: `CD719A721DB92575AA32FF01E07A590F47DD34B702B1BCD98EC698A07358BB0D`
- Size: `76,814,792` bytes; PE timestamp `0x5628A448` (`2015-10-22 08:54:32 UTC`)
- AMD64 PE32+; preferred image base `0x140000000`; `SizeOfImage=0x05D88000`; entry RVA `0x01B36728`
- PE DLL characteristics `0x8120`; the `DYNAMIC_BASE` bit is absent. The loaded address still needs confirmation in-process.
- Steam app ID `234140`; manifest build ID `1398848`.
- Sections include `.data`, `.xcode`, `.xpdata`, `.xtext`; there is no `.text` section.

The SDK [`hookmgr.h`](https://github.com/gigaHours/mm_sdk/blob/aab754c7bab016d3f043143eead03669dea3076a/hookmgr.h) comments “MadMax.exe 1.0.3.0.” Its `ADDRESS`/`CMETHOD` helpers use hard-coded **absolute virtual addresses** and do not rebase them or validate per-function bytes. Its `ACT_EXE` discriminator reads `0x14192A150` and compares it with `0x1c3c00001bf0`; on this local executable that VA maps into `.data` beyond the section's raw-file-backed range, so the local disk image cannot confirm the runtime value.

Relevant Steam addresses map inside the local image, which is only a plausibility check:

| SDK item | Steam VA | Local static mapping |
|---|---:|---|
| `CCharacterManager` singleton | `0x1417F5338` | `.data` zero-file-backed virtual tail |
| `GetPlayerCharacter` | `0x141C91250` | `.xpdata`, raw file offset `0x849E50` |
| `UpdatePostSim` callback | `0x141FA5A10` | `.xpdata` |
| `CGameState::m_InMainMenu` | `0x1417FDB2B` | `.data` zero-file-backed virtual tail |
| `CGameState::m_State` | `0x1417FDB2C` | `.data` zero-file-backed virtual tail |
| SDK Steam startup return address | `0x14303978D` | `.xpdata`, raw file offset `0x1BF238D` |

The `.xpdata` section has PE characteristics `0x40000040`, which do not include the executable-memory flag. The game may change live page protections or map decrypted code elsewhere, but this has not been observed. Any in-process probe must query the live page protection and abort unless the callback address is actually executable. The raw on-disk bytes at `GetPlayerCharacter` are `DA-75-03-4B-49-45-00-21-C2-E7-47-3B-B8-EF-55-D3`; at the SDK startup return address they are `95-E8-24-38-79-4D-E2-2F-39-00-09-8B-61-84-65-7B`. These are file bytes only; they do not verify runtime instructions or function meaning. The SDK's Steam startup hook watches `GetStartupInfoW` for return address `0x14303978D`, described in source as `_tmainCRTStartup`; our existing ASI initialization timing has not been shown to match that arrangement.

[MadMaxFix source](https://github.com/Alexxlml/MadMaxFix/blob/main/src/fix.c) provides a separate precedent: it checks the same Steam timestamp, waits for expected runtime code bytes, then validates bytes before installing hooks. This supports treating runtime-byte validation as necessary; it does not validate the SDK's player or `UpdatePostSim` addresses.

## 3. Current Rust plugin lifecycle

Read-only source review of `src/lib.rs`, `src/overlay.rs`, and `src/game_probe.rs` found:

- `InitializeASI` starts a worker that installs the DX11 overlay.
- After the overlay installs, another worker starts the existing display-mode probe.
- The display-mode candidate checks executable timestamp/size, preferred image base, waits up to 120 seconds for expected bytes, then hooks an unrelated display-mode function and copies its return value through atomics.
- The overlay render function consumes the copied value; this demonstrates a render-thread display path, not a game-thread callback.
- The source tree's display-mode candidate is not the reviewed installed plugin. Do not build/install the current source tree as if it were the approved hotkey-only baseline.

No existing Rust entry point proves safe player-object access. A new transform read must not be added to the DX11 render callback or existing worker based on current evidence.

## Go/no-go decision

`NEEDS_USER_REVIEWED_PROBE_DESIGN`.

The SDK has substantially narrowed the search: `UpdatePostSim` is the leading callback candidate, and the transform call chain is known. But address ranges within the PE, matching timestamp, source comments, and a sample mod's usage do not establish the runtime bytes, ABI, callback thread, or object lifetime on this exact Steam installation.

Prepare a narrowly scoped proposal for a read-only, in-process Rust diagnostic that validates the exact target and callback before attempting to copy the transform. Show every Rust source line to the user before build/install. The proposal should explain how it fails closed, how it avoids dereferencing stale/null objects, how it verifies readiness/thread assumptions, and what the user will see if the value is unavailable. No Cheat Engine or external memory-reading workflow is needed. If those safety checks cannot be justified, stop rather than add the call.

No code, dependencies, tests, game files, installed plugins, or saves were changed during this research.

## Provenance and license note

The SDK repository is public but has no discoverable license file in the inspected repository tree. GitHub's [licensing guidance](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/licensing-a-repository) says that without a license the default copyright rules apply; public visibility alone is not a general grant to reproduce or create derivative works. This project therefore uses the source as technical reference and does not copy, translate, vendor, or redistribute its implementation. The user prefers to keep the mod unpublished if distribution terms become an issue; that does not change this report's no-copying boundary.


## Follow-up: source-level ABI checks and crash isolation (2026-10-27)

A second exact-revision inspection clarified source declarations that were previously treated as unknown:

- [`CMETHODV` and `CCharacterManager::GetPlayerCharacter`](https://github.com/gigaHours/mm_sdk/blob/aab754c7bab016d3f043143eead03669dea3076a/hookmgr.h#L13-L16) show an instance call with `this`; [`CAvaSingle<CCharacterManager>::Instance`](https://github.com/gigaHours/mm_sdk/blob/aab754c7bab016d3f043143eead03669dea3076a/core/avasingle.h) and the Steam singleton definition in [`avasingle.cpp`](https://github.com/gigaHours/mm_sdk/blob/aab754c7bab016d3f043143eead03669dea3076a/core/avasingle.cpp) establish the SDK's intended manager pointer source.
- [`DEFHOOK(void, UpdatePostSim, (void* ctx))`](https://github.com/gigaHours/mm_sdk/blob/aab754c7bab016d3f043143eead03669dea3076a/hookmgr.h#L18-L19) and its use in [`imgui_renderer.cpp`](https://github.com/gigaHours/mm_sdk/blob/aab754c7bab016d3f043eead03669dea3076a/imgui/imgui_renderer.cpp) show the SDK callback shape as `void(void*)`, with the original callback called before queued game work.
- The SDK's [`HookMgr::Initialize`](https://github.com/gigaHours/mm_sdk/blob/aab754c7bab016d3f043143eead03669dea3076a/hookmgr.cpp) installs plugin hooks synchronously when the `GetStartupInfoW` return address matches `0x14303978D`. An earlier Rust draft spawned a worker from this gate; it may have raced the startup function's return, but this is not proven to have caused the crash. That draft is not used by the current candidate.

The previous review concern that `GetPlayerCharacter` might be static is resolved at the SDK-source level. Local runtime ABI compatibility is still not proven. The transform call remains blocked by the multiple-inheritance adjustment, vtable entry, and object-lifetime uncertainties.

### Diagnostic recommendation

The crash report does not identify whether the failure occurred during hook creation, the first `UpdatePostSim` call, or a later player-object call. The current source candidate removes every player-object operation while preserving the previous hook sequence: overlay installation, worker startup, executable-page wait, and hook installation. The callback only calls the original and increments an atomic counter; the overlay displays that count. A stable run with advancing counts would point toward the removed player-object path, but would not identify which operation was unsafe. Review the exact Rust diff before build/install.
