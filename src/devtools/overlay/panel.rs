//! ImGui panel composition for input, scripts, console, and copied telemetry.
//!
//! Widgets read prepared UI state and submit typed UI requests. They do not
//! call the game adapter or execute Rhai code.
use super::*;
#[path = "panel/console_text.rs"]
pub(super) mod console_text;

pub(super) fn strip_console_ansi(text: &str) -> String {
    console_text::console_plain_text(text)
}

pub(super) fn draw_script_panels(
    runtime: &mut super::UiRuntime,
    ui: &imgui::Ui,
    preferences: &crate::devtools::state::UiPreferences,
) {
    let mut open_values = {
        let mut state = runtime
            .app
            .world_mut()
            .resource_mut::<crate::devtools::state::ScriptPanelsUiState>();
        std::mem::take(&mut state.open)
    };
    let mut changed_events = Vec::new();
    {
        let state = runtime
            .app
            .world()
            .resource::<crate::devtools::state::ScriptPanelsUiState>();
        for publication in &state.publications {
            for panel in &publication.panels {
                // Workbench package panels render inside their registered tab.
                if publication.workbench_tab.is_some()
                    || (publication.script_id.as_str() == "debug_spawn_menu"
                        && panel.id == "spawn_workbench")
                {
                    continue;
                }
                let key = (publication.script_id.clone(), panel.id.clone());
                let is_open = open_values.entry(key.clone()).or_insert(true);
                if !*is_open {
                    continue;
                }
                let title = format!(
                    "{}  [{}]###script_panel_{}_{}",
                    panel.title,
                    publication.script_id.as_str(),
                    publication.script_id.as_str(),
                    panel.id
                );
                let mut open = true;
                let background = ui.push_style_color(
                    imgui::StyleColor::WindowBg,
                    [0.055, 0.075, 0.105, preferences.ui_opacity],
                );
                let frame =
                    ui.push_style_color(imgui::StyleColor::FrameBg, [0.10, 0.15, 0.21, 1.0]);
                let button =
                    ui.push_style_color(imgui::StyleColor::Button, [0.15, 0.33, 0.48, 1.0]);
                let header =
                    ui.push_style_color(imgui::StyleColor::Header, [0.15, 0.33, 0.48, 1.0]);
                let title_style =
                    ui.push_style_color(imgui::StyleColor::TitleBg, [0.08, 0.20, 0.31, 1.0]);
                let title_active =
                    ui.push_style_color(imgui::StyleColor::TitleBgActive, [0.12, 0.30, 0.45, 1.0]);
                ui.window(&title)
                    .opened(&mut open)
                    .position(
                        [ui.io().display_size[0] - 430.0, 18.0],
                        imgui::Condition::FirstUseEver,
                    )
                    .size([410.0, 620.0], imgui::Condition::FirstUseEver)
                    .size_constraints([320.0, 220.0], [f32::MAX, f32::MAX])
                    .flags(
                        imgui::WindowFlags::NO_SAVED_SETTINGS
                            | imgui::WindowFlags::NO_COLLAPSE
                            | if preferences.captures_game_input() {
                                imgui::WindowFlags::empty()
                            } else {
                                imgui::WindowFlags::NO_INPUTS
                            },
                    )
                    .build(|| {
                        for widget in &panel.widgets {
                            if let Some(value) = draw_script_widget(ui, widget) {
                                changed_events.push(crate::api::scripts::ScriptPanelEvent {
                                    script_id: publication.script_id.clone(),
                                    generation: publication.generation,
                                    panel_id: panel.id.clone(),
                                    widget_id: widget.id.clone(),
                                    value,
                                });
                            }
                        }
                    });
                drop((title_active, title_style, header, button, frame, background));
                *is_open = open;
            }
        }
    }
    runtime
        .app
        .world_mut()
        .resource_mut::<crate::devtools::state::ScriptPanelsUiState>()
        .open = open_values;
    for event in changed_events {
        let _ = runtime.endpoint.script_panel_events_tx.try_send(event);
    }
}

pub(super) fn draw_script_widget(
    ui: &imgui::Ui,
    widget: &crate::api::scripts::ScriptPanelWidget,
) -> Option<crate::api::scripts::ScriptPanelValue> {
    use crate::api::scripts::{ScriptPanelValue as Value, ScriptPanelWidgetKind as Kind};
    match &widget.kind {
        Kind::Heading { text } => {
            section_header(ui, text);
            None
        }
        Kind::Label { text } => {
            ui.text(text);
            None
        }
        Kind::Status { text, level } => {
            ui.text_colored(level.color(), text);
            None
        }
        Kind::Checkbox { label: text, value } => {
            let mut value = *value;
            ui.checkbox(format!("{text}##{}", widget.id), &mut value)
                .then_some(Value::Bool(value))
        }
        Kind::Slider {
            label: text,
            value,
            min,
            max,
        } => {
            let mut value = *value as f32;
            let changed = ui
                .slider_config(format!("{text}##{}", widget.id), *min as f32, *max as f32)
                .display_format("%.3f")
                .build(&mut value);
            changed.then_some(Value::Number(f64::from(value)))
        }
        Kind::Number {
            label: text,
            value,
            min,
            max,
            step,
        } => {
            let mut value = (*value as f32).clamp(*min as f32, *max as f32);
            let changed = ui
                .input_float(format!("{text}##{}", widget.id), &mut value)
                .step(*step as f32)
                .build();
            if changed && value.is_finite() {
                Some(Value::Number(f64::from(
                    value.clamp(*min as f32, *max as f32),
                )))
            } else {
                None
            }
        }
        Kind::Select {
            label: text,
            value,
            options,
        } => {
            let mut selected = options
                .iter()
                .position(|option| option == value)
                .unwrap_or(0);
            let changed =
                ui.combo_simple_string(format!("{text}##{}", widget.id), &mut selected, options);
            changed.then(|| Value::Text(options[selected].clone()))
        }
        Kind::Button { label: text } => ui
            .button(format!("{text}##{}", widget.id))
            .then_some(Value::Bool(true)),
    }
}

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
    panels: &mut crate::devtools::state::ScriptPanelsUiState,
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
                state.active_tab = crate::devtools::state::DevToolsTab::Console;
                draw_console_tab(ui, state, output, sender);
            }
            if ui.tab_item("Scripts").is_some() {
                state.active_tab = crate::devtools::state::DevToolsTab::Scripts;
                if !state.initial_scan_started {
                    state.initial_scan_started = true;
                    request_discovery(state, discovery);
                }
                draw_scripts_tab(ui, state, output, sender, discovery, panels);
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
    let _window_padding = ui.push_style_var(imgui::StyleVar::WindowPadding([5.0, 5.0]));
    let _frame_padding = ui.push_style_var(imgui::StyleVar::FramePadding([3.0, 2.0]));
    let _item_spacing = ui.push_style_var(imgui::StyleVar::ItemSpacing([4.0, 3.0]));
    ui.child_window("console_tab").size([0.0, 0.0]).build(|| {
        ui.set_window_font_scale(0.90);
        draw_console_controls(ui, state, output);
        draw_console_output(ui, state);
        draw_console_prompt(ui, state, output, sender);
        ui.set_window_font_scale(1.0);
    });
    drop((_window_padding, _frame_padding, _item_spacing));
}

fn draw_console_controls(ui: &imgui::Ui, state: &mut ScriptConsoleUiState, output: &ConsoleOutput) {
    state.refresh_transcript(output);
    ui.checkbox("Follow output", &mut state.follow_output);
    ui.same_line();
    ui.text_disabled(format!("{} events", output.events().len(),));
}

fn draw_console_output(ui: &imgui::Ui, state: &mut ScriptConsoleUiState) {
    let height = (ui.content_region_avail()[1] - ui.frame_height_with_spacing() * 2.0).max(60.0);
    ui.child_window("console_output")
        .size([0.0, height])
        .horizontal_scrollbar(true)
        .border(true)
        .build(|| draw_console_transcript_view(ui, state));
}

fn draw_console_transcript_view(ui: &imgui::Ui, state: &mut ScriptConsoleUiState) {
    if state.restore_scroll {
        ui.set_scroll_y(state.scroll_y);
        state.restore_scroll = false;
    }
    if ui.is_window_hovered()
        && (ui.io().mouse_wheel > 0.0 || ui.is_mouse_down(imgui::MouseButton::Left))
    {
        state.follow_output = false;
    }
    let line_height = ui.text_line_height_with_spacing();
    let transcript_height = (state.transcript_text.lines().count().max(1) as f32 * line_height)
        .max(ui.text_line_height_with_spacing());
    ui.input_text_multiline(
        "##console_transcript_text",
        &mut state.transcript_text,
        [ui.content_region_avail()[0], transcript_height],
    )
    .read_only(true)
    .flags(imgui::InputTextFlags::ALLOW_TAB_INPUT)
    .build();
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
    panels: &mut crate::devtools::state::ScriptPanelsUiState,
) {
    // Keep tab chrome in the parent Dev Tools window while Scripts content
    // scrolls independently inside the available tab viewport.
    ui.child_window("scripts_tab")
        .size([0.0, 0.0])
        .border(false)
        .build(|| draw_scripts_tab_contents(ui, state, output, sender, discovery, panels));
}

pub(super) fn draw_scripts_tab_contents(
    ui: &imgui::Ui,
    state: &mut ScriptConsoleUiState,
    output: &mut ConsoleOutput,
    sender: &mut ConsoleRequestSender,
    discovery: &crate::api::scripts::loader::ScriptDiscoveryRequester,
    panels: &mut crate::devtools::state::ScriptPanelsUiState,
) {
    draw_script_panel_list(ui, panels);
    ui.separator();
    draw_package_controls(ui, state, discovery);
    draw_package_list(ui, state);
    draw_selected_package(ui, state, output, sender);
    draw_loaded_runtime(ui, state, output, sender);
}

fn draw_script_panel_list(
    ui: &imgui::Ui,
    panels: &mut crate::devtools::state::ScriptPanelsUiState,
) {
    ui.text("Script panels");
    let mut rows = Vec::new();
    for publication in &panels.publications {
        for panel in &publication.panels {
            rows.push((
                publication.script_id.clone(),
                panel.id.clone(),
                panel.title.clone(),
                panels
                    .open
                    .get(&(publication.script_id.clone(), panel.id.clone()))
                    .copied()
                    .unwrap_or(true),
            ));
        }
    }
    if rows.is_empty() {
        ui.text_disabled("No script panels published.");
    }
    for (script_id, panel_id, title, is_open) in rows {
        ui.text(format!("{title}  [{}]", script_id.as_str()));
        if !is_open {
            ui.same_line();
            if ui.small_button(format!("Open##{}_{}", script_id.as_str(), panel_id)) {
                panels.reopen(&script_id, &panel_id);
            }
        }
    }
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
            ui.text(format!(
                "Package v{} · API v{}",
                manifest.version, manifest.api_version
            ));
            match &manifest.dependency_state {
                crate::api::scripts::package::DependencyState::Ready => {
                    ui.text_colored([0.32, 0.82, 0.48, 1.0], "Dependencies: ready");
                }
                crate::api::scripts::package::DependencyState::OptionalUnavailable(message) => {
                    ui.text_colored([0.96, 0.74, 0.26, 1.0], format!("Dependencies: {message}"));
                }
                crate::api::scripts::package::DependencyState::Blocked(message) => {
                    ui.text_colored([0.94, 0.36, 0.34, 1.0], format!("Dependencies: {message}"));
                }
            }
            ui.text_wrapped(format!("Callbacks: {}", manifest.callbacks.join(", ")));
            ui.text_wrapped(format!(
                "Capabilities: {}",
                if manifest.capabilities.is_empty() {
                    "none".to_owned()
                } else {
                    manifest.capabilities.join(", ")
                }
            ));
            if !manifest.requires.is_empty() {
                ui.text_wrapped(format!("Requires: {}", manifest.requires.join(", ")));
            }
            if !manifest.conflicts.is_empty() {
                ui.text_wrapped(format!("Conflicts: {}", manifest.conflicts.join(", ")));
            }
            if let Some(tab) = &manifest.workbench_tab {
                ui.text_wrapped(format!("Workbench tab: {} ({})", tab.title, tab.id));
            }
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
            let dependencies_ready = row.manifest.as_ref().is_some_and(|manifest| {
                !matches!(
                    &manifest.dependency_state,
                    crate::api::scripts::package::DependencyState::Blocked(_)
                )
            });
            let valid = row.issue.is_none() && row.manifest.is_some() && dependencies_ready;
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
) -> Option<crate::devtools::state::ScriptConsoleStatusRow> {
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
    item: &crate::devtools::state::ScriptConsoleStatusRow,
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
    rows: &[crate::devtools::state::PresentationRow],
    section: DebugSection,
) {
    for row in rows.iter().filter(|row| row.section == section) {
        ui.text(format!("{}: {}", row.label, row.value));
    }
}
