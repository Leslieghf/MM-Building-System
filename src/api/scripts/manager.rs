//! Callback-bound script lifecycle, scheduling, and effect ownership.
//!
//! # Lifecycle boundary
//!
//! The manager is mutated only while preparing a game callback. Compilation
//! results and UI lifecycle commands enter through bounded queues and are
//! applied at that boundary; a loaded package is identified by a generation so
//! stale results cannot replace a newer request.
//!
//! # Effects and state
//!
//! Each invocation receives its own Rhai scope and effect collectors. Persistent
//! state is copied between generations only after depth and value-count
//! validation. Effects are bounded per invocation and per phase, then exported
//! as typed intents for the native adapter to validate.
mod invocation;
use super::{engine::RhaiEngineFactory, *};
use crate::api::ecs::GameSnapshot;
use crate::api::rhai::bindings::{
    DebugPrimitive, GamePostSimContext, InvocationDebugDraw, InvocationLog, InvocationRaycast,
    InvocationSpawn, InvocationVehicleWrite, RaycastTicketStore,
};
use bevy_ecs::prelude::Resource;
use rhai::{AST, CallFnOptions, Dynamic, Engine, Map, Scope};
use std::collections::{BTreeMap, VecDeque};

/// Maximum number of simultaneously loaded package generations.
pub const MAX_ACTIVE_SCRIPTS: usize = 16;
/// Maximum queued lifecycle changes awaiting a callback boundary.
pub const MAX_PENDING_CHANGES: usize = 32;
/// Maximum manually queued `on_run` callbacks per phase.
pub const MAX_RUNS_PER_PHASE: usize = 16;
/// Maximum total callbacks invoked in one phase.
pub const MAX_CALLBACKS_PER_PHASE: usize = MAX_ACTIVE_SCRIPTS + MAX_RUNS_PER_PHASE;
/// Maximum vehicle velocity effects accepted per phase.
pub const MAX_VELOCITY_EFFECTS_PER_PHASE: usize = 32;
/// Maximum debug primitives accepted from one callback.
pub const MAX_DEBUG_DRAWS_PER_INVOCATION: usize = 32;
/// Maximum debug primitives accepted in one phase.
pub const MAX_DEBUG_DRAWS_PER_PHASE: usize = 128;
/// Maximum estimated geometry work accepted from one callback.
pub const MAX_DEBUG_DRAW_WORK_PER_INVOCATION: usize = 256;
/// Maximum estimated geometry work accepted in one phase.
pub const MAX_DEBUG_DRAW_WORK_PER_PHASE: usize = 512;
/// Maximum native ray queries dispatched from scripts in one game callback.
pub const MAX_RAYCAST_REQUESTS_PER_PHASE: usize = 8;
/// Maximum catalog-approved spawn requests emitted from scripts per callback.
pub const MAX_SCRIPT_SPAWNS_PER_PHASE: usize = 1;
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
pub(crate) struct ScriptSpawnEffect {
    pub request_id: u64,
    pub catalog_id: String,
    pub asset_key: String,
    pub matrix: crate::api::game::abi::types::Matrix4x4,
    pub origin: ScriptEvent,
}
/// Maximum top-level values retained in a script's persistent state map.
pub const MAX_STATE_VALUES: usize = 1024;
/// Maximum nested state container depth.
pub const MAX_STATE_DEPTH: usize = 16;

pub use super::package::PreparedScript;
struct ActiveScript {
    capabilities: Vec<String>,
    version: super::package::PackageVersion,
    requires: Vec<super::package::PackageRequirement>,
    conflicts: Vec<super::package::PackageConflict>,
    workbench_tab: Option<super::package::WorkbenchTabMetadata>,
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

struct InstallCandidate {
    id: ScriptId,
    ast: AST,
    callbacks: BTreeMap<ScriptPhaseId, String>,
    on_run: Option<String>,
    capabilities: Vec<String>,
    version: super::package::PackageVersion,
    requires: Vec<super::package::PackageRequirement>,
    conflicts: Vec<super::package::PackageConflict>,
    workbench_tab: Option<super::package::WorkbenchTabMetadata>,
    replace: bool,
}

fn take_panel_events(
    events: &mut Vec<ScriptPanelEvent>,
    id: &ScriptId,
    generation: ScriptGeneration,
) -> Vec<ScriptPanelEvent> {
    let mut selected = Vec::new();
    let mut remaining = Vec::with_capacity(events.len());
    for event in events.drain(..) {
        if &event.script_id == id && event.generation == generation {
            selected.push(event);
        } else {
            remaining.push(event);
        }
    }
    *events = remaining;
    selected
}

fn dependency_availability(
    requirements: &[super::package::PackageRequirement],
    active: &BTreeMap<ScriptId, semver::Version>,
) -> BTreeMap<String, bool> {
    requirements
        .iter()
        .map(|requirement| {
            let available = match requirement.kind {
                super::package::PackageRequirementKind::HostApi => {
                    super::package_graph::host_api_version(&requirement.id)
                        .is_some_and(|version| requirement.version.matches(&version))
                }
                super::package::PackageRequirementKind::Package => {
                    ScriptId::parse(requirement.id.clone())
                        .ok()
                        .and_then(|id| active.get(&id))
                        .is_some_and(|version| requirement.version.matches(version))
                }
            };
            (requirement.id.clone(), available)
        })
        .collect()
}
enum PendingChange {
    Install(PreparedScript, bool),
    Unload(ScriptId, Option<u64>),
    Run(ScriptId, Option<u64>),
}
#[derive(Debug, Clone)]
/// Snapshot of one active package generation for UI and status reporting.
pub struct ScriptListing {
    /// Package identifier.
    pub id: ScriptId,
    /// Active generation number.
    pub generation: ScriptGeneration,
    /// Whether callbacks are active, disabled, or unloaded.
    pub status: ScriptStatus,
    /// Callback functions registered by phase.
    pub callbacks: BTreeMap<ScriptPhaseId, String>,
    /// Whether a manual `on_run` callback is registered.
    pub has_on_run: bool,
    /// Whether file-change reload is enabled.
    pub auto_reload: bool,
    /// Latest reload failure, if any.
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
///
/// Use the `stage_*` methods when the caller already owns a validated package
/// or wants to enqueue a lifecycle action directly. Use the `request_*`
/// methods to compile from the configured package directory on the loader
/// worker. In both cases, changes become visible only when
/// [`apply_pending_changes`](ScriptManager::apply_pending_changes) runs.
pub struct ScriptManager {
    engine: Engine,
    active: BTreeMap<ScriptId, ActiveScript>,
    pending: VecDeque<PendingChange>,
    runs: Vec<(ScriptId, ScriptGeneration, Option<u64>)>,
    next_generation: u64,
    last_load_request_id: u64,
    load_requests: BTreeMap<ScriptId, PendingLoad>,
    events: ScriptEventSink,
    raycast_tickets: RaycastTicketStore,
    panel_publications: BTreeMap<ScriptId, ScriptPanelPublication>,
    pending_panel_events: Vec<ScriptPanelEvent>,
}
impl Default for ScriptManager {
    fn default() -> Self {
        // Runtime transport supplies a retained receiver when constructing GameRuntime.
        let (events, _) = event_channel();
        Self::new(events)
    }
}
impl ScriptManager {
    /// Create a script manager publishing lifecycle and callback events to `events`.
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
            raycast_tickets: RaycastTicketStore::default(),
            panel_publications: BTreeMap::new(),
            pending_panel_events: Vec::new(),
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
    /// Queue a validated package for activation at the next change boundary.
    ///
    /// # Errors
    ///
    /// Returns an error if the bounded lifecycle queue is full.
    pub fn stage_prepared(&mut self, prepared: PreparedScript) -> Result<(), String> {
        self.stage(PendingChange::Install(prepared, false))
            .map_err(str::to_owned)
    }
    /// Queue package unload without an external request identifier.
    ///
    /// The unload takes effect at the next change boundary.
    ///
    /// # Errors
    ///
    /// Returns an error when the bounded lifecycle queue is full.
    pub fn stage_unload(&mut self, id: ScriptId) -> Result<(), &'static str> {
        self.stage_unload_request(id, None)
    }
    /// Queue package unload and correlate its status event with a request.
    ///
    /// Any outstanding load for this ID is cancelled when the unload is staged.
    ///
    /// # Errors
    ///
    /// Returns an error when the bounded lifecycle queue is full.
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
    /// Queue one manual `on_run` invocation.
    ///
    /// The callback executes during the next applicable script phase.
    ///
    /// # Errors
    ///
    /// Returns an error when the bounded lifecycle queue is full.
    pub fn stage_run(&mut self, id: ScriptId) -> Result<(), &'static str> {
        self.stage_run_request(id, None)
    }
    /// Queue one manual `on_run` invocation with request correlation.
    ///
    /// # Errors
    ///
    /// Returns an error when the bounded lifecycle queue is full.
    pub fn stage_run_request(
        &mut self,
        id: ScriptId,
        request_id: Option<u64>,
    ) -> Result<(), &'static str> {
        self.stage(PendingChange::Run(id, request_id))
    }
    /// Enable or disable file-change reload for an active package.
    ///
    /// Disabling reload cancels retained automatic work. It does not unload the
    /// active generation.
    ///
    /// # Errors
    ///
    /// Returns an error if the package is not active or the worker cannot
    /// update its watch request.
    pub fn set_auto_reload(
        &mut self,
        id: &ScriptId,
        enabled: bool,
        loader: &super::loader::ScriptLoader,
    ) -> Result<(), &'static str> {
        self.set_auto_reload_request(id, enabled, loader, None)
    }
    /// Set file-change reload and correlate any status event with a request.
    ///
    /// # Errors
    ///
    /// Returns an error if the package is not active or the worker request is
    /// rejected. The previous setting remains active on rejection.
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
    /// Disable file watching and queue unload through the worker-backed loader.
    ///
    /// # Errors
    ///
    /// Returns an error when the lifecycle queue or loader request queue is full,
    /// or when the loader has stopped. A rejected unload leaves the active watch
    /// enabled.
    pub fn request_unload(
        &mut self,
        loader: &super::loader::ScriptLoader,
        id: ScriptId,
        request_id: Option<u64>,
    ) -> Result<(), &'static str> {
        if let Some(target) = self.active.get(&id) {
            let dependents = self
                .active
                .iter()
                .filter(|(dependent_id, script)| {
                    *dependent_id != &id
                        && script.requires.iter().any(|requirement| {
                            requirement.kind == super::package::PackageRequirementKind::Package
                                && !requirement.optional
                                && requirement.id == id.as_str()
                                && requirement.version.matches(&target.version.0)
                        })
                })
                .map(|(dependent_id, _)| dependent_id.as_str().to_owned())
                .collect::<Vec<_>>();
            if !dependents.is_empty() {
                self.report_failure(
                    id.clone(),
                    request_id,
                    &format!(
                        "unload blocked; required by active packages: {}",
                        dependents.join(", ")
                    ),
                );
                return Err("unload blocked by active required dependents");
            }
        }
        // Check staging capacity before accepting a worker-side watch removal.
        if self.pending.len() >= MAX_PENDING_CHANGES {
            return self.stage(PendingChange::Unload(id, request_id));
        }
        // Rejected cleanup must leave a loaded script attached to its watch.
        if self
            .active
            .get(&id)
            .is_some_and(|script| script.auto_reload)
            && let Err(message) = loader.set_auto_reload(id.clone(), false)
        {
            self.report_failure(id, request_id, message);
            return Err(message);
        }
        self.stage_unload_request(id, request_id)
    }
    fn cancel_load(&mut self, id: &ScriptId, text: &str) {
        if let Some(cancelled) = self.load_requests.remove(id) {
            self.report_current_request(id.clone(), Some(cancelled.request_id), text);
        }
    }
    /// Submit a package load or replacement to the worker-backed loader.
    ///
    /// A manual request supersedes older automatic reload work for the same ID.
    /// Compilation happens on the worker; activation is deferred to a callback
    /// boundary.
    ///
    /// # Errors
    ///
    /// Returns an error for a non-increasing request ID, duplicate active load,
    /// pending-load limit, or rejected loader request.
    pub fn request_load(
        &mut self,
        loader: &super::loader::ScriptLoader,
        request: super::loader::LoadScriptRequest,
    ) -> Result<(), String> {
        self.request_load_with_origin(loader, request, LoadOrigin::Manual)
    }
    /// Submit a file-change replacement while retaining generation ordering.
    ///
    /// Stale notices and notices for unloaded or non-watched packages are
    /// ignored. If another load is in flight, the newest notice is retained.
    ///
    /// # Errors
    ///
    /// Returns an error if worker capacity is unavailable.
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
    /// Submit deferred file-change notices when lifecycle capacity is available.
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
    /// Stage a worker result for activation or report its load failure.
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
    /// Report that a file-change notice was dropped for a package.
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
    /// Report the number of worker results dropped due to a full result queue.
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
    /// Return owned status rows for active and disabled packages.
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
    /// Return package IDs and request IDs for loads still in flight.
    pub fn pending_loads(&self) -> Vec<(ScriptId, u64)> {
        self.load_requests
            .iter()
            .map(|(id, request)| (id.clone(), request.request_id))
            .collect()
    }
    /// Apply queued install, unload, and run changes at the callback boundary.
    pub fn apply_pending_changes(&mut self) {
        for _ in 0..MAX_PENDING_CHANGES {
            let Some(change) = self.pending.pop_front() else {
                break;
            };
            match change {
                PendingChange::Install(prepared, tracked) => {
                    self.install_prepared(prepared, tracked)
                }
                PendingChange::Unload(id, request_id) => self.unload_script(id, request_id),
                PendingChange::Run(id, request_id) => self.queue_script_run(id, request_id),
            }
        }
    }

    fn install_prepared(&mut self, prepared: PreparedScript, tracked: bool) {
        let request_id = prepared.request_id();
        let prepared_id = ScriptId::parse(prepared.manifest().id.clone()).expect("validated ID");
        if tracked && !self.is_latest_load(&prepared_id, request_id) {
            return;
        }
        if tracked {
            self.load_requests.remove(&prepared_id);
        }
        let (
            id,
            ast,
            callbacks,
            on_run,
            capabilities,
            version,
            requires,
            conflicts,
            workbench_tab,
            dependencies,
            replace,
        ) = prepared.into_parts();
        let mut candidates = Vec::with_capacity(dependencies.len() + 1);
        for dependency in dependencies {
            let (
                id,
                ast,
                callbacks,
                on_run,
                capabilities,
                version,
                requires,
                conflicts,
                workbench_tab,
            ) = dependency.into_parts();
            candidates.push(InstallCandidate {
                id,
                ast,
                callbacks,
                on_run,
                capabilities,
                version,
                requires,
                conflicts,
                workbench_tab,
                replace: false,
            });
        }
        candidates.push(InstallCandidate {
            id: id.clone(),
            ast,
            callbacks,
            on_run,
            capabilities,
            version,
            requires,
            conflicts,
            workbench_tab,
            replace,
        });
        if let Err(error) = self.validate_install_graph(&candidates) {
            self.report_reload_failure(id, Some(request_id), &error);
            return;
        }
        let new_count = candidates
            .iter()
            .filter(|candidate| !self.active.contains_key(&candidate.id) || candidate.replace)
            .count();
        if self.next_generation.checked_add(new_count as u64).is_none() {
            self.report_reload_failure(id, Some(request_id), "generation counter exhausted");
            return;
        }
        for candidate in candidates {
            if self.active.contains_key(&candidate.id) && !candidate.replace {
                continue;
            }
            let Some(generation) = self.allocate_generation(&candidate.id, request_id) else {
                unreachable!("generation range was prevalidated")
            };
            self.activate_script(
                candidate.id,
                candidate.ast,
                candidate.callbacks,
                candidate.on_run,
                candidate.capabilities,
                candidate.version,
                candidate.requires,
                candidate.conflicts,
                candidate.workbench_tab,
                candidate.replace,
                request_id,
                generation,
            );
        }
    }

    fn validate_install_graph(&self, candidates: &[InstallCandidate]) -> Result<(), String> {
        let mut by_id = BTreeMap::new();
        for candidate in candidates {
            if by_id.insert(candidate.id.clone(), candidate).is_some() {
                return Err(format!(
                    "dependency graph contains duplicate package {}",
                    candidate.id
                ));
            }
            if let Some(active) = self.active.get(&candidate.id) {
                if candidate.replace {
                    continue;
                }
                if active.version.0 != candidate.version.0 {
                    return Err(format!(
                        "active dependency {} is version {}, prepared package is {}",
                        candidate.id, active.version.0, candidate.version.0
                    ));
                }
            }
        }
        for candidate in candidates {
            if candidate.id == candidates.last().expect("root is last").id {
                if self.active.contains_key(&candidate.id) && !candidate.replace {
                    return Err(format!(
                        "package {} is already loaded; use reload",
                        candidate.id
                    ));
                }
            } else if self.active.contains_key(&candidate.id) {
                continue;
            }
            for requirement in &candidate.requires {
                match requirement.kind {
                    super::package::PackageRequirementKind::HostApi => {
                        let available = super::package_graph::host_api_version(&requirement.id)
                            .is_some_and(|version| requirement.version.matches(&version));
                        if !available && !requirement.optional {
                            return Err(format!(
                                "package {} requires unavailable host API {} {}",
                                candidate.id, requirement.id, requirement.version
                            ));
                        }
                    }
                    super::package::PackageRequirementKind::Package => {
                        let dependency = ScriptId::parse(requirement.id.clone())
                            .map_err(|_| "invalid package dependency ID".to_owned())?;
                        let active_version =
                            self.active.get(&dependency).map(|script| &script.version.0);
                        let prepared_version =
                            by_id.get(&dependency).map(|script| &script.version.0);
                        let version = active_version.or(prepared_version);
                        if !requirement.optional
                            && !version.is_some_and(|version| requirement.version.matches(version))
                        {
                            return Err(format!(
                                "package {} requires {} {}, but no compatible active or prepared package is available",
                                candidate.id, requirement.id, requirement.version
                            ));
                        }
                    }
                }
            }
            for conflict in &candidate.conflicts {
                let conflict_id = ScriptId::parse(conflict.id.clone())
                    .map_err(|_| "invalid package conflict ID".to_owned())?;
                let active_version = self
                    .active
                    .get(&conflict_id)
                    .map(|script| &script.version.0);
                let prepared_version = by_id.get(&conflict_id).map(|script| &script.version.0);
                if active_version
                    .or(prepared_version)
                    .is_some_and(|version| conflict.version.matches(version))
                {
                    return Err(format!(
                        "package {} conflicts with {}",
                        candidate.id, conflict.id
                    ));
                }
            }
            for (active_id, active) in &self.active {
                if active_id == &candidate.id && candidate.replace {
                    continue;
                }
                if active.conflicts.iter().any(|conflict| {
                    conflict.id == candidate.id.as_str()
                        && conflict.version.matches(&candidate.version.0)
                }) {
                    return Err(format!(
                        "active package {active_id} conflicts with {}",
                        candidate.id
                    ));
                }
                if active.requires.iter().any(|requirement| {
                    requirement.kind == super::package::PackageRequirementKind::Package
                        && !requirement.optional
                        && requirement.id == candidate.id.as_str()
                        && !requirement.version.matches(&candidate.version.0)
                }) {
                    return Err(format!(
                        "active dependent {active_id} requires a different version of {}",
                        candidate.id
                    ));
                }
            }
        }
        let added = candidates
            .iter()
            .filter(|candidate| !self.active.contains_key(&candidate.id))
            .count();
        if self.active.len() + added > MAX_ACTIVE_SCRIPTS {
            return Err("active script limit reached for dependency graph".into());
        }
        Ok(())
    }

    fn is_latest_load(&self, id: &ScriptId, request_id: u64) -> bool {
        self.load_requests
            .get(id)
            .is_some_and(|load| load.request_id == request_id)
    }

    fn allocate_generation(&mut self, id: &ScriptId, request_id: u64) -> Option<ScriptGeneration> {
        let Some(next) = self.next_generation.checked_add(1) else {
            self.report_reload_failure(
                id.clone(),
                Some(request_id),
                "generation counter exhausted",
            );
            return None;
        };
        let generation = ScriptGeneration(self.next_generation);
        self.next_generation = next;
        Some(generation)
    }

    fn activate_script(
        &mut self,
        id: ScriptId,
        ast: AST,
        callbacks: BTreeMap<ScriptPhaseId, String>,
        on_run: Option<String>,
        capabilities: Vec<String>,
        version: super::package::PackageVersion,
        requires: Vec<super::package::PackageRequirement>,
        conflicts: Vec<super::package::PackageConflict>,
        workbench_tab: Option<super::package::WorkbenchTabMetadata>,
        replace: bool,
        request_id: u64,
        generation: ScriptGeneration,
    ) {
        self.cancel_runs(&id);
        self.raycast_tickets.remove_owner(&id);
        self.panel_publications.remove(&id);
        self.pending_panel_events
            .retain(|event| event.script_id != id);
        let mut scope = Scope::new();
        scope.push("state", Map::new());
        let auto_reload = self
            .active
            .get(&id)
            .is_some_and(|script| script.auto_reload);
        let latest_load_request_id = self.active.get(&id).map_or(request_id, |script| {
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
                version,
                requires,
                conflicts,
                workbench_tab: workbench_tab.clone(),
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
        self.panel_publications.insert(
            id.clone(),
            ScriptPanelPublication {
                script_id: id.clone(),
                generation,
                workbench_tab,
                panels: Vec::new(),
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

    fn unload_script(&mut self, id: ScriptId, request_id: Option<u64>) {
        if let Some(target) = self.active.get(&id) {
            let dependents = self
                .active
                .iter()
                .filter(|(dependent_id, _)| *dependent_id != &id)
                .filter(|(_, script)| {
                    script.requires.iter().any(|requirement| {
                        requirement.kind == super::package::PackageRequirementKind::Package
                            && !requirement.optional
                            && requirement.id == id.as_str()
                            && requirement.version.matches(&target.version.0)
                    })
                })
                .map(|(dependent_id, _)| dependent_id.as_str().to_owned())
                .collect::<Vec<_>>();
            if !dependents.is_empty() {
                self.report_failure(
                    id,
                    request_id,
                    &format!(
                        "unload blocked; required by active packages: {}",
                        dependents.join(", ")
                    ),
                );
                return;
            }
        }
        self.cancel_runs(&id);
        self.raycast_tickets.remove_owner(&id);
        self.panel_publications.remove(&id);
        self.pending_panel_events
            .retain(|event| event.script_id != id);
        let generation = self
            .active
            .remove(&id)
            .map_or(ScriptGeneration(0), |script| script.generation);
        let text = if generation.0 == 0 {
            "script is not loaded"
        } else {
            "Unloaded"
        };
        self.report_request(id, generation, ScriptStatus::Unloaded, request_id, text);
    }

    fn queue_script_run(&mut self, id: ScriptId, request_id: Option<u64>) {
        let Some(script) = self.active.get(&id) else {
            self.report_failure(id, request_id, "script is not loaded");
            return;
        };
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
            return;
        }
        self.report_request(
            id.clone(),
            script.generation,
            script.status,
            request_id,
            "Queued",
        );
        self.runs.push((id, script.generation, request_id));
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
        source_module: &str,
    ) {
        event.kind = if result == crate::api::ecs::IntentResult::Applied {
            ScriptEventKind::Status
        } else {
            ScriptEventKind::Error
        };
        event.text = format!("vehicle velocity intent {intent_id}: {result:?}");
        self.events.emit_from_rust_module(event, source_module);
    }
    pub(crate) fn report_effect_cancelled(&self, mut event: ScriptEvent) {
        event.kind = ScriptEventKind::Status;
        event.text = "Cancelled".to_owned();
        self.events.emit(event);
    }
    pub(crate) fn report_spawn_result(
        &self,
        mut event: ScriptEvent,
        catalog_id: Option<&str>,
        request_id: u64,
        native_id: Option<crate::api::game::abi::types::HashId>,
        result: crate::api::ecs::SpawnRequestResult,
        source_module: &str,
    ) {
        use crate::api::ecs::SpawnRequestResult as Result;

        event.kind = match result {
            Result::NativeCallSubmitted | Result::SpawnObserved => ScriptEventKind::Status,
            _ => ScriptEventKind::Error,
        };
        let item = catalog_id.unwrap_or("unknown catalog item");
        let native = native_id
            .map(|id| format!(" | native {:08X}", id.raw()))
            .unwrap_or_default();
        let stage = match result {
            Result::NativeCallSubmitted => "submitted; awaiting game observation",
            Result::SpawnObserved => "spawn observed",
            Result::SpawnFailedObserved => "game reported spawn failure",
            other => {
                event.text = format!("SPAWN: {item} | {other:?} | request {request_id}{native}");
                self.events.emit_from_rust_module(event, source_module);
                return;
            }
        };
        event.text = format!("SPAWN: {item} | {stage} | request {request_id}{native}");
        self.events.emit_from_rust_module(event, source_module);
    }
    pub(crate) fn dispatch(
        &mut self,
        snapshot: GameSnapshot,
        available_slots: usize,
    ) -> (
        Vec<VehicleVelocityEffect>,
        Vec<ScriptDebugDrawEffect>,
        Vec<ScriptRaycastRequest>,
        Vec<ScriptSpawnEffect>,
        Vec<ScriptPanelPublication>,
    ) {
        let mut effects = Vec::new();
        let mut debug_draws: Vec<ScriptDebugDrawEffect> = Vec::new();
        let mut raycasts = Vec::new();
        let mut spawns = Vec::new();
        let mut panel_updates = Vec::new();
        let mut panel_events = std::mem::take(&mut self.pending_panel_events);
        self.raycast_tickets.expire(snapshot.callback_sequence);
        let capacity = available_slots.min(MAX_VELOCITY_EFFECTS_PER_PHASE);
        self.dispatch_queued_runs(
            snapshot,
            capacity,
            &mut effects,
            &mut debug_draws,
            &mut raycasts,
            &mut spawns,
            &mut panel_updates,
        );
        self.dispatch_frame_callbacks(
            snapshot,
            capacity,
            &mut effects,
            &mut debug_draws,
            &mut raycasts,
            &mut spawns,
            &mut panel_events,
            &mut panel_updates,
        );
        self.retain_current_effects(&mut effects, &mut debug_draws);
        spawns.retain(|effect| self.retain_effect(&effect.origin));
        raycasts.retain(|request| {
            if self.effect_is_current(&request.source) {
                true
            } else {
                self.raycast_tickets.remove_ticket(request.ticket);
                self.report_effect_cancelled(request.source.clone());
                false
            }
        });
        for update in panel_updates {
            let mut update = update;
            update.workbench_tab = self
                .active
                .get(&update.script_id)
                .filter(|script| script.generation == update.generation)
                .and_then(|script| script.workbench_tab.clone());
            self.panel_publications
                .insert(update.script_id.clone(), update);
        }
        self.panel_publications.retain(|id, publication| {
            self.active.get(id).is_some_and(|script| {
                script.status == ScriptStatus::Active && script.generation == publication.generation
            })
        });
        (
            effects,
            debug_draws,
            raycasts,
            spawns,
            self.panel_publications.values().cloned().collect(),
        )
    }

    pub(crate) fn receive_panel_event(&mut self, event: ScriptPanelEvent) {
        if !self.active.get(&event.script_id).is_some_and(|script| {
            script.generation == event.generation
                && script.status == ScriptStatus::Active
                && script
                    .capabilities
                    .iter()
                    .any(|capability| capability == "ui.panels")
        }) {
            return;
        }
        let catalog_selection = event.script_id.as_str() == "debug_spawn_menu"
            && event.panel_id == "spawn_workbench"
            && event.widget_id == "catalog_selection"
            && self
                .panel_publications
                .get(&event.script_id)
                .is_some_and(|publication| {
                    publication.generation == event.generation
                        && publication
                            .panels
                            .iter()
                            .any(|panel| panel.id == event.panel_id)
                })
            && matches!(&event.value, ScriptPanelValue::Text(catalog_id)
                if crate::api::scripts::spawn_catalog::is_spawnable(catalog_id));
        let valid_event = catalog_selection
            || self
                .panel_publications
                .get(&event.script_id)
                .filter(|publication| publication.generation == event.generation)
                .and_then(|publication| {
                    publication
                        .panels
                        .iter()
                        .find(|panel| panel.id == event.panel_id)
                })
                .and_then(|panel| {
                    panel
                        .widgets
                        .iter()
                        .find(|widget| widget.id == event.widget_id)
                })
                .is_some_and(|widget| match (&widget.kind, &event.value) {
                    (ScriptPanelWidgetKind::Checkbox { .. }, ScriptPanelValue::Bool(_)) => true,
                    (ScriptPanelWidgetKind::Status { .. }, _) => false,
                    (ScriptPanelWidgetKind::Button { .. }, ScriptPanelValue::Bool(true)) => true,
                    (
                        ScriptPanelWidgetKind::Slider { min, max, .. },
                        ScriptPanelValue::Number(value),
                    )
                    | (
                        ScriptPanelWidgetKind::Number { min, max, .. },
                        ScriptPanelValue::Number(value),
                    ) => value.is_finite() && value >= min && value <= max,
                    (
                        ScriptPanelWidgetKind::Select { options, .. },
                        ScriptPanelValue::Text(value),
                    ) => options.iter().any(|option| option == value),
                    _ => false,
                });
        if !valid_event {
            return;
        }
        if self.pending_panel_events.len() >= super::PANEL_EVENT_QUEUE_CAPACITY {
            if let Some(existing) = self.pending_panel_events.iter_mut().find(|existing| {
                existing.script_id == event.script_id
                    && existing.generation == event.generation
                    && existing.panel_id == event.panel_id
                    && existing.widget_id == event.widget_id
            }) {
                *existing = event;
            }
            return;
        }
        self.pending_panel_events.push(event);
    }

    pub(crate) fn complete_script_raycast(&mut self, result: ScriptRaycastCompletion) -> bool {
        if !self.effect_is_current(&result.source) {
            self.report_effect_cancelled(result.source);
            return false;
        }
        self.raycast_tickets.complete(result)
    }

    pub(crate) fn latest_reticle_hit_position(&self) -> Option<Option<[f32; 3]>> {
        self.raycast_tickets.latest_result_for("reticle_ray")
    }

    fn dispatch_queued_runs(
        &mut self,
        snapshot: GameSnapshot,
        capacity: usize,
        effects: &mut Vec<VehicleVelocityEffect>,
        debug_draws: &mut Vec<ScriptDebugDrawEffect>,
        raycasts: &mut Vec<ScriptRaycastRequest>,
        spawns: &mut Vec<ScriptSpawnEffect>,
        panel_updates: &mut Vec<ScriptPanelPublication>,
    ) {
        let mut runs = std::mem::take(&mut self.runs);
        runs.sort_by(|a, b| a.0.cmp(&b.0));
        let engine = &self.engine;
        let events = &self.events;
        let raycast_tickets = self.raycast_tickets.clone();
        let active_versions = self
            .active
            .iter()
            .map(|(id, script)| (id.clone(), script.version.0.clone()))
            .collect::<BTreeMap<_, _>>();
        // One-shot requests run before recurring callbacks; duplicate requests remain distinct.
        for (id, generation, request_id) in runs {
            let Some(script) = self.active.get_mut(&id) else {
                continue;
            };
            if script.generation != generation {
                continue;
            }
            let Some(function) = script.on_run.clone() else {
                continue;
            };
            invocation::invoke(
                engine,
                events,
                &id,
                script,
                &function,
                snapshot,
                request_id,
                capacity,
                effects,
                remaining_debug_draws(debug_draws),
                remaining_debug_work(debug_draws),
                debug_draws,
                raycast_tickets.has_capacity(),
                raycast_tickets.clone(),
                raycasts,
                spawns,
                dependency_availability(&script.requires, &active_versions),
                &[],
                panel_updates,
            );
        }
    }

    fn dispatch_frame_callbacks(
        &mut self,
        snapshot: GameSnapshot,
        capacity: usize,
        effects: &mut Vec<VehicleVelocityEffect>,
        debug_draws: &mut Vec<ScriptDebugDrawEffect>,
        raycasts: &mut Vec<ScriptRaycastRequest>,
        spawns: &mut Vec<ScriptSpawnEffect>,
        panel_events: &mut Vec<ScriptPanelEvent>,
        panel_updates: &mut Vec<ScriptPanelPublication>,
    ) {
        let engine = &self.engine;
        let events = &self.events;
        let raycast_tickets = self.raycast_tickets.clone();
        let active_versions = self
            .active
            .iter()
            .map(|(id, script)| (id.clone(), script.version.0.clone()))
            .collect::<BTreeMap<_, _>>();
        for (id, script) in &mut self.active {
            let Some(function) = script.callbacks.get(&ScriptPhaseId::GamePostSim).cloned() else {
                continue;
            };
            let invocation_events = take_panel_events(panel_events, id, script.generation);
            invocation::invoke(
                engine,
                events,
                id,
                script,
                &function,
                snapshot,
                None,
                capacity,
                effects,
                remaining_debug_draws(debug_draws),
                remaining_debug_work(debug_draws),
                debug_draws,
                raycast_tickets.has_capacity(),
                raycast_tickets.clone(),
                raycasts,
                spawns,
                dependency_availability(&script.requires, &active_versions),
                invocation_events.as_slice(),
                panel_updates,
            );
        }
    }

    fn retain_current_effects(
        &self,
        effects: &mut Vec<VehicleVelocityEffect>,
        debug_draws: &mut Vec<ScriptDebugDrawEffect>,
    ) {
        effects.retain(|effect| self.retain_effect(&effect.origin));
        debug_draws.retain(|effect| self.retain_effect(&effect.origin));
    }

    fn retain_effect(&self, origin: &ScriptEvent) -> bool {
        if self.effect_is_current(origin) {
            true
        } else {
            self.report_effect_cancelled(origin.clone());
            false
        }
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

fn remaining_debug_draws(draws: &[ScriptDebugDrawEffect]) -> usize {
    MAX_DEBUG_DRAWS_PER_PHASE.saturating_sub(draws.len())
}

fn remaining_debug_work(draws: &[ScriptDebugDrawEffect]) -> usize {
    let used = draws
        .iter()
        .map(|effect| effect.primitive.work_cost())
        .sum();
    MAX_DEBUG_DRAW_WORK_PER_PHASE.saturating_sub(used)
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
