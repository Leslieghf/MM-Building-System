//! Best-effort process diagnostics independent of callback and render timing.
//!
//! [`append_log`] writes timestamped messages under the user's local
//! application data directory. Missing environment variables, directory
//! errors, and file errors are intentionally ignored so diagnostics cannot
//! prevent startup or interfere with game-thread work.
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn append_log(message: &str) {
    let Some(local_data) = env::var_os("LOCALAPPDATA") else {
        return;
    };
    let log_path = PathBuf::from(local_data)
        .join("MadMaxModding")
        .join("logs")
        .join("bootstrap.log");
    if let Some(parent) = log_path.parent()
        && fs::create_dir_all(parent).is_err()
    {
        return;
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
