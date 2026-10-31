## Current state — 2026-10-31 same-ray filter comparison probe installed

- Installed the corrected Windows MSVC release build as `scripts\MadMaxModding.asi`, SHA-256 `9B765EDA22775E733DDD5806D97DA9A422F8B862C341C1A88E3E91FD3D50B953` (11,235,328 bytes). The pre-probe ASI is preserved as `scripts\MadMaxModding.asi.before-ray-filter-ab-20261031`, SHA-256 `900B249F2F1F25EDF64815C5D0AD2C84525E07085D01D626DA441DD4BAE0736D`. Superseded probe builds are preserved as `MadMaxModding.asi.before-ray-orientation-correction-20261031` (SHA-256 `EFA3BC532CC88859EF6D98E03E25389FA283107A591AED4882CF0C1B30301CEA`) and `MadMaxModding.asi.before-vtable-readability-check-20261031` (SHA-256 `E7D7FEF326144FEC10F8C3CAA5A75E7F8B5B141946DFF2F7054A90F1C14FE047`).
- The temporary ray starts 10 units to Max's left and 5 units above him, then travels right and down through his position. That exact ray is queried through `CStaticOnlyRaycastFilter` (mask 3) and `CPlayerAimRaycastFilter` (no extra mask write); each has separate status, hit fraction, hit position, and shape readouts. This is an in-game comparison probe, not a stable scripting API.
- Mad Max was confirmed closed before replacement and has not been launched with this build. `cargo fmt --check`, `cargo test --target x86_64-pc-windows-msvc --locked` (82 passed), and `cargo build --target x86_64-pc-windows-msvc --release --locked` passed. Existing dead-code warnings remain for disabled capture taps.

## Current state — 2026-10-31 stable semantic gizmo set deployed

Kept the distinct wire variants and the two solid forms the user requested. Rhai now exposes semantic names only; old native-name aliases and duplicate solid-shape routes 37/39 are removed from the live script/native dispatch path. The solid cube uses slot 36 and the solid 3D arrow uses slot 38. Research retains the complete 36–39 Havok type mapping. `debug_gizmo` was refreshed to show each retained visual without duplicate axis lines.

- Mad Max was confirmed closed before replacement and was not launched afterward.
- Installed `scripts\MadMaxModding.asi`, SHA-256 `D8B23512F01AD098C0F9E9A2DBC6546F475324027C4B7E07C9D487E87D319D38` (11,279,872 bytes). Previous ASI preserved as `scripts\MadMaxModding.asi.before-gizmo-api-prune-20261031`.
- Installed `MadMaxModding\API.md`, SHA-256 `EE14780A5787A72093F37C62DFBB586FF645185B315096B2143E687D21D81325`; previous copy preserved as `MadMaxModding\API.md.before-gizmo-api-prune-20261031`.
- Installed `MadMaxModding\scripts\debug_gizmo\main.rhai`, SHA-256 `F2F5CA1FBC1ADE701B24E37F9DB8660938A219B6ED7001A5BBDD5E3A9A4C229E`; previous copy preserved as `MadMaxModding\scripts\debug_gizmo\main.rhai.before-gizmo-api-prune-20261031`.
- `cargo fmt --all`, `cargo check`, `cargo build --release --locked`, and `git diff --check` passed. No automated tests were run. In-game appearance after the API cleanup awaits user launch.

## Previous installed state — 2026-10-30 CDebugRenderer line/arrow probe
- Installed `scripts\MadMaxModding.asi`, SHA-256 `70700B78F946C22374B70447C22BD06A0A3F07C43AAA4525EE3C856FF54F4F0E` (10,964,992 bytes). It adds a temporary `debug_renderer_test` console command that dispatches a cyan camera-ray tick and amber auto-width arrow through `CDebugRenderer` facade slots 1 and 9 for up to 240 `UpdatePostSim` callbacks.
- Previous installed ASI preserved as `scripts\MadMaxModding.asi.before-cdebug-renderer-test-20261030`, SHA-256 `1F11555A3EDD4464C77046E45DACE19236D10E7B17B76482ADD7254C71B6497C`.
- The probe checks the supported executable image base, facade singleton/vtable, active backend pointer and executable line/arrow methods before dispatch. It only runs on explicit command; it does not change game state. The command expects a live camera-ray sample and reports a dispatch failure in the Dev Tools console.
- `cargo fmt --all -- --check`, `git diff --check`, `cargo check --release --locked`, and `cargo build --release --locked` passed. Compilation emits existing dead-code warnings for disabled ray-query capture taps. In-game startup/render behavior is pending user verification.

## Current state — 2026-10-29 repeated script source labels deduplicated
- Installed `scripts\MadMaxModding.asi`, SHA-256 `1F11555A3EDD4464C77046E45DACE19236D10E7B17B76482ADD7254C71B6497C`. Mad Max was confirmed closed before replacement; in-game presentation is pending user verification.
- Previous ASI preserved as `scripts\MadMaxModding.asi.before-script-category-dedup-20261029`, SHA-256 `2683636A450D53558576F420AA4F46ED6CAA8B2EC857D407BE76560F3AE0F0B1`.
- Consecutive script events from the same script now share one visible `SCRIPT: <id>` category header; their output rows remain grouped underneath. A command or another output source ends the grouping. Command rendering was not changed.
- `cargo fmt --all -- --check`, `cargo build --release`, and `git diff --check` succeeded. No automated tests were run.

## Current state — 2026-10-29 vehicle_watch package corrected and deployed
- Mad Max was confirmed closed before replacing `MadMaxModding\scripts\vehicle_watch\main.rhai` with the repository example. Deployed/source SHA-256: `D917DFB2CF589F32B86D1349A90072800861956507074B7DC52A15C93B94C871`.
- Previous deployed script preserved as `MadMaxModding\scripts\vehicle_watch\main.rhai.before-vehicle-only-fixed-lines-20261029`.
- The old deployed script used `player.position`, default float interpolation, and one pipe-separated line. The updated script samples position, velocity, and acceleration from `ctx.vehicle`, formats each vector through shared `ctx.format.vector` at three truncated/padded decimals, and logs them as three separate lines.
- The matching runtime ASI is the previously installed `2683636A450D53558576F420AA4F46ED6CAA8B2EC857D407BE76560F3AE0F0B1`; no ASI change was needed for this script-package correction.

## Current state — 2026-10-29 shared script and debug display formatting installed
- Installed `scripts\MadMaxModding.asi`, SHA-256 `2683636A450D53558576F420AA4F46ED6CAA8B2EC857D407BE76560F3AE0F0B1`. Mad Max was confirmed closed before replacement; in-game presentation is pending user verification.
- Previous ASI preserved as `scripts\MadMaxModding.asi.before-shared-formatting-20261029`, SHA-256 `93E9A1D532A98F1E2998A15CC7186D7A94797856C0BE67E93DE1B26BD73BBCEC`.
- Deployed generated `MadMaxModding\API.md`, SHA-256 `E6C0F36D7B00469A85871589D97A452E8F5FD2CBA910315091A6D2481B41F559`; previous API reference preserved as `MadMaxModding\API.md.before-shared-formatting-20261029`, SHA-256 `B57BE29DE71FBFAEF2155A217CADCFA4D7127FC5271B759BABFD81CDB59EC77F`.
- Script callback output no longer repeats the callback or schedule label. `vehicle_watch` emits a three-line block for Vehicle Position, Vehicle Velocity, and Vehicle Acceleration. Debug metrics and scripts share fixed three-decimal formatting through `ctx.format.fixed` and `ctx.format.vector`.
- `cargo fmt --all -- --check`, `cargo build --release`, and `git diff --check` succeeded. No automated tests were run.

## Current state — 2026-10-29 script output uses command row layout
- Installed `scripts\MadMaxModding.asi`, SHA-256 `93E9A1D532A98F1E2998A15CC7186D7A94797856C0BE67E93DE1B26BD73BBCEC`. Mad Max was confirmed closed before replacement; in-game presentation is pending user verification.
- Previous ASI preserved as `scripts\MadMaxModding.asi.before-script-output-row-layout-20261029`, SHA-256 `2F38ED3D72556902541E6410A6615244B48F2510D3B4A4DC6068FADECB2B07C5`.
- Script output now uses the same category-header and indented response-row rendering as command output. Headers read `SCRIPT: <id> @ <schedule>` and each event’s text appears on a separate row underneath.
- `cargo fmt --all -- --check`, `cargo build --release`, and `git diff --check` succeeded. No automated tests were run.

## Current state — 2026-10-29 explicit CMD/SCRIPT console labels installed
- Installed `scripts\MadMaxModding.asi`, SHA-256 `2F38ED3D72556902541E6410A6615244B48F2510D3B4A4DC6068FADECB2B07C5`. Mad Max was confirmed closed before replacement; in-game presentation is pending user verification.
- Previous ASI preserved as `scripts\MadMaxModding.asi.before-explicit-console-script-labels-20261029`, SHA-256 `1391FB15ECEA80CF08A9777F2BBE6FE63F30B46CAFD21693B7ED9CFA3776465A`.
- Console prompt headers now read `CMD: <command>`. Script callback output is rendered on one line with the explicit prefix `SCRIPT: <id> @ <schedule>:` and no extra source header or indentation. Command responses keep their existing command grouping.
- `cargo fmt --all -- --check`, `cargo build --release --locked`, and `git diff --check` succeeded. No automated tests were run.

## Current state — 2026-10-29 script category separator changed to @
- Installed `scripts\MadMaxModding.asi`, SHA-256 `1391FB15ECEA80CF08A9777F2BBE6FE63F30B46CAFD21693B7ED9CFA3776465A`. Mad Max was confirmed closed before replacement; in-game appearance is pending.
- Previous ASI preserved as `scripts\MadMaxModding.asi.before-script-at-sign-separator-20261029`, SHA-256 `543CD8B5BB140166CCC1D3E96E8FE51B111F9030C86E837659E55C5DCF13C8F6`.
- Script category headers now use `script_id @ schedule` (for example, `vehicle_watch @ game.post_sim`).
- `cargo fmt --all -- --check`, `cargo build --release --locked`, and `git diff --check` succeeded. No automated tests were run.

## Current state — 2026-10-29 script output grouped like command transcript
- Installed `scripts\MadMaxModding.asi`, SHA-256 `543CD8B5BB140166CCC1D3E96E8FE51B111F9030C86E837659E55C5DCF13C8F6`. Mad Max was confirmed closed before replacement; visual behavior is pending user verification.
- Previous ASI preserved as `scripts\MadMaxModding.asi.before-script-output-grouping-20261029`, SHA-256 `003167C97A4A2ADCF6ECD8285C489CE632672091E0958CA22F72BC0767ED2C86`.
- Script events now use the same blue, alternating full-width category header as console command rows, labeled with script ID and schedule; their text is indented underneath. Generation remains hidden from this output label.
- `cargo fmt --all -- --check`, `cargo build --release --locked`, and `git diff --check` succeeded. No automated tests were run.

## Current state — 2026-10-29 script output attribution simplified
- Installed `scripts\MadMaxModding.asi`, SHA-256 `003167C97A4A2ADCF6ECD8285C489CE632672091E0958CA22F72BC0767ED2C86`. Mad Max was confirmed closed before replacement; in-game presentation is pending user verification.
- Previous installed build preserved as `scripts\MadMaxModding.asi.before-script-output-no-generation-20261029`, SHA-256 `79374F02ED5C4DFFB282FCE7A1CB2E4CB4C8E904410F4847F6CDF67D1ABEC979`.
- Console callback attribution retains the script ID and schedule but omits the generation number. Generation tracking remains available in script lifecycle/dashboard state.
- `cargo fmt --all -- --check`, `cargo build --release --locked`, and `git diff --check` succeeded. No automated tests were run.

## Current state — 2026-10-29 direct Scripts rescan and command-backed help installed
- Installed `scripts\MadMaxModding.asi`, SHA-256 `79374F02ED5C4DFFB282FCE7A1CB2E4CB4C8E904410F4847F6CDF67D1ABEC979`. Mad Max was confirmed closed before replacement; launch and UI behavior are pending user verification.
- Previous candidate preserved as `scripts\MadMaxModding.asi.before-direct-rescan-help-cleanup-20261029`, SHA-256 `CC32619AFEB2B4650ADDFE6CF63B177644C62F9B2598962AC79FABDE246B5D5F`.
- Scripts first-open scan and Rescan now enqueue a bounded discovery request directly to the loader, without a console prompt or scan output. The real `script list` console command remains available.
- Removed `help api` and help topics for non-commands (`api`, `commands`, and bare `script`). Supported detailed help topics now correspond to actual commands.
- `cargo fmt --all -- --check`, `cargo build --release --locked`, and `git diff --check` succeeded. No automated tests were run.

## Current state — 2026-10-29 API reference and help cleanup installed; startup check pending
- Installed `scripts\MadMaxModding.asi`, SHA-256 `CC32619AFEB2B4650ADDFE6CF63B177644C62F9B2598962AC79FABDE246B5D5F`, built from the current source with Player and Vehicle enabled by default and Camera, Raycast, and Runtime disabled. Mad Max was confirmed closed before replacement. Startup has not yet been checked with this build.
- The prior diagnostic ASI was preserved as `scripts\MadMaxModding.asi.before-help-api-docs-20261029`, SHA-256 `5A9F0A3CCB26FE6044D26E1B064C165761653DACE8F2548379C35466490C58AC`.
- Removed `api list` from the console command parser and help index. Detailed `help <topic>` output now describes behavior without repeating the command syntax. The command index points to `MadMaxModding\API.md`.
- Added `build.rs` and a shared script API catalog. Every Cargo build regenerates repository-root `API.md` from that catalog; the generated file was deployed to `MadMaxModding\API.md` beside the `scripts` directory (SHA-256 `B57BE29DE71FBFAEF2155A217CADCFA4D7127FC5271B759BABFD81CDB59EC77F`).
- `cargo fmt --all -- --check`, `cargo build --release --locked`, and `git diff --check` succeeded. No automated tests were run.

## Diagnostic state — 2026-10-29 debug defaults isolation build installed
- Installed diagnostic ASI SHA-256 `5A9F0A3CCB26FE6044D26E1B064C165761653DACE8F2548379C35466490C58AC` from `C:\Users\David Jackson\Documents\ChatGPT\KSP\.madmax-crash-target\release\madmax_bootstrap_sections-on_diagnostic.dll`. It contains the new command-help code but restores Camera/Raycast visibility to the prior defaults, isolating whether the changed HUD visibility caused the crash.
- Previous rollback ASI preserved as `scripts\MadMaxModding.asi.before-debug-defaults-isolation-20261029`, SHA-256 `BD367D41A2BA6B97C901B0188B1088262561D2142A6C57843EE595766D77A9B8`. The source tree remains configured for the requested Player/Vehicle-only defaults; this installed build is diagnostic only and awaits one launch result.

## Current state — 2026-10-29 previous build restored after startup crash
- Restored `scripts\MadMaxModding.asi` from `scripts\MadMaxModding.asi.before-debug-defaults-command-help-20261029`; verified SHA-256 `BD367D41A2BA6B97C901B0188B1088262561D2142A6C57843EE595766D77A9B8`.
- Preserved the failed build as `scripts\MadMaxModding.asi.failed-debug-defaults-command-help-20261029`, SHA-256 `C0BF31348AF87EF0B4B3A4ABA00860FDC268F27A167685CEA16D9B30BB7C9C61`.
- Windows logged MadMax.exe access violations at 22:08:33 and 22:09:17 (`0xC0000005`, RVAs `0x1FA5A28` and `0x1B34211`). The bootstrap log shows overlay and UpdatePostSim-hook installation for both starts but no stable callback confirmation before termination. The latest source changes affect debug-section defaults and help command parsing/output; they do not modify the native hook path, so causality is not established. A one-launch check with the restored build is needed to distinguish the candidate from an unrelated/intermittent startup failure.

## Current state — 2026-10-29 debug defaults and command help installed
- Installed `scripts\MadMaxModding.asi`, SHA-256 `C0BF31348AF87EF0B4B3A4ABA00860FDC268F27A167685CEA16D9B30BB7C9C61`. Previous installed build preserved as `scripts\MadMaxModding.asi.before-debug-defaults-command-help-20261029`, SHA-256 `BD367D41A2BA6B97C901B0188B1088262561D2142A6C57843EE595766D77A9B8`.
- Debug overlay defaults to Player and Vehicle sections only. The help output is now a command index without an appended misc-notes block; `help <command>` and multiword topics such as `help script load` provide specific syntax and behavior.
- `cargo fmt --all -- --check`, `cargo build --release --locked`, and `git diff --check` passed. Mad Max was confirmed closed before replacement and was not launched afterward.

## Current state — 2026-10-29 concise script lifecycle output installed
- Installed `scripts\MadMaxModding.asi`, SHA-256 `BD367D41A2BA6B97C901B0188B1088262561D2142A6C57843EE595766D77A9B8`. Previous installed build preserved as `scripts\MadMaxModding.asi.before-script-lifecycle-concision-20261029`, SHA-256 `BB9AD145D58FEC990DB5660015E18DBF62502F917A3BC07709EB2B76459EF48A`.
- Script load/reload now reports concise phases (`Preparing`, `Prepared`, `Loaded`/`Reloaded`); unload/run/cancel phases are similarly short. The console omits script/generation/phase attribution for these lifecycle-only entries; detailed errors and callback output retain their context.
- `cargo fmt --all -- --check`, `cargo build --release --locked`, and `git diff --check` passed. Mad Max was confirmed closed before replacement and was not launched afterward.

## Current state — 2026-10-29 console transcript styling installed
- Installed `scripts\MadMaxModding.asi`, SHA-256 `BB9AD145D58FEC990DB5660015E18DBF62502F917A3BC07709EB2B76459EF48A`. Previous installed build preserved as `scripts\MadMaxModding.asi.before-console-transcript-20261029`, SHA-256 `07E7CE2775A6E9722AE3B53FB9F5227A40D7211A22C1CC10AC569E026C389725`.
- Console commands render as alternating full-width prompt rows; output is indented beneath its command and grouped by request ID, including delayed responses. Rejected input displays its prompt too.
- `cargo fmt --all -- --check`, `cargo build --release --locked`, and `git diff --check` passed. Mad Max was confirmed closed before replacement and was not launched afterward.

## Current state — 2026-10-29 console and Scripts cleanup installed
- Installed `scripts\MadMaxModding.asi`, SHA-256 `07E7CE2775A6E9722AE3B53FB9F5227A40D7211A22C1CC10AC569E026C389725`. Previous installed ASI preserved as `scripts\MadMaxModding.asi.before-console-layout-cleanup-20261029`, SHA-256 `6C77892B671323660CCDB987EF1583929178081124AFA1645EC34A6BA2CD9C4B`.
- Removed the global runtime/API and callback badges, package count/location and redundant package labels. Console output wraps long lines by default; **Wrap long lines** toggles horizontal scrolling. Routine lifecycle output omits request-number labels while retaining request IDs internally.
- `cargo fmt --all -- --check`, `cargo build --release --locked`, and `git diff --check` passed. Mad Max was confirmed closed before replacement and was not launched afterward.

## Current state — 2026-10-29 Dev Tools tab viewport layout fix installed
- Installed `scripts\MadMaxModding.asi`, built as `C:\Users\David Jackson\Documents\ChatGPT\KSP\.madmax-crash-target\release\madmax_bootstrap.dll`, SHA-256 `6C77892B671323660CCDB987EF1583929178081124AFA1645EC34A6BA2CD9C4B`. Previous dashboard build preserved as `scripts\MadMaxModding.asi.before-dev-tools-tab-viewport-20261029`, SHA-256 `C5A48C36EC45F30010DDB0A508AB5C86E9F8BD9CE44D2E1F013BF1336EC9B1C3`.
- Gives Scripts its own full-height scroll viewport so overflowing package details cannot scroll the parent window's tab bar out of view. Mad Max was confirmed closed before replacement and was not launched afterward.

## Previous state — 2026-10-29 Dev Tools dashboard implementation
- Installed dashboard build SHA-256 `C5A48C36EC45F30010DDB0A508AB5C86E9F8BD9CE44D2E1F013BF1336EC9B1C3`. Previous bind build preserved as `scripts\MadMaxModding.asi.before-dev-tools-dashboard-20261029`, SHA-256 `6D45D4731DE4AC00A4409759DC99C3BEFB5937817DF63DA872D5DF32AEB452A0`.
- Implements a separate Console/Scripts tabbed Dev Tools window, loader-worker manifest discovery, and explicit package lifecycle actions. Discovery does not compile or execute `main.rhai`; package inventory and runtime status are distinct.

## Previous state — 2026-10-29 source-style bind candidate installed and user-verified
- Candidate: `C:\Users\David Jackson\Documents\ChatGPT\KSP\.madmax-crash-target\release\madmax_bootstrap.dll`, SHA-256 `6D45D4731DE4AC00A4409759DC99C3BEFB5937817DF63DA872D5DF32AEB452A0`.
- The candidate accepts `bind <key> "<console command>"`, stores command strings, reports them in `show_binds`, and dispatches a pressed binding through the same console handler as typed input. It checks only currently bound keys using ImGui press/click events; the all-key input sampler and mailbox publication remain disabled.
- Installed as `scripts\MadMaxModding.asi`; SHA-256 verified after copy. The immediately previous callback diagnostic build is preserved as `scripts\MadMaxModding.asi.before-source-style-binds-20261029` (`396E86A66CD4DA144F07CD032DCBC90D922AC7911B69A347BF475B02146AFF31`). The known-good rollback remains `scripts\MadMaxModding.asi.before-callback-trace-20261029` (`089E72CB0175EE55FC609DEBBC3711A977E0AD7A672769864D47059D83CD874E`).
- `cargo fmt --check`, `cargo build --release --locked`, and `git diff --check` succeeded. No automated tests were added or run.
- The user confirmed normal startup and that the bound `show_binds` command ran after pressing G.

## Current state — 2026-10-29 callback-phase crash diagnostic installed
- Installed `scripts\MadMaxModding.asi`, built as `C:\Users\David Jackson\Documents\ChatGPT\KSP\.madmax-crash-target\release\madmax_bootstrap.dll`, SHA-256 `396E86A66CD4DA144F07CD032DCBC90D922AC7911B69A347BF475B02146AFF31`.
- It records each callback phase once: callback entry, focus synchronization, game-state probe, snapshot copy, Bevy runtime creation, and completion of the post-simulation schedule. This is diagnostic only.
- Previous known-good rollback ASI is preserved as `scripts\MadMaxModding.asi.before-callback-trace-20261029`, SHA-256 `089E72CB0175EE55FC609DEBBC3711A977E0AD7A672769864D47059D83CD874E`.
- The preceding candidate with render-side sampling and binding dispatch disabled still crashed. Windows recorded the same `MadMax.exe` abort (`0x40000015` in `MSVCR100.dll`, offset `0x761c9`); the bootstrap log reached “waiting for 60 stable UpdatePostSim callbacks,” so ASI loading and hook installation succeeded. This moves investigation to the callback/runtime path; it does not identify a specific failing phase.
- **2026-10-29 diagnostic launch result:** the user confirmed this callback-phase build reached normal launch. The log contains all six phases through “Bevy post-simulation schedule returned.” The callback/runtime path therefore completed in this run. Input sampling and bind dispatch were disabled in this build, so this does not verify them; their render-side branch remains the leading crash suspect, not a proven cause.
# Bootstrap install record

## Current state — 2026-10-29 F2 capture and vehicle-jump build installed
- Installed reviewed release build at `scripts\MadMaxModding.asi`, SHA-256 `
9BD5CB3F74D8D81CB569081FC7994D3198AE3966D917C7713176FBA7CFB5EE05
`. Previous ASI backed up as `scripts\MadMaxModding.asi.before-scripted-vehicle-jump-20261029`, SHA-256 `
1FAA727ACB9687C4FABDF611B70E8C770BD02848B64F355B4A107ACF4AC42170
`. Mad Max was confirmed closed before replacement; no launch or gameplay test was performed.
- Copied `vehicle_jump` package to `MadMaxModding\scripts\vehicle_jump` (manifest SHA-256 `
E1556D380E7C1AF9026B6C21F405DFB11EA551E0BAA9BCA7097B64954DC6B4A2
`, main.rhai SHA-256 `
FC79A85C0C258C32A420BA8BCB6B64FFBE81953F570252D5F872484C0B76D171
`). Existing `vehicle_watch` package was retained.
- F1 toggles console, F2 toggles input/cursor capture, F3 toggles debug-section picker. In-game focus, native hover suppression, and the vehicle jump have not been verified.



## Current state — 2026-10-29 managed scripting console installed
- Installed reviewed release build at `scripts\MadMaxModding.asi`, 10,626,560 bytes, SHA-256 `1FAA727ACB9687C4FABDF611B70E8C770BD02848B64F355B4A107ACF4AC42170`. Previous installed ASI preserved at `scripts\MadMaxModding.asi.before-managed-scripting-console-20261029`, SHA-256 `DD35F2090DEF106505C99A7EB00FAA58B078BCD84F285BCC60EFA958028ABCE6`. Mad Max was confirmed closed before replacement and remains closed; no launch/test was performed.
- Copied example package to `MadMaxModding\scripts\vehicle_watch` (manifest SHA-256 `
0EB3202E4A4CC861EB54B4C5A4D7943CE524CB45DA83BBB2EDA9C8516291585E
`, main.rhai SHA-256 `
11254CAEC6BCCD7A6EC11FF789A0BF6B63A3AD62A799BD576D64D266833A69F5
`). Use `Ctrl+Shift+F7`, then `script load vehicle_watch` to begin manual verification.


## Current state- **2026-10-29 bounded ray-filter information candidate installed; gameplay validation pending:** `scripts\MadMaxModding.asi`, 6,300,160 bytes, SHA-256 `DD35F2090DEF106505C99A7EB00FAA58B078BCD84F285BCC60EFA958028ABCE6`. Adds a signature-gated, call-through observer at `CRaycastFilter::AddRayHitToCRaycastFilter`, scoped to sampled reticle broadphase candidates. It records the filter vtable, exclusion/required masks, candidate filter-info and owner, optional ignored owner, and selected candidate pointer before/after. Current installed ASI was preserved as `scripts\MadMaxModding.asi.before-ray-filter-info-tap-20261029`, SHA-256 `C3875F5E3E243C41FCAE571427DC991DCC53C099EB7F2D66901AFE649B4F1EFC`. The game was confirmed closed before replacement; this candidate has not yet been launched.
- **2026-10-29 player-aim vs. static-only ray comparison installed; gameplay validation pending:** `scripts\MadMaxModding.asi`, SHA-256 `67F5ABE11F259BBF8E0D3853C8C04ACE4146F6712B4C39426F49A369811FA006`. It submits the same camera ray through the existing static-only filter and the RTTI-confirmed CPlayerAimRaycastFilter, reporting both fractions and positions. The previous installed ASI is preserved as `scripts\MadMaxModding.asi.before-player-aim-filter-ab-20261029`, SHA-256 `11370014FE0C12377A8597EC4747F5C57F944BF4D758AA61B8D46D0CB123B7E1`. Mad Max was confirmed closed during replacement and was not launched.
- **2026-10-28 camera-query capture with reserved sample region installed; gameplay validation pending:** `scripts\MadMaxModding.asi`, 5,101,568 bytes, SHA-256 `4E1103DE0C8E445B63744A35C8C9C96E638A36BC29CD07022CE4E3C9BEC88DFB`. It reserves sample indices `0..159` for up to 160 `GraphScriptRaycastStatic` captures and indices `160..575` for all other callers. Mad Max was confirmed closed during replacement and was not relaunched. The previous shared-pool camera build is backed up as `scripts\MadMaxModding.asi.before-camera-capture-reserved-buffer-20261028`, SHA-256 `AEA5AA3C255EDCB69D5611DF345B9585BE99619884AA96643E8F07365F4FBD73`.
- **2026-10-28 camera-query detail capture build installed; gameplay validation pending:** `scripts\MadMaxModding.asi`, 5,101,568 bytes, SHA-256 `AEA5AA3C255EDCB69D5611DF345B9585BE99619884AA96643E8F07365F4FBD73`. It raises only the `GraphScriptRaycastStatic` detailed sample limit from 16 to 160 and static world-query sample capacity from 432 to 576; no query behavior or hook target changed. Previous installed build preserved as `scripts\MadMaxModding.asi.before-camera-capture-20261028`, SHA-256 `1E4CAE40E1126C91560430BBD509448249990AAEF160877FB5AA1E9F5E395401`. Mad Max was confirmed closed during replacement and was not relaunched.
- **2026-10-28 sampled-height-field runtime snapshot candidate installed:** `scripts\MadMaxModding.asi`, 4,285,952 bytes, SHA-256 `1E4CAE40E1126C91560430BBD509448249990AAEF160877FB5AA1E9F5E395401`. It performs read-only executable-page checks and logs 96 bytes each from the RTTI-mapped `CCustomSampledHeightFieldShape` vtable targets at RVAs `0x024F6880` (`+0x30`) and `0x02B1B0A0` (`+0x80`); it adds no hooks or game writes. The user launched it and reached the main menu. The startup byte logger ran before the `.xpdata` functions contained their live bodies, so its bytes are not valid code evidence. Read-only process-memory snapshots taken later at the main menu recovered valid function bodies (documented below). Previous installed tenfold ammo capture build preserved at `scripts\MadMaxModding.asi.before-shape-virtual-snapshot-20261028`, 4,281,344 bytes, SHA-256 `257828A8D424B01F5FBB701E765172FA15F7A7EFA7480B9DBED9BF776B6B51E7`.

- Mad Max Steam install: `C:\Program Files (x86)\Steam\steamapps\common\Mad Max`
- Loader: Ultimate ASI Loader v9.7.4, x64 `dinput8.dll`
- Source archive SHA-256: `8272D83B2692662098746F2D0AD0E2D85F3C8358AB1D63F75FBE835C2C8135FD`
- Extracted loader SHA-256: `FA266E3513D02C08A1B808F28C10538A489EAFFAA4B0707F7CC1066E71B5AFD7`
- Installed plugin: Rust DX11 overlay and Bevy runtime at `scripts\MadMaxModding.asi` (1,752,064 bytes), SHA-256 `9814AA43AAC199DA829C1320E2FD606F1799DEAA319E8D3A9ADB8903B4D73B2B`. This revision launched successfully in the current run, installed the world-query hook, and captured eight shotgun-path calls after the user fired four shells. No crash was reported. The source has since been extended with caller stack-argument and output-record capture; that newer build is compiled locally but has not been installed.
- The immediately previous capture build is backed up as `scripts\MadMaxModding.asi.before-corrected-stack-filter-20261028` (1,751,040 bytes; SHA-256 `678CB3131DFF91F39E20A59B0E07F9FDC61759E6406ABCB7635F66C6B060829E`). It launched and ran through four user-fired shotgun volleys. The hook logged mixed hit and miss results and modified query-data words. Its extra indirect-record fields said `unreadable` because the stack-address filter incorrectly assumed 64-bit-high addresses; direct read-only process peeks confirmed the observed stack records are at low addresses around `0x7FFFxxxx` and readable.
- The previous working plugin was backed up as `scripts\MadMaxModding.asi.before-indirect-query-snapshot-20261028` (1,747,968 bytes; SHA-256 `1D77238435E87A7D2A8B89455FFB97FB9404C49AAE1B03E60676BC7AF577F68D`). Its stack-preserving return-thunk observer was exercised in-game: the user confirmed startup and firing shotgun shells; the log captured eight ammo-query returns, including both `result=1` and `result=0`, without a reported crash. On hit samples, the last query word changes from `1.0` to a pair of floats; it remains `1.0` on the miss. Later caller-frame analysis showed this qword aliases output bytes `+0x08..+0x0F`, rather than an independent query-data field. The pair's geometric meaning is still unknown.
- Known-good pre-post-call plugin backed up as `scripts\MadMaxModding.asi.before-stack-preserving-return-hook-20261028` (1,754,624 bytes; SHA-256 `95CDD7CAFF5840D3592E657A078149B2AFE4ECD9D9E4DD56221856F7128B0C47`).
- Previous stack-preserving candidate backed up as `scripts\MadMaxModding.asi.before-exact-sample-pairing-20261028` (1,747,456 bytes; SHA-256 `5FED47277257C3038EEC1C6690A61291D3ABF2BAAF037BA3E095B507EBDD36D8`).
- Previous plugin backed up as `scripts\MadMaxModding.asi.before-world-query-post-call-20261028` (1,754,624 bytes) before installing post-call capture.
- Previous plugin backed up as `scripts\MadMaxModding.asi.before-input-focus-test-20261027` before installing the focus trial.
- Previous plugin backed up as `scripts\MadMaxModding.asi.before-weapon-ray-observer-20261028` before installing the weapon ray observer.
- Previous plugin backed up as `scripts\MadMaxModding.asi.before-world-query-register-capture-20261028` before installing world-query register capture.
- Previous plugin backed up as `scripts\MadMaxModding.asi.before-world-query-buffer-snapshot-20261028` before installing query-buffer snapshots.
- Previous installed plugin backed up as `scripts\MadMaxModding.asi.before-spawn-experiment-20261027` (1,647,104 bytes).
- The game was closed during setup. The game folder had no `dinput8.dll` proxy to preserve.

## Rollback

Close Mad Max, then restore `scripts\MadMaxModding.asi.before-corrected-stack-filter-20261028` over `scripts\MadMaxModding.asi` to return to the tested capture build. To remove post-call query capture, restore `scripts\MadMaxModding.asi.before-indirect-query-snapshot-20261028` (the tested 1,747,968-byte stack-preserving build). The prior nested-call revision (SHA-256 `064A43A7B2844C569D1ABCB1722DEA23E533C1C8A208BB5BA4BBB05D7F49ECE`) produced repeated access violations at MadMax.exe RVA `0x252F00C` and is not installed. Restore `scripts\MadMaxModding.asi.before-world-query-register-capture-20261028` to undo world-query register capture. Restore `scripts\MadMaxModding.asi.before-weapon-ray-observer-20261028` to undo the weapon ray observer. Restore `scripts\MadMaxModding.asi.before-input-focus-test-20261027` to undo the focus trial while retaining the working spawn build. Restore `scripts\MadMaxModding.asi.before-spawn-experiment-20261027` to return to the pre-spawn plugin. The original install did not contain a `dinput8.dll` proxy; the original `bink2w64.dll` was not changed.

## Safety boundary

The installed revision includes an experimental in-process spawn call using SDK-derived constructor/destructor/function addresses and descriptor-field offsets, plus a direct write to the SDK-declared input focus field. The user has confirmed the test buggy appears in game; the focus-field behavior still needs an in-game trial. The field address and layout are derived from the pinned SDK and guarded by page checks, but are not independently runtime-verified. Keep all backups available for rollback. The weapon observer checks the function entry signature and samples at most 64 calls. The installed world-query observer records caller registers and a bounded 96-byte pre-call and post-call buffer for up to eight calls from the shotgun and `HeightOverGround` sites. It tail-jumps to the helper with the original stack intact; for selected samples it temporarily replaces the return address with a Rust return thunk that restores the helper result and resumes the original caller. The thunk uses a thread-local LIFO stack and sample indices to pair call results. The corrected stack-filter build was exercised in the current game run without a reported crash. The current local candidate additionally copies Win64 stack slots 5–11, snapshots a bounded printable string from `RDX`, uses 128 fixed sample slots for the first 32 ammo calls, first 8 `HeightOverGround` calls, first 16 `GraphScriptRaycastStatic` calls, and one first-use sample for each other observed site (at most 108 samples under the current 55-site map), and captures the helper's 0x50-byte output record and 0xA8-byte query context for every sampled caller. Callsite disassembly verifies profiles for ammo, `HeightOverGround`, Force Pulse, bullet-ray, `CWeapon::GetRayWorldPosition`, `CRayCastCondition::Evaluate`, `GraphScriptRaycastStatic`, and `CRayCastFanCondition::Evaluate`; the `CRayCastFanCondition::Evaluate` callsite at return RVA `0x1C284DA` appeared only in the unique-address log without a detail sample; its second callsite and the Force Pulse, bullet-ray, weapon, `CRayCastCondition`, and `GraphScriptRaycastStatic` callsites were absent from the current caller list. The helper reads slot 9 as a byte selector, slot 10 as an index input, and dereferences slot 11 as an optional u32 override; the helper has no direct read of slot 8 in its body. Weapon and ammo callers use output `+0x14` as a segment fraction in hit-position calculations. For ammo queries the candidate logs the caller-proven target pointer, target byte at `+0x10`, and derived hit position, or the segment endpoint on a false result. It decodes the first six query floats and segment length for `GraphScriptRaycastStatic`, keeps `HeightOverGround` slot 5 raw, captures the frame offset at `RCX+0x40` so both zero-radius bounds-check points can be reconstructed from each sampled call, and snapshots the argument-11 u32 pointee at call time when readable, reporting null and unreadable separately. It has not been installed or runtime-tested. Current uninstalled candidate SHA-256: `D67A54D7C9EE0CA1B1E317DAE37C8B66EEC7CE4E472FBDA1EB464A6970B468A5` (1,784,320 bytes). The earlier nested-call wrapper (SHA-256 `064A43A7B2844C569D1ABCB1722DEA23E533C1C8A208BB5BA4BBB05D7F49ECE`) caused repeated access violations at RVA `0x252F00C` and was removed. `hudhook` is third-party MIT-licensed code with unsafe FFI internals and an upstream warning that it may crash host applications.

## Latest world-query capture candidate (2026-10-28)

- Local artifact: `target\release\madmax_bootstrap.dll`, 1,791,488 bytes, SHA-256 `823E4B644BCFE74C239F5F31ACE65660CDE74CC38981A15C84ACA6194E3FE810`.
- This candidate adds 32-byte raw snapshots of each original `GraphScriptRaycastStatic` argument for at most 16 samples, while keeping the existing fixed sample caps and pre/post query output/context capture.
- Ammo diagnostics separate the helper Boolean return from the output `+0x14` scalar, the caller-computed position/endpoint, the dispatch pointer at output `+0x28`, and its byte `+0x10`. They do not claim that true/false alone proves a physical hit/miss.
- `cargo fmt --check`, `git diff --check`, and `cargo build --release` passed. The candidate is not installed. Mad Max PID `20880` remains responsive; installed plugin SHA-256 is `9814AA43AAC199DA829C1320E2FD606F1799DEAA319E8D3A9ADB8903B4D73B2B`; the log is 19,086 bytes, last written 2026-10-28 02:26:57 +01.

## Current bounded world-query candidate (2026-10-28 08:29 +01)

- Artifact: `C:\Users\David Jackson\.codex\visualizations\2026\10\27\01a17d99-9f8c-70c1-a157-16ff162f15cf\mm-build-target\release\madmax_bootstrap.dll`, 2,484,736 bytes, SHA-256 `561CF45849E8C7A211B01AEFF45EB5FF6AC6341B91EDEF7DBC1A1A16D7F7CE53`.
- The capture retains 144 fixed query samples: prioritized ammo, height, script, named repeat callers, sparse samples for `wind_occlusion` and `vehicle_sound_occlusion` at caller ordinals 0/256/2048, then first-use samples for other observed sites. It records per-caller counts in five-second background summaries, capped to the top twelve active callers per interval.
- For each selected query it snapshots the 0x50-byte output record and 0xA8-byte raw context before/after, raw Win64 arguments/registers, bounded candidate callback records at ordinals 0/1/2/4/8/16/32, and the minimum finite in-range callback fraction. The point-in-bounds observer records up to two endpoint/radius/result tuples only while a sampled query is active on the same thread.
- `cargo fmt --check` and `cargo build --release --locked` passed on 2026-10-28; `git diff --check` passed, with existing line-ending warnings. No tests were run. A later rebuild reproduced the same SHA-256. The candidate remains uninstalled and has no in-game validation; callback/output correlation, hook stability, and performance are unknown. The bounds-predicate detour forwards all calls, including calls outside a sampled query; only data reads/recording are gated by the active sampled-query association.
- Static review found a signed-zero edge in the aggregate minimum: `-0.0` passes the `[0,1]` test but its raw bits do not order numerically under unsigned `fetch_min`. The detailed per-candidate raw fraction is unaffected. Normalize zero for aggregation or use a float-aware atomic update before any install; the running samples do not establish whether this edge occurs.
- Static review also found that only sampled helper calls push return frames. A nested unsampled helper call could leave the sampled outer frame visible to same-thread bounds/candidate taps and contaminate its counts if the callback path re-enters the helper. No such re-entry is confirmed. Mask unsampled calls with no-sample TLS frames (and route their returns through the pop thunk), or establish that re-entry cannot occur, before using candidate callback counts as complete.
- Mad Max PID `20880` remains open on the installed ASI SHA-256 `9814AA43AAC199DA829C1320E2FD606F1799DEAA319E8D3A9ADB8903B4D73B2B`. The current-run bootstrap log remains 19,086 bytes, last written at `02:26:57 +01`. The full Rust diff still needs the user's line-by-line review before any install.

## Caller-activity capture candidate refresh (2026-10-28 05:19 +01)

- Local artifact: `target\release\madmax_bootstrap.dll`, 1,808,896 bytes, SHA-256 `ECC1EC3599214C9B8C088B0A2949F1375188F639D256BD16C3EF844B63FC8830`.
- Adds one relaxed atomic increment per registered caller and a background five-second activity summary. It logs only active windows and includes the top twelve caller RVAs by call count. Detailed snapshots and their limits are unchanged.
- `cargo fmt --check`, `git diff --check`, and `cargo build --release` passed. This build is not installed or runtime-verified. Mad Max remains PID `20880` with the prior plugin loaded; the log remains 19,086 bytes with last write `2026-10-28 02:26:57 +01`.

- Candidate-only isolation build installed 2026-10-28 13:56 +01: candidate SHA-256 `37D96B62F89738C17753B409112CEAB892825CD202668C488E9A135718EA155A` (2,650,112 bytes). It leaves the existing helper observer enabled and installs only the candidate callback tap; bounds and group-filter taps are disabled. The prior known-good installed ASI was backed up as `scripts\MadMaxModding.asi.before-candidate-only-20261028` (SHA-256 `9814AA43AAC199DA829C1320E2FD606F1799DEAA319E8D3A9ADB8903B4D73B2B`). Runtime test pending. Restore that backup over `scripts\MadMaxModding.asi` if the candidate prevents startup.
- Manual validation: user launched the candidate build and fired several shotgun shells at the ground. `bootstrap.log` recorded the candidate-only startup marker and 32 sampled `CAmmunition::CheckCollisions` queries between 14:00:56 and 14:01:12 +01. Each sampled helper return was `1`, with 10–13 candidate callbacks; the minimum candidate fraction exactly matched the ammunition output fraction. This is promising callback/output correlation, not yet a decoded hit-result contract. No in-game UI tap was added; all diagnostics are file-only.
- Extended result analysis: among 61 samples with candidate callback fields, 48/48 helper returns of `1` had minimum fraction `<1.0`; 13/13 returns of `0` had minimum `1.0`. All 32 shotgun `CAmmunition::CheckCollisions` samples were true, and each candidate minimum exactly matched the ammo caller’s consumed output scalar. The callback returns the per-ray record’s `+0x08` by static disassembly, strongly tying it to the ammo segment fraction. This is bounded single-session evidence; physical-object and universal helper semantics remain open. A controlled clear-sky shotgun miss is still needed for the ammo caller specifically.
- Follow-up sky test: the user fired three shells into clear sky in the same process. Aggregate activity summaries recorded ammo-caller batches of 10 and 20 calls around 14:10:35–14:10:40 +01, confirming the call path ran. The detailed first-32-ammo-call quota had already been consumed by the earlier ground test, so no sky return/fraction was captured. A fresh game process resets the counter; reuse the installed candidate and fire the clear-sky control first.
- Controlled clear-sky validation after a fresh process: the user fired one shotgun shell upward before any ground shots. The log captured ten `CAmmunition::CheckCollisions` rays (15 m segments, Y direction about +0.76 to +0.85). All ten helper returns were `0`, each had nine candidate callbacks and a minimum fraction of `1.0`, and each selected the full-segment endpoint path. This contrasts with 32/32 previous ground-directed ammo samples returning `1` with minimum/output fraction below 1. Static caller and callback disassembly plus this controlled test now validate the hit-like fraction/no-result sentinel behavior for the ammo query path in the supported Steam build. Do not generalize it to other callers or to camera placement raycasts yet.

- **2026-10-28 reticle-ray query prototype installed; gameplay validation pending:** `scripts\MadMaxModding.asi` SHA-256 `22C76380068A0B6998DC17874974441EDCA6079C1B514A0342A0FE6BDA61BF76` (5,118,464 bytes). Samples the camera-derived ray through `WorldQueryHelper` every 250 ms using a temporary 16-byte-aligned player-aim filter context; the debug Camera section now reports query availability/state, fraction, and an optional candidate position, while Runtime is trimmed to frame delta and recent operation outcomes. Prior installed ASI preserved as `scripts\MadMaxModding.asi.before-reticle-ray-20261028` (SHA-256 `BBC5A40E385C68E53009CCE75B4C7BA1E6E241CF88D2146D1DD3E229F03B264A`). Mad Max was closed during replacement and was not launched; native query behavior is unverified.

- **2026-10-28 camera-forward sign candidate installed**: Negates the SDK CMatrix4f::Forward() row when constructing the experimental reticle ray; camera origin and query setup are unchanged. This is a gameplay hypothesis from the user's observed direction (0.11, 0.66, 0.74) while aiming at the ground, not yet a confirmed matrix mapping. Candidate SHA-256: $newHash (5,118,464 bytes). Previous installed ASI backed up to scripts\MadMaxModding.asi.before-forward-sign-test-20261028 (SHA-256: $oldHash). Mad Max was confirmed closed before replacement; launch and ground/sky validation pending.

- **2026-10-28 reticle filter-context correction installed:** Candidate pairs the CStaticOnlyRaycastFilter vtable with category mask 3 at context +0x90 and retains the negated camera Forward() ray direction. SHA-256: $newHash (5,118,464 bytes). The previous sign-test ASI is backed up as scripts\MadMaxModding.asi.before-filter-context-fix-20261028 (SHA-256: $oldHash). Mad Max was confirmed closed before replacement; ground/sky runtime validation pending.

## Current state — 2026-10-29 console/UI comfort build installed
- Installed `target\release\madmax_bootstrap.dll` as `scripts\MadMaxModding.asi`, 10,742,272 bytes, SHA-256 `43B2F7F3E0FEB42ECED3434C48F48908520BD3545F77E740FE323D92F0E6510D`. Previous install preserved as `scripts\MadMaxModding.asi.before-console-ui-comfort-20261029`, SHA-256 `9BD5CB3F74D8D81CB569081FC7994D3198AE3966D917C7713176FBA7CFB5EE05`. Mad Max was closed during replacement; this build has not yet been launched or behavior-tested.
- F1 remains hardcoded for input/cursor capture, F2 for the UI configurator, and F3 for the console. The console/UI comfort changes include prompt clearing after failed submissions, local `clear`, bounded ANSI SGR output, fixed three-decimal float formatting, removal of command-only UI buttons, and configurable panel opacity. Mod-owned key bindings now use the Rust runtime's ImGui input stream; they do not suppress selected Mad Max actions. Global UI capture still blocks all game input while active.

- **2026-10-29 Escape key-up capture candidate (built, not installed):** `target\release\madmax_bootstrap.dll`, SHA-256 `089E72CB0175EE55FC609DEBBC3711A977E0AD7A672769864D47059D83CD874E` (10,742,272 bytes). Escape closes the overlay but keeps the global capture override active until ImGui observes Escape released; Mad Max's input-focus flag is restored on a later game callback. `cargo fmt --check`, `cargo build --release --locked`, and `git diff --check` passed. In-game validation pending. Install only after Mad Max exits; the currently installed console/UI comfort build remains untouched.

- **2026-10-29 mod-owned input build rolled back after startup crashes:** the candidate (`50C1628F71F8040BE4664D6A1FC557594BA0E40D99A6700983D354C655980A1C`) is preserved as `scripts\MadMaxModding.asi.failed-runtime-input-20261029`. Windows recorded two `MadMax.exe` APPCRASH events at 19:37:37 and 19:40:05, both `0x40000015` in `MSVCR100.dll` at offset `0x761c9`. The bootstrap log shows the render hook and UpdatePostSim probe installed before each failure, but does not identify the failing callback. The prior Escape key-up build (`089E72CB0175EE55FC609DEBBC3711A977E0AD7A672769864D47059D83CD874E`) was restored as `scripts\MadMaxModding.asi`; the user confirmed it launched to the menu. Static review identified one unsafe API mismatch candidate: mouse-button IDs were queried through ImGui's keyboard query function in the new sampler. An isolated build now routes mouse buttons through `is_mouse_down` while leaving keyboard sampling unchanged. This is a test hypothesis, not a proven root cause.
- **2026-10-29 mouse-input query isolation result:** the candidate (`5CC4982CC4EAFC214238A8A7A3BD1D507A5EE601B8D8AA903AB0A5F8DD53328A`) crashed. Windows logged another APPCRASH at 20:21:39, `0x40000015` in `MSVCR100.dll` offset `0x761c9`. It is preserved as `scripts\MadMaxModding.asi.failed-mouse-input-poll-20261029`; the known-good `089E72...` build was restored before preparing the next isolation candidate.
- **2026-10-29 input-disabled diagnostic candidate:** built at `C:\Users\David Jackson\Documents\ChatGPT\KSP\.madmax-crash-target\release\madmax_bootstrap.dll`, SHA-256 `D834A49734C06BEBD9EECC4DECDA3861FDC813984955B3F5B0B60F177C206BCC`. `cargo fmt --check`, `cargo build --release --locked`, and `git diff --check` passed. Render-side key polling and dispatch are compile-time disabled; candidate is installed pending one-launch validation.
## Previous installed state — 2026-10-30 script-driven CDebugRenderer gizmo
- Release test suite passes: 24 passed, 0 failed. The example `debug_gizmo` manifest and Rhai source compile through the package loader test.
- Installed `MadMaxModding.asi`, SHA-256 `AED42848CD3A55BE73F09D8A26E69CDE92C03EAA2BCED785DEA24C8E44A1EBEE`; previous ASI preserved as `MadMaxModding.asi.before-debug-gizmo-20261030`, SHA-256 `70700B78F946C22374B70447C22BD06A0A3F07C43AAA4525EE3C856FF54F4F0E`. It replaces the one-off console test with a bounded, experimental `debug.draw` capability (`ctx.gizmos.axes/line/arrow`) and logs the first accepted batch once to `MadMaxModding\bootstrap.log`.
- Deployed generated `MadMaxModding\API.md`, SHA-256 `019CD133A081ADCC0C2CB4C6F4E49A61F2E2A40235F3F5ACB8A31D4C0A7E99E3`; previous API preserved as `API.md.before-debug-gizmo-20261030`. Deployed `MadMaxModding\scripts\debug_gizmo` from `examples\scripts\debug_gizmo`; it draws 40-unit RGB axes and long colored lines plus a vertical arrow centered two units above the copied player position on every `game.post_sim` callback.
- Mad Max was closed before installation and remains closed. The 24-test release suite, formatting check, and diff check passed; in-game startup and gizmo visibility are pending user verification.
## Installed — 2026-10-30 Rhai gizmo dispatch and console clipboard
- Installed `MadMaxModding.asi`, SHA-256 `E6D4A97ADB2093DDE0D9AE337F98D5570389230DBD7DFDF54EE8DEA54C917FEB`; previous ASI preserved as `MadMaxModding.asi.before-rhai-clipboard-20261030`, SHA-256 `AED42848CD3A55BE73F09D8A26E69CDE92C03EAA2BCED785DEA24C8E44A1EBEE`.
- Fixed Rhai dispatch by registering `axes`, `line`, and `arrow` using explicit mutable-receiver closures and a concrete float signature. A runtime regression now invokes the actual `ctx.gizmos.axes(ctx.player.position, 40.0)` getter/method path, plus line and arrow.
- Console transcript headers and output rows are click-selectable; Ctrl+C copies the selected whole row with ANSI sequences stripped. ImGui's command prompt now uses a Win32 Unicode clipboard backend for standard Ctrl+C/X/V editing.
- Release tests: 26 passed, 0 failed. `cargo fmt --all -- --check`, `cargo check --release --locked`, `cargo build --release --locked`, and `git diff --check` passed. In-game startup, clipboard interaction, and visible gizmo behavior await the user's next launch.

## Installed — 2026-10-30 expanded CDebugRenderer script API
- Installed `target\release\madmax_bootstrap.dll` as the ASI entry point `scripts\MadMaxModding.asi`, SHA-256 `BC98CB20AA25689A857832365D8AB3EC57B67417DEC666B126DBBDA6F80A71CE` (11,097,600 bytes). The prior binary is preserved as `scripts\MadMaxModding.asi.before-expanded-gizmos-20261030`.
- Updated `MadMaxModding\scripts\debug_gizmo\main.rhai` with the mapped sphere, AABB, transformed AABB, and bounds-shape calls in addition to the existing axes, lines, and arrow. Its backup is at `MadMaxModding\backups\debug_gizmo-before-expanded-gizmos-20261030`.
- Deployed generated `MadMaxModding\API.md`, SHA-256 `6DFA9099A6A88CB60A3303C09661E7BC0C5D77E8FEF5A2D4802E9E8682B4BA0A`; previous reference preserved as `MadMaxModding\API.md.before-expanded-gizmos-20261030`.
- Added a geometry-work budget to bound per-callback submissions, especially the tessellated sphere. Bounds-shape slots 36–39 are marked experimental because their 64-bit transform argument has not been confirmed at a game callsite.
- `cargo fmt --all -- --check`, `cargo build --release --locked`, and `git diff --check` passed. No automated tests were run. Mad Max was confirmed closed before install and was not launched; in-game visibility of the new shapes is pending user verification.
- **2026-10-30 gizmo layout lowered:** backed up the deployed example as `MadMaxModding\backups\debug_gizmo-main.before-lowered-layout-20261030.rhai`; replaced only `MadMaxModding\scripts\debug_gizmo\main.rhai` with the source example. Source and installed SHA-256 both `A16683914CCBDA14D65ADE9525BA6986648E6C5556EB85906C7E6529B7B49521`. Marker center moved from 2 to 1.5 units above Max; test rows moved from 5/7–12-unit offsets to 2/4–7-unit offsets, and their horizontal spread/radii were reduced. `git diff --check` passed. Reload the package in-game to view the updated positions.

## 2026-10-30 — transform-forward gizmo offset

Installed the release build and updated `debug_gizmo` package while Mad Max was closed. The gizmo test cluster now uses the available vehicle transform position and horizontal forward vector, falling back to Max's transform, and places its base 20 world units forward before applying the cluster's vertical offsets. Added `ctx.player.forward` and `ctx.vehicle.forward` to the scripting API and regenerated the deployed API reference.

- Installed ASI: `C:\Program Files (x86)\Steam\steamapps\common\Mad Max\scripts\MadMaxModding.asi`
- SHA-256: `8375C3BEE89FB731A537DFC4F2751873477915CC3FDD526E8781A47FB088AA70`
- ASI backup: `C:\Program Files (x86)\Steam\steamapps\common\Mad Max\scripts\MadMaxModding.asi.before-forward-offset-20261030`
- Installed gizmo script SHA-256: `5FF1E6D66C065C966E397F3826E6068B932D34F99CB193476268DD4BC3DD1F4B`
- Script backup: `C:\Program Files (x86)\Steam\steamapps\common\Mad Max\MadMaxModding\backups\debug_gizmo-main.before-forward-offset-20261030.rhai`
- Installed API.md SHA-256: `75F6913F0634FD59F10F148F88610BFFC04812CF70D69A5870B4FCB26A6EB227`
- API backup: `C:\Program Files (x86)\Steam\steamapps\common\Mad Max\MadMaxModding\API.md.before-forward-offset-20261030`
- Verification: release build succeeded; format check and `git diff --check` passed; deployed files matched source hashes. Mad Max was not launched, so in-game direction is unverified.

## 2026-10-30 — raw transform orientation diagnostic

Deployed a separate `debug_orientation` Rhai package and release ASI while Mad Max was closed. The diagnostic draws the copied player and vehicle transforms directly with the mapped native transformed-axes primitive: red is matrix row 0, green row 1, and blue row 2. This avoids deriving a forward vector or modifying `debug_gizmo`, so in-game observation can distinguish a matrix/layout interpretation error from a bad transform source.

- Installed ASI SHA-256: `C1D599105CBD02075E72FEA6716B29402902FD01C5D32D4D678C9DE0886CFB47` (source artifact: `target\\release\\madmax_bootstrap.dll`). Prior ASI backup: `scripts\\MadMaxModding.asi.backup-20261030-132754`.
- Installed package: `MadMaxModding\\scripts\\debug_orientation`; manifest and script match `examples\\scripts\\debug_orientation`.
- Installed `API.md`; prior copy: `MadMaxModding\\API.md.backup-20261030-132754`.
- Verification before install: `cargo test --locked` passed (29 passed); release build succeeded; `cargo fmt --all -- --check` and `git diff --check` passed. In-game axis orientation remains to be tested.

Updated `debug_orientation/main.rhai` after the first visual comparison to draw both the current positive row-1 forward interpretation (green arrow) and its negation (magenta arrow) for player and vehicle. Raw RGB axes remain visible. The Rhai package compilation test passed; the previous script is preserved as `MadMaxModding\\scripts\\debug_orientation\\main.rhai.before-forward-sign-test-20261030`. Script source and installed hashes match (`1DBF9B69C9EFFACCF083DB2F0EBA01B83A643F46832A0FD5A44B3DF93ADE24CB`). The new sign comparison awaits in-game observation.

## 2026-10-30 — actor and vehicle forward basis correction

The user visually confirmed that actor transforms render row 0 red/right, row 1 green/up, and row 2 blue/backward. The previous helper projected row 1 onto XZ; near-zero components were normalized and caused wandering arrows. `horizontal_forward` now takes the negated XZ projection of row 2 and rejects non-finite or degenerate magnitudes. Camera-ray direction logic is unchanged. Updated the generated API reference to describe this actor-specific basis mapping.

- Installed ASI SHA-256: `8C77C5A8C9634101CCE52AB2272C1253C44E73AF15F6F826C91775FF751B3953`.
- Prior ASI backup: `scripts\\MadMaxModding.asi.before-actor-forward-fix-20261030-134325`.
- Installed API.md SHA-256: `FE0064CEA2D2E3464C1D722FEED99149929AD6948CFCEE02EE07786508F774CA`.
- Prior API backup: `MadMaxModding\\API.md.before-actor-forward-fix-20261030-134325`.
- `debug_orientation` script hash: `1DBF9B69C9EFFACCF083DB2F0EBA01B83A643F46832A0FD5A44B3DF93ADE24CB`.
- Verification: all 30 Rust tests passed; release build, `cargo fmt --all -- --check`, and `git diff --check` passed. The corrected forward direction awaits in-game confirmation.
## 2026-10-31 — single reticle-ray script diagnostic

Reduced the mod-issued reticle sampling pass to one query using the selected `CStaticOnlyRaycastFilter` configuration. The previous player-aim, ignore-actors, and ignore-player query variants are no longer issued by this probe and were removed from the visible Raycast panel to keep the test focused. Added read-only `ctx.raycast` fields (`available`, `state_present`, `origin`, `direction`, `hit_fraction`, `hit_position`, and `has_hit`) and a discoverable `reticle_ray` script. The script draws a thin 3D wire arrow along the copied camera ray, ending at the candidate position when present; otherwise it draws a 50-unit direction preview. The latter is a visualization length, not the query's maximum distance.

- Installed ASI SHA-256: `5A5ACA0481CE9E2B7B47E98E68E1C4F2202E84A9F6EB5DA6E5ECBA13F8019C4A`; prior ASI backup: `scripts\MadMaxModding.asi.before-reticle-ray-20261031`.
- Installed API SHA-256: `690016D9C67AE49516C1F86CB69EC84ADFD6CCD128B0BEEDDECCD9BBF34A746A`; prior copy: `MadMaxModding\API.md.before-reticle-ray-20261031`.
- Installed `reticle_ray` package hashes match the repository: `manifest.toml` `C9DC179751E97B330E20F59413E3B38A24A50BCD772CCB76B53F65AF1FCC0EAB`; `main.rhai` `EF71B5676BD88039C346B057CE5BB240AF5F12B4AA9D8C9EE8274C6DD665DF64`.
- Verification: `cargo fmt --all -- --check`, all 80 Rust tests, `cargo build --release --locked`, and `git diff --check` passed. Existing dead-code warnings remain for disabled world-query diagnostic hooks.
- Mad Max was confirmed closed before install and was not launched; in-game appearance and hit behavior await user check.

## 2026-10-31 — ray-origin and direction-sign visual discriminator

Updated only the `reticle_ray` Rhai package while Mad Max remained open; the ASI and native query path were not changed. The script still issues one native query. It now marks the copied ray origin in cyan, the copied player position in yellow, and draws a short magenta arrow along the opposite of the queried direction as a visual sign reference. This distinguishes an origin offset from a direction-sign problem without adding another query.

- The previous installed script is preserved as `MadMaxModding\scripts\reticle_ray\main.rhai.before-ray-origin-sign-diagnostic-20261031`.
- Installed script SHA-256: `113C31105D9D5EC3A2A154E64A7F6598BFD0BACC07C4DA40CF3DAFEE5F4F107F`.
- The Rhai callback test passed for both hit and miss cases, with line counts updated for the new markers and opposite-direction reference. Mad Max is running; the updated package still needs to be reloaded and visually checked.

## 2026-10-31 — player-relative right/down ray probe

Replaced the camera-derived ray input with a controlled diagnostic ray built from Max's copied actor transform. Its origin is 10 world units along Max's horizontal forward direction and 5 units above his transform position. Its normalized direction is 45 degrees toward actor-right and world-down. It continues to issue exactly one `CStaticOnlyRaycastFilter` query, keeping query/filter behavior fixed while removing camera-matrix interpretation from this test. The `reticle_ray` script marks the player position and ray origin, then draws the ray and optional hit marker.

- Installed ASI SHA-256: `900B249F2F1F25EDF64815C5D0AD2C84525E07085D01D626DA441DD4BAE0736D`; previous ASI preserved as `scripts\MadMaxModding.asi.before-player-relative-ray-20261031` (SHA-256 `5A5ACA0481CE9E2B7B47E98E68E1C4F2202E84A9F6EB5DA6E5ECBA13F8019C4A`).
- Installed API SHA-256: `592DE97E6AA852477BDEBE20480DAF0507935F4707F3CF67C703089819BCE1A0`; previous copy preserved as `MadMaxModding\API.md.before-player-relative-ray-20261031` (SHA-256 `690016D9C67AE49516C1F86CB69EC84ADFD6CCD128B0BEEDDECCD9BBF34A746A`).
- Installed script SHA-256: `36749401A2A34A6A833B75F59376BA7591BDF61A28CEEE96FC3BE6C784399459`; previous copy preserved as `MadMaxModding\scripts\reticle_ray\main.rhai.before-player-relative-ray-20261031` (SHA-256 `113C31105D9D5EC3A2A154E64A7F6598BFD0BACC07C4DA40CF3DAFEE5F4F107F`).
- Verification at install: all 81 Rust tests passed; release build, format check, and `git diff --check` passed. Mad Max was closed for installation. The user then launched the game, tested the installed controlled ray, and reported that it “works perfectly.” The exact filter behavior for rays crossing Max remains unverified.

## 2026-10-31 — general Rhai raycast API and camera reticle

Installed the general-purpose deferred raycast API and camera-driven `reticle_ray` consumer while Mad Max was closed. Rhai scripts can request a world ray with copied origin/direction, range, and the named `static_only` or `player_aim` profile, then read generation-scoped copied ticket results on a later callback. Requests are bounded to one per script callback and eight native queries per game callback. The debug Raycast section contains only the optional `Hit position` row; the example draws no additional geometry.

- Installed ASI: `C:\Program Files (x86)\Steam\steamapps\common\Mad Max\scripts\MadMaxModding.asi`
- ASI SHA-256: `24BD2BC2AC4A92F77E1C63920141D2EAFB3077BD66477D4CA1DC69459298D5C1` (11,335,168 bytes; source artifact `target\x86_64-pc-windows-msvc\release\madmax_bootstrap.dll`)
- Previous ASI SHA-256: `93C389513B4930BB368BE54D82326554887F42FC367F45CC4E313B856F9CA959`
- Backup directory: `C:\Program Files (x86)\Steam\steamapps\common\Mad Max\MadMaxModding\backups\general-raycast-20261031-044822`
- Installed `API.md` SHA-256: `1618573FD3CF1FF7019B12A0C4C22129687C834AB039E875776264D9CBFFC912`; it matches `docs\rhai-api.md`.
- Installed `reticle_ray\manifest.toml` SHA-256: `4053F403F6D6621D662825107A823BFA62986BE5D93279E31719F46DA89F32D7`.
- Installed `reticle_ray\main.rhai` SHA-256: `8A2DE18E17036EBFB4AFFD65D542899964B3B844EE1326CB4A678EBD7D5FADD6`.
- Verification: `cargo fmt --all -- --check`, `cargo check --target x86_64-pc-windows-msvc --locked`, and `cargo build --release --target x86_64-pc-windows-msvc --locked` completed; deployed hashes match source/build hashes. Existing warnings concern unused world-query capture hooks. No game launch was performed; camera-direction sign, in-game script loading, and hit behavior remain unverified pending user check.

## 2026-10-31 — oversized reticle-hit gizmo cluster

Updated only the `reticle_ray` package and generated API reference; the ASI was not changed. The script keeps `game.raycast` data-only and separately requests the `debug.draw` capability. On a completed hit it places a large cluster at the reported hit position: 24-unit RGB axes, red radius-1 wire sphere, magenta radius-3 x-ray sphere, cyan radius-6 surface-wire sphere, yellow 16-unit-wide wire box, and a white 16-unit upward arrow. The debug panel remains one `Hit position` row.

- Backup directory: `C:\Program Files (x86)\Steam\steamapps\common\Mad Max\MadMaxModding\backups\reticle-hit-gizmos-20261031-045851`.
- Installed `API.md` SHA-256: `B459A25913E699480A2B638777B9F47647F6EE04CE73CA56CBD102A1CC971F90`.
- Installed `reticle_ray\manifest.toml` SHA-256: `648456D63926CD265CA3836E8843F12F14E6FBBD9E1C74D22D3CCA14B0A4E5F1`.
- Installed `reticle_ray\main.rhai` SHA-256: `7AC8BF29ABFE3D5D3C9D313FC38931827292E423879CB0B34FEFEBC305B34BCB`.
- `cargo fmt --all` and the Windows-target `cargo check --locked` completed. Mad Max was closed for the file replacement; the updated Rhai package still needs to be loaded in-game and visually checked.

## 2026-10-31 — camera-basis sign diagnostic

Kept the existing single ray and hit-marker cluster, and added a camera-origin marker plus two short arrows: green follows the submitted camera direction, magenta follows its negation. This isolates camera-axis/sign geometry from filter selection and hit-point reconstruction. Only the hot-reloadable Rhai script changed; the ASI was not replaced. The previous installed script is preserved as `MadMaxModding\scripts\reticle_ray\main.rhai.before-camera-basis-diagnostic-20261031`.

- Installed script SHA-256: `BB218C353A312FD8161CCB9CC09FB3A4A22710650620D159F01FCEBADDF96D77` (matches `examples\scripts\reticle_ray\main.rhai`).
- Mad Max remained open. The visible direction comparison awaits user observation; no game launch or process modification was performed.
