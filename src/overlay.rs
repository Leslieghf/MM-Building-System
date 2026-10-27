use super::game_probe;
use hudhook::hooks::dx11::ImguiDx11Hooks;
use hudhook::{Hudhook, ImguiRenderLoop, MessageFilter, imgui};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::panic;
use std::path::PathBuf;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, PartialEq, Eq)]
enum OverlayMode {
    Hud,
    SectionPicker,
}

impl OverlayMode {
    fn owns_game_input(self) -> bool {
        matches!(self, Self::SectionPicker)
    }
}

struct DebugEntry {
    category: &'static str,
    label: &'static str,
    value: String,
}

struct BootstrapNotice {
    mode: OverlayMode,
    show_player: bool,
    show_camera: bool,
    show_runtime: bool,
}

impl BootstrapNotice {
    fn new() -> Self {
        Self {
            mode: OverlayMode::Hud,
            show_player: true,
            show_camera: true,
            show_runtime: false,
        }
    }

    fn debug_entries(&self) -> Vec<DebugEntry> {
        let sample = game_probe::player_sample();
        let status = sample.map_or("waiting for game callback", |sample| sample.0);
        let position = sample.and_then(|sample| sample.1).map_or_else(
            || "--".to_owned(),
            |[x, y, z]| format!("{x:.2}, {y:.2}, {z:.2}"),
        );
        let rotation = sample.and_then(|sample| sample.2).map_or_else(
            || "--".to_owned(),
            |[x, y, z]| format!("{x:.1}°, {y:.1}°, {z:.1}°"),
        );
        let view_rotation = sample.and_then(|sample| sample.3).map_or_else(
            || "--".to_owned(),
            |[x, y, z]| format!("{x:.1}°, {y:.1}°, {z:.1}°"),
        );
        let status = if status == "position stale" {
            "STALE: no successful sample for 1 second"
        } else {
            status
        };
        let callbacks = game_probe::callback_count()
            .map_or_else(|| "waiting".to_owned(), |count| count.to_string());

        vec![
            DebugEntry {
                category: "Player",
                label: "Status",
                value: status.to_owned(),
            },
            DebugEntry {
                category: "Player",
                label: "Position",
                value: position,
            },
            DebugEntry {
                category: "Player",
                label: "Rotation (player/model)",
                value: rotation,
            },
            DebugEntry {
                category: "Camera",
                label: "Rotation (view)",
                value: view_rotation,
            },
            DebugEntry {
                category: "Runtime",
                label: "UpdatePostSim callbacks",
                value: callbacks,
            },
            DebugEntry {
                category: "Runtime",
                label: "Player sample cadence",
                value: "every 5 game callbacks".to_owned(),
            },
        ]
    }
}

impl ImguiRenderLoop for BootstrapNotice {
    fn before_render(
        &mut self,
        context: &mut imgui::Context,
        _render_context: &mut dyn hudhook::RenderContext,
    ) {
        context.io_mut().mouse_draw_cursor = self.mode.owns_game_input();
    }

    fn render(&mut self, ui: &mut imgui::Ui) {
        let toggle_pressed =
            ui.is_key_pressed_no_repeat(imgui::Key::F6) && ui.io().key_ctrl && ui.io().key_shift;
        if self.mode.owns_game_input() && ui.is_key_pressed_no_repeat(imgui::Key::Escape) {
            self.mode = OverlayMode::Hud;
        }
        if toggle_pressed {
            self.mode = if self.mode == OverlayMode::Hud {
                OverlayMode::SectionPicker
            } else {
                OverlayMode::Hud
            };
        }

        let entries = self.debug_entries();
        let style = ui.push_style_color(imgui::StyleColor::WindowBg, [0.055, 0.075, 0.105, 0.96]);
        ui.window("Mad Max Debug")
            .position([12.0, 12.0], imgui::Condition::Always)
            .bg_alpha(0.96)
            .flags(imgui::WindowFlags::NO_SAVED_SETTINGS | imgui::WindowFlags::ALWAYS_AUTO_RESIZE)
            .build(|| {
                if self.show_player {
                    if ui.collapsing_header("Player", imgui::TreeNodeFlags::DEFAULT_OPEN) {
                        for entry in entries.iter().filter(|entry| entry.category == "Player") {
                            ui.text(format!("{}: {}", entry.label, entry.value));
                        }
                    }
                }
                if self.show_camera {
                    if ui.collapsing_header("Camera", imgui::TreeNodeFlags::DEFAULT_OPEN) {
                        for entry in entries.iter().filter(|entry| entry.category == "Camera") {
                            ui.text(format!("{}: {}", entry.label, entry.value));
                        }
                    }
                }
                if self.show_runtime {
                    if ui.collapsing_header("Runtime", imgui::TreeNodeFlags::empty()) {
                        for entry in entries.iter().filter(|entry| entry.category == "Runtime") {
                            ui.text(format!("{}: {}", entry.label, entry.value));
                        }
                    }
                }
                ui.separator();
                ui.text("Ctrl+Shift+F6: choose sections");
            });
        style.pop();

        if self.mode == OverlayMode::SectionPicker {
            let mut close_requested = false;
            let style =
                ui.push_style_color(imgui::StyleColor::WindowBg, [0.055, 0.075, 0.105, 0.98]);
            ui.window("Debug Sections")
                .position([12.0, 300.0], imgui::Condition::FirstUseEver)
                .flags(
                    imgui::WindowFlags::NO_SAVED_SETTINGS | imgui::WindowFlags::ALWAYS_AUTO_RESIZE,
                )
                .build(|| {
                    ui.text("Show in the debug panel:");
                    ui.checkbox("Player", &mut self.show_player);
                    ui.checkbox("Camera", &mut self.show_camera);
                    ui.checkbox("Runtime", &mut self.show_runtime);
                    ui.separator();
                    if ui.button("Close") {
                        close_requested = true;
                    }
                    ui.same_line();
                    ui.text("Escape returns input to the game.");
                });
            style.pop();
            if close_requested {
                self.mode = OverlayMode::Hud;
            }
        }
    }

    fn message_filter(&self, _io: &imgui::Io) -> MessageFilter {
        if self.mode.owns_game_input() {
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

pub(crate) fn append_log(message: &str) {
    let Some(local_data) = env::var_os("LOCALAPPDATA") else {
        return;
    };
    let log_path = PathBuf::from(local_data)
        .join("MadMaxModding")
        .join("logs")
        .join("bootstrap.log");
    if let Some(parent) = log_path.parent() {
        if fs::create_dir_all(parent).is_err() {
            return;
        }
    }
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) else {
        return;
    };
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let _ = writeln!(file, "Unix time: {seconds}\n{message}");
}
