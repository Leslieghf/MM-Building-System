use crate::display::{
    angles as format_angles, fixed as format_fixed, magnitude as format_magnitude,
    vector as format_vector,
};
use crate::scripting::console::{
    self, ConsoleCommand, ConsoleEvent, ConsoleEventReceiver, ConsoleEventSink, ConsoleOutput,
    ConsoleRequest, ConsoleRequestSender, ConsoleSeverity,
};
use crate::sdk::game::world_query::C_CUSTOM_SAMPLED_HEIGHT_FIELD_SHAPE_VFTABLE_RVA;
use crate::sdk::types::Matrix4x4;
use bevy_app::App;
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{Schedule, ScheduleLabel, SingleThreadedExecutor};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::time::Instant;

const STEAM_IMAGE_BASE: usize = 0x14000_0000;

pub const SNAPSHOT_QUEUE_CAPACITY: usize = 8;
pub const INTENT_QUEUE_CAPACITY: usize = 32;
pub const OUTCOME_QUEUE_CAPACITY: usize = 32;
pub const MAX_INTENTS_PER_RENDER: usize = 8;
pub const SPAWN_INTENT_QUEUE_CAPACITY: usize = 1;
pub const MAX_SPAWNS_PER_RENDER: usize = 1;
pub const MAX_SPAWN_ASSET_KEY_BYTES: usize = 240;
static NEXT_INTENT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(ScheduleLabel, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MadMaxPostSim;

#[derive(ScheduleLabel, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MadMaxRender;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MadMaxPostSimSet {
    Import,
    ModSystems,
    Export,
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MadMaxRenderSet {
    ModSystems,
    Presentation,
}

#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct CurrentGameFrame(pub GameSnapshot);

/// Diagnostic identity captured from the accepted shape pointer in a native
/// world-query context. These addresses are observations, not safe object handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RaycastCandidateShape {
    pub address: usize,
    pub vtable: usize,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct GameSnapshot {
    /// Mod-owned input sampled by the render hook, not a native game keymap.
    pub input_frame: crate::sdk::core::input::InputFrame,
    pub callback_sequence: u64,
    pub camera_raycast_hook_installed: bool,
    pub camera_raycast_calls: u64,
    pub world_query_hook_installed: bool,
    pub world_query_calls: u64,
    pub world_query_last_return_address: Option<usize>,
    pub callback_delta_seconds: f32,
    pub callback_elapsed_seconds: f64,
    pub sample_sequence: u64,
    pub sample_age_seconds: Option<f32>,
    pub post_sim_schedule_runs: u64,
    pub game_mod_system_runs: u64,
    pub callback_thread_id: Option<u32>,
    pub callback_stable_streak: u32,
    pub callback_overlap_count: u64,
    pub callback_thread_mismatch_count: u64,
    pub callback_gate_open: bool,
    pub callback_panics: u64,
    pub dropped_snapshots: u64,
    pub dropped_intents: u64,
    pub last_intent_outcome: Option<IntentOutcome>,
    pub last_spawn_outcome: Option<SpawnOutcome>,
    pub player_position: Option<[f32; 3]>,
    /// Horizontal projection of the player transform's forward basis.
    pub player_forward: Option<[f32; 3]>,
    pub player_transform: Option<Matrix4x4>,
    pub player_rotation: Option<[f32; 3]>,
    pub view_rotation: Option<[f32; 3]>,
    /// Camera ray submitted through the static-only query filter.
    pub reticle_ray_origin: Option<[f32; 3]>,
    pub reticle_ray_direction: Option<[f32; 3]>,
    pub reticle_query_available: bool,
    pub reticle_query_state_present: bool,
    /// Caller-observed segment fraction; values below one are candidate hits
    /// for this filter/query contract, not a universal WorldQueryHelper rule.
    pub reticle_hit_fraction: Option<f32>,
    /// Point reconstructed as origin + direction * (range * fraction).
    pub reticle_candidate_position: Option<[f32; 3]>,
    pub reticle_candidate_shape: Option<RaycastCandidateShape>,
    pub player_aim_query_available: bool,
    pub player_aim_query_state_present: bool,
    pub player_aim_hit_fraction: Option<f32>,
    pub player_aim_candidate_position: Option<[f32; 3]>,
    pub player_aim_candidate_shape: Option<RaycastCandidateShape>,
    pub ignore_characters_vehicles_query_available: bool,
    pub ignore_characters_vehicles_query_state_present: bool,
    pub ignore_characters_vehicles_hit_fraction: Option<f32>,
    pub ignore_characters_vehicles_candidate_position: Option<[f32; 3]>,
    pub ignore_characters_vehicles_candidate_shape: Option<RaycastCandidateShape>,
    pub ignore_player_owner_query_available: bool,
    pub ignore_player_owner_query_state_present: bool,
    pub ignore_player_owner_hit_fraction: Option<f32>,
    pub ignore_player_owner_candidate_position: Option<[f32; 3]>,
    pub ignore_player_owner_candidate_shape: Option<RaycastCandidateShape>,
    pub player_velocity: Option<[f32; 3]>,
    pub player_acceleration: Option<[f32; 3]>,
    pub vehicle_position: Option<[f32; 3]>,
    pub vehicle_transform: Option<Matrix4x4>,
    /// Horizontal projection of the current vehicle transform's forward basis.
    pub vehicle_forward: Option<[f32; 3]>,
    pub vehicle_rotation: Option<[f32; 3]>,
    pub vehicle_velocity: Option<[f32; 3]>,
    pub vehicle_acceleration: Option<[f32; 3]>,
    pub vehicle_angular_velocity: Option<[f32; 3]>,
    pub vehicle_velocity_last_frame: Option<[f32; 3]>,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerCharacterProxy;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurrentPlayerVehicleProxy;

#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct PlayerFrameData {
    pub position: Option<[f32; 3]>,
    pub forward: Option<[f32; 3]>,
    pub transform: Option<Matrix4x4>,
    pub rotation: Option<[f32; 3]>,
    pub view_rotation: Option<[f32; 3]>,
    pub velocity: Option<[f32; 3]>,
    pub acceleration: Option<[f32; 3]>,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct VehicleFrameData {
    pub position: Option<[f32; 3]>,
    pub forward: Option<[f32; 3]>,
    pub transform: Option<Matrix4x4>,
    pub rotation: Option<[f32; 3]>,
    pub velocity: Option<[f32; 3]>,
    pub acceleration: Option<[f32; 3]>,
    pub angular_velocity: Option<[f32; 3]>,
    pub velocity_last_frame: Option<[f32; 3]>,
}

/// ECS intent requesting a velocity on whichever vehicle the player occupies
/// when the native adapter consumes it. It contains no engine pointer/handle.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct SetPlayerVehicleVelocity(pub [f32; 3]);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetCurrentPlayerVehicle;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct IntentId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentResult {
    Applied,
    InvalidValue,
    NoCurrentVehicle,
    SetterUnavailable,
    CallbackNotExclusive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntentOutcome {
    pub intent_id: u64,
    pub result: IntentResult,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntentOutcomeRecord(pub IntentOutcome);

/// Session-local identifier for one requested game spawn. It is not a game
/// spawn ID and must never be passed to the native SDK as one.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SpawnRequestId(pub u64);

/// An owned exported entity key, without the game's `.ee` suffix.
///
/// Construction validates only the key's syntax. It does not prove that the
/// game build contains the entity or that the entity can be spawned safely.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct SpawnAssetKey(String);

impl SpawnAssetKey {
    pub fn parse(value: impl Into<String>) -> Result<Self, SpawnRequestError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= MAX_SPAWN_ASSET_KEY_BYTES
            && value.is_ascii()
            && value.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"/_-".contains(&byte)
            })
            && value.starts_with("editor/entities/")
            && !value.ends_with('/')
            && !value
                .split('/')
                .any(|segment| segment.is_empty() || segment == "." || segment == "..");
        if valid {
            Ok(Self(value))
        } else {
            Err(SpawnRequestError::InvalidAssetKey)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Owned row-major placement matrix copied from the game's pointer-free matrix
/// value. Native calls remain outside this ECS component.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct SpawnTransform {
    pub matrix: Matrix4x4,
}

impl SpawnTransform {
    pub fn new(matrix: Matrix4x4) -> Result<Self, SpawnRequestError> {
        if matrix
            .values
            .iter()
            .flatten()
            .all(|value| value.is_finite())
        {
            Ok(Self { matrix })
        } else {
            Err(SpawnRequestError::InvalidTransform)
        }
    }
}

/// Lifecycle vocabulary for spawn intents. The native adapter currently
/// reports submission only; the later observer must confirm spawned state.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnLifecycle {
    Queued,
    Rejected,
    Dispatching,
    PendingObservation,
    Spawned,
    DespawnRequested,
    Despawned,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnRequestError {
    InvalidAssetKey,
    InvalidTransform,
    QueueFull,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnRequestResult {
    CallbackNotExclusive,
    NativeSymbolUnavailable,
    SpawnIdUnavailable,
    InvalidRequest,
    NativeCallSubmitted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnOutcome {
    pub request_id: u64,
    pub result: SpawnRequestResult,
    pub native_return: Option<usize>,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnOutcomeRecord(pub SpawnOutcome);

#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct SpawnTestUiState {
    pub attempted: bool,
}

/// Create an ECS bundle for an in-process mod system to enqueue.
///
/// This is data only: it does not call the SDK or claim the entity key exists
/// in the installed game build.
pub fn spawn_entity_intent(
    asset_key: impl Into<String>,
    matrix: Matrix4x4,
) -> Result<impl Bundle, SpawnRequestError> {
    let asset_key = SpawnAssetKey::parse(asset_key)?;
    let transform = SpawnTransform::new(matrix)?;
    let request_id = SpawnRequestId(NEXT_INTENT_ID.fetch_add(1, Ordering::Relaxed));
    Ok((request_id, asset_key, transform, SpawnLifecycle::Queued))
}

#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct RuntimeDiagnostics {
    pub post_sim_schedule_runs: u64,
    pub render_schedule_runs: u64,
    pub game_mod_system_runs: u64,
    pub ui_mod_system_runs: u64,
    pub snapshots_dropped: u64,
    pub intents_dropped: u64,
    pub intent_transport_full_count: u64,
    pub last_intent_outcome: Option<IntentOutcome>,
    pub last_spawn_outcome: Option<SpawnOutcome>,
    pub callback_thread_id: Option<u32>,
    pub callback_stable_streak: u32,
    pub callback_overlap_count: u64,
    pub callback_thread_mismatch_count: u64,
    pub callback_gate_open: bool,
}

#[derive(Resource, Debug, Clone, Copy)]
pub struct UiPreferences {
    pub mode: OverlayMode,
    /// F1 override; panel transitions restore their default input ownership.
    pub input_capture_override: Option<bool>,
    /// Keep game input captured until an Escape key that closed the overlay is released.
    pub escape_capture_release_pending: bool,
    pub ui_opacity: f32,
    pub show_player: bool,
    pub show_camera: bool,
    pub show_raycast: bool,
    pub show_vehicle: bool,
    pub show_runtime: bool,
}

pub const UI_OPACITY_MIN: f32 = 0.25;
pub const UI_OPACITY_MAX: f32 = 1.0;
pub const UI_OPACITY_DEFAULT: f32 = 0.96;

impl Default for UiPreferences {
    fn default() -> Self {
        Self {
            mode: OverlayMode::Hud,
            input_capture_override: None,
            escape_capture_release_pending: false,
            ui_opacity: UI_OPACITY_DEFAULT,
            show_player: true,
            show_camera: false,
            show_raycast: false,
            show_vehicle: true,
            show_runtime: false,
        }
    }
}

impl UiPreferences {
    pub fn captures_game_input(self) -> bool {
        self.input_capture_override
            .unwrap_or_else(|| self.mode.owns_game_input())
    }

    pub fn set_mode(&mut self, mode: OverlayMode) {
        self.mode = mode;
        self.input_capture_override = None;
        self.escape_capture_release_pending = false;
    }

    pub fn toggle_input_capture(&mut self) {
        self.escape_capture_release_pending = false;
        self.input_capture_override = Some(!self.captures_game_input());
    }

    pub fn set_ui_opacity(&mut self, opacity: f32) {
        self.ui_opacity = if opacity.is_finite() {
            opacity.clamp(UI_OPACITY_MIN, UI_OPACITY_MAX)
        } else {
            UI_OPACITY_DEFAULT
        };
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayMode {
    Hud,
    UiConfigurator,
    ScriptConsole,
}

impl OverlayMode {
    pub fn owns_game_input(self) -> bool {
        matches!(self, Self::UiConfigurator | Self::ScriptConsole)
    }
}

pub const MAX_CONSOLE_HISTORY_ENTRIES: usize = 64;
pub const MAX_CONSOLE_STATUS_ROWS: usize = crate::scripting::manager::MAX_ACTIVE_SCRIPTS;

#[derive(Debug, Clone)]
pub struct ScriptConsoleStatusRow {
    pub id: crate::scripting::ScriptId,
    pub generation: crate::scripting::ScriptGeneration,
    pub status: crate::scripting::ScriptStatus,
    pub auto_reload: bool,
    pub reload_error: Option<String>,
}

/// Retained only in the UI world. Prompt, history, and selection survive toggles.
#[derive(Resource)]
pub struct ScriptConsoleUiState {
    pub input: String,
    pub input_over_limit: bool,
    pub history: VecDeque<String>,
    pub history_cursor: Option<usize>,
    pub history_draft: String,
    pub follow_output: bool,
    pub wrap_console_lines: bool,
    pub selected_console_text: Option<String>,
    pub scroll_y: f32,
    pub restore_scroll: bool,
    pub focus_prompt: bool,
    pub loaded_scripts: Vec<ScriptConsoleStatusRow>,
    pub selected_script: Option<ScriptConsoleStatusRow>,
    pub active_tab: DevToolsTab,
    pub discovered_scripts: Vec<crate::scripting::package::DiscoveredScript>,
    pub selected_package: Option<String>,
    pub discovery_status: String,
    pub discovery_pending: bool,
    pub initial_scan_started: bool,
    pub pending_discovery_request: Option<u64>,
    observed_output_revision: u64,
}

impl Default for ScriptConsoleUiState {
    fn default() -> Self {
        Self {
            // Keep one overflow sentinel byte plus ImGui's terminating NUL in this fixed buffer.
            input: String::with_capacity(console::MAX_COMMAND_BYTES + 2),
            input_over_limit: false,
            history: VecDeque::new(),
            history_cursor: None,
            history_draft: String::new(),
            follow_output: true,
            wrap_console_lines: true,
            selected_console_text: None,
            scroll_y: 0.0,
            restore_scroll: true,
            focus_prompt: true,
            loaded_scripts: Vec::new(),
            selected_script: None,
            active_tab: DevToolsTab::Console,
            discovered_scripts: Vec::new(),
            selected_package: None,
            discovery_status: "Open Scripts to scan packages.".into(),
            discovery_pending: false,
            initial_scan_started: false,
            pending_discovery_request: None,
            observed_output_revision: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DevToolsTab {
    #[default]
    Console,
    Scripts,
}

impl ScriptConsoleUiState {
    pub fn remember_input(&mut self) {
        if !self.input.trim().is_empty() && self.history.back() != Some(&self.input) {
            self.history.push_back(crate::scripting::bounded_text(
                &self.input,
                console::MAX_COMMAND_BYTES,
            ));
            while self.history.len() > MAX_CONSOLE_HISTORY_ENTRIES {
                self.history.pop_front();
            }
        }
        self.history_cursor = None;
        self.history_draft.clear();
    }

    pub fn observe_output(&mut self, output: &ConsoleOutput) {
        let new_events = output
            .revision()
            .saturating_sub(self.observed_output_revision);
        self.observed_output_revision = output.revision();
        let skip = output
            .events()
            .len()
            .saturating_sub(new_events.min(usize::MAX as u64) as usize);
        for event in output.events().iter().skip(skip) {
            if event.request_id.is_none()
                && (event
                    .text
                    .starts_with("script discovery result queue overflow")
                    || event.text.starts_with("console event queue overload:"))
            {
                self.discovery_pending = false;
                self.discovery_status = if event.text.starts_with("console event queue overload:") {
                    "Console output overloaded; the scan may be incomplete. Rescan when ready."
                        .into()
                } else {
                    event.text.clone()
                };
                self.pending_discovery_request = None;
            }
            if event.request_id.is_some() && event.request_id == self.pending_discovery_request {
                if event.severity == ConsoleSeverity::Error {
                    self.discovery_pending = false;
                    self.discovery_status = event.text.clone();
                    self.pending_discovery_request = None;
                }
            }
            let (Some(id), Some(generation), Some(status)) =
                (&event.script, event.generation, event.status)
            else {
                continue;
            };
            let previous = self.loaded_scripts.iter().find(|row| row.id == *id);
            let row = ScriptConsoleStatusRow {
                id: id.clone(),
                generation,
                status,
                auto_reload: event
                    .auto_reload
                    .unwrap_or_else(|| previous.is_some_and(|row| row.auto_reload)),
                reload_error: if event.auto_reload.is_some() {
                    event.reload_error.clone()
                } else {
                    previous.and_then(|row| row.reload_error.clone())
                },
            };
            if self
                .selected_script
                .as_ref()
                .is_some_and(|selected| selected.id == *id)
            {
                self.selected_script = Some(row.clone());
            }
            self.loaded_scripts.retain(|loaded| loaded.id != *id);
            if status != crate::scripting::ScriptStatus::Unloaded {
                if self.loaded_scripts.len() == MAX_CONSOLE_STATUS_ROWS {
                    self.loaded_scripts.remove(0);
                }
                if self.selected_script.is_none() {
                    self.selected_script = Some(row.clone());
                }
                self.loaded_scripts.push(row);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugSection {
    Player,
    Camera,
    Raycast,
    Vehicle,
    Runtime,
}

#[derive(Debug, Clone)]
pub struct PresentationRow {
    pub section: DebugSection,
    pub label: &'static str,
    pub value: String,
}

#[derive(Resource, Debug, Default)]
pub struct UiPresentation(pub Vec<PresentationRow>);

#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct UiGameSnapshot(pub GameSnapshot);

#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct PendingUiInput(pub crate::sdk::core::input::InputFrame);

/// Private wire DTO: UI and game Bevy entities cannot cross World boundaries.
#[derive(Debug, Clone, Copy)]
pub struct IntentEnvelope {
    pub intent_id: u64,
    pub velocity: [f32; 3],
}

#[derive(Debug, Clone)]
pub struct SpawnIntentEnvelope {
    pub request_id: u64,
    pub asset_key: String,
    pub matrix: Matrix4x4,
}

pub struct GameEndpoint {
    pub console_requests: Receiver<ConsoleRequest>,
    pub console_events: ConsoleEventSink,
    pub script_loader: crate::scripting::loader::ScriptLoader,
    pub discovery_tx: SyncSender<crate::scripting::loader::ScriptDiscoveryResult>,
    pub script_events: crate::scripting::ScriptEventSink,
    pub snapshot_tx: SyncSender<GameSnapshot>,
    pub intent_rx: Receiver<IntentEnvelope>,
    pub outcome_tx: SyncSender<IntentOutcome>,
    pub spawn_rx: Receiver<SpawnIntentEnvelope>,
    pub input_mailbox: crate::sdk::core::input::InputFrameMailbox,
}

pub struct UiEndpoint {
    pub console_requests: ConsoleRequestSender,
    pub console_events: ConsoleEventReceiver,
    pub discovery_requester: crate::scripting::loader::ScriptDiscoveryRequester,
    pub discovery_rx: Receiver<crate::scripting::loader::ScriptDiscoveryResult>,
    pub snapshot_rx: Receiver<GameSnapshot>,
    pub intent_tx: SyncSender<IntentEnvelope>,
    pub outcome_rx: Receiver<IntentOutcome>,
    pub spawn_tx: SyncSender<SpawnIntentEnvelope>,
    pub input_mailbox: crate::sdk::core::input::InputFrameMailbox,
}

pub fn create_endpoints() -> (GameEndpoint, UiEndpoint) {
    let (script_events_tx, script_events_rx) = crate::scripting::event_channel();
    let (console_request_tx, console_request_rx) = console::request_channel();
    let (snapshot_tx, snapshot_rx) = sync_channel(SNAPSHOT_QUEUE_CAPACITY);
    let (discovery_tx, discovery_rx) =
        sync_channel(crate::scripting::loader::LOADER_QUEUE_CAPACITY);
    let (intent_tx, intent_rx) = sync_channel(INTENT_QUEUE_CAPACITY);
    let (outcome_tx, outcome_rx) = sync_channel(OUTCOME_QUEUE_CAPACITY);
    let (spawn_tx, spawn_rx) = sync_channel(SPAWN_INTENT_QUEUE_CAPACITY);
    let input_mailbox = crate::sdk::core::input::InputFrameMailbox::default();
    let script_loader = crate::scripting::loader::ScriptLoader::start_for_game(
        crate::scripting::engine::RhaiEngineFactory,
    );
    let discovery_requester = script_loader.discovery_requester();

    (
        GameEndpoint {
            console_requests: console_request_rx,
            console_events: script_events_tx.0.clone(),
            script_loader,
            discovery_tx,
            script_events: script_events_tx,
            snapshot_tx,
            intent_rx,
            outcome_tx,
            spawn_rx,
            input_mailbox: input_mailbox.clone(),
        },
        UiEndpoint {
            console_requests: console_request_tx,
            console_events: script_events_rx,
            discovery_requester,
            discovery_rx,
            snapshot_rx,
            intent_tx,
            outcome_rx,
            spawn_tx,
            input_mailbox,
        },
    )
}

pub fn create_game_app() -> App {
    let mut app = App::empty();
    let mut schedule = Schedule::new(MadMaxPostSim);
    schedule.set_executor(SingleThreadedExecutor::new());
    app.add_schedule(schedule);
    app.configure_sets(
        MadMaxPostSim,
        (
            MadMaxPostSimSet::Import,
            MadMaxPostSimSet::ModSystems,
            MadMaxPostSimSet::Export,
        )
            .chain(),
    );
    app.init_resource::<CurrentGameFrame>();
    app.init_resource::<crate::sdk::core::input::InputFrame>();
    app.init_resource::<PendingUiInput>();
    app.init_resource::<crate::scripting::manager::ScriptManager>();
    app.init_resource::<crate::scripting::dispatch::PendingScriptDebugDraws>();
    app.init_resource::<RuntimeDiagnostics>();
    app.add_systems(
        MadMaxPostSim,
        project_frame_to_proxies.in_set(MadMaxPostSimSet::Import),
    );

    app.world_mut()
        .spawn((PlayerCharacterProxy, PlayerFrameData::default()));
    app.world_mut()
        .spawn((CurrentPlayerVehicleProxy, VehicleFrameData::default()));
    app
}

fn project_frame_to_proxies(
    frame: Res<CurrentGameFrame>,
    mut players: Query<&mut PlayerFrameData, With<PlayerCharacterProxy>>,
    mut vehicles: Query<&mut VehicleFrameData, With<CurrentPlayerVehicleProxy>>,
) {
    let snapshot = frame.0;
    for mut player in &mut players {
        *player = PlayerFrameData {
            position: snapshot.player_position,
            forward: snapshot.player_forward,
            transform: snapshot.player_transform,
            rotation: snapshot.player_rotation,
            view_rotation: snapshot.view_rotation,
            velocity: snapshot.player_velocity,
            acceleration: snapshot.player_acceleration,
        };
    }
    for mut vehicle in &mut vehicles {
        *vehicle = VehicleFrameData {
            position: snapshot.vehicle_position,
            forward: snapshot.vehicle_forward,
            transform: snapshot.vehicle_transform,
            rotation: snapshot.vehicle_rotation,
            velocity: snapshot.vehicle_velocity,
            acceleration: snapshot.vehicle_acceleration,
            angular_velocity: snapshot.vehicle_angular_velocity,
            velocity_last_frame: snapshot.vehicle_velocity_last_frame,
        };
    }
}

pub fn update_game_frame(app: &mut App, snapshot: GameSnapshot) {
    app.world_mut().insert_resource(CurrentGameFrame(snapshot));
}

pub fn create_ui_app() -> App {
    let mut app = App::empty();
    let mut schedule = Schedule::new(MadMaxRender);
    schedule.set_executor(SingleThreadedExecutor::new());
    app.add_schedule(schedule);
    app.configure_sets(
        MadMaxRender,
        (MadMaxRenderSet::ModSystems, MadMaxRenderSet::Presentation).chain(),
    );
    app.init_resource::<UiGameSnapshot>();
    app.init_resource::<crate::sdk::core::input::InputFrame>();
    app.init_resource::<UiPreferences>();
    app.init_resource::<UiPresentation>();
    app.init_resource::<RuntimeDiagnostics>();
    app.init_resource::<SpawnTestUiState>();
    app.init_resource::<ConsoleOutput>();
    app.init_resource::<ScriptConsoleUiState>();
    app.add_systems(
        MadMaxRender,
        (count_render_schedule, prepare_presentation)
            .chain()
            .in_set(MadMaxRenderSet::Presentation),
    );
    crate::modding::register_ui_systems(&mut app);
    app
}

/// Build an ECS intent bundle. A Bevy system can enqueue it with
/// `commands.spawn(player_vehicle_velocity_intent(velocity));`; the native
/// adapter later resolves the current vehicle and reports the result.
pub fn player_vehicle_velocity_intent(velocity: [f32; 3]) -> impl Bundle {
    (
        IntentId(NEXT_INTENT_ID.fetch_add(1, Ordering::Relaxed)),
        SetPlayerVehicleVelocity(velocity),
        TargetCurrentPlayerVehicle,
    )
}

/// Transfer UI-world intent entities into the bounded game-world transport.
/// Full queues leave the entities in the UI world to retry on a later render.
pub fn flush_ui_intents(app: &mut App, endpoint: &UiEndpoint) {
    let world = app.world_mut();
    let mut pending: Vec<_> = {
        let mut query = world.query_filtered::<
            (Entity, &IntentId, &SetPlayerVehicleVelocity),
            Without<IntentOutcomeRecord>,
        >();
        query
            .iter(world)
            .map(|(entity, id, velocity)| (entity, id.0, velocity.0))
            .collect()
    };
    pending.sort_by_key(|(_, intent_id, _)| *intent_id);

    let mut sent = 0;
    for (entity, intent_id, velocity) in pending {
        if sent >= MAX_INTENTS_PER_RENDER {
            break;
        }
        match endpoint.intent_tx.try_send(IntentEnvelope {
            intent_id,
            velocity,
        }) {
            Ok(()) => {
                world.entity_mut(entity).despawn();
                sent += 1;
            }
            Err(std::sync::mpsc::TrySendError::Full(_)) => {
                let mut diagnostics = world.resource_mut::<RuntimeDiagnostics>();
                diagnostics.intent_transport_full_count =
                    diagnostics.intent_transport_full_count.saturating_add(1);
                break;
            }
            Err(std::sync::mpsc::TrySendError::Disconnected(_)) => break,
        }
    }

    let mut pending_spawns: Vec<_> = {
        let mut query = world.query_filtered::<
            (Entity, &SpawnRequestId, &SpawnAssetKey, &SpawnTransform),
            Without<SpawnOutcomeRecord>,
        >();
        query
            .iter(world)
            .map(|(entity, id, key, transform)| {
                (entity, id.0, key.as_str().to_owned(), transform.matrix)
            })
            .collect()
    };
    pending_spawns.sort_by_key(|(_, request_id, _, _)| *request_id);
    for (entity, request_id, asset_key, matrix) in
        pending_spawns.into_iter().take(MAX_SPAWNS_PER_RENDER)
    {
        match endpoint.spawn_tx.try_send(SpawnIntentEnvelope {
            request_id,
            asset_key,
            matrix,
        }) {
            Ok(()) => {
                world.entity_mut(entity).despawn();
            }
            Err(std::sync::mpsc::TrySendError::Full(_)) => break,
            Err(std::sync::mpsc::TrySendError::Disconnected(_)) => break,
        }
    }
}

fn count_render_schedule(mut diagnostics: ResMut<RuntimeDiagnostics>) {
    diagnostics.render_schedule_runs = diagnostics.render_schedule_runs.saturating_add(1);
}

fn prepare_presentation(
    snapshot: Res<UiGameSnapshot>,
    diagnostics: Res<RuntimeDiagnostics>,
    mut presentation: ResMut<UiPresentation>,
) {
    let data = snapshot.0;
    let rows = &mut presentation.0;
    rows.clear();
    rows.reserve(18);
    rows.push(row(
        DebugSection::Player,
        "Position",
        format_vector(data.player_position),
    ));
    rows.push(row(
        DebugSection::Player,
        "Rotation (player/model)",
        format_angles(data.player_rotation),
    ));
    rows.push(row(
        DebugSection::Player,
        "Speed magnitude (estimated)",
        format_magnitude(data.player_velocity, " units/s"),
    ));
    rows.push(row(
        DebugSection::Player,
        "Acceleration magnitude (estimated)",
        format_magnitude(data.player_acceleration, " units/s²"),
    ));
    rows.push(row(
        DebugSection::Camera,
        "Rotation (view)",
        format_angles(data.view_rotation),
    ));
    rows.push(row(
        DebugSection::Raycast,
        "Static-only query",
        if data.reticle_query_available {
            format!(
                "active / {}",
                if data.reticle_query_state_present {
                    "result state present"
                } else {
                    "no result state"
                }
            )
        } else {
            "unavailable".to_owned()
        },
    ));
    rows.push(row(
        DebugSection::Raycast,
        "Ray origin",
        format_vector(data.reticle_ray_origin),
    ));
    rows.push(row(
        DebugSection::Raycast,
        "Ray direction",
        format_vector(data.reticle_ray_direction),
    ));
    rows.push(row(
        DebugSection::Raycast,
        "Static-only hit fraction",
        data.reticle_hit_fraction.map_or_else(
            || "unavailable".to_owned(),
            |value| {
                if value >= 1.0 {
                    "no hit".to_owned()
                } else {
                    format_fixed(f64::from(value))
                }
            },
        ),
    ));
    rows.push(row(
        DebugSection::Raycast,
        "Static-only hit position",
        data.reticle_candidate_position.map_or_else(
            || {
                if !data.reticle_query_available {
                    "unavailable".to_owned()
                } else if data
                    .reticle_hit_fraction
                    .is_some_and(|fraction| fraction >= 1.0)
                {
                    "no hit".to_owned()
                } else {
                    "unknown".to_owned()
                }
            },
            |position| format_vector(Some(position)),
        ),
    ));
    rows.push(row(
        DebugSection::Raycast,
        "Static-only hit shape",
        format_raycast_candidate_shape(
            data.reticle_candidate_shape,
            data.reticle_query_available,
            data.reticle_hit_fraction,
        ),
    ));
    rows.push(row(
        DebugSection::Raycast,
        "Player-aim query",
        if data.player_aim_query_available {
            format!(
                "active / {}",
                if data.player_aim_query_state_present {
                    "result state present"
                } else {
                    "no result state"
                }
            )
        } else {
            "unavailable".to_owned()
        },
    ));
    rows.push(row(
        DebugSection::Raycast,
        "Player-aim filtered fraction",
        data.player_aim_hit_fraction.map_or_else(
            || {
                if !data.player_aim_query_available {
                    "unavailable".to_owned()
                } else {
                    "no hit".to_owned()
                }
            },
            |value| {
                if value >= 1.0 {
                    "no hit".to_owned()
                } else {
                    format_fixed(f64::from(value))
                }
            },
        ),
    ));
    rows.push(row(
        DebugSection::Raycast,
        "Player-aim filtered position",
        data.player_aim_candidate_position.map_or_else(
            || {
                if !data.player_aim_query_available {
                    "unavailable".to_owned()
                } else {
                    "no hit".to_owned()
                }
            },
            |position| format_vector(Some(position)),
        ),
    ));
    rows.push(row(
        DebugSection::Raycast,
        "Player-aim hit shape",
        format_raycast_candidate_shape(
            data.player_aim_candidate_shape,
            data.player_aim_query_available,
            data.player_aim_hit_fraction,
        ),
    ));
    rows.push(row(
        DebugSection::Raycast,
        "Ignore characters/vehicles query",
        if data.ignore_characters_vehicles_query_available {
            if data.ignore_characters_vehicles_query_state_present {
                "active / result state present".to_owned()
            } else {
                "active / no result state".to_owned()
            }
        } else {
            "unavailable".to_owned()
        },
    ));
    rows.push(row(
        DebugSection::Raycast,
        "Ignore characters/vehicles fraction",
        data.ignore_characters_vehicles_hit_fraction.map_or_else(
            || {
                if data.ignore_characters_vehicles_query_available {
                    "no hit".to_owned()
                } else {
                    "unavailable".to_owned()
                }
            },
            |value| {
                if value >= 1.0 {
                    "no hit".to_owned()
                } else {
                    format_fixed(f64::from(value))
                }
            },
        ),
    ));
    rows.push(row(
        DebugSection::Raycast,
        "Ignore characters/vehicles position",
        data.ignore_characters_vehicles_candidate_position
            .map_or_else(
                || {
                    if data.ignore_characters_vehicles_query_available {
                        "no hit".to_owned()
                    } else {
                        "unavailable".to_owned()
                    }
                },
                |position| format_vector(Some(position)),
            ),
    ));
    rows.push(row(
        DebugSection::Raycast,
        "Ignore characters/vehicles hit shape",
        format_raycast_candidate_shape(
            data.ignore_characters_vehicles_candidate_shape,
            data.ignore_characters_vehicles_query_available,
            data.ignore_characters_vehicles_hit_fraction,
        ),
    ));

    rows.push(row(
        DebugSection::Raycast,
        "Ignore-player-owner query",
        if data.ignore_player_owner_query_available {
            if data.ignore_player_owner_query_state_present {
                "active / result state present".to_owned()
            } else {
                "active / no result state".to_owned()
            }
        } else {
            "unavailable".to_owned()
        },
    ));
    rows.push(row(
        DebugSection::Raycast,
        "Ignore-player-owner fraction",
        data.ignore_player_owner_hit_fraction.map_or_else(
            || {
                if data.ignore_player_owner_query_available {
                    "no hit".to_owned()
                } else {
                    "unavailable".to_owned()
                }
            },
            |value| {
                if value >= 1.0 {
                    "no hit".to_owned()
                } else {
                    format_fixed(f64::from(value))
                }
            },
        ),
    ));
    rows.push(row(
        DebugSection::Raycast,
        "Ignore-player-owner position",
        data.ignore_player_owner_candidate_position.map_or_else(
            || {
                if data.ignore_player_owner_query_available {
                    "no hit".to_owned()
                } else {
                    "unavailable".to_owned()
                }
            },
            |position| format_vector(Some(position)),
        ),
    ));
    rows.push(row(
        DebugSection::Raycast,
        "Ignore-player-owner hit shape",
        format_raycast_candidate_shape(
            data.ignore_player_owner_candidate_shape,
            data.ignore_player_owner_query_available,
            data.ignore_player_owner_hit_fraction,
        ),
    ));

    let vehicle_status = if data.vehicle_position.is_some()
        || data.vehicle_velocity.is_some()
        || data.vehicle_acceleration.is_some()
        || data.vehicle_angular_velocity.is_some()
        || data.vehicle_velocity_last_frame.is_some()
    {
        "vehicle found".to_owned()
    } else {
        "no vehicle available".to_owned()
    };
    rows.push(row(DebugSection::Vehicle, "Status", vehicle_status));
    rows.push(row(
        DebugSection::Vehicle,
        "Position",
        format_vector(data.vehicle_position),
    ));
    rows.push(row(
        DebugSection::Vehicle,
        "Rotation",
        format_angles(data.vehicle_rotation),
    ));
    rows.push(row(
        DebugSection::Vehicle,
        "Speed magnitude (physics)",
        format_magnitude(data.vehicle_velocity, " units/s"),
    ));
    rows.push(row(
        DebugSection::Vehicle,
        "Acceleration magnitude (physics)",
        format_magnitude(data.vehicle_acceleration, " units/s²"),
    ));
    rows.push(row(
        DebugSection::Vehicle,
        "Angular velocity magnitude",
        format_magnitude(data.vehicle_angular_velocity, ""),
    ));
    rows.push(row(
        DebugSection::Vehicle,
        "Previous-frame speed magnitude",
        format_magnitude(data.vehicle_velocity_last_frame, ""),
    ));
    rows.push(row(
        DebugSection::Runtime,
        "Frame delta",
        format!(
            "{} ms",
            format_fixed(f64::from(data.callback_delta_seconds) * 1000.0)
        ),
    ));
    if let Some(outcome) = data.last_intent_outcome.or(diagnostics.last_intent_outcome) {
        rows.push(row(
            DebugSection::Runtime,
            "Last velocity intent",
            format!("{}: {:?}", outcome.intent_id, outcome.result),
        ));
    }
    if let Some(outcome) = data.last_spawn_outcome.or(diagnostics.last_spawn_outcome) {
        rows.push(row(
            DebugSection::Runtime,
            "Last spawn request",
            format!(
                "{}: {:?} ({:?})",
                outcome.request_id, outcome.result, outcome.native_return
            ),
        ));
    }
}

fn row(section: DebugSection, label: &'static str, value: String) -> PresentationRow {
    PresentationRow {
        section,
        label,
        value,
    }
}

fn format_raycast_candidate_shape(
    shape: Option<RaycastCandidateShape>,
    query_available: bool,
    hit_fraction: Option<f32>,
) -> String {
    let Some(shape) = shape else {
        if !query_available {
            return "unavailable".to_owned();
        }
        return if hit_fraction.is_some_and(|fraction| fraction >= 1.0) {
            "no hit".to_owned()
        } else {
            "unknown".to_owned()
        };
    };
    let height_field_vtable = STEAM_IMAGE_BASE + C_CUSTOM_SAMPLED_HEIGHT_FIELD_SHAPE_VFTABLE_RVA;
    let kind = if shape.vtable == height_field_vtable {
        "sampled heightfield terrain"
    } else {
        "unknown shape"
    };
    format!("{kind} 0x{:X} (vtable 0x{:X})", shape.address, shape.vtable)
}

pub struct GameRuntime {
    pub app: App,
    endpoint: GameEndpoint,
    identity_echo_queued: bool,
    last_callback_at: Option<Instant>,
    elapsed_seconds: f64,
    console_requests_disconnected: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct PendingVelocityIntent {
    pub entity: Entity,
    pub intent_id: u64,
    pub velocity: [f32; 3],
}

#[derive(Debug, Clone)]
pub struct PendingSpawnIntent {
    pub request_id: u64,
    pub asset_key: String,
    pub matrix: Matrix4x4,
}

#[derive(Debug, Clone)]
pub enum PendingGameIntent {
    Spawn(PendingSpawnIntent),
    Velocity(PendingVelocityIntent),
}

impl GameRuntime {
    pub fn new(endpoint: GameEndpoint) -> Self {
        let mut app = create_game_app();
        app.insert_resource(crate::scripting::manager::ScriptManager::new(
            endpoint.script_events.clone(),
        ));
        crate::modding::register_game_systems(&mut app);
        Self {
            app,
            endpoint,
            identity_echo_queued: false,
            last_callback_at: None,
            elapsed_seconds: 0.0,
            console_requests_disconnected: false,
        }
    }

    /// Route console load/reload commands here at a game callback boundary.
    pub fn request_script_load(
        &mut self,
        request: crate::scripting::loader::LoadScriptRequest,
    ) -> Result<(), String> {
        self.app
            .world_mut()
            .resource_mut::<crate::scripting::manager::ScriptManager>()
            .request_load(&self.endpoint.script_loader, request)
    }

    fn drain_console_requests(&mut self) -> bool {
        for _ in 0..console::MAX_COMMANDS_PER_GAME_CALLBACK {
            match self.endpoint.console_requests.try_recv() {
                Ok(request) => self.process_console_request(request),
                Err(std::sync::mpsc::TryRecvError::Empty) => return true,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    if !self.console_requests_disconnected {
                        self.console_requests_disconnected = true;
                        self.endpoint.console_events.emit(ConsoleEvent::new(
                            None,
                            ConsoleSeverity::Warning,
                            "UI console request channel disconnected",
                        ));
                    }
                    return true;
                }
            }
        }
        // Preserve the command budget; defer async work until the UI queue drains.
        false
    }

    fn process_console_request(&mut self, request: ConsoleRequest) {
        use crate::scripting::{loader::LoadScriptRequest, manager::ScriptManager};
        let request_id = request.request_id;
        match request.command {
            ConsoleCommand::Help(topic) => {
                console::emit_help(&self.endpoint.console_events, request_id, topic.as_deref())
            }
            ConsoleCommand::ShowKeys
            | ConsoleCommand::ShowBinds
            | ConsoleCommand::Bind { .. }
            | ConsoleCommand::Unbind { .. } => {
                self.endpoint.console_events.emit(ConsoleEvent::new(
                    Some(request_id),
                    ConsoleSeverity::Error,
                    "input binding commands are local to the mod-owned render input runtime",
                ));
            }
            ConsoleCommand::ScriptLoad(id) => {
                let _ = self.request_script_load(LoadScriptRequest {
                    request_id,
                    id,
                    replace: false,
                    snapshot: None,
                });
            }
            ConsoleCommand::ScriptReload(id) => {
                let _ = self.request_script_load(LoadScriptRequest {
                    request_id,
                    id,
                    replace: true,
                    snapshot: None,
                });
            }
            ConsoleCommand::ScriptAutoReload { id, enabled } => {
                let _ = self
                    .app
                    .world_mut()
                    .resource_mut::<ScriptManager>()
                    .set_auto_reload_request(
                        &id,
                        enabled,
                        &self.endpoint.script_loader,
                        Some(request_id),
                    );
            }
            command @ (ConsoleCommand::ScriptRun(_) | ConsoleCommand::ScriptUnload(_)) => {
                let id = command.script().expect("script command").clone();
                let mut scripts = self.app.world_mut().resource_mut::<ScriptManager>();
                let result = match command {
                    ConsoleCommand::ScriptRun(_) => {
                        scripts.stage_run_request(id.clone(), Some(request_id))
                    }
                    _ => scripts.request_unload(
                        &self.endpoint.script_loader,
                        id.clone(),
                        Some(request_id),
                    ),
                };
                // Manager lifecycle errors already carry one correlated console event.
                let _ = result;
                // Complete each lifecycle command before inspecting the next one.
                scripts.apply_pending_changes();
            }
            ConsoleCommand::ScriptList => {
                let result = self.endpoint.script_loader.try_discover(
                    crate::scripting::loader::DiscoverScriptsRequest {
                        request_id: Some(request_id),
                    },
                );
                let (severity, text) = match result {
                    Ok(()) => (ConsoleSeverity::Info, "package discovery queued"),
                    Err(error) => (ConsoleSeverity::Error, error),
                };
                self.endpoint.console_events.emit(ConsoleEvent::new(
                    Some(request_id),
                    severity,
                    text,
                ));
            }
            command @ ConsoleCommand::ScriptStatus(_) => {
                let scripts = self.app.world().resource::<ScriptManager>();
                let target = command.script();
                let mut found = false;
                for listing in scripts
                    .list()
                    .iter()
                    .filter(|row| target.is_none_or(|id| id == &row.id))
                {
                    found = true;
                    let callbacks = listing
                        .callbacks
                        .iter()
                        .map(|(phase, callback)| format!("{}={callback}", phase.as_str()))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let mut event = ConsoleEvent::new(
                        Some(request_id),
                        ConsoleSeverity::Info,
                        &format!(
                            "{}: {:?}; callbacks: [{}]; on_run: {}",
                            listing.id.as_str(),
                            listing.status,
                            callbacks,
                            listing.has_on_run
                        ),
                    );
                    event.script = Some(listing.id.clone());
                    event.generation = Some(listing.generation);
                    event.status = Some(listing.status);
                    event.auto_reload = Some(listing.auto_reload);
                    event.reload_error = listing.reload_error.clone();
                    event.phase = Some(crate::scripting::ScriptPhaseId::GamePostSim);
                    self.endpoint.console_events.emit(event);
                }
                for (id, _load_request) in scripts
                    .pending_loads()
                    .into_iter()
                    .filter(|(id, _)| target.is_none_or(|target| target == id))
                {
                    found = true;
                    let mut event = ConsoleEvent::new(
                        Some(request_id),
                        ConsoleSeverity::Info,
                        &format!("{}: preparation pending", id.as_str()),
                    );
                    event.script = Some(id);
                    self.endpoint.console_events.emit(event);
                }
                if !found {
                    let mut event = ConsoleEvent::new(
                        Some(request_id),
                        if target.is_some() {
                            ConsoleSeverity::Error
                        } else {
                            ConsoleSeverity::Info
                        },
                        if target.is_some() {
                            "unknown script ID: script is not loaded or preparing"
                        } else {
                            "no scripts loaded or preparing"
                        },
                    );
                    event.script = target.cloned();
                    self.endpoint.console_events.emit(event);
                }
            }
        }
    }

    pub(crate) fn take_script_debug_draws(
        &mut self,
    ) -> Vec<crate::scripting::manager::ScriptDebugDrawEffect> {
        std::mem::take(
            &mut self
                .app
                .world_mut()
                .resource_mut::<crate::scripting::dispatch::PendingScriptDebugDraws>()
                .0,
        )
    }

    pub fn prepare_and_run(&mut self, mut snapshot: GameSnapshot) -> Option<PendingGameIntent> {
        let input = self.endpoint.input_mailbox.take();
        {
            let mut current = self
                .app
                .world_mut()
                .resource_mut::<crate::sdk::core::input::InputFrame>();
            current.clear_edges();
            if let Some(input) = input {
                *current = input;
            }
        }
        snapshot.input_frame = *self
            .app
            .world()
            .resource::<crate::sdk::core::input::InputFrame>();
        let now = Instant::now();
        let delta_seconds = self
            .last_callback_at
            .replace(now)
            .map_or(0.0, |previous| now.duration_since(previous).as_secs_f32());
        self.elapsed_seconds += f64::from(delta_seconds);
        snapshot.callback_delta_seconds = delta_seconds;
        snapshot.callback_elapsed_seconds = self.elapsed_seconds;
        update_game_frame(&mut self.app, snapshot);
        let mut imported = 0;
        while imported < INTENT_QUEUE_CAPACITY {
            match self.endpoint.intent_rx.try_recv() {
                Ok(envelope) => {
                    self.import_velocity_intent(envelope.intent_id, envelope.velocity);
                    imported += 1;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
            }
        }
        for _ in 0..MAX_SPAWNS_PER_RENDER {
            match self.endpoint.spawn_rx.try_recv() {
                Ok(envelope) => {
                    if let (Ok(asset_key), Ok(transform)) = (
                        SpawnAssetKey::parse(envelope.asset_key),
                        SpawnTransform::new(envelope.matrix),
                    ) {
                        self.app.world_mut().spawn((
                            SpawnRequestId(envelope.request_id),
                            asset_key,
                            transform,
                            SpawnLifecycle::Dispatching,
                        ));
                    } else {
                        self.app
                            .world_mut()
                            .resource_mut::<RuntimeDiagnostics>()
                            .last_spawn_outcome = Some(SpawnOutcome {
                            request_id: envelope.request_id,
                            result: SpawnRequestResult::InvalidRequest,
                            native_return: None,
                        });
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty)
                | Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
            }
        }

        if !self.identity_echo_queued {
            if let Some(velocity) = snapshot.vehicle_velocity {
                self.spawn_velocity_intent(velocity);
                self.identity_echo_queued = true;
            }
        }

        if self.drain_console_requests() {
            let mut scripts = self
                .app
                .world_mut()
                .resource_mut::<crate::scripting::manager::ScriptManager>();
            for id in self.endpoint.script_loader.take_auto_reload_overflow() {
                scripts.report_auto_reload_overflow(id);
            }
            for _ in 0..crate::scripting::loader::LOADER_QUEUE_CAPACITY {
                let Some(request) = self.endpoint.script_loader.try_auto_reload() else {
                    break;
                };
                let _ = scripts.request_auto_reload(&self.endpoint.script_loader, request);
            }
            for _ in 0..crate::scripting::loader::LOADER_QUEUE_CAPACITY {
                let Some(result) = self.endpoint.script_loader.try_result() else {
                    break;
                };
                scripts.receive_load_result(result);
            }
            scripts.report_loader_overflow(self.endpoint.script_loader.take_overflow());
            scripts.apply_pending_changes();
            scripts.process_deferred_auto_reloads(&self.endpoint.script_loader);
        }
        for _ in 0..crate::scripting::loader::LOADER_QUEUE_CAPACITY {
            let Some(result) = self.endpoint.script_loader.try_discovery_result() else {
                break;
            };
            if let Some(request_id) = result.request_id {
                self.endpoint.console_events.emit(ConsoleEvent::new(
                    Some(request_id),
                    ConsoleSeverity::Info,
                    "Package scan complete.",
                ));
                for package in &result.packages {
                    let metadata = package
                        .manifest
                        .as_ref()
                        .map_or_else(String::new, |manifest| {
                            format!(
                                " | API v{} | callbacks [{}] | capabilities [{}]",
                                manifest.api_version,
                                manifest.callbacks.join(", "),
                                manifest.capabilities.join(", ")
                            )
                        });
                    let issue = package
                        .issue
                        .as_ref()
                        .map_or_else(String::new, |issue| format!(" | invalid: {issue}"));
                    self.endpoint.console_events.emit(ConsoleEvent::new(
                        Some(request_id),
                        if package.issue.is_some() {
                            ConsoleSeverity::Warning
                        } else {
                            ConsoleSeverity::Info
                        },
                        &format!("{}{}{}", package.package_name, metadata, issue),
                    ));
                }
                for listing in self
                    .app
                    .world()
                    .resource::<crate::scripting::manager::ScriptManager>()
                    .list()
                {
                    self.endpoint.console_events.emit(ConsoleEvent::new(
                        Some(request_id),
                        ConsoleSeverity::Info,
                        &format!(
                            "runtime {}: {:?} | generation {}",
                            listing.id.as_str(),
                            listing.status,
                            listing.generation.0
                        ),
                    ));
                }
                if let Some(error) = &result.error {
                    self.endpoint.console_events.emit(ConsoleEvent::new(
                        Some(request_id),
                        ConsoleSeverity::Warning,
                        error,
                    ));
                }
            }
            if let Err(error) = self.endpoint.discovery_tx.try_send(result) {
                let (request_id, text) = match error {
                    std::sync::mpsc::TrySendError::Full(result) => (
                        result.request_id,
                        "discovery result queue full; rescan from Scripts",
                    ),
                    std::sync::mpsc::TrySendError::Disconnected(result) => {
                        (result.request_id, "discovery UI channel disconnected")
                    }
                };
                self.endpoint.console_events.emit(ConsoleEvent::new(
                    request_id,
                    ConsoleSeverity::Error,
                    text,
                ));
            }
        }
        let dropped_discoveries = self.endpoint.script_loader.take_discovery_overflow();
        if dropped_discoveries != 0 {
            self.endpoint.console_events.emit(ConsoleEvent::new(
                None,
                ConsoleSeverity::Error,
                &format!("script discovery result queue overflow: {dropped_discoveries} result(s) dropped; rescan from Scripts"),
            ));
        }
        self.app.world_mut().run_schedule(MadMaxPostSim);
        {
            let mut diagnostics = self.app.world_mut().resource_mut::<RuntimeDiagnostics>();
            diagnostics.post_sim_schedule_runs =
                diagnostics.post_sim_schedule_runs.saturating_add(1);
            snapshot.post_sim_schedule_runs = diagnostics.post_sim_schedule_runs;
            snapshot.game_mod_system_runs = diagnostics.game_mod_system_runs;
            snapshot.dropped_snapshots = diagnostics.snapshots_dropped;
            snapshot.dropped_intents = diagnostics.intents_dropped;
            snapshot.last_intent_outcome = diagnostics.last_intent_outcome;
            snapshot.last_spawn_outcome = diagnostics.last_spawn_outcome;
        }
        match self.endpoint.snapshot_tx.try_send(snapshot) {
            Ok(()) => {}
            Err(std::sync::mpsc::TrySendError::Full(_)) => {
                let mut diagnostics = self.app.world_mut().resource_mut::<RuntimeDiagnostics>();
                diagnostics.snapshots_dropped = diagnostics.snapshots_dropped.saturating_add(1);
            }
            Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {}
        }
        self.take_next_spawn_intent()
            .map(PendingGameIntent::Spawn)
            .or_else(|| {
                self.take_next_velocity_intent()
                    .map(PendingGameIntent::Velocity)
            })
    }

    pub fn complete_spawn_intent(&mut self, pending: PendingSpawnIntent, outcome: SpawnOutcome) {
        let world = self.app.world_mut();
        let mut query = world.query_filtered::<(Entity, &SpawnRequestId), With<SpawnTransform>>();
        if let Some((entity, _)) = query.iter(world).find(|(_, id)| id.0 == pending.request_id) {
            world.entity_mut(entity).despawn();
        }
        world
            .resource_mut::<RuntimeDiagnostics>()
            .last_spawn_outcome = Some(outcome);
    }

    fn take_next_spawn_intent(&mut self) -> Option<PendingSpawnIntent> {
        let world = self.app.world_mut();
        let candidate = {
            let mut query = world.query_filtered::<
                (&SpawnRequestId, &SpawnAssetKey, &SpawnTransform),
                With<SpawnLifecycle>,
            >();
            query
                .iter(world)
                .min_by_key(|(id, _, _)| id.0)
                .map(|(id, key, transform)| (id.0, key.as_str().to_owned(), transform.matrix))
        }?;
        Some(PendingSpawnIntent {
            request_id: candidate.0,
            asset_key: candidate.1,
            matrix: candidate.2,
        })
    }

    pub fn complete_intent(&mut self, pending: PendingVelocityIntent, result: IntentResult) {
        if let Some(origin) = self
            .app
            .world()
            .get::<crate::scripting::dispatch::ScriptVelocityOrigin>(pending.entity)
            .cloned()
        {
            self.app
                .world()
                .resource::<crate::scripting::manager::ScriptManager>()
                .report_velocity_result(origin.0, pending.intent_id, result);
        }
        self.app.world_mut().entity_mut(pending.entity).despawn();
        let outcome = IntentOutcome {
            intent_id: pending.intent_id,
            result,
        };
        self.app
            .world_mut()
            .resource_mut::<RuntimeDiagnostics>()
            .last_intent_outcome = Some(outcome);
        match self.endpoint.outcome_tx.try_send(outcome) {
            Ok(()) => {}
            Err(std::sync::mpsc::TrySendError::Full(_)) => {
                let mut diagnostics = self.app.world_mut().resource_mut::<RuntimeDiagnostics>();
                diagnostics.intents_dropped = diagnostics.intents_dropped.saturating_add(1);
            }
            Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {}
        }
    }

    fn spawn_velocity_intent(&mut self, velocity: [f32; 3]) {
        let intent = player_vehicle_velocity_intent(velocity);
        self.app.world_mut().spawn(intent);
    }

    fn import_velocity_intent(&mut self, id: u64, velocity: [f32; 3]) {
        self.app.world_mut().spawn((
            IntentId(id),
            SetPlayerVehicleVelocity(velocity),
            TargetCurrentPlayerVehicle,
        ));
    }

    fn take_next_velocity_intent(&mut self) -> Option<PendingVelocityIntent> {
        let world = self.app.world_mut();
        let candidate = {
            let mut query = world.query_filtered::<
                (Entity, &IntentId, &SetPlayerVehicleVelocity),
                Without<IntentOutcomeRecord>,
            >();
            query
                .iter(world)
                .min_by_key(|(_, id, _)| id.0)
                .map(|(entity, id, velocity)| (entity, id.0, velocity.0))
        }?;
        Some(PendingVelocityIntent {
            entity: candidate.0,
            intent_id: candidate.1,
            velocity: candidate.2,
        })
    }
}

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
