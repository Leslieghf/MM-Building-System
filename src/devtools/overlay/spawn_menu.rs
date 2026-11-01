use super::UiRuntime;
use super::spawn_menu_catalog::CATALOG;
use crate::api::ecs::SpawnLifecycle;
use crate::devtools::state::{UiGameSnapshot, UiPreferences};
use hudhook::imgui;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

const STOCKPILE_CATALOG_ID: &str = "props.scrap_stockpile_small";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CatalogTileStatus {
    Stable,
    #[allow(dead_code)]
    NoSpawnPath,
    Experimental,
}

impl CatalogTileStatus {
    const fn label(self) -> &'static str {
        match self {
            Self::Stable => "STABLE",
            Self::NoSpawnPath => "NO SPAWN PATH",
            Self::Experimental => "EXPERIMENTAL",
        }
    }

    const fn rgb(self) -> [u8; 3] {
        match self {
            Self::Stable => [82, 209, 122],
            Self::NoSpawnPath => [240, 92, 86],
            Self::Experimental => [245, 189, 67],
        }
    }
}

pub(super) struct CatalogEntry {
    pub(super) catalog_id: &'static str,
    pub(super) name: &'static str,
    pub(super) category: &'static str,
    pub(super) section: &'static str,
    pub(super) asset_key: &'static str,
    pub(super) status: CatalogTileStatus,
    pub(super) spawnable: bool,
}

#[derive(Clone)]
struct VisibleWorkbenchTab {
    package_id: crate::api::scripts::ScriptId,
    metadata: crate::api::scripts::package::WorkbenchTabMetadata,
}

#[derive(Default)]
struct BrowserState {
    query: String,
    matched: Vec<&'static CatalogEntry>,
    cached_query: String,
    cached_tab: Option<BrowserTab>,
    selected_folder: Option<String>,
    cached_folder: Option<String>,
    selected_workbench_panel: Option<String>,
    side_tab: SpawnSideTab,
    tile_size: TileSize,
    placement_feedback: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SpawnSideTab {
    #[default]
    Tools,
    Settings,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum TileSize {
    #[default]
    Compact,
    Medium,
    Large,
}

impl TileSize {
    const ALL: [Self; 3] = [Self::Compact, Self::Medium, Self::Large];

    const fn label(self) -> &'static str {
        match self {
            Self::Compact => "Compact",
            Self::Medium => "Medium",
            Self::Large => "Large",
        }
    }

    const fn dimensions(self) -> (f32, f32, f32) {
        match self {
            Self::Compact => (112.0, 102.0, 36.0),
            Self::Medium => (150.0, 132.0, 54.0),
            Self::Large => (190.0, 168.0, 78.0),
        }
    }
}

struct FolderNode {
    path: String,
    display_label: String,
    depth: usize,
}

thread_local! {
    static BROWSER_STATE: RefCell<BrowserState> = RefCell::new(BrowserState::default());
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BrowserTab {
    Props,
    Vehicles,
    World,
    Other,
}

impl BrowserTab {
    const ALL: [Self; 4] = [Self::Props, Self::Vehicles, Self::World, Self::Other];

    const fn label(self) -> &'static str {
        match self {
            Self::Props => "Props",
            Self::Vehicles => "Vehicles",
            Self::World => "World",
            Self::Other => "Other",
        }
    }

    const fn section(self) -> &'static str {
        match self {
            Self::Props => "Props",
            Self::Vehicles => "Vehicles",
            Self::World => "World Compositions",
            Self::Other => "Discovered",
        }
    }
}

static ACTIVE_TAB: AtomicU8 = AtomicU8::new(0);
pub(super) static SPAWN_MENU_TOGGLE_LOCKED: AtomicBool = AtomicBool::new(false);
static CATALOG_COUNTS: OnceLock<[usize; 4]> = OnceLock::new();

pub(super) fn draw_spawn_menu(
    runtime: &mut UiRuntime,
    ui: &imgui::Ui,
    preferences: &UiPreferences,
) {
    let colors = [
        ui.push_style_color(
            imgui::StyleColor::WindowBg,
            [0.055, 0.075, 0.105, preferences.ui_opacity],
        ),
        ui.push_style_color(imgui::StyleColor::FrameBg, [0.10, 0.15, 0.21, 1.0]),
        ui.push_style_color(imgui::StyleColor::Button, [0.12, 0.25, 0.36, 1.0]),
        ui.push_style_color(imgui::StyleColor::ButtonHovered, [0.18, 0.36, 0.50, 1.0]),
        ui.push_style_color(imgui::StyleColor::ButtonActive, [0.22, 0.43, 0.58, 1.0]),
        ui.push_style_color(imgui::StyleColor::TitleBg, [0.08, 0.20, 0.31, 1.0]),
        ui.push_style_color(imgui::StyleColor::TitleBgActive, [0.12, 0.30, 0.45, 1.0]),
    ];

    let snapshot = *runtime.app.world().resource::<UiGameSnapshot>();
    let script_tabs = visible_workbench_tabs(runtime);
    let aim_available =
        snapshot.0.camera_position.is_some() && snapshot.0.camera_direction.is_some();
    let display = ui.io().display_size;
    let max_size = [
        (display[0] - 20.0).max(560.0),
        (display[1] - 20.0).max(420.0),
    ];
    let min_size = [max_size[0].min(760.0), max_size[1].min(500.0)];
    let initial_size = [
        (display[0] * 0.90).clamp(min_size[0], max_size[0]),
        (display[1] * 0.86).clamp(min_size[1], max_size[1]),
    ];
    let position = [
        ((display[0] - initial_size[0]) * 0.5).max(8.0),
        ((display[1] - initial_size[1]) * 0.5).max(8.0),
    ];
    let _window_padding = ui.push_style_var(imgui::StyleVar::WindowPadding([7.0, 7.0]));
    let _frame_padding = ui.push_style_var(imgui::StyleVar::FramePadding([4.0, 3.0]));
    let _item_spacing = ui.push_style_var(imgui::StyleVar::ItemSpacing([5.0, 4.0]));
    let mut allowed_target_interaction = false;
    ui.window("Spawn Menu###spawn_menu")
        .position(position, imgui::Condition::FirstUseEver)
        .size(initial_size, imgui::Condition::FirstUseEver)
        .size_constraints(min_size, max_size)
        .bg_alpha(preferences.ui_opacity)
        .flags(imgui::WindowFlags::NO_SAVED_SETTINGS | imgui::WindowFlags::NO_COLLAPSE)
        .build(|| {
            let selected_index = ACTIVE_TAB.load(Ordering::Acquire) as usize;
            let selected_index = if selected_index >= BrowserTab::ALL.len()
                && script_tabs
                    .get(selected_index.saturating_sub(BrowserTab::ALL.len()))
                    .is_none()
            {
                ACTIVE_TAB.store(0, Ordering::Release);
                0
            } else {
                selected_index
            };
            let active_asset_tab = BrowserTab::ALL.get(selected_index).copied();
            for (index, tab) in BrowserTab::ALL.into_iter().enumerate() {
                if index > 0 {
                    ui.same_line();
                }
                if tab_button(
                    ui,
                    tab,
                    active_asset_tab.unwrap_or(BrowserTab::Props),
                    section_count(tab),
                ) {
                    ACTIVE_TAB.store(index as u8, Ordering::Release);
                    BROWSER_STATE.with(|browser| {
                        let mut browser = browser.borrow_mut();
                        browser.selected_folder = None;
                        browser.cached_tab = None;
                    });
                }
                allowed_target_interaction |= ui.is_item_hovered() || ui.is_item_active();
            }
            for (index, tab) in script_tabs.iter().enumerate() {
                ui.same_line();
                let selected = selected_index == BrowserTab::ALL.len() + index;
                let style = selected.then(|| {
                    ui.push_style_color(imgui::StyleColor::Button, [0.20, 0.37, 0.50, 1.0])
                });
                let label = format!(
                    "{}##{}_{}",
                    tab.metadata.title, tab.package_id, tab.metadata.id
                );
                if ui.button_with_size(label, [105.0, 27.0]) {
                    ACTIVE_TAB.store((BrowserTab::ALL.len() + index) as u8, Ordering::Release);
                    BROWSER_STATE.with(|browser| {
                        let mut browser = browser.borrow_mut();
                        browser.selected_folder = None;
                        browser.selected_workbench_panel = None;
                        browser.cached_tab = None;
                    });
                }
                drop(style);
                allowed_target_interaction |= ui.is_item_hovered() || ui.is_item_active();
            }
            ui.separator();

            BROWSER_STATE.with(|browser| {
                let mut browser = browser.borrow_mut();
                let available = ui.content_region_avail();
                let pane_height = available[1].max(120.0);
                let gap = ui.clone_style().item_spacing[0];
                let left_width = (available[0] * 0.20).clamp(120.0, 250.0);
                let right_width = (available[0] * 0.29).clamp(190.0, 340.0);
                let center_width = (available[0] - left_width - right_width - gap * 2.0).max(120.0);

                ui.child_window("spawn_folder_tree")
                    .size([left_width, pane_height])
                    .border(true)
                    .build(|| {
                        if let Some(tab) = active_asset_tab {
                            draw_folder_tree(ui, tab, &mut browser);
                        } else if let Some(tab) =
                            script_tabs.get(selected_index.saturating_sub(BrowserTab::ALL.len()))
                        {
                            draw_workbench_outline(ui, runtime, tab, &mut browser);
                        } else {
                            draw_folder_tree(ui, BrowserTab::Props, &mut browser);
                        }
                    });
                ui.same_line();
                ui.child_window("spawn_catalog_browser")
                    .size([center_width, pane_height])
                    .border(true)
                    .build(|| {
                        if let Some(tab) = active_asset_tab {
                            allowed_target_interaction |=
                                draw_catalog_browser(runtime, ui, tab, &mut browser, aim_available);
                        } else if let Some(tab) =
                            script_tabs.get(selected_index.saturating_sub(BrowserTab::ALL.len()))
                        {
                            draw_workbench_content(runtime, ui, tab, &browser);
                            allowed_target_interaction |=
                                ui.is_any_item_hovered() || ui.is_any_item_active();
                        }
                    });
                ui.same_line();
                ui.child_window("spawn_tools_sidebar")
                    .size([right_width, pane_height])
                    .border(true)
                    .build(|| draw_spawn_sidebar(runtime, ui, &snapshot, &mut browser));
            });
        });

    let has_ui_interaction = ui.is_any_item_hovered() || ui.is_any_item_active();
    let toggle_locked =
        ui.io().want_text_input || (has_ui_interaction && !allowed_target_interaction);
    SPAWN_MENU_TOGGLE_LOCKED.store(toggle_locked, Ordering::Release);

    drop((colors, _window_padding, _frame_padding, _item_spacing));
}

fn visible_workbench_tabs(runtime: &UiRuntime) -> Vec<VisibleWorkbenchTab> {
    let mut tabs = runtime
        .app
        .world()
        .resource::<crate::devtools::state::ScriptPanelsUiState>()
        .publications
        .iter()
        .filter_map(|publication| {
            publication
                .workbench_tab
                .as_ref()
                .map(|metadata| VisibleWorkbenchTab {
                    package_id: publication.script_id.clone(),
                    metadata: metadata.clone(),
                })
        })
        .collect::<Vec<_>>();
    tabs.sort_by(|left, right| {
        (
            left.metadata.order,
            left.package_id.as_str(),
            left.metadata.id.as_str(),
        )
            .cmp(&(
                right.metadata.order,
                right.package_id.as_str(),
                right.metadata.id.as_str(),
            ))
    });
    tabs
}

fn draw_workbench_outline(
    ui: &imgui::Ui,
    runtime: &UiRuntime,
    tab: &VisibleWorkbenchTab,
    browser: &mut BrowserState,
) {
    ui.text_disabled("SECTIONS");
    ui.separator();
    let publications = &runtime
        .app
        .world()
        .resource::<crate::devtools::state::ScriptPanelsUiState>()
        .publications;
    let Some(publication) = publications.iter().find(|publication| {
        publication.script_id == tab.package_id
            && publication
                .workbench_tab
                .as_ref()
                .is_some_and(|metadata| metadata.id == tab.metadata.id)
    }) else {
        ui.text_disabled("Package is preparing its tab");
        return;
    };
    for panel in &publication.panels {
        let key = workbench_panel_key(tab, &panel.id);
        let selected = browser.selected_workbench_panel.as_deref() == Some(key.as_str());
        if ui
            .selectable_config(&panel.title)
            .selected(selected)
            .build()
        {
            browser.selected_workbench_panel = Some(key);
        }
    }
}

fn draw_workbench_content(
    runtime: &mut UiRuntime,
    ui: &imgui::Ui,
    tab: &VisibleWorkbenchTab,
    browser: &BrowserState,
) {
    let mut events = Vec::new();
    {
        let publications = &runtime
            .app
            .world()
            .resource::<crate::devtools::state::ScriptPanelsUiState>()
            .publications;
        let Some(publication) = publications.iter().find(|publication| {
            publication.script_id == tab.package_id
                && publication
                    .workbench_tab
                    .as_ref()
                    .is_some_and(|metadata| metadata.id == tab.metadata.id)
        }) else {
            ui.text_disabled("Package is preparing its tab");
            return;
        };
        ui.text_disabled(&tab.metadata.title);
        ui.separator();
        let selected_panel = publication.panels.iter().find(|panel| {
            browser.selected_workbench_panel.as_deref()
                == Some(workbench_panel_key(tab, &panel.id).as_str())
        });
        let panels = publication
            .panels
            .iter()
            .filter(|panel| selected_panel.is_none_or(|selected| selected.id == panel.id));
        for panel in panels {
            for widget in &panel.widgets {
                let _widget_id = ui.push_id(widget.id.as_str());
                if let Some(value) = super::draw_script_widget(ui, widget) {
                    events.push(crate::api::scripts::ScriptPanelEvent {
                        script_id: publication.script_id.clone(),
                        generation: publication.generation,
                        panel_id: panel.id.clone(),
                        widget_id: widget.id.clone(),
                        value,
                    });
                }
            }
        }
        if publication.panels.is_empty() {
            ui.text_disabled("Waiting for the package's first callback");
        }
    }
    for event in events {
        let _ = runtime.endpoint.script_panel_events_tx.try_send(event);
    }
}

fn workbench_panel_key(tab: &VisibleWorkbenchTab, panel_id: &str) -> String {
    format!("{}::{}::{}", tab.package_id, tab.metadata.id, panel_id)
}

fn section_count(tab: BrowserTab) -> usize {
    let counts = CATALOG_COUNTS.get_or_init(|| {
        let mut counts = [0; BrowserTab::ALL.len()];
        for entry in CATALOG {
            if let Some(index) = BrowserTab::ALL
                .iter()
                .position(|tab| entry.section == tab.section())
            {
                counts[index] += 1;
            }
        }
        counts
    });
    BrowserTab::ALL
        .iter()
        .position(|candidate| *candidate == tab)
        .map(|index| counts[index])
        .unwrap_or_default()
}

fn tab_button(ui: &imgui::Ui, tab: BrowserTab, selected: BrowserTab, count: usize) -> bool {
    let label = format!("{} ({count})", tab.label());
    let width = match tab {
        BrowserTab::Props => 84.0,
        BrowserTab::Vehicles => 100.0,
        BrowserTab::World => 110.0,
        BrowserTab::Other => 96.0,
    };
    let selected_style = if tab == selected {
        Some(ui.push_style_color(imgui::StyleColor::Button, [0.20, 0.37, 0.50, 1.0]))
    } else {
        None
    };
    let clicked = ui.button_with_size(label, [width, 27.0]);
    drop(selected_style);
    clicked
}

fn draw_folder_tree(ui: &imgui::Ui, tab: BrowserTab, browser: &mut BrowserState) {
    ui.text_disabled("FOLDERS");
    ui.separator();
    let total = section_count(tab);
    let all_label = format!("All items  ({total})");
    if ui
        .selectable_config(&all_label)
        .selected(browser.selected_folder.is_none())
        .build()
    {
        browser.selected_folder = None;
    }

    let folders = folder_index(tab);
    let style = ui.clone_style();
    let row_height = ui.text_line_height() + style.frame_padding[1] * 2.0 + style.item_spacing[1];
    let clipper = imgui::ListClipper::new(folders.len() as i32).items_height(row_height);
    for index in clipper.begin(ui).iter() {
        let folder = &folders[index as usize];
        for _ in 0..folder.depth {
            ui.indent();
        }
        if ui
            .selectable_config(&folder.display_label)
            .selected(browser.selected_folder.as_deref() == Some(folder.path.as_str()))
            .build()
        {
            browser.selected_folder = Some(folder.path.clone());
        }
        for _ in 0..folder.depth {
            ui.unindent();
        }
    }
}

fn folder_index(tab: BrowserTab) -> &'static [FolderNode] {
    static FOLDERS: OnceLock<[Vec<FolderNode>; 4]> = OnceLock::new();
    let index = BrowserTab::ALL
        .iter()
        .position(|candidate| *candidate == tab)
        .unwrap_or_default();
    &FOLDERS.get_or_init(|| {
        let mut folders: [Vec<FolderNode>; 4] = std::array::from_fn(|_| Vec::new());
        let mut counts = BTreeMap::<(&'static str, String), usize>::new();
        for entry in CATALOG {
            let parts = asset_directory_parts(entry);
            for depth in 1..=parts.len() {
                let path = parts[..depth].join("/");
                *counts.entry((entry.section, path)).or_default() += 1;
            }
        }
        for ((section, path), count) in counts {
            let Some(index) = BrowserTab::ALL
                .iter()
                .position(|tab| tab.section() == section)
            else {
                continue;
            };
            let label = path.rsplit('/').next().unwrap_or(&path);
            folders[index].push(FolderNode {
                display_label: format!("{label}  ({count})"),
                depth: path.bytes().filter(|byte| *byte == b'/').count(),
                path,
            });
        }
        folders
    })[index]
}

fn asset_directory_parts(entry: &CatalogEntry) -> Vec<&str> {
    let parts: Vec<_> = entry.asset_key.split('/').collect();
    let start = parts
        .iter()
        .position(|part| *part == "entities")
        .map(|index| index + 1);
    if let Some(start) = start {
        parts[start..parts.len().saturating_sub(1)].to_vec()
    } else {
        entry.category.split(" / ").skip(1).collect()
    }
}

fn folder_matches(entry: &CatalogEntry, selected: Option<&str>) -> bool {
    let Some(selected) = selected else {
        return true;
    };
    let path = asset_directory_parts(entry).join("/");
    path == selected || path.starts_with(&format!("{selected}/"))
}

fn draw_catalog_browser(
    runtime: &mut UiRuntime,
    ui: &imgui::Ui,
    tab: BrowserTab,
    browser: &mut BrowserState,
    can_spawn: bool,
) -> bool {
    ui.set_next_item_width(-1.0);
    ui.input_text("##spawn_catalog_filter", &mut browser.query)
        .hint("Search items or asset paths")
        .build();

    let selected_folder = browser.selected_folder.clone();
    if browser.cached_query != browser.query
        || browser.cached_tab != Some(tab)
        || browser.cached_folder != selected_folder
    {
        let trimmed = browser.query.trim();
        browser.matched.clear();
        browser.matched.extend(CATALOG.iter().filter(|entry| {
            entry.section == tab.section()
                && folder_matches(entry, selected_folder.as_deref())
                && (trimmed.is_empty() || entry_matches(entry, trimmed))
        }));
        browser.cached_query.clone_from(&browser.query);
        browser.cached_tab = Some(tab);
        browser.cached_folder = selected_folder;
    }

    if let Some(feedback) = browser.placement_feedback.take() {
        ui.text_colored([1.0, 0.72, 0.34, 1.0], feedback);
    }
    let mut allowed_target_interaction = false;
    ui.child_window("spawn_items")
        .size([0.0, 0.0])
        .border(false)
        .build(|| {
            allowed_target_interaction = draw_catalog_grid(
                runtime,
                ui,
                &browser.matched,
                can_spawn,
                !browser.query.trim().is_empty(),
                browser.tile_size,
                &mut browser.placement_feedback,
            );
        });
    allowed_target_interaction
}

fn draw_spawn_sidebar(
    runtime: &mut UiRuntime,
    ui: &imgui::Ui,
    snapshot: &UiGameSnapshot,
    browser: &mut BrowserState,
) {
    for (index, tab) in [SpawnSideTab::Tools, SpawnSideTab::Settings]
        .into_iter()
        .enumerate()
    {
        if index > 0 {
            ui.same_line();
        }
        let selected = browser.side_tab == tab;
        let label = match tab {
            SpawnSideTab::Tools => "Tools",
            SpawnSideTab::Settings => "Settings",
        };
        let _selected = selected
            .then(|| ui.push_style_color(imgui::StyleColor::Button, [0.20, 0.37, 0.50, 1.0]));
        if ui.button_with_size(label, [92.0, 25.0]) {
            browser.side_tab = tab;
        }
    }
    ui.separator();

    match browser.side_tab {
        SpawnSideTab::Tools => draw_spawn_tools(runtime, ui, snapshot),
        SpawnSideTab::Settings => draw_spawn_settings(ui, browser),
    }
}

fn draw_spawn_tools(runtime: &mut UiRuntime, ui: &imgui::Ui, snapshot: &UiGameSnapshot) {
    let mut events = Vec::new();
    let mut panel_found = false;
    {
        let publications = &runtime
            .app
            .world()
            .resource::<crate::devtools::state::ScriptPanelsUiState>()
            .publications;
        if let Some(publication) = publications
            .iter()
            .find(|publication| publication.script_id.as_str() == "debug_spawn_menu")
            && let Some(panel) = publication
                .panels
                .iter()
                .find(|panel| panel.id == "spawn_workbench")
        {
            panel_found = true;
            for widget in &panel.widgets {
                let _widget_id = ui.push_id(widget.id.as_str());
                if let Some(value) = super::draw_script_widget(ui, widget) {
                    events.push(crate::api::scripts::ScriptPanelEvent {
                        script_id: publication.script_id.clone(),
                        generation: publication.generation,
                        panel_id: panel.id.clone(),
                        widget_id: widget.id.clone(),
                        value,
                    });
                }
            }
        }
    }
    for event in events {
        if runtime
            .endpoint
            .script_panel_events_tx
            .try_send(event)
            .is_err()
        {
            ui.text_colored([1.0, 0.55, 0.25, 1.0], "Tool event queue is full");
        }
    }
    if !panel_found {
        ui.text_wrapped("Load the debug_spawn_menu script to enable placement tools.");
    }

    ui.separator();
    let spawned_count = snapshot
        .0
        .owned_spawns
        .iter()
        .flatten()
        .filter(|record| record.lifecycle == SpawnLifecycle::Spawned)
        .count();
    ui.text_disabled(format!("SPAWNED BY MOD  ·  {spawned_count}"));
    let mut removal_queue_full = false;
    for record in snapshot.0.owned_spawns.iter().flatten() {
        let _record_id = ui.push_id(record.request_id.to_string());
        ui.text_colored(
            lifecycle_color(record.lifecycle),
            format!(
                "{:08X}  ·  {}",
                record.native_spawn_id.raw(),
                spawn_lifecycle_label(record.lifecycle)
            ),
        );
        if record.lifecycle == SpawnLifecycle::Spawned {
            ui.same_line();
            if ui.small_button("Remove")
                && runtime
                    .endpoint
                    .spawn_removal_tx
                    .try_send(crate::api::ecs::SpawnRemovalEnvelope {
                        request_id: record.request_id,
                    })
                    .is_err()
            {
                removal_queue_full = true;
            }
        }
    }
    if removal_queue_full {
        ui.text_colored([1.0, 0.55, 0.25, 1.0], "Removal queue is full");
    }
}

fn draw_spawn_settings(ui: &imgui::Ui, browser: &mut BrowserState) {
    ui.text_disabled("THUMBNAILS");
    ui.text_wrapped(
        "Compact tiles keep more of the catalog visible. Choose a larger preview when useful.",
    );
    for size in TileSize::ALL {
        let selected = browser.tile_size == size;
        let _selected = selected
            .then(|| ui.push_style_color(imgui::StyleColor::Button, [0.20, 0.37, 0.50, 1.0]));
        if ui.button_with_size(size.label(), [92.0, 24.0]) {
            browser.tile_size = size;
        }
    }
    ui.separator();
    ui.text_disabled("STATUS COLORS");
    ui.text_colored([0.32, 0.82, 0.48, 1.0], "GREEN  Verified");
    ui.text_colored([0.96, 0.74, 0.26, 1.0], "YELLOW  Experimental attempt");
    ui.text_colored([0.94, 0.36, 0.34, 1.0], "RED  No spawn path");
}

fn entry_matches(entry: &CatalogEntry, query: &str) -> bool {
    contains_ascii_case_insensitive(entry.name, query)
        || contains_ascii_case_insensitive(entry.category, query)
        || contains_ascii_case_insensitive(entry.asset_key, query)
}

fn contains_ascii_case_insensitive(value: &str, query: &str) -> bool {
    value
        .as_bytes()
        .windows(query.len())
        .any(|window| window.eq_ignore_ascii_case(query.as_bytes()))
}

fn draw_catalog_grid(
    runtime: &mut UiRuntime,
    ui: &imgui::Ui,
    entries: &[&CatalogEntry],
    can_spawn: bool,
    query_nonempty: bool,
    tile_size: TileSize,
    placement_feedback: &mut Option<&'static str>,
) -> bool {
    if entries.is_empty() {
        ui.text_disabled(if query_nonempty {
            "No catalog entries match this filter."
        } else {
            "No entries in this category yet."
        });
        return false;
    }

    let available_width = ui.content_region_avail()[0];
    let spacing = ui.clone_style().item_spacing[0];
    let (preferred_width, tile_height, icon_height) = tile_size.dimensions();
    let columns = ((available_width + spacing) / (preferred_width + spacing))
        .floor()
        .max(1.0) as usize;
    let tile_width = ((available_width - spacing * (columns.saturating_sub(1) as f32))
        / columns as f32)
        .max(96.0);
    let rows = entries.len().div_ceil(columns);
    let mut allowed_target_interaction = false;
    let clipper = imgui::ListClipper::new(rows as i32).items_height(tile_height + spacing);
    for row in clipper.begin(ui).iter() {
        let start = row as usize * columns;
        for column in 0..columns {
            if column > 0 {
                ui.same_line();
            }
            if let Some(entry) = entries.get(start + column) {
                let (clicked, eligible_interaction) =
                    draw_catalog_tile(ui, entry, can_spawn, tile_width, tile_height, icon_height);
                allowed_target_interaction |= eligible_interaction;
                if clicked {
                    let event = catalog_selection_event(runtime, entry);
                    if let Some(event) = event {
                        match runtime.endpoint.script_panel_events_tx.try_send(event) {
                            Ok(()) => *placement_feedback = None,
                            Err(_) => {
                                *placement_feedback = Some("Script event queue is unavailable.");
                            }
                        }
                    } else {
                        *placement_feedback =
                            Some("The active spawn script does not expose this item yet.");
                    }
                }
            } else {
                ui.dummy([tile_width, tile_height]);
            }
        }
    }
    allowed_target_interaction
}

fn catalog_selection_event(
    runtime: &UiRuntime,
    entry: &CatalogEntry,
) -> Option<crate::api::scripts::ScriptPanelEvent> {
    if !entry.spawnable {
        return None;
    }
    let state = runtime
        .app
        .world()
        .resource::<crate::devtools::state::ScriptPanelsUiState>();
    let publication = state.publications.iter().find(|publication| {
        publication.script_id.as_str() == "debug_spawn_menu"
            && publication
                .panels
                .iter()
                .any(|panel| panel.id == "spawn_workbench")
    })?;
    Some(crate::api::scripts::ScriptPanelEvent {
        script_id: publication.script_id.clone(),
        generation: publication.generation,
        panel_id: "spawn_workbench".to_owned(),
        widget_id: "catalog_selection".to_owned(),
        value: crate::api::scripts::ScriptPanelValue::Text(entry.catalog_id.to_owned()),
    })
}

fn draw_catalog_tile(
    ui: &imgui::Ui,
    entry: &CatalogEntry,
    can_spawn: bool,
    width: f32,
    height: f32,
    icon_height: f32,
) -> (bool, bool) {
    let enabled = can_spawn && entry.spawnable;
    let start = ui.cursor_screen_pos();
    let size = [width, height];
    let entry_id = ui.push_id(entry.catalog_id);
    let clicked = ui.invisible_button(entry.name, size);
    drop(entry_id);
    let status = entry.status.rgb();
    let hovered = ui.is_item_hovered();
    if hovered {
        ui.tooltip(|| {
            ui.text(entry.name);
            ui.text_disabled(entry.category);
            ui.text_disabled(entry.asset_key);
            ui.text_colored(
                [
                    status[0] as f32 / 255.0,
                    status[1] as f32 / 255.0,
                    status[2] as f32 / 255.0,
                    1.0,
                ],
                entry.status.label(),
            );
        });
    }
    let draw = ui.get_window_draw_list();
    let border = [status[0], status[1], status[2], 255];
    let bg = if hovered && enabled {
        [35, 49, 63, 255]
    } else {
        [20, 31, 43, 255]
    };
    draw.add_rect(start, [start[0] + size[0], start[1] + size[1]], color(bg))
        .filled(true)
        .rounding(4.0)
        .build();
    draw.add_rect(
        start,
        [start[0] + size[0], start[1] + size[1]],
        color(border),
    )
    .rounding(4.0)
    .thickness(if hovered && enabled { 2.0 } else { 1.0 })
    .build();

    let icon_min = [start[0] + 6.0, start[1] + 5.0];
    let icon_max = [start[0] + size[0] - 6.0, start[1] + 5.0 + icon_height];
    draw.add_rect(icon_min, icon_max, color([12, 20, 28, 255]))
        .filled(true)
        .rounding(3.0)
        .build();
    if entry.catalog_id == STOCKPILE_CATALOG_ID {
        draw_stockpile_icon(&draw, icon_min, icon_max);
    } else {
        draw_catalog_icon(&draw, entry, icon_min, icon_max);
    }

    let title = shorten_title(entry.name, (width / 7.2) as usize);
    let text_y = icon_max[1] + 4.0;
    draw.add_text([start[0] + 7.0, text_y], color([225, 233, 241, 255]), title);
    let category = shorten_title(entry.category, (width / 7.2) as usize);
    draw.add_text(
        [start[0] + 7.0, text_y + ui.text_line_height() + 1.0],
        color([145, 160, 176, 255]),
        category,
    );

    let status_y = start[1] + height - ui.text_line_height() - 4.0;
    draw.add_circle(
        [start[0] + 10.0, status_y + ui.text_line_height() * 0.5],
        3.0,
        color(border),
    )
    .filled(true)
    .build();
    let status_label = shorten_title(entry.status.label(), ((width - 27.0) / 7.0) as usize);
    draw.add_text([start[0] + 17.0, status_y], color(border), status_label);
    (
        clicked && enabled,
        (hovered || ui.is_item_active()) && entry.spawnable,
    )
}

fn shorten_title(value: &str, max_chars: usize) -> &str {
    value
        .char_indices()
        .nth(max_chars)
        .map_or(value, |(byte_index, _)| &value[..byte_index])
}

fn draw_catalog_icon(
    draw: &imgui::DrawListMut<'_>,
    entry: &CatalogEntry,
    min: [f32; 2],
    max: [f32; 2],
) {
    let center_x = (min[0] + max[0]) * 0.5;
    let center_y = (min[1] + max[1]) * 0.5;
    let icon_color = match entry.section {
        "Vehicles" => color([210, 176, 112, 255]),
        "World Compositions" => color([128, 173, 190, 255]),
        _ => color([170, 190, 205, 255]),
    };
    if entry.section == "Vehicles" {
        let w = (max[0] - min[0]).min(30.0);
        let h = (max[1] - min[1]).min(16.0);
        draw.add_rect(
            [center_x - w * 0.5, center_y - h * 0.20],
            [center_x + w * 0.5, center_y + h * 0.42],
            icon_color,
        )
        .filled(true)
        .rounding(3.0)
        .build();
        draw.add_rect(
            [center_x - w * 0.28, center_y - h * 0.58],
            [center_x + w * 0.23, center_y - h * 0.18],
            icon_color,
        )
        .filled(true)
        .rounding(2.0)
        .build();
        for x in [center_x - w * 0.30, center_x + w * 0.30] {
            draw.add_circle([x, center_y + h * 0.38], 2.2, color([24, 34, 42, 255]))
                .filled(true)
                .build();
        }
    } else {
        let half = (max[1] - min[1]).min(18.0) * 0.5;
        let half_x = (max[0] - min[0]).min(28.0) * 0.5;
        draw.add_rect(
            [center_x - half_x, center_y - half],
            [center_x + half_x, center_y + half],
            icon_color,
        )
        .rounding(2.0)
        .thickness(1.5)
        .build();
        if entry.section == "World Compositions" {
            draw.add_line(
                [center_x - half_x, center_y],
                [center_x + half_x, center_y],
                icon_color,
            )
            .build();
            draw.add_line(
                [center_x, center_y - half],
                [center_x, center_y + half],
                icon_color,
            )
            .build();
        }
    }
}

fn draw_stockpile_icon(draw: &imgui::DrawListMut<'_>, min: [f32; 2], max: [f32; 2]) {
    let metal = [170, 125, 72, 255];
    let light_metal = [203, 160, 95, 255];
    let shadow = [100, 76, 52, 255];
    let w = max[0] - min[0];
    let h = max[1] - min[1];
    let x = min[0] + w * 0.15;
    let y = min[1] + h * 0.16;
    let sx = w * 0.70 / 98.0;
    let sy = h * 0.70 / 55.0;
    let rect = |x0: f32, y0: f32, x1: f32, y1: f32| {
        ([x + x0 * sx, y + y0 * sy], [x + x1 * sx, y + y1 * sy])
    };
    let (a, b) = rect(2.0, 43.0, 94.0, 55.0);
    draw.add_rect(a, b, color(shadow)).filled(true).build();
    let (a, b) = rect(8.0, 29.0, 85.0, 42.0);
    draw.add_rect(a, b, color(metal)).filled(true).build();
    let (a, b) = rect(0.0, 15.0, 70.0, 28.0);
    draw.add_rect(a, b, color(light_metal)).filled(true).build();
    let (a, b) = rect(33.0, 2.0, 98.0, 14.0);
    draw.add_rect(a, b, color(metal)).filled(true).build();
}

fn spawn_lifecycle_label(lifecycle: SpawnLifecycle) -> &'static str {
    match lifecycle {
        SpawnLifecycle::Queued | SpawnLifecycle::Dispatching => "Queued",
        SpawnLifecycle::Rejected | SpawnLifecycle::Failed => "Failed",
        SpawnLifecycle::PendingObservation => "Waiting for game",
        SpawnLifecycle::Spawned => "Spawned",
        SpawnLifecycle::DespawnRequested => "Removing",
        SpawnLifecycle::Despawned => "Removed",
    }
}

fn lifecycle_color(lifecycle: SpawnLifecycle) -> [f32; 4] {
    use crate::api::scripts::ScriptStatusLevel as Level;
    let level = match lifecycle {
        SpawnLifecycle::Queued
        | SpawnLifecycle::Dispatching
        | SpawnLifecycle::PendingObservation
        | SpawnLifecycle::DespawnRequested => Level::Experimental,
        SpawnLifecycle::Rejected | SpawnLifecycle::Failed => Level::Problematic,
        SpawnLifecycle::Spawned | SpawnLifecycle::Despawned => Level::Stable,
    };
    level.color()
}

fn color(color: [u8; 4]) -> imgui::ImColor32 {
    imgui::ImColor32::from_rgba(color[0], color[1], color[2], color[3])
}
