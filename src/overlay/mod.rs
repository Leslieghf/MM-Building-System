mod config;
use config::*;

use crate::game_probe;
use crate::logging::append_log;
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

struct BootstrapNotice;

struct UiRuntime {
    app: App,
    endpoint: UiEndpoint,
}

static UI_ENDPOINT: OnceLock<Mutex<Option<UiEndpoint>>> = OnceLock::new();
static CLIPBOARD_BACKEND_INITIALIZED: AtomicBool = AtomicBool::new(false);

#[link(name = "user32")]
unsafe extern "system" {
    fn OpenClipboard(owner: *mut std::ffi::c_void) -> i32;
    fn CloseClipboard() -> i32;
    fn EmptyClipboard() -> i32;
    fn GetClipboardData(format: u32) -> *mut std::ffi::c_void;
    fn SetClipboardData(format: u32, memory: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GlobalAlloc(flags: u32, bytes: usize) -> *mut std::ffi::c_void;
    fn GlobalFree(memory: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    fn GlobalLock(memory: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    fn GlobalUnlock(memory: *mut std::ffi::c_void) -> i32;
    fn GlobalSize(memory: *mut std::ffi::c_void) -> usize;
}

struct Win32ClipboardBackend;

impl imgui::ClipboardBackend for Win32ClipboardBackend {
    fn get(&mut self) -> Option<String> {
        unsafe {
            if OpenClipboard(std::ptr::null_mut()) == 0 {
                return None;
            }
            let memory = GetClipboardData(CF_UNICODETEXT);
            if memory.is_null() {
                CloseClipboard();
                return None;
            }
            let bytes = GlobalSize(memory);
            let locked = GlobalLock(memory);
            if locked.is_null() || bytes < std::mem::size_of::<u16>() {
                if !locked.is_null() {
                    GlobalUnlock(memory);
                }
                CloseClipboard();
                return None;
            }
            let unit_count = (bytes / std::mem::size_of::<u16>()).min(MAX_CLIPBOARD_UTF16_UNITS);
            let units = std::slice::from_raw_parts(locked.cast::<u16>(), unit_count);
            let length = units
                .iter()
                .position(|unit| *unit == 0)
                .unwrap_or(unit_count);
            let text = String::from_utf16_lossy(&units[..length]);
            GlobalUnlock(memory);
            CloseClipboard();
            Some(text)
        }
    }

    fn set(&mut self, value: &str) {
        let mut wide: Vec<u16> = value
            .encode_utf16()
            .take(MAX_CLIPBOARD_UTF16_UNITS - 1)
            .collect();
        wide.push(0);
        unsafe {
            if OpenClipboard(std::ptr::null_mut()) == 0 {
                return;
            }
            if EmptyClipboard() == 0 {
                CloseClipboard();
                return;
            }
            let memory = GlobalAlloc(
                GMEM_MOVEABLE,
                wide.len().saturating_mul(std::mem::size_of::<u16>()),
            );
            if memory.is_null() {
                CloseClipboard();
                return;
            }
            let locked = GlobalLock(memory);
            if locked.is_null() {
                GlobalFree(memory);
                CloseClipboard();
                return;
            }
            std::ptr::copy_nonoverlapping(wide.as_ptr(), locked.cast::<u16>(), wide.len());
            GlobalUnlock(memory);
            if SetClipboardData(CF_UNICODETEXT, memory).is_null() {
                GlobalFree(memory);
            }
            CloseClipboard();
        }
    }
}

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

impl BootstrapNotice {
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

impl ImguiRenderLoop for BootstrapNotice {
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

fn sample_input_key(ui: &imgui::Ui, key: imgui::Key) -> bool {
    match key {
        imgui::Key::MouseLeft => ui.is_mouse_down(imgui::MouseButton::Left),
        imgui::Key::MouseRight => ui.is_mouse_down(imgui::MouseButton::Right),
        imgui::Key::MouseMiddle => ui.is_mouse_down(imgui::MouseButton::Middle),
        imgui::Key::MouseX1 => ui.is_mouse_down(imgui::MouseButton::Extra1),
        imgui::Key::MouseX2 => ui.is_mouse_down(imgui::MouseButton::Extra2),
        _ => ui.is_key_down(key),
    }
}

struct ConsoleInputCallbacks<'a> {
    history: &'a VecDeque<String>,
    cursor: &'a mut Option<usize>,
    draft: &'a mut String,
    input_over_limit: &'a mut bool,
}

impl imgui::InputTextCallbackHandler for ConsoleInputCallbacks<'_> {
    fn on_history(
        &mut self,
        direction: imgui::HistoryDirection,
        mut data: imgui::TextCallbackData,
    ) {
        if self.history.is_empty() {
            return;
        }
        if direction == imgui::HistoryDirection::Down && self.cursor.is_none() {
            return;
        }
        *self.cursor = match direction {
            imgui::HistoryDirection::Up => Some(match *self.cursor {
                Some(index) => index.saturating_sub(1),
                None => {
                    *self.draft = data.str().to_owned();
                    self.history.len() - 1
                }
            }),
            imgui::HistoryDirection::Down => self
                .cursor
                .and_then(|index| (index + 1 < self.history.len()).then_some(index + 1)),
        };
        let replacement = self
            .cursor
            .map_or(self.draft.as_str(), |index| self.history[index].as_str());
        data.clear();
        data.push_str(replacement);
        data.set_cursor_pos(replacement.len());
        data.clear_selection();
    }

    fn on_edit(&mut self, _: imgui::TextCallbackData) {
        *self.cursor = None;
        self.draft.clear();
        *self.input_over_limit = false;
    }
}

fn draw_dev_tools(
    ui: &imgui::Ui,
    state: &mut ScriptConsoleUiState,
    output: &mut ConsoleOutput,
    sender: &mut ConsoleRequestSender,
    discovery: &crate::scripting::loader::ScriptDiscoveryRequester,
    capture_input: bool,
    ui_opacity: f32,
) -> bool {
    let _input_disabled = ui.begin_disabled(!capture_input);
    let background = ui.push_style_color(
        imgui::StyleColor::WindowBg,
        [0.055, 0.075, 0.105, ui_opacity],
    );
    let frame = ui.push_style_color(imgui::StyleColor::FrameBg, [0.10, 0.15, 0.21, 1.0]);
    let button = ui.push_style_color(imgui::StyleColor::Button, [0.15, 0.33, 0.48, 1.0]);
    let header = ui.push_style_color(imgui::StyleColor::Header, [0.15, 0.33, 0.48, 1.0]);
    let mut open = true;
    ui.window("Mad Max Dev Tools")
        .opened(&mut open)
        .position([24.0, 24.0], imgui::Condition::FirstUseEver)
        .size([740.0, 560.0], imgui::Condition::FirstUseEver)
        .size_constraints([500.0, 420.0], [f32::MAX, f32::MAX])
        .focused(state.focus_prompt && capture_input)
        .flags(
            imgui::WindowFlags::NO_SAVED_SETTINGS
                | imgui::WindowFlags::NO_COLLAPSE
                | if capture_input {
                    imgui::WindowFlags::empty()
                } else {
                    imgui::WindowFlags::NO_INPUTS
                },
        )
        .build(|| {
            let _tabs = ui.tab_bar("dev_tools_tabs");
            if ui.tab_item("Console").is_some() {
                state.active_tab = crate::runtime::DevToolsTab::Console;
                draw_console_tab(ui, state, output, sender);
            }
            if ui.tab_item("Scripts").is_some() {
                state.active_tab = crate::runtime::DevToolsTab::Scripts;
                if !state.initial_scan_started {
                    state.initial_scan_started = true;
                    request_discovery(state, discovery);
                }
                draw_scripts_tab(ui, state, output, sender, discovery);
            }
        });
    drop((header, button, frame, background));
    !open
}

fn draw_console_tab(
    ui: &imgui::Ui,
    state: &mut ScriptConsoleUiState,
    output: &mut ConsoleOutput,
    sender: &mut ConsoleRequestSender,
) {
    ui.child_window("console_tab").size([0.0, 0.0]).build(|| {
        ui.checkbox("Follow output", &mut state.follow_output);
        ui.same_line();
        ui.checkbox("Wrap long lines", &mut state.wrap_console_lines);
        ui.same_line();
        ui.text_disabled(format!(
            "{} / {} events",
            output.events().len(),
            crate::scripting::console::MAX_SCROLLBACK_ENTRIES
        ));
        let output_height =
            (ui.content_region_avail()[1] - ui.frame_height_with_spacing() * 2.0).max(60.0);
        ui.child_window("console_output")
            .size([0.0, output_height])
            .horizontal_scrollbar(!state.wrap_console_lines)
            .border(true)
            .build(|| {
                let _wrap = state.wrap_console_lines.then(|| ui.push_text_wrap_pos());
                if state.restore_scroll {
                    ui.set_scroll_y(state.scroll_y);
                    state.restore_scroll = false;
                }
                if ui.is_window_hovered()
                    && (ui.io().mouse_wheel > 0.0 || ui.is_mouse_down(imgui::MouseButton::Left))
                {
                    state.follow_output = false;
                }
                draw_console_transcript(
                    ui,
                    output.events(),
                    &mut state.selected_console_text,
                    state.wrap_console_lines,
                );
                if ui.is_window_focused()
                    && ui.io().key_ctrl
                    && ui.is_key_pressed_no_repeat(imgui::Key::C)
                    && let Some(selected) = state.selected_console_text.as_deref()
                {
                    ui.set_clipboard_text(selected);
                }
                if state.follow_output {
                    ui.set_scroll_here_y_with_ratio(1.0);
                }
                state.scroll_y = ui.scroll_y();
            });
        if state.focus_prompt {
            ui.set_keyboard_focus_here();
            state.focus_prompt = false;
        }
        ui.set_next_item_width(-70.0);
        let entered = ui
            .input_text("##console_command", &mut state.input)
            .hint("help | clear | script load <id>")
            .flags(imgui::InputTextFlags::ENTER_RETURNS_TRUE)
            .callback(
                imgui::InputTextCallback::HISTORY | imgui::InputTextCallback::EDIT,
                ConsoleInputCallbacks {
                    history: &state.history,
                    cursor: &mut state.history_cursor,
                    draft: &mut state.history_draft,
                    input_over_limit: &mut state.input_over_limit,
                },
            )
            .build();
        let mut end = state
            .input
            .len()
            .min(crate::scripting::console::MAX_COMMAND_BYTES);
        if state.input.len() > crate::scripting::console::MAX_COMMAND_BYTES {
            state.input_over_limit = true;
        }
        while !state.input.is_char_boundary(end) {
            end -= 1;
        }
        state.input.truncate(end);
        ui.same_line();
        if ui.button("Send") || entered {
            if state.input_over_limit {
                output.record(ConsoleEvent::new(
                    None,
                    ConsoleSeverity::Error,
                    "console input exceeds 256 UTF-8 bytes",
                ));
            } else {
                state.remember_input();
                let input = state.input.clone();
                submit_console_input(state, output, sender, &input);
            }
            state.input.clear();
            state.input_over_limit = false;
            state.focus_prompt = true;
        }
        ui.text_disabled("Click a transcript row, Ctrl+C copies it | prompt supports Ctrl+C/X/V | Up/Down: history");
    });
}

fn request_discovery(
    state: &mut ScriptConsoleUiState,
    discovery: &crate::scripting::loader::ScriptDiscoveryRequester,
) {
    if state.discovery_pending {
        return;
    }
    state.discovery_pending = true;
    state.discovery_status = "Scanning package manifests…".into();
    state.pending_discovery_request = None;
    if let Err(error) = discovery
        .try_discover(crate::scripting::loader::DiscoverScriptsRequest { request_id: None })
    {
        state.discovery_pending = false;
        state.discovery_status = format!("Could not queue package scan: {error}.");
    }
}

fn draw_scripts_tab(
    ui: &imgui::Ui,
    state: &mut ScriptConsoleUiState,
    output: &mut ConsoleOutput,
    sender: &mut ConsoleRequestSender,
    discovery: &crate::scripting::loader::ScriptDiscoveryRequester,
) {
    // Keep tab chrome in the parent Dev Tools window while Scripts content
    // scrolls independently inside the available tab viewport.
    ui.child_window("scripts_tab")
        .size([0.0, 0.0])
        .border(false)
        .build(|| draw_scripts_tab_contents(ui, state, output, sender, discovery));
}

fn draw_scripts_tab_contents(
    ui: &imgui::Ui,
    state: &mut ScriptConsoleUiState,
    output: &mut ConsoleOutput,
    sender: &mut ConsoleRequestSender,
    discovery: &crate::scripting::loader::ScriptDiscoveryRequester,
) {
    if ui.button("Rescan") {
        request_discovery(state, discovery);
    }
    if !state.discovery_status.is_empty() {
        ui.same_line();
        ui.text_disabled(&state.discovery_status);
    }
    let available_height = ui.content_region_avail()[1];
    ui.child_window("discovered_packages")
        .size([0.0, (available_height * 0.56).max(100.0)])
        .border(true)
        .build(|| {
            if state.discovered_scripts.is_empty() && !state.discovery_pending {
                ui.text_disabled("No packages found.");
            }
            for row in &state.discovered_scripts {
                let selected = state.selected_package.as_ref() == Some(&row.package_name);
                let status = row
                    .id
                    .as_ref()
                    .and_then(|id| state.loaded_scripts.iter().find(|loaded| &loaded.id == id));
                let label = format!(
                    "{}  |  {}",
                    row.package_name,
                    status.map_or_else(
                        || if row.issue.is_some() {
                            "invalid".to_owned()
                        } else {
                            "discovered".to_owned()
                        },
                        |loaded| format!("{:?} · gen {}", loaded.status, loaded.generation.0)
                    )
                );
                if ui.selectable_config(label).selected(selected).build() {
                    state.selected_package = Some(row.package_name.clone());
                }
            }
        });
    if let Some(selected_name) = state.selected_package.clone() {
        if let Some(row) = state
            .discovered_scripts
            .iter()
            .find(|row| row.package_name == selected_name)
            .cloned()
        {
            ui.separator();
            if let Some(manifest) = &row.manifest {
                ui.text(format!("API v{}", manifest.api_version));
                ui.text_wrapped(format!("Callbacks: {}", manifest.callbacks.join(", ")));
                ui.text_wrapped(format!(
                    "Capabilities: {}",
                    if manifest.capabilities.is_empty() {
                        "none".to_owned()
                    } else {
                        manifest.capabilities.join(", ")
                    }
                ));
            }
            if let Some(issue) = &row.issue {
                ui.text_colored([1.0, 0.48, 0.45, 1.0], format!("Invalid: {issue}"));
            }
            if let Some(id) = &row.id {
                let loaded = state
                    .loaded_scripts
                    .iter()
                    .find(|item| &item.id == id)
                    .map(|item| (item.status, item.generation));
                let valid = row.issue.is_none() && row.manifest.is_some();
                ui.separator();
                if valid && loaded.is_none() {
                    if ui.button("Load") {
                        issue_lifecycle(state, output, sender, "load", id);
                    }
                }
                if valid && loaded.is_some() {
                    if ui.button("Reload") {
                        issue_lifecycle(state, output, sender, "reload", id);
                    }
                    ui.same_line();
                }
                let on_run = row.manifest.as_ref().is_some_and(|manifest| {
                    manifest
                        .callbacks
                        .iter()
                        .any(|callback| callback.starts_with("on_run="))
                });
                if loaded
                    .is_some_and(|(status, _)| status == crate::scripting::ScriptStatus::Active)
                    && on_run
                {
                    if ui.button("Run") {
                        issue_lifecycle(state, output, sender, "run", id);
                    }
                    ui.same_line();
                }
                if loaded.is_some() && ui.button("Unload") {
                    issue_lifecycle(state, output, sender, "unload", id);
                }
            }
        }
    }
    if !state.loaded_scripts.is_empty() {
        section_header(ui, "Loaded runtime");
        let mut clicked_script = None;
        ui.child_window("loaded_runtime")
            .size([0.0, ui.text_line_height_with_spacing() * 3.0])
            .border(true)
            .build(|| {
                for item in &state.loaded_scripts {
                    let selected = state
                        .selected_script
                        .as_ref()
                        .is_some_and(|selected| selected.id == item.id);
                    let _row_id = ui.push_id(item.id.as_str());
                    let label = format!(
                        "{} | {:?} | generation {}",
                        item.id.as_str(),
                        item.status,
                        item.generation.0
                    );
                    if ui
                        .selectable_config(&label)
                        .size([ui.calc_text_size(&label)[0], 0.0])
                        .selected(selected)
                        .build()
                    {
                        clicked_script = Some(item.clone());
                    }
                    ui.same_line();
                    let toggle = if item.auto_reload {
                        "Auto Reload: On"
                    } else {
                        "Auto Reload: Off"
                    };
                    if ui.small_button(toggle) {
                        issue_auto_reload(output, sender, item);
                    }
                    if let Some(error) = &item.reload_error {
                        let _color =
                            ui.push_style_color(imgui::StyleColor::Text, [1.0, 0.48, 0.45, 1.0]);
                        ui.text_wrapped(error);
                    }
                }
            });
        if let Some(clicked_script) = clicked_script {
            state.selected_script = Some(clicked_script);
        }
        if let Some(selected) = state.selected_script.clone() {
            if state
                .loaded_scripts
                .iter()
                .any(|item| item.id == selected.id)
            {
                ui.same_line();
                if ui.button(format!("Unload {}", selected.id.as_str())) {
                    issue_lifecycle(state, output, sender, "unload", &selected.id);
                }
            }
        }
    }
}

fn issue_auto_reload(
    output: &mut ConsoleOutput,
    sender: &mut ConsoleRequestSender,
    item: &crate::runtime::ScriptConsoleStatusRow,
) {
    if let Err(mut event) = sender.submit(ConsoleCommand::ScriptAutoReload {
        id: item.id.clone(),
        enabled: !item.auto_reload,
    }) {
        // Rejection retains the last accepted state and reports through the same
        // status-event path as runtime reload failures, once in each UI surface.
        event.script = Some(item.id.clone());
        event.generation = Some(item.generation);
        event.status = Some(item.status);
        event.auto_reload = Some(item.auto_reload);
        event.reload_error = Some(event.text.clone());
        output.record(event);
    }
}

fn issue_lifecycle(
    state: &mut ScriptConsoleUiState,
    output: &mut ConsoleOutput,
    sender: &mut ConsoleRequestSender,
    action: &str,
    id: &crate::scripting::ScriptId,
) {
    let command = format!("script {action} {}", id.as_str());
    submit_console_input(state, output, sender, &command);
}

fn submit_console_input(
    state: &mut ScriptConsoleUiState,
    output: &mut ConsoleOutput,
    sender: &mut ConsoleRequestSender,
    input: &str,
) -> bool {
    if input.trim() == "clear" {
        output.clear();
        return true;
    }
    if let Ok(command) = ConsoleCommand::parse(input) {
        match command {
            ConsoleCommand::ShowKeys => {
                record_local_command(output, input);
                let names = imgui::Key::VARIANTS
                    .into_iter()
                    .filter_map(crate::sdk::core::input::key_name)
                    .collect::<Vec<_>>()
                    .join("  ");
                output.record(ConsoleEvent::new(None, ConsoleSeverity::Info, &names));
                return true;
            }
            ConsoleCommand::ShowBinds => {
                record_local_command(output, input);
                if sender.bindings().is_empty() {
                    output.record(ConsoleEvent::new(
                        None,
                        ConsoleSeverity::Info,
                        "no key bindings",
                    ));
                } else {
                    for (key, command) in sender.bindings() {
                        let key_name = imgui::Key::VARIANTS
                            .into_iter()
                            .find(|candidate| *candidate as u32 == *key)
                            .and_then(crate::sdk::core::input::key_name)
                            .unwrap_or("unknown");
                        output.record(ConsoleEvent::new(
                            None,
                            ConsoleSeverity::Info,
                            &format!("{key_name} -> {command}"),
                        ));
                    }
                }
                return true;
            }
            ConsoleCommand::Bind { key, command } => {
                record_local_command(output, input);
                sender.bind(key, command.clone());
                output.record(ConsoleEvent::new(
                    None,
                    ConsoleSeverity::Info,
                    &format!("bound {} to {command}", input_key_name(key)),
                ));
                return true;
            }
            ConsoleCommand::Unbind { key } => {
                record_local_command(output, input);
                let removed = sender.unbind(key);
                let message = if removed {
                    format!("unbound {}", input_key_name(key))
                } else {
                    "key was not bound".to_owned()
                };
                output.record(ConsoleEvent::new(
                    None,
                    if removed {
                        ConsoleSeverity::Info
                    } else {
                        ConsoleSeverity::Warning
                    },
                    &message,
                ));
                return true;
            }
            _ => {}
        }
    }
    let Some(request_id) = output.submit_input(sender, input) else {
        return false;
    };
    if ConsoleCommand::parse(input) == Ok(ConsoleCommand::ScriptList) {
        state.discovery_pending = true;
        state.discovery_status = "Scanning package manifests…".into();
        state.pending_discovery_request = Some(request_id);
    }
    true
}

fn record_local_command(output: &mut ConsoleOutput, input: &str) {
    output.record(ConsoleEvent::new(
        None,
        ConsoleSeverity::Info,
        &format!("> {input}"),
    ));
}

fn input_key_name(key: u32) -> &'static str {
    imgui::Key::VARIANTS
        .into_iter()
        .find(|candidate| *candidate as u32 == key)
        .and_then(crate::sdk::core::input::key_name)
        .unwrap_or("unknown")
}

fn fire_input_binds(runtime: &mut UiRuntime, ui: &imgui::Ui, text_input_active: bool) {
    if text_input_active {
        return;
    }
    let triggered: Vec<_> = runtime
        .endpoint
        .console_requests
        .bindings()
        .iter()
        .filter(|(key, _)| bound_key_just_pressed(ui, **key))
        .map(|(key, command)| (*key, command.clone()))
        .collect();
    let UiRuntime { app, endpoint } = runtime;
    for (_key, command) in triggered {
        app.world_mut().resource_scope(
            |world, mut state: bevy_ecs::prelude::Mut<ScriptConsoleUiState>| {
                let mut output = world.resource_mut::<ConsoleOutput>();
                submit_console_input(
                    &mut state,
                    &mut output,
                    &mut endpoint.console_requests,
                    &command,
                );
            },
        );
    }
}

fn bound_key_just_pressed(ui: &imgui::Ui, key: u32) -> bool {
    let Some(key) = imgui::Key::VARIANTS
        .into_iter()
        .find(|candidate| *candidate as u32 == key)
    else {
        return false;
    };
    match key {
        imgui::Key::MouseLeft => ui.is_mouse_clicked(imgui::MouseButton::Left),
        imgui::Key::MouseRight => ui.is_mouse_clicked(imgui::MouseButton::Right),
        imgui::Key::MouseMiddle => ui.is_mouse_clicked(imgui::MouseButton::Middle),
        imgui::Key::MouseX1 => ui.is_mouse_clicked(imgui::MouseButton::Extra1),
        imgui::Key::MouseX2 => ui.is_mouse_clicked(imgui::MouseButton::Extra2),
        _ => ui.is_key_pressed_no_repeat(key),
    }
}

fn draw_console_event(
    ui: &imgui::Ui,
    event: &crate::scripting::console::ConsoleEvent,
    selected_text: &mut Option<String>,
    wrap_lines: bool,
) {
    let color = match event.severity {
        ConsoleSeverity::Info => [0.78, 0.86, 0.93, 1.0],
        ConsoleSeverity::Warning => [0.98, 0.78, 0.36, 1.0],
        ConsoleSeverity::Error => [1.0, 0.48, 0.45, 1.0],
    };
    let text = console_plain_text(&event.text);
    let position = ui.cursor_screen_pos();
    let width = ui.content_region_avail()[0];
    let wrap_width = if wrap_lines { width } else { -1.0 };
    let text_size = ui.calc_text_size_with_opts(&text, false, wrap_width);
    let row_height = text_size[1].max(ui.text_line_height()) + 4.0;
    let _id = ui.push_id_ptr(event);
    let _invisible_text = ui.push_style_color(imgui::StyleColor::Text, [0.0, 0.0, 0.0, 0.0]);
    let clicked = ui
        .selectable_config("##console_output_row")
        .selected(selected_text.as_deref() == Some(text.as_str()))
        .span_all_columns(true)
        .size([width, row_height])
        .build();
    let next_row_y = ui.cursor_screen_pos()[1];
    drop(_invisible_text);
    if clicked {
        *selected_text = Some(text.clone());
    }
    ui.set_cursor_screen_pos(position);
    let _wrap = wrap_lines.then(|| ui.push_text_wrap_pos());
    draw_ansi_text(ui, &event.text, color);
    drop(_wrap);
    ui.set_cursor_screen_pos([position[0], next_row_y]);
}

fn draw_console_category_header(
    ui: &imgui::Ui,
    label: &str,
    stripe: &mut bool,
    selected_text: &mut Option<String>,
) {
    let (background, hovered) = if *stripe {
        ([0.13, 0.20, 0.28, 1.0], [0.17, 0.26, 0.36, 1.0])
    } else {
        ([0.10, 0.16, 0.23, 1.0], [0.14, 0.22, 0.31, 1.0])
    };
    let _header = ui.push_style_color(imgui::StyleColor::Header, background);
    let _hovered = ui.push_style_color(imgui::StyleColor::HeaderHovered, hovered);
    let _active = ui.push_style_color(imgui::StyleColor::HeaderActive, background);
    let clicked = ui
        .selectable_config(label)
        .selected(selected_text.as_deref() == Some(label))
        .span_all_columns(true)
        .size([ui.content_region_avail()[0], 0.0])
        .build();
    if clicked {
        *selected_text = Some(label.to_owned());
    }
    *stripe = !*stripe;
}

fn draw_script_event(
    ui: &imgui::Ui,
    event: &crate::scripting::console::ConsoleEvent,
    last_script_source: &mut Option<crate::scripting::ScriptId>,
    stripe: &mut bool,
    selected_text: &mut Option<String>,
    wrap_lines: bool,
) -> bool {
    let is_lifecycle_phase = matches!(
        event.text.as_str(),
        "Preparing"
            | "Prepared"
            | "Loaded"
            | "Reloaded"
            | "Unloaded"
            | "Queued"
            | "Cancelled"
            | "Superseded"
    );
    if let Some(script) = event.script.as_ref().filter(|_| !is_lifecycle_phase) {
        let is_same_source = last_script_source.as_ref() == Some(script);
        if !is_same_source {
            let label = format!("SCRIPT: {}", script.as_str());
            draw_console_category_header(ui, &label, stripe, selected_text);
        }
        ui.indent();
        draw_console_event(ui, event, selected_text, wrap_lines);
        ui.unindent();
        if !is_same_source {
            ui.spacing();
        }
        *last_script_source = Some(script.clone());
        return true;
    }
    *last_script_source = None;
    false
}

fn draw_console_transcript(
    ui: &imgui::Ui,
    events: &VecDeque<crate::scripting::console::ConsoleEvent>,
    selected_text: &mut Option<String>,
    wrap_lines: bool,
) {
    let mut rendered_requests = std::collections::HashSet::new();
    let mut stripe = false;
    let mut last_script_source = None;
    let mut index = 0;
    while index < events.len() {
        let event = &events[index];
        if let Some(command) = event.text.strip_prefix("> ") {
            last_script_source = None;
            let request_id = event.request_id;
            draw_console_category_header(
                ui,
                &format!("CMD: {command}"),
                &mut stripe,
                selected_text,
            );

            ui.indent();
            if let Some(request_id) = request_id {
                rendered_requests.insert(request_id);
                for response in events.iter().filter(|response| {
                    response.request_id == Some(request_id) && !response.text.starts_with("> ")
                }) {
                    if !draw_script_event(
                        ui,
                        response,
                        &mut last_script_source,
                        &mut stripe,
                        selected_text,
                        wrap_lines,
                    ) {
                        draw_console_event(ui, response, selected_text, wrap_lines);
                    }
                }
            } else {
                let mut response_index = index + 1;
                while response_index < events.len() {
                    let response = &events[response_index];
                    if response.text.starts_with("> ") || response.request_id.is_some() {
                        break;
                    }
                    if !draw_script_event(
                        ui,
                        response,
                        &mut last_script_source,
                        &mut stripe,
                        selected_text,
                        wrap_lines,
                    ) {
                        draw_console_event(ui, response, selected_text, wrap_lines);
                    }
                    response_index += 1;
                }
                index = response_index.saturating_sub(1);
            }
            ui.unindent();
            ui.spacing();
        } else if event
            .request_id
            .is_some_and(|request_id| rendered_requests.contains(&request_id))
        {
            // Async results are rendered under their command, even if other output
            // arrived between the prompt and the result.
        } else {
            if !draw_script_event(
                ui,
                event,
                &mut last_script_source,
                &mut stripe,
                selected_text,
                wrap_lines,
            ) {
                ui.indent();
                draw_console_event(ui, event, selected_text, wrap_lines);
                ui.unindent();
            }
        }
        index += 1;
    }
}

fn console_plain_text(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for control in chars.by_ref() {
                if ('@'..='~').contains(&control) {
                    break;
                }
            }
        } else if ch != '\u{1b}' {
            plain.push(ch);
        }
    }
    plain
}

#[derive(Clone, Copy, Default)]
struct AnsiStyle {
    color: Option<[f32; 3]>,
    bold: bool,
}

fn draw_ansi_text(ui: &imgui::Ui, text: &str, severity_color: [f32; 4]) {
    let mut style = AnsiStyle::default();
    let mut plain = String::new();
    let mut spans = 0;
    let mut has_drawn_span = false;
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\u{1b}' || chars.peek() != Some(&'[') {
            if ch != '\u{1b}' {
                plain.push(ch);
            }
            continue;
        }
        chars.next();
        let mut params = String::new();
        let mut valid = true;
        let mut terminated = false;
        for control in chars.by_ref().take(32) {
            if control == 'm' {
                terminated = true;
                break;
            }
            if !control.is_ascii_digit() && control != ';' {
                valid = false;
                break;
            }
            params.push(control);
        }
        if !valid || !terminated || spans >= crate::scripting::console::MAX_ANSI_SGR_SPANS {
            continue;
        }
        let previous = style;
        if apply_sgr(&params, &mut style) {
            if !plain.is_empty() {
                draw_ansi_span(ui, &plain, previous, severity_color, &mut has_drawn_span);
                plain.clear();
            }
            spans += 1;
        }
    }
    if !plain.is_empty() {
        draw_ansi_span(ui, &plain, style, severity_color, &mut has_drawn_span);
    }
}

fn apply_sgr(params: &str, style: &mut AnsiStyle) -> bool {
    let codes = if params.is_empty() {
        vec![0]
    } else {
        let Ok(codes) = params
            .split(';')
            .map(str::parse::<u8>)
            .collect::<Result<Vec<_>, _>>()
        else {
            return false;
        };
        codes
    };
    if codes
        .iter()
        .any(|code| !matches!(*code, 0 | 1 | 30..=37 | 90..=97))
    {
        return false;
    }
    for code in codes {
        match code {
            0 => *style = AnsiStyle::default(),
            1 => style.bold = true,
            30..=37 => style.color = Some(ansi_color(code - 30, false)),
            90..=97 => style.color = Some(ansi_color(code - 90, true)),
            _ => unreachable!(),
        }
    }
    true
}

fn ansi_color(index: u8, bright: bool) -> [f32; 3] {
    const NORMAL: [[f32; 3]; 8] = [
        [0.0, 0.0, 0.0],
        [0.8, 0.0, 0.0],
        [0.0, 0.65, 0.0],
        [0.8, 0.55, 0.0],
        [0.0, 0.0, 0.8],
        [0.7, 0.0, 0.7],
        [0.0, 0.65, 0.65],
        [0.8, 0.8, 0.8],
    ];
    const BRIGHT: [[f32; 3]; 8] = [
        [0.45, 0.45, 0.45],
        [1.0, 0.35, 0.35],
        [0.35, 1.0, 0.35],
        [1.0, 0.9, 0.35],
        [0.4, 0.55, 1.0],
        [1.0, 0.4, 1.0],
        [0.4, 1.0, 1.0],
        [1.0, 1.0, 1.0],
    ];
    if bright {
        BRIGHT[index as usize]
    } else {
        NORMAL[index as usize]
    }
}

fn draw_ansi_span(
    ui: &imgui::Ui,
    text: &str,
    style: AnsiStyle,
    severity: [f32; 4],
    has_drawn_span: &mut bool,
) {
    if *has_drawn_span {
        ui.same_line();
    }
    let mut color = style
        .color
        .unwrap_or([severity[0], severity[1], severity[2]]);
    if style.bold {
        for component in &mut color {
            *component = (*component * 1.2).min(1.0);
        }
    }
    let _color = ui.push_style_color(
        imgui::StyleColor::Text,
        [color[0], color[1], color[2], severity[3]],
    );
    ui.text_colored([color[0], color[1], color[2], severity[3]], text);
    *has_drawn_span = true;
}

#[cfg(test)]
mod console_text_tests {
    use super::console_plain_text;

    #[test]
    fn copied_console_text_does_not_include_ansi_color_sequences() {
        assert_eq!(
            console_plain_text("\u{1b}[1;92mSCRIPT: test\u{1b}[0m ready"),
            "SCRIPT: test ready"
        );
    }
}

fn section_header(ui: &imgui::Ui, title: &str) {
    let position = ui.cursor_screen_pos();
    let width = ui.content_region_avail()[0];
    let height = ui.text_line_height() + 8.0;
    let draw_list = ui.get_window_draw_list();
    let bar_color = imgui::ImColor32::from_rgba(38, 83, 123, 255);
    draw_list.add_rect_filled_multicolor(
        position,
        [position[0] + width, position[1] + height],
        bar_color,
        bar_color,
        bar_color,
        bar_color,
    );
    draw_list.add_text(
        [position[0] + 6.0, position[1] + 4.0],
        imgui::ImColor32::from_rgba(235, 243, 250, 255),
        title,
    );
    ui.dummy([width, height]);
}

fn draw_rows(ui: &imgui::Ui, rows: &[crate::runtime::PresentationRow], section: DebugSection) {
    for row in rows.iter().filter(|row| row.section == section) {
        ui.text(format!("{}: {}", row.label, row.value));
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
            .with::<ImguiDx11Hooks>(BootstrapNotice::new())
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
