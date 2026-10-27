# First in-game effect: temporary DX11 notice

**Date:** 2026-10-27
**Status:** Rust prototype built, reviewed by the user, installed, and manually confirmed visible in Mad Max on the user's Steam install.

## Finding

The public Wasteland Storms source draws its notice from a Direct3D 11 `IDXGISwapChain::Present` detour and ImGui. It does not route the notice through the game's HUD message queue. The code initializes the render hook after its `CPlayer::UpdateController` hook runs, uses a DirectX swap-chain vtable address, and extracts the game's font from installed game archives. Its source comments say the Steam path was still not tested against Steam at that source revision; its gameplay signatures scan the executable's `.text` section. The local Steam executable previously inspected has no `.text` section on disk, so its game-function path is not yet evidence for this installation.

The narrow prototype therefore tests only the renderer path: a ten-second, input-transparent ImGui message in Mad Max's Direct3D 11 frame. It does not call Mad Max functions, change game state, touch saves, or display the game's proprietary font. Success demonstrates an in-process, visible graphics hook—not access to game systems and not construction capability.

## Implementation boundary

- Loader: Ultimate ASI Loader v9.7.4 through `dinput8.dll`; installed into the game folder while the game was closed.
- Plugin: Rust x64 `cdylib`, installed as `scripts\MadMaxModding.asi` after user review.
- Renderer hook: `hudhook` 0.9.3, pinned exactly, with only its `dx11` feature enabled. Its upstream documentation warns that it contains substantial FFI/unsafe code and may crash the host application. Treat this as experimental and keep rollback straightforward.
- User-authored Rust has no explicit `unsafe` blocks. The renderer hook and MinHook implementation are third-party dependency code and are not presented as audited by this project.
- Notice: `Rust DirectX 11 render hook active`, centered near the top of the screen for ten seconds; no input capture or persistent ImGui settings.
- Log: `%LOCALAPPDATA%\MadMaxModding\logs\bootstrap.log`.
- No automated tests have been added or run. Verification included `cargo fmt --check`, a release build, PE export inspection confirming x64 plus the `InitializeASI` export, and the user's report that the banner appeared in game. The bootstrap log confirms that the ASI entrypoint ran and the DirectX 11 render hook installed.

## Evidence and references

- [Wasteland Storms source README](https://github.com/PabloCGSilva/MadMax-WastelandStorms/blob/main/README.md) describes its status notice and Steam caveat.
- [Wasteland Storms notice.cpp](https://github.com/PabloCGSilva/MadMax-WastelandStorms/blob/main/notice.cpp) shows its Direct3D 11 `Present` overlay and use of ImGui.
- [Wasteland Storms storms.cpp](https://github.com/PabloCGSilva/MadMax-WastelandStorms/blob/main/storms.cpp) shows its `.text` signature scanning and delayed Steam startup path.
- [hudhook](https://github.com/veeenu/hudhook) is an MIT-licensed Rust rendering-hook framework supporting DirectX 11; its own crate documentation cautions about possible host crashes.
- [Microsoft's Windows crate overview](https://learn.microsoft.com/en-us/windows/dev-environment/rust/rust-for-windows) confirms Rust's Windows API projection. No direct Windows API bindings are currently written in this prototype.

## Next checkpoint

The next prototype should add one user-triggered toast to prove a controlled input-to-render path without calling game internals: a hotkey causes a short message to appear in the existing overlay. Keep the hook and message state inside the plugin, use a non-conflicting default key only after checking known community-mod defaults, avoid capturing input, and keep the message queue bounded. The user reviews every Rust line and dependency change before rebuilding or installing. After the event path works, choose a read-only Mad Max signal based on Steam-supported function evidence before touching game state.

## Hotkey toast prototype

- Local candidate changes the temporary banner into a two-second toast when `Ctrl+Shift+F6` is pressed while Mad Max has keyboard focus. It does not block or consume the key event.
- Known community defaults found during research include F5 for Wasteland Storms, and F1/F10 in a fuel cheat table. No documented conflict for the Ctrl+Shift+F6 chord was found in the pages checked; that is not a guarantee that no local mod uses it.
- The candidate reuses the same pinned `hudhook` dependency; there are no new dependencies or game-state calls.
- Candidate source passed `cargo fmt --check` and an x64 release build, exports `InitializeASI`, and was reviewed by the user. The approved build is installed as `scripts\MadMaxModding.asi`; the hotkey toast still needs an in-game check.
- The user confirmed Ctrl+Shift+F6 displays the toast in game. This verifies window-input-to-Rust-to-render delivery while the game is focused; the key remains unconsumed.
