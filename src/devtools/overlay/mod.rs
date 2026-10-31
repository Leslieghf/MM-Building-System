//! DX11/ImGui rendering, input ownership, and hook lifecycle for developer tools.
//!
//! # Thread and ownership boundary
//!
//! The swap-chain callback owns ImGui frame construction and overlay input.
//! It reads copied presentation resources and submits UI intents through
//! bounded channels; game reads, script callbacks, and native effects remain
//! on the accepted game callback. Overlay installation is guarded by the
//! supported executable/build checks in the game adapter.
//!
//! # Failure behavior
//!
//! Hook setup failures are reported through process diagnostics and leave game
//! sampling disabled. Rendering must not wait for script compilation or game
//! work; UI state remains local to this module and `devtools::state`.
mod clipboard;
mod config;
mod panel;
use clipboard::Win32ClipboardBackend;
use config::*;
use panel::*;

use crate::api::ecs::{
    GameSnapshot, MadMaxRender, RuntimeDiagnostics, UiEndpoint, spawn_entity_intent,
};
use crate::api::game::abi::types::Vector3;
use crate::api::game::native as game_probe;
use crate::api::scripts::console::{
    ConsoleCommand, ConsoleEvent, ConsoleOutput, ConsoleRequestSender, ConsoleSeverity,
};
use crate::devtools::presentation::{create_ui_app, flush_ui_intents};
use crate::devtools::state::{
    DebugSection, OverlayMode, PendingUiInput, ScriptConsoleUiState, SpawnTestUiState,
    UI_OPACITY_MAX, UI_OPACITY_MIN, UiGameSnapshot, UiPreferences, UiPresentation,
};
use crate::kernel::diagnostics::append_log;
use bevy_app::App;
use hudhook::hooks::dx11::ImguiDx11Hooks;
use hudhook::windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use hudhook::windows::Win32::UI::WindowsAndMessaging::{WM_ACTIVATEAPP, WM_KILLFOCUS, WM_SETFOCUS};
use hudhook::{Hudhook, ImguiRenderLoop, MessageFilter, imgui};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Mutex, OnceLock};

/// Owns one frame of ImGui presentation and the UI-side runtime bridge.
struct OverlayRenderDriver;

struct UiRuntime {
    app: App,
    endpoint: UiEndpoint,
}

static UI_ENDPOINT: OnceLock<Mutex<Option<UiEndpoint>>> = OnceLock::new();
static CLIPBOARD_BACKEND_INITIALIZED: AtomicBool = AtomicBool::new(false);

thread_local! {
    static UI_RUNTIME: std::cell::RefCell<Option<UiRuntime>> = const { std::cell::RefCell::new(None) };
}

pub(crate) fn set_ui_endpoint(endpoint: UiEndpoint) {
    if UI_ENDPOINT.set(Mutex::new(Some(endpoint))).is_err() {
        append_log("Bevy UI endpoint was already initialized; keeping the original endpoint.");
    }
}

impl UiRuntime {
    fn take_from_startup() -> Option<Self> {
        let endpoint = UI_ENDPOINT
            .get()?
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()?;
        Some(Self {
            app: create_ui_app(),
            endpoint,
        })
    }

    fn sync_worlds(&mut self) {
        if let Some(snapshot) = take_latest_snapshot(&self.endpoint.snapshot_rx) {
            publish_snapshot(&mut self.app, snapshot);
        }
        publish_latest_outcome(&mut self.app, &self.endpoint.outcome_rx);
        self.app
            .world_mut()
            .resource_mut::<ConsoleOutput>()
            .drain(&self.endpoint.console_events);
        publish_discovery_results(&mut self.app, &self.endpoint.discovery_rx);
        self.app.world_mut().resource_scope(
            |world, mut state: bevy_ecs::prelude::Mut<ScriptConsoleUiState>| {
                state.observe_output(world.resource::<ConsoleOutput>());
            },
        );
        self.app.world_mut().run_schedule(MadMaxRender);
        flush_ui_intents(&mut self.app, &self.endpoint);
    }
}

fn take_latest_snapshot(receiver: &Receiver<GameSnapshot>) -> Option<GameSnapshot> {
    let mut latest = None;
    while let Ok(snapshot) = receiver.try_recv() {
        latest = Some(snapshot);
    }
    latest
}

fn publish_snapshot(app: &mut bevy_app::App, snapshot: GameSnapshot) {
    *app.world_mut().resource_mut::<UiGameSnapshot>() = UiGameSnapshot(snapshot);
    let mut diagnostics = app.world_mut().resource_mut::<RuntimeDiagnostics>();
    diagnostics.callback_thread_id = snapshot.callback_thread_id;
    diagnostics.callback_stable_streak = snapshot.callback_stable_streak;
    diagnostics.callback_overlap_count = snapshot.callback_overlap_count;
    diagnostics.callback_thread_mismatch_count = snapshot.callback_thread_mismatch_count;
    diagnostics.callback_gate_open = snapshot.callback_gate_open;
    diagnostics.snapshots_dropped = snapshot.dropped_snapshots;
    diagnostics.intents_dropped = snapshot.dropped_intents;
    diagnostics.post_sim_schedule_runs = snapshot.post_sim_schedule_runs;
    diagnostics.last_intent_outcome = snapshot.last_intent_outcome;
}

fn publish_latest_outcome(
    app: &mut bevy_app::App,
    receiver: &Receiver<crate::api::ecs::IntentOutcome>,
) {
    let mut latest = None;
    while let Ok(outcome) = receiver.try_recv() {
        latest = Some(outcome);
    }
    if let Some(outcome) = latest {
        app.world_mut()
            .resource_mut::<RuntimeDiagnostics>()
            .last_intent_outcome = Some(outcome);
    }
}

fn publish_discovery_results(
    app: &mut bevy_app::App,
    receiver: &Receiver<crate::api::scripts::loader::ScriptDiscoveryResult>,
) {
    let mut latest = None;
    for _ in 0..crate::api::scripts::loader::LOADER_QUEUE_CAPACITY {
        if let Ok(result) = receiver.try_recv() {
            latest = Some(result);
        } else {
            break;
        }
    }
    let Some(result) = latest else {
        return;
    };
    let mut state = app.world_mut().resource_mut::<ScriptConsoleUiState>();
    state.discovered_scripts = result.packages;
    state.discovery_status = result
        .error
        .unwrap_or_else(|| "Package scan complete.".into());
    state.discovery_pending = false;
    state.pending_discovery_request = None;
    if state.selected_package.as_ref().is_none_or(|selected| {
        !state
            .discovered_scripts
            .iter()
            .any(|row| &row.package_name == selected)
    }) {
        state.selected_package = state
            .discovered_scripts
            .first()
            .map(|row| row.package_name.clone());
    }
}

impl OverlayRenderDriver {
    fn new() -> Self {
        Self
    }
}

fn with_ui_runtime<R>(operation: impl FnOnce(&mut UiRuntime) -> R) -> Option<R> {
    UI_RUNTIME.with(|runtime_cell| {
        let mut runtime_cell = runtime_cell.borrow_mut();
        if runtime_cell.is_none() {
            *runtime_cell = UiRuntime::take_from_startup();
        }
        runtime_cell.as_mut().map(operation)
    })
}

impl ImguiRenderLoop for OverlayRenderDriver {
    fn before_render(
        &mut self,
        context: &mut imgui::Context,
        _render_context: &mut dyn hudhook::RenderContext,
    ) {
        if CLIPBOARD_BACKEND_INITIALIZED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            context.set_clipboard_backend(Win32ClipboardBackend);
        }
        context.io_mut().mouse_draw_cursor = with_ui_runtime(|runtime| {
            runtime
                .app
                .world()
                .resource::<UiPreferences>()
                .captures_game_input()
        })
        .unwrap_or(false);
    }

    fn render(&mut self, ui: &mut imgui::Ui) {
        let _ = with_ui_runtime(|runtime| {
            runtime.sync_worlds();
            sample_and_publish_input(runtime, ui);
            fire_input_binds(runtime, ui, ui.io().want_text_input);
            let mut preferences = *runtime.app.world().resource::<UiPreferences>();
            preferences.set_ui_opacity(preferences.ui_opacity);
            handle_overlay_shortcuts(runtime, ui, &mut preferences);
            let rows = runtime.app.world().resource::<UiPresentation>().0.clone();
            draw_debug_panel(ui, &preferences, &rows);
            if preferences.mode == OverlayMode::UiConfigurator
                && draw_ui_configurator(runtime, ui, &mut preferences)
            {
                preferences.set_mode(OverlayMode::Hud);
            }
            if preferences.mode == OverlayMode::ScriptConsole
                && draw_script_console(runtime, ui, &preferences)
            {
                preferences.set_mode(OverlayMode::Hud);
            }
            *runtime.app.world_mut().resource_mut::<UiPreferences>() = preferences;
            game_probe::set_ui_input_capture(preferences.captures_game_input());
        });
    }
    fn after_wnd_proc(&self, _hwnd: HWND, message: u32, wparam: WPARAM, _lparam: LPARAM) {
        match message {
            WM_ACTIVATEAPP => game_probe::set_game_window_active(wparam.0 != 0),
            WM_SETFOCUS => game_probe::set_game_window_active(true),
            WM_KILLFOCUS => game_probe::set_game_window_active(false),
            _ => {}
        }
    }

    fn message_filter(&self, _io: &imgui::Io) -> MessageFilter {
        if with_ui_runtime(|runtime| {
            runtime
                .app
                .world()
                .resource::<UiPreferences>()
                .captures_game_input()
        })
        .unwrap_or(false)
        {
            MessageFilter::InputAll
        } else {
            MessageFilter::empty()
        }
    }
}

fn sample_and_publish_input(runtime: &mut UiRuntime, ui: &imgui::Ui) {
    if !ENABLE_RUNTIME_INPUT_SAMPLING {
        return;
    }
    let frame = {
        let mut frame = runtime
            .app
            .world_mut()
            .resource_mut::<crate::api::ecs::input::InputFrame>();
        frame.clear_edges();
        for key in imgui::Key::VARIANTS {
            if let Some(key_id) = crate::devtools::input::from_imgui(key) {
                frame.set_down(key_id.id(), sample_input_key(ui, key));
            }
        }
        *frame
    };
    let mut pending = runtime.app.world_mut().resource_mut::<PendingUiInput>();
    pending.0.accumulate_sample(frame);
    if runtime.endpoint.input_mailbox.publish(pending.0) {
        pending.0.clear_edges();
    }
}

fn handle_overlay_shortcuts(
    runtime: &mut UiRuntime,
    ui: &imgui::Ui,
    preferences: &mut UiPreferences,
) {
    let escape_pressed = ui.is_key_pressed_no_repeat(imgui::Key::Escape);
    let escape_down = ui.is_key_down(imgui::Key::Escape);
    if preferences.escape_capture_release_pending && !escape_down {
        preferences.escape_capture_release_pending = false;
        preferences.input_capture_override = None;
    }
    let capture = ui.is_key_pressed_no_repeat(imgui::Key::F1);
    let configurator = ui.is_key_pressed_no_repeat(imgui::Key::F2);
    let console = ui.is_key_pressed_no_repeat(imgui::Key::F3);
    if escape_pressed && (preferences.mode != OverlayMode::Hud || preferences.captures_game_input())
    {
        release_input_capture(preferences);
    } else if capture {
        preferences.toggle_input_capture();
        focus_console_prompt(runtime, preferences);
    } else if configurator {
        toggle_overlay_mode(preferences, OverlayMode::UiConfigurator);
    } else if console {
        toggle_overlay_mode(preferences, OverlayMode::ScriptConsole);
        if preferences.mode == OverlayMode::ScriptConsole {
            prepare_console_window(runtime);
        }
    }
}

fn release_input_capture(preferences: &mut UiPreferences) {
    let keep_capture_until_key_up = preferences.captures_game_input();
    preferences.set_mode(OverlayMode::Hud);
    if keep_capture_until_key_up {
        preferences.input_capture_override = Some(true);
        preferences.escape_capture_release_pending = true;
    }
}

fn focus_console_prompt(runtime: &mut UiRuntime, preferences: &UiPreferences) {
    if preferences.mode == OverlayMode::ScriptConsole && preferences.captures_game_input() {
        runtime
            .app
            .world_mut()
            .resource_mut::<ScriptConsoleUiState>()
            .focus_prompt = true;
    }
}

fn toggle_overlay_mode(preferences: &mut UiPreferences, mode: OverlayMode) {
    preferences.set_mode(if preferences.mode == mode {
        OverlayMode::Hud
    } else {
        mode
    });
}

fn prepare_console_window(runtime: &mut UiRuntime) {
    runtime.app.world_mut().resource_scope(
        |_world, mut state: bevy_ecs::prelude::Mut<ScriptConsoleUiState>| {
            state.focus_prompt = true;
            state.restore_scroll = true;
        },
    );
}

fn draw_debug_panel(
    ui: &imgui::Ui,
    preferences: &UiPreferences,
    rows: &[crate::devtools::state::PresentationRow],
) {
    let style = ui.push_style_color(
        imgui::StyleColor::WindowBg,
        [0.055, 0.075, 0.105, preferences.ui_opacity],
    );
    ui.window("Mad Max Debug")
        .position([12.0, 12.0], imgui::Condition::Always)
        .bg_alpha(preferences.ui_opacity)
        .flags(overlay_window_flags(preferences))
        .build(|| {
            draw_debug_sections(ui, rows, preferences);
            draw_overlay_shortcut_hints(ui, preferences);
        });
    style.pop();
}

fn overlay_window_flags(preferences: &UiPreferences) -> imgui::WindowFlags {
    imgui::WindowFlags::NO_SAVED_SETTINGS
        | imgui::WindowFlags::ALWAYS_AUTO_RESIZE
        | if preferences.captures_game_input() {
            imgui::WindowFlags::empty()
        } else {
            imgui::WindowFlags::NO_INPUTS
        }
}

fn draw_debug_sections(
    ui: &imgui::Ui,
    rows: &[crate::devtools::state::PresentationRow],
    preferences: &UiPreferences,
) {
    for (visible, title, section) in [
        (preferences.show_player, "Player", DebugSection::Player),
        (preferences.show_camera, "Camera", DebugSection::Camera),
        (preferences.show_raycast, "Raycast", DebugSection::Raycast),
        (preferences.show_vehicle, "Vehicle", DebugSection::Vehicle),
        (preferences.show_runtime, "Runtime", DebugSection::Runtime),
    ] {
        if visible {
            section_header(ui, title);
            draw_rows(ui, rows, section);
        }
    }
}

fn draw_overlay_shortcut_hints(ui: &imgui::Ui, preferences: &UiPreferences) {
    ui.separator();
    ui.text(if preferences.captures_game_input() {
        "F1: release mouse/input"
    } else {
        "F1: capture mouse/input"
    });
    ui.text("F2: UI configurator");
    ui.text("F3: Dev Tools");
}

fn draw_ui_configurator(
    runtime: &mut UiRuntime,
    ui: &imgui::Ui,
    preferences: &mut UiPreferences,
) -> bool {
    let mut close_requested = false;
    let style = ui.push_style_color(
        imgui::StyleColor::WindowBg,
        [0.055, 0.075, 0.105, preferences.ui_opacity],
    );
    ui.window("UI Configurator")
        .position([12.0, 300.0], imgui::Condition::FirstUseEver)
        .flags(overlay_window_flags(preferences))
        .build(|| {
            draw_configurator_controls(ui, preferences);
            draw_buggy_spawn_control(runtime, ui);
            ui.separator();
            close_requested = ui.button("Close");
            ui.same_line();
            ui.text("Escape closes the UI and returns input to the game.");
        });
    style.pop();
    close_requested
}

fn draw_configurator_controls(ui: &imgui::Ui, preferences: &mut UiPreferences) {
    ui.text("Show in the debug panel:");
    ui.checkbox("Player", &mut preferences.show_player);
    ui.checkbox("Camera", &mut preferences.show_camera);
    ui.checkbox("Raycast", &mut preferences.show_raycast);
    ui.checkbox("Vehicle", &mut preferences.show_vehicle);
    ui.checkbox("Runtime", &mut preferences.show_runtime);
    ui.separator();
    ui.slider_config("Background opacity", UI_OPACITY_MIN, UI_OPACITY_MAX)
        .flags(imgui::SliderFlags::ALWAYS_CLAMP)
        .display_format("%.2f")
        .build(&mut preferences.ui_opacity);
    preferences.set_ui_opacity(preferences.ui_opacity);
    ui.text_disabled("F1: toggle capture | F2: close configurator | F3: Dev Tools");
}

fn draw_buggy_spawn_control(runtime: &mut UiRuntime, ui: &imgui::Ui) {
    ui.separator();
    if runtime.app.world().resource::<SpawnTestUiState>().attempted {
        ui.text("One buggy request submitted; check Runtime for status.");
        return;
    }
    let Some(mut transform) = runtime
        .app
        .world()
        .resource::<UiGameSnapshot>()
        .0
        .player_transform
    else {
        ui.text("Waiting for a fresh player transform.");
        return;
    };
    if !ui.button("Submit one test buggy spawn") {
        return;
    }
    let position =
        transform.translation() + transform.forward() * 8.0 + Vector3::new(0.0, 0.2, 0.0);
    transform.set_translation(position);
    if let Ok(bundle) = spawn_entity_intent(
        "editor/entities/a00/exported/a00_vehicle_empty_buggy",
        transform,
    ) {
        runtime.app.world_mut().spawn(bundle);
        runtime
            .app
            .world_mut()
            .resource_mut::<SpawnTestUiState>()
            .attempted = true;
    }
}

fn draw_script_console(
    runtime: &mut UiRuntime,
    ui: &imgui::Ui,
    preferences: &UiPreferences,
) -> bool {
    let mut close_requested = false;
    runtime.app.world_mut().resource_scope(
        |world, mut state: bevy_ecs::prelude::Mut<ScriptConsoleUiState>| {
            let mut output = world.resource_mut::<ConsoleOutput>();
            close_requested = draw_dev_tools(
                ui,
                &mut state,
                &mut output,
                &mut runtime.endpoint.console_requests,
                &runtime.endpoint.discovery_requester,
                preferences.captures_game_input(),
                preferences.ui_opacity,
            );
        },
    );
    close_requested
}

pub(crate) fn install_overlay_hook() -> bool {
    match Hudhook::builder()
        .with::<ImguiDx11Hooks>(OverlayRenderDriver::new())
        .build()
        .apply()
    {
        Ok(()) => {
            append_log("DirectX 11 render hook installed.");
            true
        }
        Err(error) => {
            append_log(&format!("DirectX 11 render hook failed: {error:?}"));
            false
        }
    }
}

#[cfg(test)]
mod scripts_tab_tests {
    use super::*;
    use crate::api::scripts::{ScriptGeneration, ScriptId, ScriptStatus};

    #[test]
    fn inline_toggle_submits_typed_inverse_without_console_or_prompt_changes() {
        let (mut sender, receiver) = crate::api::scripts::console::request_channel();
        let mut state = ScriptConsoleUiState::default();
        state.input = "keep my draft".into();
        let mut output = ConsoleOutput::default();
        output.record(status_event("first", false, None));
        state.observe_output(&output);
        let revision = output.revision();
        issue_auto_reload(&mut output, &mut sender, &state.loaded_scripts[0]);
        assert_eq!(
            receiver.try_recv().unwrap().command,
            ConsoleCommand::ScriptAutoReload {
                id: ScriptId::parse("first").unwrap(),
                enabled: true,
            }
        );
        assert_eq!(output.revision(), revision);
        assert!(!state.loaded_scripts[0].auto_reload);
        assert_eq!(state.input, "keep my draft");
        assert!(state.history.is_empty());
        output.record(status_event("first", true, None));
        state.observe_output(&output);
        issue_auto_reload(&mut output, &mut sender, &state.loaded_scripts[0]);
        assert_eq!(
            receiver.try_recv().unwrap().command,
            ConsoleCommand::ScriptAutoReload {
                id: ScriptId::parse("first").unwrap(),
                enabled: false,
            }
        );
        assert!(state.loaded_scripts[0].auto_reload);
    }

    #[test]
    fn rejected_inline_toggle_records_one_row_error_without_changing_accepted_state() {
        let (mut sender, receiver) = crate::api::scripts::console::request_channel();
        drop(receiver);
        let mut state = ScriptConsoleUiState::default();
        let mut output = ConsoleOutput::default();
        output.record(status_event("first", false, None));
        state.observe_output(&output);
        issue_auto_reload(&mut output, &mut sender, &state.loaded_scripts[0]);
        state.observe_output(&output);
        assert!(!state.loaded_scripts[0].auto_reload);
        assert!(
            state.loaded_scripts[0]
                .reload_error
                .as_ref()
                .unwrap()
                .contains("disconnected")
        );
        assert_eq!(
            output
                .events()
                .iter()
                .filter(|event| event.severity == ConsoleSeverity::Error)
                .count(),
            1
        );
        assert!(
            !output
                .events()
                .iter()
                .any(|event| event.text.starts_with("CMD:"))
        );
    }

    fn status_event(id: &str, enabled: bool, error: Option<&str>) -> ConsoleEvent {
        let mut event = ConsoleEvent::new(None, ConsoleSeverity::Info, "script status");
        event.script = Some(ScriptId::parse(id).unwrap());
        event.generation = Some(ScriptGeneration(1));
        event.status = Some(ScriptStatus::Active);
        event.auto_reload = Some(enabled);
        event.reload_error = error.map(str::to_owned);
        event
    }

    #[test]
    fn accepted_auto_reload_updates_only_the_matching_row_and_selection() {
        let mut state = ScriptConsoleUiState::default();
        let mut output = ConsoleOutput::default();
        output.record(status_event("first", false, None));
        output.record(status_event("second", false, None));
        state.observe_output(&output);
        output.record(status_event("second", true, None));
        state.observe_output(&output);
        assert!(
            !state
                .loaded_scripts
                .iter()
                .find(|row| row.id.as_str() == "first")
                .unwrap()
                .auto_reload
        );
        assert!(
            state
                .loaded_scripts
                .iter()
                .find(|row| row.id.as_str() == "second")
                .unwrap()
                .auto_reload
        );
        assert_eq!(state.selected_script.as_ref().unwrap().id.as_str(), "first");
        assert!(!state.selected_script.as_ref().unwrap().auto_reload);
        output.record(status_event("first", true, None));
        state.observe_output(&output);
        assert!(state.selected_script.as_ref().unwrap().auto_reload);
    }

    #[test]
    fn reload_error_updates_one_row_once_and_success_clears_it() {
        let mut state = ScriptConsoleUiState::default();
        let mut output = ConsoleOutput::default();
        output.record(status_event("first", true, None));
        output.record(status_event("second", false, None));
        state.observe_output(&output);
        let mut failure = status_event("first", true, Some("entry.rhai: syntax error"));
        failure.severity = ConsoleSeverity::Error;
        failure.text = "entry.rhai: syntax error".into();
        output.record(failure);
        state.observe_output(&output);
        state.observe_output(&output);
        assert_eq!(
            state
                .loaded_scripts
                .iter()
                .find(|row| row.id.as_str() == "first")
                .unwrap()
                .reload_error
                .as_deref(),
            Some("entry.rhai: syntax error")
        );
        assert!(
            state
                .loaded_scripts
                .iter()
                .find(|row| row.id.as_str() == "second")
                .unwrap()
                .reload_error
                .is_none()
        );
        assert_eq!(
            output
                .events()
                .iter()
                .filter(|event| event.severity == ConsoleSeverity::Error)
                .count(),
            1
        );
        assert_eq!(state.loaded_scripts.len(), 2);
        assert!(
            state
                .selected_script
                .as_ref()
                .unwrap()
                .reload_error
                .is_some()
        );
        let mut success = status_event("first", true, None);
        success.generation = Some(ScriptGeneration(2));
        output.record(success);
        state.observe_output(&output);
        let first = state
            .loaded_scripts
            .iter()
            .find(|row| row.id.as_str() == "first")
            .unwrap();
        assert!(first.auto_reload);
        assert_eq!(first.generation, ScriptGeneration(2));
        assert!(first.reload_error.is_none());
        assert!(
            state
                .selected_script
                .as_ref()
                .unwrap()
                .reload_error
                .is_none()
        );
    }
}
