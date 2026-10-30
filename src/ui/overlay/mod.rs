//! DX11/ImGui presentation and lifecycle for the developer overlay.
mod clipboard;
mod config;
mod panel;
use clipboard::Win32ClipboardBackend;
use config::*;
use panel::*;

use crate::native::game_probe;
use crate::native::logging::append_log;
use crate::runtime::{
    DebugSection, MadMaxRender, OverlayMode, PendingUiInput, RuntimeDiagnostics,
    ScriptConsoleUiState, SpawnTestUiState, UiEndpoint, UiGameSnapshot, UiPreferences,
    UiPresentation, create_ui_app, spawn_entity_intent,
};
use crate::scripting::console::{
    ConsoleCommand, ConsoleEvent, ConsoleOutput, ConsoleRequestSender, ConsoleSeverity,
};
use crate::sdk::types::Vector3;
use bevy_app::App;
use hudhook::hooks::dx11::ImguiDx11Hooks;
use hudhook::windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use hudhook::windows::Win32::UI::WindowsAndMessaging::{WM_ACTIVATEAPP, WM_KILLFOCUS, WM_SETFOCUS};
use hudhook::{Hudhook, ImguiRenderLoop, MessageFilter, imgui};
use std::collections::VecDeque;
use std::panic;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::TryRecvError;
use std::sync::{Mutex, OnceLock};
use std::thread;

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

fn set_ui_endpoint(endpoint: UiEndpoint) {
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
        let mut latest_snapshot = None;
        loop {
            match self.endpoint.snapshot_rx.try_recv() {
                Ok(snapshot) => latest_snapshot = Some(snapshot),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
        if let Some(snapshot) = latest_snapshot {
            *self.app.world_mut().resource_mut::<UiGameSnapshot>() = UiGameSnapshot(snapshot);
            let mut diagnostics = self.app.world_mut().resource_mut::<RuntimeDiagnostics>();
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
        loop {
            match self.endpoint.outcome_rx.try_recv() {
                Ok(outcome) => {
                    self.app
                        .world_mut()
                        .resource_mut::<RuntimeDiagnostics>()
                        .last_intent_outcome = Some(outcome);
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
        self.app
            .world_mut()
            .resource_mut::<ConsoleOutput>()
            .drain(&self.endpoint.console_events);
        let mut discoveries = Vec::new();
        for _ in 0..crate::scripting::loader::LOADER_QUEUE_CAPACITY {
            match self.endpoint.discovery_rx.try_recv() {
                Ok(result) => discoveries.push(result),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
        if !discoveries.is_empty() {
            let mut state = self.app.world_mut().resource_mut::<ScriptConsoleUiState>();
            for result in discoveries {
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
        }
        self.app.world_mut().resource_scope(
            |world, mut state: bevy_ecs::prelude::Mut<ScriptConsoleUiState>| {
                state.observe_output(world.resource::<ConsoleOutput>());
            },
        );
        self.app.world_mut().run_schedule(MadMaxRender);
        crate::runtime::flush_ui_intents(&mut self.app, &self.endpoint);
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
            let input_frame = if ENABLE_RUNTIME_INPUT_SAMPLING {
                let mut frame = runtime
                    .app
                    .world_mut()
                    .resource_mut::<crate::sdk::core::input::InputFrame>();
                frame.clear_edges();
                for key in imgui::Key::VARIANTS {
                    if crate::sdk::core::input::key_name(key).is_some() {
                        frame.set_down(key as u32, sample_input_key(ui, key));
                    }
                }
                *frame
            } else {
                crate::sdk::core::input::InputFrame::default()
            };
            if ENABLE_RUNTIME_INPUT_SAMPLING {
                let mut pending = runtime.app.world_mut().resource_mut::<PendingUiInput>();
                pending.0.accumulate_sample(input_frame);
                if runtime.endpoint.input_mailbox.publish(pending.0) {
                    pending.0.clear_edges();
                }
            }
            fire_input_binds(runtime, ui, ui.io().want_text_input);
            let mut preferences = *runtime.app.world().resource::<UiPreferences>();
            let opacity = preferences.ui_opacity;
            preferences.set_ui_opacity(opacity);
            let escape_pressed = ui.is_key_pressed_no_repeat(imgui::Key::Escape);
            let escape_down = ui.is_key_down(imgui::Key::Escape);
            if preferences.escape_capture_release_pending && !escape_down {
                preferences.escape_capture_release_pending = false;
                preferences.input_capture_override = None;
            }
            let capture_pressed = ui.is_key_pressed_no_repeat(imgui::Key::F1);
            let configurator_pressed = ui.is_key_pressed_no_repeat(imgui::Key::F2);
            let console_pressed = ui.is_key_pressed_no_repeat(imgui::Key::F3);

            if escape_pressed
                && (preferences.mode != OverlayMode::Hud || preferences.captures_game_input())
            {
                let keep_capture_until_key_up = preferences.captures_game_input();
                preferences.set_mode(OverlayMode::Hud);
                if keep_capture_until_key_up {
                    preferences.input_capture_override = Some(true);
                    preferences.escape_capture_release_pending = true;
                }
            } else if capture_pressed {
                preferences.toggle_input_capture();
                if preferences.mode == OverlayMode::ScriptConsole
                    && preferences.captures_game_input()
                {
                    runtime
                        .app
                        .world_mut()
                        .resource_mut::<ScriptConsoleUiState>()
                        .focus_prompt = true;
                }
            } else if configurator_pressed {
                preferences.set_mode(if preferences.mode == OverlayMode::UiConfigurator {
                    OverlayMode::Hud
                } else {
                    OverlayMode::UiConfigurator
                });
            } else if console_pressed {
                preferences.set_mode(if preferences.mode == OverlayMode::ScriptConsole {
                    OverlayMode::Hud
                } else {
                    runtime.app.world_mut().resource_scope(
                        |_world, mut state: bevy_ecs::prelude::Mut<ScriptConsoleUiState>| {
                            state.focus_prompt = true;
                            state.restore_scroll = true;
                        },
                    );
                    OverlayMode::ScriptConsole
                });
            }

            let ui_opacity = preferences.ui_opacity;
            let rows = runtime.app.world().resource::<UiPresentation>().0.clone();
            let style = ui.push_style_color(
                imgui::StyleColor::WindowBg,
                [0.055, 0.075, 0.105, ui_opacity],
            );
            ui.window("Mad Max Debug")
                .position([12.0, 12.0], imgui::Condition::Always)
                .bg_alpha(ui_opacity)
                .flags(
                    imgui::WindowFlags::NO_SAVED_SETTINGS
                        | imgui::WindowFlags::ALWAYS_AUTO_RESIZE
                        | if preferences.captures_game_input() {
                            imgui::WindowFlags::empty()
                        } else {
                            imgui::WindowFlags::NO_INPUTS
                        },
                )
                .build(|| {
                    if preferences.show_player {
                        section_header(ui, "Player");
                        draw_rows(ui, &rows, DebugSection::Player);
                    }
                    if preferences.show_camera {
                        section_header(ui, "Camera");
                        draw_rows(ui, &rows, DebugSection::Camera);
                    }
                    if preferences.show_raycast {
                        section_header(ui, "Raycast");
                        draw_rows(ui, &rows, DebugSection::Raycast);
                    }
                    if preferences.show_vehicle {
                        section_header(ui, "Vehicle");
                        draw_rows(ui, &rows, DebugSection::Vehicle);
                    }
                    if preferences.show_runtime {
                        section_header(ui, "Runtime");
                        draw_rows(ui, &rows, DebugSection::Runtime);
                    }
                    ui.separator();
                    ui.text(if preferences.captures_game_input() {
                        "F1: release mouse/input"
                    } else {
                        "F1: capture mouse/input"
                    });
                    ui.text("F2: UI configurator");
                    ui.text("F3: Dev Tools");
                });
            style.pop();

            if preferences.mode == OverlayMode::UiConfigurator {
                let mut close_requested = false;
                let style = ui.push_style_color(
                    imgui::StyleColor::WindowBg,
                    [0.055, 0.075, 0.105, ui_opacity],
                );
                ui.window("UI Configurator")
                    .position([12.0, 300.0], imgui::Condition::FirstUseEver)
                    .flags(
                        imgui::WindowFlags::NO_SAVED_SETTINGS
                            | imgui::WindowFlags::ALWAYS_AUTO_RESIZE
                            | if preferences.captures_game_input() {
                                imgui::WindowFlags::empty()
                            } else {
                                imgui::WindowFlags::NO_INPUTS
                            },
                    )
                    .build(|| {
                        ui.text("Show in the debug panel:");
                        ui.checkbox("Player", &mut preferences.show_player);
                        ui.checkbox("Camera", &mut preferences.show_camera);
                        ui.checkbox("Raycast", &mut preferences.show_raycast);
                        ui.checkbox("Vehicle", &mut preferences.show_vehicle);
                        ui.checkbox("Runtime", &mut preferences.show_runtime);
                        ui.separator();
                        ui.slider_config(
                            "Background opacity",
                            crate::runtime::UI_OPACITY_MIN,
                            crate::runtime::UI_OPACITY_MAX,
                        )
                        .flags(imgui::SliderFlags::ALWAYS_CLAMP)
                        .display_format("%.2f")
                        .build(&mut preferences.ui_opacity);
                        let opacity = preferences.ui_opacity;
                        preferences.set_ui_opacity(opacity);
                        ui.text_disabled(
                            "F1: toggle capture | F2: close configurator | F3: Dev Tools",
                        );
                        ui.separator();
                        let spawn_state = *runtime.app.world().resource::<SpawnTestUiState>();
                        if spawn_state.attempted {
                            ui.text("One buggy request submitted; check Runtime for status.");
                        } else if let Some(mut transform) = runtime
                            .app
                            .world()
                            .resource::<UiGameSnapshot>()
                            .0
                            .player_transform
                        {
                            if ui.button("Submit one test buggy spawn") {
                                let position = transform.translation()
                                    + transform.forward() * 8.0
                                    + Vector3::new(0.0, 0.2, 0.0);
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
                        } else {
                            ui.text("Waiting for a fresh player transform.");
                        }
                        ui.separator();
                        if ui.button("Close") {
                            close_requested = true;
                        }
                        ui.same_line();
                        ui.text("Escape closes the UI and returns input to the game.");
                    });
                style.pop();
                if close_requested {
                    preferences.set_mode(OverlayMode::Hud);
                }
            }
            if preferences.mode == OverlayMode::ScriptConsole {
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
                            ui_opacity,
                        );
                    },
                );
                if close_requested {
                    preferences.set_mode(OverlayMode::Hud);
                }
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

#[unsafe(no_mangle)]
pub extern "system" fn InitializeASI() {
    let _ = panic::catch_unwind(startup);
}

fn startup() {
    append_log("ASI entry point called; starting the DirectX 11 overlay hook.");
    let worker = thread::Builder::new()
        .name("madmax-overlay-hook".to_owned())
        .spawn(start_overlay);
    if let Err(error) = worker {
        append_log(&format!("Could not start overlay worker: {error}"));
    }
}

fn start_overlay() {
    let (game_endpoint, ui_endpoint) = crate::runtime::create_endpoints();
    game_probe::set_runtime_endpoint(game_endpoint);
    set_ui_endpoint(ui_endpoint);
    let result = panic::catch_unwind(|| {
        Hudhook::builder()
            .with::<ImguiDx11Hooks>(OverlayRenderDriver::new())
            .build()
            .apply()
    });
    match result {
        Ok(Ok(())) => {
            append_log("DirectX 11 render hook installed.");
            game_probe::start_update_callback_probe();
        }
        Ok(Err(error)) => append_log(&format!("DirectX 11 render hook failed: {error:?}")),
        Err(_) => append_log("Overlay setup panicked; no game operation was attempted."),
    }
}

#[cfg(test)]
mod scripts_tab_tests {
    use super::*;
    use crate::scripting::{ScriptGeneration, ScriptId, ScriptStatus};

    #[test]
    fn inline_toggle_submits_typed_inverse_without_console_or_prompt_changes() {
        let (mut sender, receiver) = crate::scripting::console::request_channel();
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
        let (mut sender, receiver) = crate::scripting::console::request_channel();
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
