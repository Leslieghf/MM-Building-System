# Bootstrap install record

## Current state

- Mad Max Steam install: `C:\Program Files (x86)\Steam\steamapps\common\Mad Max`
- Loader: Ultimate ASI Loader v9.7.4, x64 `dinput8.dll`
- Source archive SHA-256: `8272D83B2692662098746F2D0AD0E2D85F3C8358AB1D63F75FBE835C2C8135FD`
- Extracted loader SHA-256: `FA266E3513D02C08A1B808F28C10538A489EAFFAA4B0707F7CC1066E71B5AFD7`
- Plugin: user-reviewed Rust DX11 hotkey-toast prototype installed as `scripts\MadMaxModding.asi` (690,176 bytes). SHA-256: `3E9DEAC81E64D5F9F7E8054C48DE83C472E075A929D1E129D57ED0786E2F2D3F`, matching the locally built DLL. This revision has not yet been confirmed in game.
- The game was closed during setup. The game folder had no `dinput8.dll` proxy to preserve.

## Rollback

Close Mad Max, then delete `scripts\MadMaxModding.asi` and `dinput8.dll` from the game root. The original install did not contain a `dinput8.dll` or `scripts` directory at inspection time. The original `bink2w64.dll` was not changed.

## Safety boundary

The user reviewed the plugin source before installation. It hooks the DirectX 11 render path to display a temporary message; it does not call Mad Max game functions, write game memory, access saves, or change gameplay state. `hudhook` is third-party MIT-licensed code with unsafe FFI internals and an upstream warning that it may crash host applications. See `docs/research/first-in-game-effect.md`.
