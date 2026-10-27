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
    hotkey_notice_until: Option<Instant>,
}

impl BootstrapNotice {
    fn new() -> Self {
        Self {
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
    }

    fn render(&mut self, ui: &mut imgui::Ui) {
        let now = Instant::now();
        game_probe::advance_overlay_frame();
        if ui.is_key_pressed_no_repeat(imgui::Key::F6) && ui.io().key_ctrl && ui.io().key_shift {
            self.hotkey_notice_until = Some(now + Duration::from_secs(2));
        }

        let sample = game_probe::player_sample();
        let status = sample.map_or("waiting for game callback", |sample| sample.0);
        let position = sample
            .and_then(|sample| sample.1)
            .map_or_else(|| "--".to_owned(), |[x, y, z]| format!("{x:.2}, {y:.2}, {z:.2}"));
        let rotation = sample
            .and_then(|sample| sample.2)
            .map_or_else(|| "--".to_owned(), |[x, y, z]| format!("{x:.1}°, {y:.1}°, {z:.1}°"));
        let view_rotation = sample
            .and_then(|sample| sample.3)
            .map_or_else(|| "--".to_owned(), |[x, y, z]| format!("{x:.1}°, {y:.1}°, {z:.1}°"));
        let callbacks = game_probe::callback_count()
            .map_or_else(|| "waiting".to_owned(), |count| count.to_string());
        let hotkey_notice = self.hotkey_notice_until.is_some_and(|until| now < until);

        ui.window("Mad Max Debug")
            .position([12.0, 12.0], imgui::Condition::Always)
            .flags(
                imgui::WindowFlags::NO_RESIZE
                    | imgui::WindowFlags::NO_COLLAPSE
                    | imgui::WindowFlags::NO_SAVED_SETTINGS
                    | imgui::WindowFlags::ALWAYS_AUTO_RESIZE,
            )
            .build(|| {
                ui.text(format!("Position: {position}"));
                ui.text(format!("Rotation (player/model): {rotation}"));
                ui.text(format!("Rotation (view): {view_rotation}"));
                ui.separator();
                if status == "position stale" {
                    ui.text("STALE: 5 frames beyond the refresh interval");
                } else {
                    ui.text(format!("Player: {status}"));
                }
                ui.text(format!("UpdatePostSim frames: {callbacks} (sample every 5)"));
                if hotkey_notice {
                    ui.text("Ctrl+Shift+F6 received by the Rust overlay");
                }
            });
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
