//! ImGui panels, widgets, and presentation formatting for the developer overlay.
#[path = "panel/console_text.rs"]
mod console_text;
use super::*;
use console_text::{console_plain_text, draw_ansi_text};

pub(super) fn sample_input_key(ui: &imgui::Ui, key: imgui::Key) -> bool {
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

pub(super) fn draw_dev_tools(
    ui: &imgui::Ui,
    state: &mut ScriptConsoleUiState,
    output: &mut ConsoleOutput,
    sender: &mut ConsoleRequestSender,
    discovery: &crate::api::scripts::loader::ScriptDiscoveryRequester,
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
                state.active_tab = crate::devtools::model::DevToolsTab::Console;
                draw_console_tab(ui, state, output, sender);
            }
            if ui.tab_item("Scripts").is_some() {
                state.active_tab = crate::devtools::model::DevToolsTab::Scripts;
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

pub(super) fn draw_console_tab(
    ui: &imgui::Ui,
    state: &mut ScriptConsoleUiState,
    output: &mut ConsoleOutput,
    sender: &mut ConsoleRequestSender,
) {
    ui.child_window("console_tab").size([0.0, 0.0]).build(|| {
        draw_console_controls(ui, state, output);
        draw_console_output(ui, state, output);
        draw_console_prompt(ui, state, output, sender);
        ui.text_disabled(
            "Click a transcript row or press Ctrl+C to copy; Up/Down navigates history.",
        );
    });
}

fn draw_console_controls(ui: &imgui::Ui, state: &mut ScriptConsoleUiState, output: &ConsoleOutput) {
    ui.checkbox("Follow output", &mut state.follow_output);
    ui.same_line();
    ui.checkbox("Wrap long lines", &mut state.wrap_console_lines);
    ui.same_line();
    ui.text_disabled(format!(
        "{} / {} events",
        output.events().len(),
        crate::api::scripts::console::MAX_SCROLLBACK_ENTRIES
    ));
}

fn draw_console_output(ui: &imgui::Ui, state: &mut ScriptConsoleUiState, output: &ConsoleOutput) {
    let height = (ui.content_region_avail()[1] - ui.frame_height_with_spacing() * 2.0).max(60.0);
    ui.child_window("console_output")
        .size([0.0, height])
        .horizontal_scrollbar(!state.wrap_console_lines)
        .border(true)
        .build(|| draw_console_transcript_view(ui, state, output));
}

fn draw_console_transcript_view(
    ui: &imgui::Ui,
    state: &mut ScriptConsoleUiState,
    output: &ConsoleOutput,
) {
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
}

fn draw_console_prompt(
    ui: &imgui::Ui,
    state: &mut ScriptConsoleUiState,
    output: &mut ConsoleOutput,
    sender: &mut ConsoleRequestSender,
) {
    if state.focus_prompt {
        ui.set_keyboard_focus_here();
        state.focus_prompt = false;
    }
    ui.set_next_item_width(-70.0);
    let entered = draw_console_input(ui, state);
    ui.same_line();
    if ui.button("Send") || entered {
        submit_prompt(state, output, sender);
    }
}

fn draw_console_input(ui: &imgui::Ui, state: &mut ScriptConsoleUiState) -> bool {
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
    limit_console_input(state);
    entered
}

fn limit_console_input(state: &mut ScriptConsoleUiState) {
    let limit = crate::api::scripts::console::MAX_COMMAND_BYTES;
    state.input_over_limit |= state.input.len() > limit;
    let mut end = state.input.len().min(limit);
    while !state.input.is_char_boundary(end) {
        end -= 1;
    }
    state.input.truncate(end);
}

fn submit_prompt(
    state: &mut ScriptConsoleUiState,
    output: &mut ConsoleOutput,
    sender: &mut ConsoleRequestSender,
) {
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

pub(super) fn request_discovery(
    state: &mut ScriptConsoleUiState,
    discovery: &crate::api::scripts::loader::ScriptDiscoveryRequester,
) {
    if state.discovery_pending {
        return;
    }
    state.discovery_pending = true;
    state.discovery_status = "Scanning package manifests…".into();
    state.pending_discovery_request = None;
    if let Err(error) = discovery
        .try_discover(crate::api::scripts::loader::DiscoverScriptsRequest { request_id: None })
    {
        state.discovery_pending = false;
        state.discovery_status = format!("Could not queue package scan: {error}.");
    }
}

pub(super) fn draw_scripts_tab(
    ui: &imgui::Ui,
    state: &mut ScriptConsoleUiState,
    output: &mut ConsoleOutput,
    sender: &mut ConsoleRequestSender,
    discovery: &crate::api::scripts::loader::ScriptDiscoveryRequester,
) {
    // Keep tab chrome in the parent Dev Tools window while Scripts content
    // scrolls independently inside the available tab viewport.
    ui.child_window("scripts_tab")
        .size([0.0, 0.0])
        .border(false)
        .build(|| draw_scripts_tab_contents(ui, state, output, sender, discovery));
}

pub(super) fn draw_scripts_tab_contents(
    ui: &imgui::Ui,
    state: &mut ScriptConsoleUiState,
    output: &mut ConsoleOutput,
    sender: &mut ConsoleRequestSender,
    discovery: &crate::api::scripts::loader::ScriptDiscoveryRequester,
) {
    draw_package_controls(ui, state, discovery);
    draw_package_list(ui, state);
    draw_selected_package(ui, state, output, sender);
    draw_loaded_runtime(ui, state, output, sender);
}

fn draw_package_controls(
    ui: &imgui::Ui,
    state: &mut ScriptConsoleUiState,
    discovery: &crate::api::scripts::loader::ScriptDiscoveryRequester,
) {
    if ui.button("Rescan") {
        request_discovery(state, discovery);
    }
    if !state.discovery_status.is_empty() {
        ui.same_line();
        ui.text_disabled(&state.discovery_status);
    }
}

fn draw_package_list(ui: &imgui::Ui, state: &mut ScriptConsoleUiState) {
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
}

fn draw_selected_package(
    ui: &imgui::Ui,
    state: &mut ScriptConsoleUiState,
    output: &mut ConsoleOutput,
    sender: &mut ConsoleRequestSender,
) {
    if let Some(selected_name) = state.selected_package.clone()
        && let Some(row) = state
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
            if valid && loaded.is_none() && ui.button("Load") {
                issue_lifecycle(state, output, sender, "load", id);
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
            if loaded.is_some_and(|(status, _)| status == crate::api::scripts::ScriptStatus::Active)
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

fn draw_loaded_runtime(
    ui: &imgui::Ui,
    state: &mut ScriptConsoleUiState,
    output: &mut ConsoleOutput,
    sender: &mut ConsoleRequestSender,
) {
    if !state.loaded_scripts.is_empty() {
        section_header(ui, "Loaded runtime");
        let clicked_script = draw_loaded_rows(ui, state, output, sender);
        if let Some(clicked_script) = clicked_script {
            state.selected_script = Some(clicked_script);
        }
        draw_selected_unload(ui, state, output, sender);
    }
}

fn draw_loaded_rows(
    ui: &imgui::Ui,
    state: &ScriptConsoleUiState,
    output: &mut ConsoleOutput,
    sender: &mut ConsoleRequestSender,
) -> Option<crate::devtools::model::ScriptConsoleStatusRow> {
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
    clicked_script
}

fn draw_selected_unload(
    ui: &imgui::Ui,
    state: &mut ScriptConsoleUiState,
    output: &mut ConsoleOutput,
    sender: &mut ConsoleRequestSender,
) {
    let Some(selected) = state.selected_script.clone() else {
        return;
    };
    if !state
        .loaded_scripts
        .iter()
        .any(|item| item.id == selected.id)
    {
        return;
    }
    ui.same_line();
    if ui.button(format!("Unload {}", selected.id.as_str())) {
        issue_lifecycle(state, output, sender, "unload", &selected.id);
    }
}

pub(super) fn issue_auto_reload(
    output: &mut ConsoleOutput,
    sender: &mut ConsoleRequestSender,
    item: &crate::devtools::model::ScriptConsoleStatusRow,
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

pub(super) fn issue_lifecycle(
    state: &mut ScriptConsoleUiState,
    output: &mut ConsoleOutput,
    sender: &mut ConsoleRequestSender,
    action: &str,
    id: &crate::api::scripts::ScriptId,
) {
    let command = format!("script {action} {}", id.as_str());
    submit_console_input(state, output, sender, &command);
}

pub(super) fn submit_console_input(
    state: &mut ScriptConsoleUiState,
    output: &mut ConsoleOutput,
    sender: &mut ConsoleRequestSender,
    input: &str,
) -> bool {
    if input.trim() == "clear" {
        output.clear();
        return true;
    }
    let is_script_list = if let Ok(command) = ConsoleCommand::parse(input) {
        if handle_local_console_command(sender, output, input, command.clone()) {
            return true;
        }
        command == ConsoleCommand::ScriptList
    } else {
        false
    };
    let Some(request_id) = output.submit_input(sender, input) else {
        return false;
    };
    if is_script_list {
        state.discovery_pending = true;
        state.discovery_status = "Scanning package manifests…".into();
        state.pending_discovery_request = Some(request_id);
    }
    true
}

fn handle_local_console_command(
    sender: &mut ConsoleRequestSender,
    output: &mut ConsoleOutput,
    input: &str,
    command: ConsoleCommand,
) -> bool {
    match command {
        ConsoleCommand::ShowKeys => show_input_keys(output, input),
        ConsoleCommand::ShowBinds => show_input_binds(sender, output, input),
        ConsoleCommand::Bind { key, command } => {
            bind_input_key(sender, output, input, key, command)
        }
        ConsoleCommand::Unbind { key } => unbind_input_key(sender, output, input, key),
        _ => false,
    }
}

fn show_input_keys(output: &mut ConsoleOutput, input: &str) -> bool {
    record_local_command(output, input);
    let names = imgui::Key::VARIANTS
        .into_iter()
        .filter_map(crate::devtools::input::from_imgui)
        .map(|key| key.name())
        .collect::<Vec<_>>()
        .join("  ");
    output.record(ConsoleEvent::new(None, ConsoleSeverity::Info, &names));
    true
}

fn show_input_binds(
    sender: &ConsoleRequestSender,
    output: &mut ConsoleOutput,
    input: &str,
) -> bool {
    record_local_command(output, input);
    if sender.bindings().is_empty() {
        output.record(ConsoleEvent::new(
            None,
            ConsoleSeverity::Info,
            "no key bindings",
        ));
        return true;
    }
    for (key, command) in sender.bindings() {
        output.record(ConsoleEvent::new(
            None,
            ConsoleSeverity::Info,
            &format!("{} -> {command}", input_key_name(*key)),
        ));
    }
    true
}

fn bind_input_key(
    sender: &mut ConsoleRequestSender,
    output: &mut ConsoleOutput,
    input: &str,
    key: u32,
    command: String,
) -> bool {
    record_local_command(output, input);
    sender.bind(key, command.clone());
    output.record(ConsoleEvent::new(
        None,
        ConsoleSeverity::Info,
        &format!("bound {} to {command}", input_key_name(key)),
    ));
    true
}

fn unbind_input_key(
    sender: &mut ConsoleRequestSender,
    output: &mut ConsoleOutput,
    input: &str,
    key: u32,
) -> bool {
    record_local_command(output, input);
    let removed = sender.unbind(key);
    let (severity, message) = if removed {
        (
            ConsoleSeverity::Info,
            format!("unbound {}", input_key_name(key)),
        )
    } else {
        (ConsoleSeverity::Warning, "key was not bound".to_owned())
    };
    output.record(ConsoleEvent::new(None, severity, &message));
    true
}

pub(super) fn record_local_command(output: &mut ConsoleOutput, input: &str) {
    output.record(ConsoleEvent::new(
        None,
        ConsoleSeverity::Info,
        &format!("> {input}"),
    ));
}

pub(super) fn input_key_name(key: u32) -> &'static str {
    crate::api::ecs::input_key::InputKey::from_id(key)
        .map(|key| key.name())
        .unwrap_or("unknown")
}

pub(super) fn fire_input_binds(runtime: &mut UiRuntime, ui: &imgui::Ui, text_input_active: bool) {
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

pub(super) fn bound_key_just_pressed(ui: &imgui::Ui, key: u32) -> bool {
    let Some(key) =
        crate::api::ecs::input_key::InputKey::from_id(key).map(crate::devtools::input::to_imgui)
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

pub(super) fn draw_console_event(
    ui: &imgui::Ui,
    event: &crate::api::scripts::console::ConsoleEvent,
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

pub(super) fn draw_console_category_header(
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

pub(super) fn draw_script_event(
    ui: &imgui::Ui,
    event: &crate::api::scripts::console::ConsoleEvent,
    last_script_source: &mut Option<crate::api::scripts::ScriptId>,
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

pub(super) fn draw_console_transcript(
    ui: &imgui::Ui,
    events: &VecDeque<crate::api::scripts::console::ConsoleEvent>,
    selected_text: &mut Option<String>,
    wrap_lines: bool,
) {
    let mut view = TranscriptView::default();
    let mut index = 0;
    while index < events.len() {
        let event = &events[index];
        if let Some(command) = event.text.strip_prefix("> ") {
            index = draw_command_group(
                ui,
                events,
                index,
                command,
                selected_text,
                wrap_lines,
                &mut view,
            );
        } else {
            draw_unattached_event(ui, event, selected_text, wrap_lines, &mut view);
        }
        index += 1;
    }
}

#[derive(Default)]
struct TranscriptView {
    rendered_requests: std::collections::HashSet<u64>,
    stripe: bool,
    last_script_source: Option<crate::api::scripts::ScriptId>,
}

fn draw_command_group(
    ui: &imgui::Ui,
    events: &VecDeque<crate::api::scripts::console::ConsoleEvent>,
    index: usize,
    command: &str,
    selected_text: &mut Option<String>,
    wrap_lines: bool,
    view: &mut TranscriptView,
) -> usize {
    view.last_script_source = None;
    draw_console_category_header(
        ui,
        &format!("CMD: {command}"),
        &mut view.stripe,
        selected_text,
    );
    ui.indent();
    let next_index = if let Some(request_id) = events[index].request_id {
        view.rendered_requests.insert(request_id);
        draw_request_responses(ui, events, request_id, selected_text, wrap_lines, view);
        index
    } else {
        draw_adjacent_responses(ui, events, index, selected_text, wrap_lines, view)
    };
    ui.unindent();
    ui.spacing();
    next_index
}

fn draw_request_responses(
    ui: &imgui::Ui,
    events: &VecDeque<crate::api::scripts::console::ConsoleEvent>,
    request_id: u64,
    selected_text: &mut Option<String>,
    wrap_lines: bool,
    view: &mut TranscriptView,
) {
    for response in events.iter().filter(|response| {
        response.request_id == Some(request_id) && !response.text.starts_with("> ")
    }) {
        draw_response(ui, response, selected_text, wrap_lines, view);
    }
}

fn draw_adjacent_responses(
    ui: &imgui::Ui,
    events: &VecDeque<crate::api::scripts::console::ConsoleEvent>,
    index: usize,
    selected_text: &mut Option<String>,
    wrap_lines: bool,
    view: &mut TranscriptView,
) -> usize {
    let mut response_index = index + 1;
    while response_index < events.len() {
        let response = &events[response_index];
        if response.text.starts_with("> ") || response.request_id.is_some() {
            break;
        }
        draw_response(ui, response, selected_text, wrap_lines, view);
        response_index += 1;
    }
    response_index.saturating_sub(1)
}

fn draw_response(
    ui: &imgui::Ui,
    event: &crate::api::scripts::console::ConsoleEvent,
    selected_text: &mut Option<String>,
    wrap_lines: bool,
    view: &mut TranscriptView,
) {
    if !draw_script_event(
        ui,
        event,
        &mut view.last_script_source,
        &mut view.stripe,
        selected_text,
        wrap_lines,
    ) {
        draw_console_event(ui, event, selected_text, wrap_lines);
    }
}

fn draw_unattached_event(
    ui: &imgui::Ui,
    event: &crate::api::scripts::console::ConsoleEvent,
    selected_text: &mut Option<String>,
    wrap_lines: bool,
    view: &mut TranscriptView,
) {
    if event
        .request_id
        .is_some_and(|id| view.rendered_requests.contains(&id))
    {
        return;
    }
    if !draw_script_event(
        ui,
        event,
        &mut view.last_script_source,
        &mut view.stripe,
        selected_text,
        wrap_lines,
    ) {
        ui.indent();
        draw_console_event(ui, event, selected_text, wrap_lines);
        ui.unindent();
    }
}

pub(super) fn section_header(ui: &imgui::Ui, title: &str) {
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

pub(super) fn draw_rows(
    ui: &imgui::Ui,
    rows: &[crate::devtools::model::PresentationRow],
    section: DebugSection,
) {
    for row in rows.iter().filter(|row| row.section == section) {
        ui.text(format!("{}: {}", row.label, row.value));
    }
}
