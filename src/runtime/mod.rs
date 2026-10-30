//! Bevy schedules, owned telemetry, bounded intents, and UI/game coordination.
//! Native hooks publish snapshots here; effects return as validated intents.

mod endpoints;
mod game;
mod game_loop;
mod model;
mod presentation;

pub use endpoints::*;
pub use game::*;
pub use game_loop::*;
pub use model::*;
pub use presentation::*;

mod modding;

use crate::scripting::console::{
    self, ConsoleCommand, ConsoleEvent, ConsoleEventReceiver, ConsoleEventSink, ConsoleOutput,
    ConsoleRequest, ConsoleRequestSender, ConsoleSeverity,
};
use crate::sdk::game::world_query::C_CUSTOM_SAMPLED_HEIGHT_FIELD_SHAPE_VFTABLE_RVA;
use crate::sdk::types::Matrix4x4;
use crate::ui::format::{
    angles as format_angles, fixed as format_fixed, magnitude as format_magnitude,
    vector as format_vector,
};
use bevy_app::App;
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{Schedule, ScheduleLabel, SingleThreadedExecutor};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::time::Instant;

pub const SNAPSHOT_QUEUE_CAPACITY: usize = 8;
pub const INTENT_QUEUE_CAPACITY: usize = 32;
pub const OUTCOME_QUEUE_CAPACITY: usize = 32;
pub const MAX_INTENTS_PER_RENDER: usize = 8;
pub const SPAWN_INTENT_QUEUE_CAPACITY: usize = 1;
pub const MAX_SPAWNS_PER_RENDER: usize = 1;
pub const MAX_SPAWN_ASSET_KEY_BYTES: usize = 240;
static NEXT_INTENT_ID: AtomicU64 = AtomicU64::new(1);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scripting::{
        ScriptId,
        engine::RhaiEngineFactory,
        loader::{DiscoverScriptsRequest, ScriptLoader},
        manager::ScriptManager,
    };
    use std::{
        fs,
        path::PathBuf,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    struct Fixture(PathBuf);
    impl Fixture {
        fn runtime() -> (Self, GameRuntime, UiEndpoint) {
            let root = std::env::temp_dir().join(format!(
                "madmax-runtime-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir_all(root.join("example")).unwrap();
            fs::write(root.join("example/manifest.toml"), "id = \"example\"\napi_version = 1\ncapabilities = []\n[callbacks]\n\"game.post_sim\" = \"on_post_sim\"\n").unwrap();
            fs::write(root.join("example/main.rhai"), "fn on_post_sim(ctx) {}\n").unwrap();
            let (mut game, ui) = create_endpoints();
            game.script_loader = ScriptLoader::start(root.clone(), RhaiEngineFactory);
            let mut runtime = GameRuntime::new(game);
            runtime.app.world_mut().resource_mut::<ScriptManager>().stage_prepared(crate::scripting::package::PreparedScript::compile_package(0, Self::id(), toml::from_str("id = \"example\"\napi_version = 1\ncapabilities = []\n[callbacks]\n\"game.post_sim\" = \"on_post_sim\"\n").unwrap(), "fn on_post_sim(ctx) {}", false, RhaiEngineFactory).unwrap()).unwrap();
            runtime.prepare_and_run(GameSnapshot::default());
            while ui.console_events.try_recv().is_ok() {}
            (Self(root), runtime, ui)
        }
        fn id() -> ScriptId {
            ScriptId::parse("example").unwrap()
        }
        fn enable(runtime: &mut GameRuntime, ui: &mut UiEndpoint) {
            ui.console_requests
                .submit(ConsoleCommand::ScriptAutoReload {
                    id: Self::id(),
                    enabled: true,
                })
                .unwrap();
            runtime.prepare_and_run(GameSnapshot::default());
            runtime
                .endpoint
                .script_loader
                .try_discover(DiscoverScriptsRequest { request_id: None })
                .unwrap();
            let until = Instant::now() + Duration::from_secs(5);
            while runtime
                .endpoint
                .script_loader
                .try_discovery_result()
                .is_none()
            {
                assert!(Instant::now() < until);
                std::thread::sleep(Duration::from_millis(10));
            }
            while ui.console_events.try_recv().is_ok() {}
        }
        fn edit(&self) {
            fs::write(
                self.0.join("example/main.rhai"),
                "fn on_post_sim(ctx) { let changed = 2; }",
            )
            .unwrap();
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn automatic_notice_uses_worker_then_activates_at_callback_boundary() {
        let (fixture, mut runtime, mut ui) = Fixture::runtime();
        Fixture::enable(&mut runtime, &mut ui);
        let generation = runtime.app.world().resource::<ScriptManager>().list()[0].generation;
        fixture.edit();
        std::thread::sleep(Duration::from_millis(1200));
        assert_eq!(
            runtime.app.world().resource::<ScriptManager>().list()[0].generation,
            generation
        );
        let until = Instant::now() + Duration::from_secs(5);
        while runtime.app.world().resource::<ScriptManager>().list()[0].generation == generation {
            runtime.prepare_and_run(GameSnapshot::default());
            assert!(
                Instant::now() < until,
                "automatic notice did not enter activation pipeline"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        let mut output = ConsoleOutput::default();
        while let Ok(event) = ui.console_events.try_recv() {
            output.record(event);
        }
        assert!(
            output
                .events()
                .iter()
                .any(|event| event.text == "Preparing")
        );
        assert!(output.events().iter().any(|event| event.text == "Prepared"));
        assert!(output.events().iter().any(|event| event.text == "Reloaded"));
        let mut state = ScriptConsoleUiState::default();
        state.observe_output(&output);
        assert!(state.loaded_scripts[0].auto_reload);
    }
    #[test]
    fn queued_manual_request_precedes_newer_watcher_notice() {
        let (fixture, mut runtime, mut ui) = Fixture::runtime();
        Fixture::enable(&mut runtime, &mut ui);
        let manual_id = ui
            .console_requests
            .submit(ConsoleCommand::ScriptReload(Fixture::id()))
            .unwrap();
        fixture.edit();
        std::thread::sleep(Duration::from_millis(1200));
        runtime.prepare_and_run(GameSnapshot::default());
        let mut events = Vec::new();
        while let Ok(event) = ui.console_events.try_recv() {
            events.push(event);
        }
        let preparing: Vec<_> = events
            .iter()
            .filter(|event| event.text == "Preparing")
            .collect();
        assert_eq!(preparing.len(), 1);
        assert_eq!(preparing[0].request_id, Some(manual_id));
    }
    #[test]
    fn queued_off_request_precedes_watcher_notice() {
        let (fixture, mut runtime, mut ui) = Fixture::runtime();
        Fixture::enable(&mut runtime, &mut ui);
        fixture.edit();
        std::thread::sleep(Duration::from_millis(1200));
        ui.console_requests
            .submit(ConsoleCommand::ScriptAutoReload {
                id: Fixture::id(),
                enabled: false,
            })
            .unwrap();
        runtime.prepare_and_run(GameSnapshot::default());
        let listing = &runtime.app.world().resource::<ScriptManager>().list()[0];
        assert!(!listing.auto_reload);
        assert_eq!(listing.generation.0, 1);
        assert!(
            runtime
                .app
                .world()
                .resource::<ScriptManager>()
                .pending_loads()
                .is_empty()
        );
    }
    #[test]
    fn auto_reload_is_typed_only_and_row_errors_remain_bounded() {
        assert!(ConsoleCommand::parse("script auto_reload example").is_err());
        let (fixture, mut runtime, mut ui) = Fixture::runtime();
        Fixture::enable(&mut runtime, &mut ui);
        runtime
            .app
            .world_mut()
            .resource_mut::<ScriptManager>()
            .report_auto_reload_overflow(Fixture::id());
        let mut output = ConsoleOutput::default();
        while let Ok(event) = ui.console_events.try_recv() {
            output.record(event);
        }
        assert_eq!(
            output
                .events()
                .iter()
                .filter(|event| event.severity == ConsoleSeverity::Error)
                .count(),
            1
        );
        let mut state = ScriptConsoleUiState::default();
        state.observe_output(&output);
        assert!(state.loaded_scripts[0].auto_reload);
        assert!(
            state.loaded_scripts[0].reload_error.as_ref().unwrap().len()
                <= crate::scripting::MAX_EVENT_BYTES
        );
        drop(fixture);
    }
}
