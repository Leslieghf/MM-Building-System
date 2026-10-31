//! Loader-facing entry point and one-time runtime composition.
//!
//! # Lifecycle
//!
//! [`InitializeASI`] contains panics at the foreign-function boundary and
//! delegates startup to a named worker thread. The worker creates bounded
//! endpoints, installs each endpoint with its owning subsystem, then attempts
//! to install the overlay hook. Game sampling starts only when hook
//! installation succeeds; this module never calls game functions directly.
use crate::api::ecs::create_endpoints;
use crate::api::game::adapter as game_access;
use crate::devtools::overlay;
use crate::kernel::diagnostics::append_log;
use std::panic::{self, AssertUnwindSafe};
use std::thread;

/// Entry called by the ASI loader after this library is loaded into Mad Max.
#[unsafe(no_mangle)]
pub extern "system" fn InitializeASI() {
    let _ = panic::catch_unwind(AssertUnwindSafe(startup));
}

fn startup() {
    append_log("ASI entry point called; starting the DirectX 11 overlay hook.");
    if let Err(error) = thread::Builder::new()
        .name("madmax-overlay-hook".to_owned())
        .spawn(install_runtime)
    {
        append_log(&format!("Could not start overlay worker: {error}"));
    }
}

fn install_runtime() {
    let (game_endpoint, ui_endpoint) = create_endpoints();
    game_access::set_runtime_endpoint(game_endpoint);
    overlay::set_ui_endpoint(ui_endpoint);
    let installed = panic::catch_unwind(AssertUnwindSafe(overlay::install_overlay_hook));
    match installed {
        Ok(true) => game_access::start_update_callback_probe(),
        Ok(false) => {}
        Err(_) => append_log("Overlay setup panicked; no game operation was attempted."),
    }
}
