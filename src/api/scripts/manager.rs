//! Callback-bound script lifecycle, scheduling, and effect ownership.
mod invocation;
use super::{engine::RhaiEngineFactory, *};
use crate::api::ecs::GameSnapshot;
use crate::api::rhai::bindings::{
    DebugPrimitive, GamePostSimContext, InvocationDebugDraw, InvocationLog, InvocationVehicleWrite,
};
use bevy_ecs::prelude::Resource;
use rhai::{AST, CallFnOptions, Dynamic, Engine, Map, Scope};
use std::collections::{BTreeMap, VecDeque};

pub const MAX_ACTIVE_SCRIPTS: usize = 16;
pub const MAX_PENDING_CHANGES: usize = 32;
pub const MAX_RUNS_PER_PHASE: usize = 16;
pub const MAX_CALLBACKS_PER_PHASE: usize = MAX_ACTIVE_SCRIPTS + MAX_RUNS_PER_PHASE;
pub const MAX_VELOCITY_EFFECTS_PER_PHASE: usize = 32;
pub const MAX_DEBUG_DRAWS_PER_INVOCATION: usize = 32;
pub const MAX_DEBUG_DRAWS_PER_PHASE: usize = 128;
pub const MAX_DEBUG_DRAW_WORK_PER_INVOCATION: usize = 256;
pub const MAX_DEBUG_DRAW_WORK_PER_PHASE: usize = 512;
/// Limit outstanding effects too: the native adapter drains one intent per callback.
pub const MAX_PENDING_VELOCITY_EFFECTS: usize = 32;
pub(crate) struct VehicleVelocityEffect {
    pub velocity: [f32; 3],
    pub origin: ScriptEvent,
}
pub(crate) struct ScriptDebugDrawEffect {
    pub primitive: DebugPrimitive,
    pub origin: ScriptEvent,
}
pub const MAX_STATE_VALUES: usize = 1024;
pub const MAX_STATE_DEPTH: usize = 16;

pub use super::package::PreparedScript;
struct ActiveScript {
    capabilities: Vec<String>,
    ast: AST,
    scope: Scope<'static>,
    generation: ScriptGeneration,
    callbacks: BTreeMap<ScriptPhaseId, String>,
    on_run: Option<String>,
    status: ScriptStatus,
    auto_reload: bool,
    reload_error: Option<String>,
    latest_load_request_id: u64,
    deferred_auto_reload: Option<super::loader::LoadScriptRequest>,
}
enum PendingChange {
    Install(PreparedScript, bool),
    Unload(ScriptId, Option<u64>),
    Run(ScriptId, Option<u64>),
}
#[derive(Debug, Clone)]
pub struct ScriptListing {
    pub id: ScriptId,
    pub generation: ScriptGeneration,
    pub status: ScriptStatus,
    pub callbacks: BTreeMap<ScriptPhaseId, String>,
    pub has_on_run: bool,
    pub auto_reload: bool,
    pub reload_error: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoadOrigin {
    Manual,
    Automatic,
}
#[derive(Clone, Copy)]
struct PendingLoad {
    request_id: u64,
    origin: LoadOrigin,
}
#[derive(Resource)]
/// Owns active script generations and applies package changes at callback boundaries.
pub struct ScriptManager {
    engine: Engine,
    active: BTreeMap<ScriptId, ActiveScript>,
    pending: VecDeque<PendingChange>,
    runs: Vec<(ScriptId, ScriptGeneration, Option<u64>)>,
    next_generation: u64,
    last_load_request_id: u64,
    load_requests: BTreeMap<ScriptId, PendingLoad>,
    events: ScriptEventSink,
}
impl Default for ScriptManager {
    fn default() -> Self {
        // Runtime transport supplies a retained receiver when constructing GameRuntime.
        let (events, _) = event_channel();
        Self::new(events)
    }
}
impl ScriptManager {
    pub fn new(events: ScriptEventSink) -> Self {
        Self {
            engine: RhaiEngineFactory::build(),
            active: BTreeMap::new(),
            pending: VecDeque::new(),
            runs: Vec::new(),
            next_generation: 1,
            last_load_request_id: 0,
            load_requests: BTreeMap::new(),
            events,
        }
    }
    fn stage(&mut self, change: PendingChange) -> Result<(), &'static str> {
        if self.pending.len() >= MAX_PENDING_CHANGES {
            let (id, request_id) = match &change {
                PendingChange::Install(prepared, _) => (
                    ScriptId::parse(prepared.manifest().id.clone()).expect("validated ID"),
                    Some(prepared.request_id()),
                ),
                PendingChange::Unload(id, request_id) | PendingChange::Run(id, request_id) => {
                    (id.clone(), *request_id)
                }
            };
            self.report_failure(id, request_id, "script change queue full");
            return Err("script change queue full");
        }
        self.pending.push_back(change);
        Ok(())
    }
    pub fn stage_prepared(&mut self, prepared: PreparedScript) -> Result<(), String> {
        self.stage(PendingChange::Install(prepared, false))
            .map_err(str::to_owned)
    }
    pub fn stage_unload(&mut self, id: ScriptId) -> Result<(), &'static str> {
        self.stage_unload_request(id, None)
    }
    pub fn stage_unload_request(
        &mut self,
        id: ScriptId,
        request_id: Option<u64>,
    ) -> Result<(), &'static str> {
        self.stage(PendingChange::Unload(id.clone(), request_id))?;
        // Cancel preparations already requested; a later load remains valid.
        if let Some(cancelled) = self.load_requests.remove(&id) {
            self.report_current_request(id, Some(cancelled.request_id), "Cancelled");
        }
        Ok(())
    }
    pub fn stage_run(&mut self, id: ScriptId) -> Result<(), &'static str> {
        self.stage_run_request(id, None)
    }
    pub fn stage_run_request(
        &mut self,
        id: ScriptId,
        request_id: Option<u64>,
    ) -> Result<(), &'static str> {
        self.stage(PendingChange::Run(id, request_id))
    }
    pub fn set_auto_reload(
        &mut self,
        id: &ScriptId,
        enabled: bool,
        loader: &super::loader::ScriptLoader,
    ) -> Result<(), &'static str> {
        self.set_auto_reload_request(id, enabled, loader, None)
    }
    pub fn set_auto_reload_request(
        &mut self,
        id: &ScriptId,
        enabled: bool,
        loader: &super::loader::ScriptLoader,
        request_id: Option<u64>,
    ) -> Result<(), &'static str> {
        let result = if !self.active.contains_key(id) {
            Err("script is not loaded")
        } else {
            loader.set_auto_reload(id.clone(), enabled)
        };
        if let Err(message) = result {
            self.report_reload_failure(id.clone(), request_id, message);
            return Err(message);
        }
        let script = self.active.get_mut(id).expect("loaded script");
        script.auto_reload = enabled;
        if !enabled {
            script.deferred_auto_reload = None;
        }
        if !enabled
            && self
                .load_requests
                .get(id)
                .is_some_and(|load| load.origin == LoadOrigin::Automatic)
        {
            self.cancel_load(id, "Cancelled");
        }
        self.report_current_request(
            id.clone(),
            request_id,
            if enabled {
                "Auto Reload On"
            } else {
                "Auto Reload Off"
            },
        );
        Ok(())
    }
    pub fn request_unload(
        &mut self,
        loader: &super::loader::ScriptLoader,
        id: ScriptId,
        request_id: Option<u64>,
    ) -> Result<(), &'static str> {
        // Check staging capacity before accepting a worker-side watch removal.
        if self.pending.len() >= MAX_PENDING_CHANGES {
            return self.stage(PendingChange::Unload(id, request_id));
        }
        // Rejected cleanup must leave a loaded script attached to its watch.
        if self
            .active
            .get(&id)
            .is_some_and(|script| script.auto_reload)
        {
            if let Err(message) = loader.set_auto_reload(id.clone(), false) {
                self.report_failure(id, request_id, message);
                return Err(message);
            }
        }
        self.stage_unload_request(id, request_id)
    }
    fn cancel_load(&mut self, id: &ScriptId, text: &str) {
        if let Some(cancelled) = self.load_requests.remove(id) {
            self.report_current_request(id.clone(), Some(cancelled.request_id), text);
        }
    }
    /// Manual intent wins over automatic work, including prepared generations.
    pub fn request_load(
        &mut self,
        loader: &super::loader::ScriptLoader,
        request: super::loader::LoadScriptRequest,
    ) -> Result<(), String> {
        self.request_load_with_origin(loader, request, LoadOrigin::Manual)
    }
    pub fn request_auto_reload(
        &mut self,
        loader: &super::loader::ScriptLoader,
        mut request: super::loader::LoadScriptRequest,
    ) -> Result<(), String> {
        let Some(script) = self.active.get_mut(&request.id) else {
            return Ok(());
        };
        // Notices may already be queued when Off/unload/manual intent is processed.
        if !script.auto_reload || request.request_id <= script.latest_load_request_id {
            return Ok(());
        }
        request.replace = true;
        if self.load_requests.contains_key(&request.id) {
            // A manual replacement stays authoritative for its activation, but
            // a later stable filesystem notice must survive until that result
            // settles. Otherwise the worker has acknowledged the fingerprint
            // and will not emit it again.
            if script
                .deferred_auto_reload
                .as_ref()
                .is_none_or(|queued| queued.request_id < request.request_id)
            {
                script.deferred_auto_reload = Some(request);
            }
            return Ok(());
        }
        self.request_load_with_origin(loader, request, LoadOrigin::Automatic)
    }
    pub fn process_deferred_auto_reloads(&mut self, loader: &super::loader::ScriptLoader) {
        // One retained notice per loaded script, never a second automatic preparation.
        let ready: Vec<_> = self
            .active
            .iter_mut()
            .filter_map(|(id, script)| {
                if !self.load_requests.contains_key(id) {
                    script.deferred_auto_reload.take()
                } else {
                    None
                }
            })
            .collect();
        for request in ready {
            let _ = self.request_auto_reload(loader, request);
        }
    }
    fn request_load_with_origin(
        &mut self,
        loader: &super::loader::ScriptLoader,
        request: super::loader::LoadScriptRequest,
        origin: LoadOrigin,
    ) -> Result<(), String> {
        let id = request.id.clone();
        let request_id = request.request_id;
        if origin == LoadOrigin::Manual && request_id <= self.last_load_request_id {
            self.report_failure(id, Some(request_id), "script load request ID must increase");
            return Err("script load request ID must increase".into());
        }
        // Invalidate this script's tracked install BEFORE applying earlier changes.
        if origin == LoadOrigin::Manual {
            self.cancel_load(&id, "Superseded");
            if let Some(script) = self.active.get_mut(&id) {
                script.deferred_auto_reload = None;
            }
        }
        self.apply_pending_changes();
        let result = if !request.replace && self.active.contains_key(&id) {
            Err("script already loaded; use reload")
        } else if self.load_requests.len() >= MAX_PENDING_CHANGES
            && !self.load_requests.contains_key(&id)
        {
            Err("pending script load limit reached")
        } else {
            loader.try_request(request)
        };
        match result {
            Ok(()) => {
                if origin == LoadOrigin::Manual {
                    self.last_load_request_id = request_id;
                }
                if let Some(script) = self.active.get_mut(&id) {
                    script.latest_load_request_id = script.latest_load_request_id.max(request_id);
                }
                self.load_requests
                    .insert(id.clone(), PendingLoad { request_id, origin });
                self.report_current_request(id, Some(request_id), "Preparing");
                Ok(())
            }
            Err(message) => {
                self.report_reload_failure(id, Some(request_id), message);
                Err(message.into())
            }
        }
    }
    pub fn receive_load_result(&mut self, result: super::loader::ScriptLoadResult) {
        let (id, request_id) = match &result {
            Ok(prepared) => (
                ScriptId::parse(prepared.manifest().id.clone()).expect("validated ID"),
                prepared.request_id(),
            ),
            Err(failure) => (failure.id.clone(), failure.request_id),
        };
        if !self
            .load_requests
            .get(&id)
            .is_some_and(|load| load.request_id == request_id)
        {
            return;
        }
        match result {
            Ok(prepared) if self.pending.len() < MAX_PENDING_CHANGES => {
                self.pending
                    .push_back(PendingChange::Install(prepared, true));
                self.report_current_request(id, Some(request_id), "Prepared");
            }
            result => {
                self.load_requests.remove(&id);
                let message = match result {
                    Err(failure) => failure.message,
                    Ok(_) => "script change queue full".into(),
                };
                self.report_reload_failure(id, Some(request_id), &message);
            }
        }
    }
    fn report_reload_failure(&mut self, id: ScriptId, request_id: Option<u64>, text: &str) {
        if let Some(script) = self.active.get_mut(&id) {
            script.reload_error = Some(bounded_text(text, MAX_EVENT_BYTES));
        }
        self.report_failure(id, request_id, text);
    }
    pub fn report_auto_reload_overflow(&mut self, id: ScriptId) {
        if self
            .active
            .get(&id)
            .is_some_and(|script| script.auto_reload)
        {
            self.report_reload_failure(
                id,
                None,
                "auto reload notification queue full; reload delayed",
            );
        }
    }
    pub fn report_loader_overflow(&mut self, dropped: usize) {
        if dropped == 0 {
            return;
        }
        let requests = std::mem::take(&mut self.load_requests);
        for (id, request) in requests {
            self.report_reload_failure(id, Some(request.request_id), &format!("loader result queue overflow ({dropped} dropped); pending load cancelled; retry"));
        }
    }
    fn report_current_request(&self, id: ScriptId, request_id: Option<u64>, text: &str) {
        let (generation, status) = self
            .active
            .get(&id)
            .map_or((ScriptGeneration(0), ScriptStatus::Unloaded), |script| {
                (script.generation, script.status)
            });
        self.report_request(id, generation, status, request_id, text);
    }
    fn report_failure(&self, id: ScriptId, request_id: Option<u64>, text: &str) {
        let script = self.active.get(&id);
        self.events.emit(ScriptEvent {
            request_id,
            id: id.clone(),
            generation: script.map_or(ScriptGeneration(0), |s| s.generation),
            phase: ScriptPhaseId::GamePostSim,
            callback: String::new(),
            kind: ScriptEventKind::Error,
            status: Some(script.map_or(ScriptStatus::Unloaded, |s| s.status)),
            auto_reload: script.is_some_and(|s| s.auto_reload),
            reload_error: script.and_then(|s| s.reload_error.clone()),
            line: None,
            column: None,
            text: text.to_owned(),
        });
    }
    pub fn list(&self) -> Vec<ScriptListing> {
        self.active
            .iter()
            .map(|(id, script)| ScriptListing {
                id: id.clone(),
                generation: script.generation,
                status: script.status,
                callbacks: script.callbacks.clone(),
                has_on_run: script.on_run.is_some(),
                auto_reload: script.auto_reload,
                reload_error: script.reload_error.clone(),
            })
            .collect()
    }
    pub fn pending_loads(&self) -> Vec<(ScriptId, u64)> {
        self.load_requests
            .iter()
            .map(|(id, request)| (id.clone(), request.request_id))
            .collect()
    }
    pub fn apply_pending_changes(&mut self) {
        for _ in 0..MAX_PENDING_CHANGES {
            let Some(change) = self.pending.pop_front() else {
                break;
            };
            match change {
                PendingChange::Install(prepared, tracked) => {
                    let request_id = prepared.request_id();
                    let prepared_id =
                        ScriptId::parse(prepared.manifest().id.clone()).expect("validated ID");
                    if tracked {
                        if !self
                            .load_requests
                            .get(&prepared_id)
                            .is_some_and(|load| load.request_id == request_id)
                        {
                            continue;
                        }
                        self.load_requests.remove(&prepared_id);
                    }
                    let (id, ast, callbacks, on_run, capabilities, replace) = prepared.into_parts();
                    if !replace && self.active.contains_key(&id) {
                        self.report_reload_failure(
                            id,
                            Some(request_id),
                            "load rejected: script already loaded; use reload",
                        );
                        continue;
                    }
                    if !self.active.contains_key(&id) && self.active.len() >= MAX_ACTIVE_SCRIPTS {
                        self.report_reload_failure(
                            id,
                            Some(request_id),
                            "active script limit reached",
                        );
                        continue;
                    }
                    let Some(next) = self.next_generation.checked_add(1) else {
                        self.report_reload_failure(
                            id,
                            Some(request_id),
                            "generation counter exhausted",
                        );
                        continue;
                    };
                    let generation = ScriptGeneration(self.next_generation);
                    self.next_generation = next;
                    self.cancel_runs(&id);
                    let mut scope = Scope::new();
                    scope.push("state", Map::new());

                    let auto_reload = self
                        .active
                        .get(&id)
                        .is_some_and(|script| script.auto_reload);
                    let latest_load_request_id =
                        self.active.get(&id).map_or(request_id, |script| {
                            script.latest_load_request_id.max(request_id)
                        });
                    let deferred_auto_reload = self
                        .active
                        .get_mut(&id)
                        .and_then(|script| script.deferred_auto_reload.take());
                    self.active.insert(
                        id.clone(),
                        ActiveScript {
                            ast,
                            capabilities,
                            scope,
                            generation,
                            callbacks,
                            on_run,
                            status: ScriptStatus::Active,
                            auto_reload,
                            reload_error: None,
                            latest_load_request_id,
                            deferred_auto_reload,
                        },
                    );
                    self.report_request(
                        id,
                        generation,
                        ScriptStatus::Active,
                        Some(request_id),
                        if replace { "Reloaded" } else { "Loaded" },
                    );
                }
                PendingChange::Unload(id, request_id) => {
                    self.cancel_runs(&id);
                    if let Some(old) = self.active.remove(&id) {
                        self.report_request(
                            id,
                            old.generation,
                            ScriptStatus::Unloaded,
                            request_id,
                            "Unloaded",
                        );
                    } else {
                        self.report_request(
                            id,
                            ScriptGeneration(0),
                            ScriptStatus::Unloaded,
                            request_id,
                            "script is not loaded",
                        );
                    }
                }
                PendingChange::Run(id, request_id) => {
                    if let Some(script) = self.active.get(&id) {
                        let rejection = if script.on_run.is_none() {
                            Some("script has no on_run entry point")
                        } else if script.status != ScriptStatus::Active {
                            Some("run rejected: script is disabled")
                        } else if self.runs.len() >= MAX_RUNS_PER_PHASE {
                            Some("run rejected: run queue full")
                        } else {
                            None
                        };
                        if let Some(message) = rejection {
                            self.report_failure(id, request_id, message);
                        } else {
                            self.report_request(
                                id.clone(),
                                script.generation,
                                script.status,
                                request_id,
                                "Queued",
                            );
                            self.runs.push((id, script.generation, request_id));
                        }
                    } else {
                        self.report_failure(id, request_id, "script is not loaded");
                    }
                }
            }
        }
    }
    fn report_request(
        &self,
        id: ScriptId,
        generation: ScriptGeneration,
        status: ScriptStatus,
        request_id: Option<u64>,
        text: &str,
    ) {
        self.events.emit(ScriptEvent {
            request_id,
            id: id.clone(),
            generation,
            phase: ScriptPhaseId::GamePostSim,
            callback: String::new(),
            kind: ScriptEventKind::Status,
            status: Some(status),
            auto_reload: self
                .active
                .get(&id)
                .is_some_and(|script| script.auto_reload),
            reload_error: self
                .active
                .get(&id)
                .and_then(|script| script.reload_error.clone()),
            line: None,
            column: None,
            text: text.to_owned(),
        });
    }
    pub(crate) fn effect_is_current(&self, event: &ScriptEvent) -> bool {
        self.active.get(&event.id).is_some_and(|script| {
            script.generation == event.generation && script.status == ScriptStatus::Active
        })
    }
    pub(crate) fn report_velocity_result(
        &self,
        mut event: ScriptEvent,
        intent_id: u64,
        result: crate::api::ecs::IntentResult,
    ) {
        event.kind = if result == crate::api::ecs::IntentResult::Applied {
            ScriptEventKind::Status
        } else {
            ScriptEventKind::Error
        };
        event.text = format!("vehicle velocity intent {intent_id}: {result:?}");
        self.events.emit(event);
    }
    pub(crate) fn report_effect_cancelled(&self, mut event: ScriptEvent) {
        event.kind = ScriptEventKind::Status;
        event.text = "Cancelled".to_owned();
        self.events.emit(event);
    }
    pub(crate) fn dispatch(
        &mut self,
        snapshot: GameSnapshot,
        available_slots: usize,
    ) -> (Vec<VehicleVelocityEffect>, Vec<ScriptDebugDrawEffect>) {
        let mut effects = Vec::new();
        let mut debug_draws: Vec<ScriptDebugDrawEffect> = Vec::new();
        let capacity = available_slots.min(MAX_VELOCITY_EFFECTS_PER_PHASE);
        let mut runs = std::mem::take(&mut self.runs);
        runs.sort_by(|a, b| a.0.cmp(&b.0));
        // All one-shots precede all recurring callbacks. Duplicate requests run once each.
        for (id, generation, request_id) in runs {
            if let Some(script) = self.active.get_mut(&id) {
                if script.generation == generation {
                    if let Some(function) = script.on_run.clone() {
                        invocation::invoke(
                            &self.engine,
                            &self.events,
                            &id,
                            script,
                            &function,
                            snapshot,
                            request_id,
                            capacity,
                            &mut effects,
                            MAX_DEBUG_DRAWS_PER_PHASE.saturating_sub(debug_draws.len()),
                            MAX_DEBUG_DRAW_WORK_PER_PHASE.saturating_sub(
                                debug_draws
                                    .iter()
                                    .map(|effect| effect.primitive.work_cost())
                                    .sum(),
                            ),
                            &mut debug_draws,
                        );
                    }
                }
            }
        }
        for (id, script) in &mut self.active {
            if let Some(function) = script.callbacks.get(&ScriptPhaseId::GamePostSim).cloned() {
                invocation::invoke(
                    &self.engine,
                    &self.events,
                    id,
                    script,
                    &function,
                    snapshot,
                    None,
                    capacity,
                    &mut effects,
                    MAX_DEBUG_DRAWS_PER_PHASE.saturating_sub(debug_draws.len()),
                    MAX_DEBUG_DRAW_WORK_PER_PHASE.saturating_sub(
                        debug_draws
                            .iter()
                            .map(|effect| effect.primitive.work_cost())
                            .sum(),
                    ),
                    &mut debug_draws,
                );
            }
        }
        effects.retain(|effect| {
            if self.effect_is_current(&effect.origin) {
                true
            } else {
                self.report_effect_cancelled(effect.origin.clone());
                false
            }
        });
        debug_draws.retain(|effect| {
            if self.effect_is_current(&effect.origin) {
                true
            } else {
                self.report_effect_cancelled(effect.origin.clone());
                false
            }
        });
        (effects, debug_draws)
    }
    fn cancel_runs(&mut self, id: &ScriptId) {
        for (queued, generation, request_id) in &self.runs {
            if queued == id {
                self.report_request(
                    id.clone(),
                    *generation,
                    ScriptStatus::Unloaded,
                    *request_id,
                    "Cancelled",
                );
            }
        }
        self.runs.retain(|(queued, _, _)| queued != id);
    }
}
#[cfg(test)]
mod tests {
    use super::super::loader::{LoadScriptRequest, ScriptLoadResult, ScriptLoader};
    use super::*;
    use std::{
        fs,
        path::PathBuf,
        time::{Duration, Instant},
    };

    pub(super) struct PackageRoot(pub PathBuf);
    impl PackageRoot {
        pub(super) fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "madmax-manager-{}-{}",
                std::process::id(),
                console::next_request_id().unwrap()
            ));
            fs::create_dir_all(path.join("example")).unwrap();
            fs::write(path.join("example/manifest.toml"), "id = \"example\"\napi_version = 1\ncapabilities = []\n[callbacks]\n\"game.post_sim\" = \"on_post_sim\"\n").unwrap();
            fs::write(path.join("example/main.rhai"), "fn on_post_sim(ctx) {}\n").unwrap();
            Self(path)
        }
        fn edit(&self, source: &str) {
            fs::write(self.0.join("example/main.rhai"), source).unwrap();
        }
    }
    impl Drop for PackageRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn id() -> ScriptId {
        ScriptId::parse("example").unwrap()
    }
    fn request() -> LoadScriptRequest {
        LoadScriptRequest {
            id: id(),
            request_id: console::next_request_id().unwrap(),
            replace: true,
            snapshot: None,
        }
    }
    fn result(loader: &ScriptLoader) -> ScriptLoadResult {
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(result) = loader.try_result() {
                return result;
            }
            assert!(Instant::now() < until, "loader did not finish");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn setup() -> (
        PackageRoot,
        ScriptLoader,
        ScriptManager,
        console::ConsoleEventReceiver,
    ) {
        let root = PackageRoot::new();
        let loader = ScriptLoader::start(root.0.clone(), RhaiEngineFactory);
        let (sink, events) = event_channel();
        let mut manager = ScriptManager::new(sink);
        manager.request_load(&loader, request()).unwrap();
        manager.receive_load_result(result(&loader));
        manager.apply_pending_changes();
        while events.try_recv().is_ok() {}
        (root, loader, manager, events)
    }
    fn barrier(loader: &ScriptLoader) {
        loader
            .try_discover(super::super::loader::DiscoverScriptsRequest { request_id: None })
            .unwrap();
        let until = Instant::now() + Duration::from_secs(5);
        while loader.try_discovery_result().is_none() {
            assert!(Instant::now() < until);
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn notice(loader: &ScriptLoader) -> LoadScriptRequest {
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(request) = loader.try_auto_reload() {
                return request;
            }
            assert!(Instant::now() < until, "watcher did not notify");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn auto_reload_defaults_off_and_toggle_rejects_unloaded_ids() {
        let (_root, loader, mut manager, events) = setup();
        assert!(!manager.list()[0].auto_reload);
        assert!(manager.list()[0].reload_error.is_none());
        assert!(
            manager
                .set_auto_reload(&ScriptId::parse("missing").unwrap(), true, &loader)
                .is_err()
        );
        let rejected = events.try_recv().unwrap();
        assert_eq!(rejected.severity, console::ConsoleSeverity::Error);
        manager.set_auto_reload(&id(), true, &loader).unwrap();
        assert!(manager.list()[0].auto_reload);
        assert!(events.try_recv().unwrap().auto_reload.unwrap());
        manager.set_auto_reload(&id(), false, &loader).unwrap();
        assert!(!manager.list()[0].auto_reload);
    }
    #[test]
    fn successful_manual_and_automatic_replacements_preserve_toggle() {
        let (_root, loader, mut manager, _events) = setup();
        manager.set_auto_reload(&id(), true, &loader).unwrap();
        let first = manager.list()[0].generation;
        manager.request_load(&loader, request()).unwrap();
        manager.receive_load_result(result(&loader));
        manager.apply_pending_changes();
        assert!(manager.list()[0].auto_reload);
        assert_ne!(manager.list()[0].generation, first);
        let second = manager.list()[0].generation;
        manager.request_auto_reload(&loader, request()).unwrap();
        manager.receive_load_result(result(&loader));
        assert_eq!(
            manager.list()[0].generation,
            second,
            "prepare must not activate"
        );
        manager.apply_pending_changes();
        assert!(manager.list()[0].auto_reload);
        assert_ne!(manager.list()[0].generation, second);
    }
    #[test]
    fn unload_removes_watch_and_a_fresh_load_starts_off() {
        let (root, loader, mut manager, _events) = setup();
        manager.set_auto_reload(&id(), true, &loader).unwrap();
        barrier(&loader);
        manager.request_unload(&loader, id(), None).unwrap();
        manager.apply_pending_changes();
        barrier(&loader);
        root.edit("fn on_post_sim(ctx) { let x = 2; }");
        std::thread::sleep(Duration::from_millis(1200));
        assert!(loader.try_auto_reload().is_none());
        assert!(manager.list().is_empty());
        manager.request_load(&loader, request()).unwrap();
        manager.receive_load_result(result(&loader));
        manager.apply_pending_changes();
        assert!(!manager.list()[0].auto_reload);
    }
    #[test]
    fn unchanged_bad_fingerprint_reports_one_error_and_keeps_generation() {
        let (root, loader, mut manager, events) = setup();
        manager.set_auto_reload(&id(), true, &loader).unwrap();
        barrier(&loader);
        let generation = manager.list()[0].generation;
        root.edit("this is invalid rhai !!!");
        let changed = notice(&loader);
        manager.request_auto_reload(&loader, changed).unwrap();
        manager.receive_load_result(result(&loader));
        manager.apply_pending_changes();
        assert_eq!(manager.list()[0].generation, generation);
        assert!(manager.list()[0].auto_reload);
        assert!(manager.list()[0].reload_error.is_some());
        let mut errors = 0;
        while let Ok(event) = events.try_recv() {
            if event.severity == console::ConsoleSeverity::Error {
                errors += 1;
                assert!(event.reload_error.is_some());
            }
        }
        assert_eq!(errors, 1);
        std::thread::sleep(Duration::from_millis(1200));
        assert!(loader.try_auto_reload().is_none());
        assert!(events.try_recv().is_err());
    }
    #[test]
    fn pending_manual_load_wins_over_newer_automatic_notice() {
        let (_root, loader, mut manager, _events) = setup();
        manager.set_auto_reload(&id(), true, &loader).unwrap();
        let manual = request();
        let automatic = request();
        manager.request_load(&loader, manual.clone()).unwrap();
        manager.request_auto_reload(&loader, automatic).unwrap();
        assert_eq!(manager.pending_loads(), vec![(id(), manual.request_id)]);
        manager.receive_load_result(result(&loader));
        manager.apply_pending_changes();
        assert!(manager.list()[0].auto_reload);
    }

    #[test]
    fn later_stable_notice_waits_for_manual_result_then_converges() {
        let (root, loader, mut manager, _events) = setup();
        root.edit("fn on_post_sim(ctx) { let manual_a = true; }");
        manager.set_auto_reload(&id(), true, &loader).unwrap();
        barrier(&loader);

        manager.request_load(&loader, request()).unwrap();
        let manual_result = result(&loader);
        root.edit("fn on_post_sim(ctx) { let automatic_b = true; }");
        let later_notice = notice(&loader);
        let original_generation = manager.list()[0].generation;

        manager.request_auto_reload(&loader, later_notice).unwrap();
        manager.receive_load_result(manual_result);
        manager.apply_pending_changes();
        assert_eq!(manager.list()[0].generation.0, original_generation.0 + 1);
        assert!(manager.pending_loads().is_empty());

        manager.process_deferred_auto_reloads(&loader);
        assert_eq!(
            manager.pending_loads().len(),
            1,
            "later notice must be retained"
        );
        manager.receive_load_result(result(&loader));
        manager.apply_pending_changes();
        assert_eq!(manager.list()[0].generation.0, original_generation.0 + 2);
        assert!(manager.pending_loads().is_empty());
    }
    #[test]
    fn manual_request_cancels_prepared_auto_without_activating_it() {
        let (_root, loader, mut manager, _events) = setup();
        manager.set_auto_reload(&id(), true, &loader).unwrap();
        let generation = manager.list()[0].generation;
        let manual = request(); // UI ID may precede a worker-issued notice.
        manager.request_auto_reload(&loader, request()).unwrap();
        manager.receive_load_result(result(&loader));
        manager.request_load(&loader, manual.clone()).unwrap();
        assert_eq!(manager.list()[0].generation, generation);
        manager.apply_pending_changes();
        assert_eq!(manager.list()[0].generation, generation);
        manager.receive_load_result(result(&loader));
        manager.apply_pending_changes();
        assert_eq!(manager.list()[0].generation.0, generation.0 + 1);
    }
    #[test]
    fn off_cancels_prepared_automatic_replacement() {
        let (_root, loader, mut manager, _events) = setup();
        manager.set_auto_reload(&id(), true, &loader).unwrap();
        let generation = manager.list()[0].generation;
        manager.request_auto_reload(&loader, request()).unwrap();
        manager.receive_load_result(result(&loader));
        manager.set_auto_reload(&id(), false, &loader).unwrap();
        manager.apply_pending_changes();
        assert_eq!(manager.list()[0].generation, generation);
        assert!(!manager.list()[0].auto_reload);
    }
    #[test]
    fn edits_during_automatic_preparation_coalesce_to_latest_notice() {
        let (_root, loader, mut manager, _events) = setup();
        manager.set_auto_reload(&id(), true, &loader).unwrap();
        let first = request();
        manager.request_auto_reload(&loader, first.clone()).unwrap();
        let ready = result(&loader);
        manager.request_auto_reload(&loader, request()).unwrap();
        let latest = request();
        manager
            .request_auto_reload(&loader, latest.clone())
            .unwrap();
        assert_eq!(manager.pending_loads(), vec![(id(), first.request_id)]);
        manager.receive_load_result(ready);
        manager.apply_pending_changes();
        manager.process_deferred_auto_reloads(&loader);
        assert_eq!(manager.pending_loads(), vec![(id(), latest.request_id)]);
        manager.receive_load_result(result(&loader));
        manager.apply_pending_changes();
        assert!(manager.pending_loads().is_empty());
    }

    #[test]
    fn automatic_activation_failure_keeps_generation_and_sets_row_error_once() {
        let (_root, loader, mut manager, events) = setup();
        manager.set_auto_reload(&id(), true, &loader).unwrap();
        let generation = manager.list()[0].generation;
        manager.next_generation = u64::MAX;
        manager.request_auto_reload(&loader, request()).unwrap();
        manager.receive_load_result(result(&loader));
        manager.apply_pending_changes();
        assert_eq!(manager.list()[0].generation, generation);
        assert!(manager.list()[0].reload_error.is_some());
        let mut errors = 0;
        while let Ok(event) = events.try_recv() {
            if event.severity == console::ConsoleSeverity::Error {
                errors += 1;
                assert!(event.reload_error.is_some());
            }
        }
        assert_eq!(errors, 1);
    }

    #[test]
    fn automatic_failure_recovers_after_valid_edit_and_clears_row_error() {
        let (root, loader, mut manager, _events) = setup();
        manager.set_auto_reload(&id(), true, &loader).unwrap();
        barrier(&loader);
        let generation = manager.list()[0].generation;
        root.edit("invalid rhai !!!");
        manager
            .request_auto_reload(&loader, notice(&loader))
            .unwrap();
        manager.receive_load_result(result(&loader));
        assert!(manager.list()[0].reload_error.is_some());
        root.edit("fn on_post_sim(ctx) { let restored = true; }");
        manager
            .request_auto_reload(&loader, notice(&loader))
            .unwrap();
        manager.receive_load_result(result(&loader));
        manager.apply_pending_changes();
        assert_ne!(manager.list()[0].generation, generation);
        assert!(manager.list()[0].auto_reload);
        assert!(manager.list()[0].reload_error.is_none());
    }

    #[test]
    fn stale_automatic_result_cannot_replace_later_manual_request() {
        let (_root, loader, mut manager, _events) = setup();
        manager.set_auto_reload(&id(), true, &loader).unwrap();
        manager.request_auto_reload(&loader, request()).unwrap();
        let stale = result(&loader);
        let manual = request();
        manager.request_load(&loader, manual.clone()).unwrap();
        manager.receive_load_result(stale);
        assert_eq!(manager.pending_loads(), vec![(id(), manual.request_id)]);
        manager.receive_load_result(result(&loader));
        manager.apply_pending_changes();
        assert!(manager.pending_loads().is_empty());
    }
}
