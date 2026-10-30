# Console and UI Comfort Pass

**Date:** 2026-10-29  
**Status:** Implementing from the user's explicit requirements  
**Scope:** Console behavior, ImGui presentation, UI hotkeys, and key-binding feasibility

## Intent

Make the live developer console comfortable for repeated use and make the overlay controls consistent. All floating-point telemetry is rendered as signed fixed three-decimal values truncated toward zero; integer identifiers/counters remain integers. Console submission always clears the prompt, including invalid and rejected commands. `clear` removes retained output. Output supports terminal-like severity colors and a bounded ANSI SGR subset. Remove buttons that only submit commands, but retain script-cache refresh. Add configurable overlay opacity. Bindings should expose a discoverable key-ID list and `bind`/`unbind`/`show_binds` commands only when their keys can be reliably intercepted without also triggering Mad Max actions; the user requires native action suppression.

## Goals and non-goals

- F1 toggles mouse and game-input capture, F2 toggles the UI configurator, F3 toggles the console.
- Escape while UI capture is active closes the mod panel while global input capture remains active through the Escape key-up; only then is Mad Max input focus restored. This prevents a game polling path from observing the same held Escape after the UI closes.
- The configurator includes an opacity slider applied consistently to overlay panels.
- Console colors distinguish command echoes, informational output, warnings, and errors; supported SGR colors are parsed into ImGui spans.
- Remove the API List, Help, and per-row Status command buttons; typed commands remain available. Keep Refresh scripts because it refreshes the UI's cached list.
- Keep command parsing, output, and key identifiers bounded and reject unsupported ANSI/control sequences safely.
- No arbitrary script editor, unrestricted terminal escape processing, or unreviewed game-input behavior.

## Approaches considered

1. Keep adding command-launch buttons and severity-only colors: rejected because it preserves the clutter and weak styling the user called out.
2. Typed console commands plus styled output, an opacity preference, and explicit hotkeys (selected): fits the current ImGui console and bounded command API.
3. Full Source Engine bindings immediately: defer until a per-key hook can consume both normal key messages and Mad Max's Raw Input path. Hudhook 0.9.3 provides only category-wide input filtering and calls its render-loop WndProc callback asynchronously after the original procedure has already decided whether to forward the message; presenting a parser-only or non-exclusive bind as a working game bind would be misleading. The user requires native action suppression.

## Implementation calls

- The existing section picker becomes the UI configurator while retaining its section checkboxes and spawn prototype.
- Fixed numeric metric precision remains three digits; integral counters and hexadecimal addresses retain their natural integer forms.
- ANSI support is restricted to SGR foreground colors, bright colors, bold, and reset. Other controls are ignored as controls, never executed.
- `clear` is local to the console UI and clears scrollback without sending a game callback request.
- Keep the existing global input capture for overlay interaction; do not add a per-key hook until raw-input suppression is understood and verified.

## Acceptance

- F1/F2/F3 have the requested functions and the on-screen hints match.
- Prompt clears after successful and failed submissions.
- `clear` empties the visible/retained scrollback.
- Command and severity styling is legible; supported SGR spans render as colors.
- Command-launch buttons listed above are removed.
- Opacity slider affects debug HUD, configurator, and console.
- All float telemetry uses fixed three-decimal truncation; integer values remain readable.
- Escape is filtered while the UI owns input, including the Raw Input category.
- Build/format verification passes; no automated tests are added or run.

## Deferred: key commands

Implement `bind`, `unbind`, `show_binds`, and `show_keys` only after a focused input-hook spike proves it can suppress one selected key in both ordinary key messages and relevant Raw Input without suppressing unrelated gameplay input. Native action overlap is not accepted; until then, do not expose partially functional bind commands.
