use super::game_probe;
use hudhook::hooks::dx11::ImguiDx11Hooks;
use hudhook::{Hudhook, ImguiRenderLoop, imgui};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::panic;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

struct BootstrapNotice {
    started_at: Option<Instant>,
    hotkey_notice_until: Option<Instant>,
}

impl BootstrapNotice {
    fn new() -> Self {
        Self {
            started_at: None,
            hotkey_notice_until: None,
        }
    }
}

impl ImguiRenderLoop for BootstrapNotice {
    fn initialize(
        &mut self,
        _context: &mut imgui::Context,
        _render_context: &mut dyn hudhook::RenderContext,
    ) {
        self.started_at = Some(Instant::now());
    }

    fn render(&mut self, ui: &mut imgui::Ui) {
        let now = Instant::now();
        if ui.is_key_pressed_no_repeat(imgui::Key::F6) && ui.io().key_ctrl && ui.io().key_shift {
            self.hotkey_notice_until = Some(now + Duration::from_secs(2));
        }

        let message = if self.hotkey_notice_until.is_some_and(|until| now < until) {
            "Ctrl+Shift+F6 received by the Rust overlay".to_owned()
        } else if let Some((x, y, z)) = game_probe::latest_player_position() {
            format!("Max position: X {x:.1}  Y {y:.1}  Z {z:.1}")
        } else if self
            .started_at
            .is_some_and(|started_at| now.duration_since(started_at) < Duration::from_secs(10))
        {
            "Rust DirectX 11 render hook active".to_owned()
        } else {
            return;
        };

        let screen_size = ui.io().display_size;
        ui.window("Mad Max Modding Bootstrap")
            .position([screen_size[0] * 0.5, 72.0], imgui::Condition::Always)
            .position_pivot([0.5, 0.0])
            .flags(
                imgui::WindowFlags::NO_DECORATION
                    | imgui::WindowFlags::NO_BACKGROUND
                    | imgui::WindowFlags::NO_INPUTS
                    | imgui::WindowFlags::NO_SAVED_SETTINGS
                    | imgui::WindowFlags::ALWAYS_AUTO_RESIZE,
            )
            .build(|| ui.text(&message));
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn InitializeASI() {
    let _ = panic::catch_unwind(startup);
}

fn startup() {
    append_log("ASI entry point called; installing startup gate and DirectX 11 overlay.");
    game_probe::install_startup_gate();
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
