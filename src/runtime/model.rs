//! Runtime data types: copied game state, UI presentation, and validated intents.
use super::*;

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
