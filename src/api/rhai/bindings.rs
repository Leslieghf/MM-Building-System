//! Composition of the versioned Rhai context and callback-scoped capabilities.
//!
//! Registration is driven by [`super::catalog`]. Each invocation receives an
//! owned frame, stable input snapshot, formatter, pure math helpers, and only
//! the effect collectors granted by its package capabilities. Collectors are
//! discarded after the callback and validated before becoming typed intents;
//! no live game pointer is registered with Rhai.
#[path = "bindings/debug_draw.rs"]
mod debug_draw;
use super::catalog::{API_CALLBACKS, API_CAPABILITIES};
use super::math::ScriptMath;
use crate::api::scripts::*;
use crate::api::{
    ecs::{GameSnapshot, input::InputFrame},
    game::abi::types::Matrix4x4,
};
pub(crate) use debug_draw::{DebugPrimitive, InvocationDebugDraw};
use rhai::{Array, Dynamic, Engine, Map};
use std::collections::BTreeMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
};

/// Runtime API version and descriptor catalog used by package validation.
pub struct ScriptApiRegistry {
    /// Callback signatures, values, capabilities, and effects for this version.
    pub descriptors: Vec<ScriptApiDescriptor>,
    /// Capability identifiers accepted by the manifest validator.
    pub capabilities: &'static [&'static str],
}
impl ScriptApiRegistry {
    /// Build the API v1 descriptor registry used by the current host.
    pub fn v1() -> Self {
        Self {
            capabilities: API_CAPABILITIES,
            descriptors: API_CALLBACKS
                .iter()
                .map(|entry| ScriptApiDescriptor {
                    api_version: API_VERSION,
                    phase: ScriptPhaseId::GamePostSim,
                    callback: entry.callback,
                    values: entry.values,
                    methods: entry.methods,
                    unavailable: entry.unavailable,
                    capabilities: API_CAPABILITIES,
                    effect: entry.effect,
                })
                .collect(),
        }
    }
}
#[derive(Clone)]
/// Rhai callback context containing copied frame data and bounded helpers.
///
/// Scripts access its registered Rhai properties; Rust fields are private.
pub struct GamePostSimContext {
    frame: Map,
    math: ScriptMath,
    camera: CopiedObject,
    camera_control: InvocationCameraControl,
    player: CopiedObject,
    vehicle: CopiedObject,
    input: CopiedInput,
    raycast: InvocationRaycast,
    spawn: InvocationSpawn,
    ui: InvocationScriptUi,
    dependencies: InvocationScriptDependencies,
    debug: InvocationDebugDraw,
    log: InvocationLog,
    can_read: bool,
    can_log: bool,
}

#[derive(Clone)]
pub(crate) struct InvocationCameraControl {
    allowed: bool,
    sequence: u64,
    request: Arc<Mutex<Option<Option<crate::api::scripts::camera_override::CameraOverride>>>>,
}

impl InvocationCameraControl {
    pub(crate) fn new(allowed: bool, sequence: u64) -> Self {
        Self {
            allowed,
            sequence,
            request: Arc::new(Mutex::new(None)),
        }
    }

    fn set_offset(
        &mut self,
        offset: Map,
        euler_degrees: Map,
    ) -> Result<(), Box<rhai::EvalAltResult>> {
        require_capability(self.allowed, "game.camera.control")?;
        let offset = parse_script_vector(offset, "camera offset")?;
        let euler_degrees = parse_script_vector(euler_degrees, "camera rotation")?;
        if offset.iter().any(|value| value.abs() > 1000.0) {
            return Err("camera world offset components must be within ±1000".into());
        }
        if euler_degrees.iter().any(|value| value.abs() > 180.0) {
            return Err("camera yaw, pitch, and roll must be within ±180 degrees".into());
        }
        *self
            .request
            .lock()
            .map_err(|_| Box::<rhai::EvalAltResult>::from("camera override lock unavailable"))? =
            Some(Some(crate::api::scripts::camera_override::CameraOverride {
                offset,
                euler_degrees,
            }));
        Ok(())
    }

    fn clear(&mut self) -> Result<(), Box<rhai::EvalAltResult>> {
        require_capability(self.allowed, "game.camera.control")?;
        *self
            .request
            .lock()
            .map_err(|_| Box::<rhai::EvalAltResult>::from("camera override lock unavailable"))? =
            Some(None);
        Ok(())
    }

    fn available(&mut self) -> bool {
        crate::api::scripts::camera_override::hook_available()
    }

    fn external_calls(&mut self) -> i64 {
        crate::api::scripts::camera_override::external_getter_calls().min(i64::MAX as u64) as i64
    }

    fn external_override_calls(&mut self) -> i64 {
        crate::api::scripts::camera_override::external_override_calls().min(i64::MAX as u64) as i64
    }

    pub(crate) fn finish(
        &self,
    ) -> Option<Option<crate::api::scripts::camera_override::CameraOverride>> {
        self.request.lock().ok()?.take()
    }

    pub(crate) fn sequence(&self) -> u64 {
        self.sequence
    }
}

#[derive(Clone)]
pub(crate) struct InvocationScriptUi {
    allowed: bool,
    events: Vec<ScriptPanelEvent>,
    panels: Arc<Mutex<Vec<ScriptPanelModel>>>,
    published: Arc<AtomicBool>,
}

#[derive(Clone, Default)]
pub(crate) struct InvocationScriptDependencies {
    available: BTreeMap<String, bool>,
}

impl InvocationScriptDependencies {
    pub(crate) fn new(available: BTreeMap<String, bool>) -> Self {
        Self { available }
    }

    fn available(&mut self, id: Dynamic) -> Result<bool, Box<rhai::EvalAltResult>> {
        let id = ui_string(id, "dependency ID")?;
        if id.is_empty() || id.len() > 64 || id.chars().any(char::is_control) {
            return Err("dependency ID must be a short non-empty identifier".into());
        }
        Ok(self.available.get(&id).copied().unwrap_or(false))
    }
}

#[derive(Clone)]
pub(crate) struct ScriptPanelBuilder {
    model: ScriptPanelModel,
}

#[derive(Clone)]
struct CopiedInput {
    frame: InputFrame,
    allowed: bool,
}

/// Invocation-local, catalog-gated spawn effect collector.
#[derive(Clone)]
pub(crate) struct InvocationSpawn {
    allowed: bool,
    capacity: bool,
    source: ScriptEvent,
    request: Arc<Mutex<Option<crate::api::scripts::manager::ScriptSpawnEffect>>>,
}

impl InvocationSpawn {
    fn disabled() -> Self {
        Self {
            allowed: false,
            capacity: false,
            source: ScriptEvent {
                request_id: None,
                id: ScriptId::parse("disabled").expect("static script ID is valid"),
                generation: ScriptGeneration(0),
                phase: ScriptPhaseId::GamePostSim,
                callback: String::new(),
                kind: ScriptEventKind::Log,
                status: None,
                auto_reload: false,
                reload_error: None,
                line: None,
                column: None,
                text: String::new(),
            },
            request: Arc::new(Mutex::new(None)),
        }
    }

    pub(crate) fn new(allowed: bool, capacity: bool, source: ScriptEvent) -> Self {
        Self {
            allowed,
            capacity,
            source,
            request: Arc::new(Mutex::new(None)),
        }
    }

    fn ready(&mut self) -> bool {
        self.allowed && self.capacity && self.request.lock().is_ok_and(|request| request.is_none())
    }

    fn request(
        &mut self,
        catalog_id: Dynamic,
        transform: Array,
    ) -> Result<i64, Box<rhai::EvalAltResult>> {
        require_capability(self.allowed, "game.spawn")?;
        if !self.capacity {
            return Err("spawn request capacity is full".into());
        }
        let catalog_id = ui_string(catalog_id, "catalog ID")?;
        let asset_key = crate::api::scripts::spawn_catalog::asset_key(&catalog_id)
            .ok_or("catalog entry is not spawnable (known problematic/red catalog status)")?;
        if transform.len() != 16 {
            return Err("spawn transform must contain exactly 16 numbers".into());
        }
        let mut values = [0.0_f32; 16];
        for (index, value) in transform.into_iter().enumerate() {
            let value = numeric_value(value, "spawn transform values must be numbers")?;
            if !value.is_finite() || value.abs() > 100_000.0 {
                return Err("spawn transform values must be finite and within ±100000".into());
            }
            values[index] = value as f32;
        }
        let matrix = Matrix4x4::from_rows([
            [values[0], values[1], values[2], values[3]],
            [values[4], values[5], values[6], values[7]],
            [values[8], values[9], values[10], values[11]],
            [values[12], values[13], values[14], values[15]],
        ]);
        let request_id = crate::api::ecs::NEXT_INTENT_ID.fetch_add(1, Ordering::Relaxed);
        let script_request_id = i64::try_from(request_id)
            .map_err(|_| "spawn request ID exceeded script integer range")?;
        let mut request = self
            .request
            .lock()
            .map_err(|_| Box::<rhai::EvalAltResult>::from("spawn collector unavailable"))?;
        if request.is_some() {
            return Err("only one spawn request is allowed per callback".into());
        }
        *request = Some(crate::api::scripts::manager::ScriptSpawnEffect {
            request_id,
            catalog_id,
            asset_key: asset_key.to_owned(),
            matrix,
            origin: self.source.clone(),
        });
        Ok(script_request_id)
    }

    pub(crate) fn finish(&self) -> Option<crate::api::scripts::manager::ScriptSpawnEffect> {
        self.request.lock().ok()?.take()
    }
}

#[derive(Clone, Default)]
struct ScriptFormatter;

impl ScriptFormatter {
    fn fixed(&mut self, value: Dynamic) -> Result<String, Box<rhai::EvalAltResult>> {
        let value = numeric_value(value, "fixed formatter requires a number")?;
        Ok(crate::api::format::fixed(value))
    }

    fn vector(&mut self, value: Dynamic) -> Result<String, Box<rhai::EvalAltResult>> {
        if value.is_unit() {
            return Ok("--".to_owned());
        }
        let map = value.try_cast::<Map>().ok_or_else(|| {
            Box::<rhai::EvalAltResult>::from("vector formatter requires a vector map or ()")
        })?;
        if map.len() != 3 {
            return Err("vector formatter requires exactly x, y, z components".into());
        }
        let mut components = [0.0; 3];
        for (index, name) in ["x", "y", "z"].into_iter().enumerate() {
            let value = map.get(name).cloned().ok_or_else(|| {
                Box::<rhai::EvalAltResult>::from(format!("vector component {name} is missing"))
            })?;
            components[index] = numeric_value(value, "vector components must be numbers")?;
        }
        Ok(crate::api::format::vector_components(components))
    }
}

pub(super) fn numeric_value(
    value: Dynamic,
    error: &'static str,
) -> Result<f64, Box<rhai::EvalAltResult>> {
    value
        .clone()
        .try_cast::<rhai::FLOAT>()
        .or_else(|| value.try_cast::<rhai::INT>().map(|value| value as f64))
        .ok_or_else(|| error.into())
}
impl CopiedInput {
    fn available(&mut self) -> Result<bool, Box<rhai::EvalAltResult>> {
        require_capability(self.allowed, "input.read")?;
        Ok(self.frame.is_available())
    }

    fn query(&self, key: &str, state: u8) -> Result<bool, Box<rhai::EvalAltResult>> {
        require_capability(self.allowed, "input.read")?;
        let key = crate::api::ecs::input_key::InputKey::parse(key).ok_or_else(|| {
            Box::<rhai::EvalAltResult>::from("unknown input key; use show_keys for supported names")
        })?;
        let key = key.id();
        Ok(match state {
            0 => self.frame.is_down(key),
            1 => self.frame.just_pressed(key),
            _ => self.frame.just_released(key),
        })
    }
}

impl InvocationScriptUi {
    pub(crate) fn new(allowed: bool, events: Vec<ScriptPanelEvent>) -> Self {
        Self {
            allowed,
            events,
            panels: Arc::new(Mutex::new(Vec::new())),
            published: Arc::new(AtomicBool::new(false)),
        }
    }

    #[cfg(test)]
    pub(crate) fn disabled() -> Self {
        Self::new(false, Vec::new())
    }

    fn panel(
        &mut self,
        id: Dynamic,
        title: Dynamic,
    ) -> Result<ScriptPanelBuilder, Box<rhai::EvalAltResult>> {
        require_capability(self.allowed, "ui.panels")?;
        let id = ui_string(id, "panel ID")?;
        let title = ui_string(title, "panel title")?;
        validate_ui_id(&id, "panel")?;
        validate_ui_text(&title)?;
        Ok(ScriptPanelBuilder {
            model: ScriptPanelModel {
                id,
                title,
                widgets: Vec::new(),
            },
        })
    }

    fn events(&mut self, panel_id: Dynamic) -> Result<Map, Box<rhai::EvalAltResult>> {
        require_capability(self.allowed, "ui.panels")?;
        let panel_id = ui_string(panel_id, "panel ID")?;
        validate_ui_id(&panel_id, "panel")?;
        let mut values = Map::new();
        for event in self
            .events
            .iter()
            .filter(|event| event.panel_id == panel_id)
        {
            let value = match &event.value {
                ScriptPanelValue::Bool(value) => Dynamic::from(*value),
                ScriptPanelValue::Number(value) => Dynamic::from(*value),
                ScriptPanelValue::Text(value) => Dynamic::from(value.clone()),
            };
            values.insert(event.widget_id.clone().into(), value);
        }
        Ok(values)
    }

    fn publish(&mut self, builder: ScriptPanelBuilder) -> Result<(), Box<rhai::EvalAltResult>> {
        require_capability(self.allowed, "ui.panels")?;
        validate_ui_id(&builder.model.id, "panel")?;
        validate_ui_text(&builder.model.title)?;
        if builder.model.widgets.len() > crate::api::scripts::MAX_WIDGETS_PER_PANEL {
            return Err("panel widget limit exceeded".into());
        }
        let Ok(mut panels) = self.panels.lock() else {
            return Err("panel publication collector unavailable".into());
        };
        if panels.len() >= crate::api::scripts::MAX_PANELS_PER_SCRIPT {
            return Err("script panel limit exceeded".into());
        }
        if panels.iter().any(|panel| panel.id == builder.model.id) {
            return Err("duplicate panel ID in this callback".into());
        }
        panels.push(builder.model);
        self.published.store(true, Ordering::Release);
        Ok(())
    }

    pub(crate) fn finish(&self) -> Option<Vec<ScriptPanelModel>> {
        self.published.load(Ordering::Acquire).then(|| {
            self.panels
                .lock()
                .map(|mut panels| std::mem::take(&mut *panels))
                .unwrap_or_default()
        })
    }
}

impl ScriptPanelBuilder {
    fn append(
        &mut self,
        id: &str,
        kind: ScriptPanelWidgetKind,
    ) -> Result<(), Box<rhai::EvalAltResult>> {
        validate_ui_id(id, "widget")?;
        if self.model.widgets.len() >= crate::api::scripts::MAX_WIDGETS_PER_PANEL {
            return Err("panel widget limit exceeded".into());
        }
        if self.model.widgets.iter().any(|widget| widget.id == id) {
            return Err("duplicate widget ID in panel".into());
        }
        self.model.widgets.push(ScriptPanelWidget {
            id: id.to_owned(),
            kind,
        });
        Ok(())
    }

    fn heading(&mut self, id: Dynamic, text: Dynamic) -> Result<Self, Box<rhai::EvalAltResult>> {
        let id = ui_string(id, "widget ID")?;
        let text = ui_string(text, "heading text")?;
        validate_ui_text(&text)?;
        self.append(&id, ScriptPanelWidgetKind::Heading { text })?;
        Ok(self.clone())
    }

    fn label(&mut self, id: Dynamic, text: Dynamic) -> Result<Self, Box<rhai::EvalAltResult>> {
        let id = ui_string(id, "widget ID")?;
        let text = ui_string(text, "label text")?;
        validate_ui_text(&text)?;
        self.append(&id, ScriptPanelWidgetKind::Label { text })?;
        Ok(self.clone())
    }

    fn status(
        &mut self,
        id: Dynamic,
        text: Dynamic,
        level: Dynamic,
    ) -> Result<Self, Box<rhai::EvalAltResult>> {
        let id = ui_string(id, "widget ID")?;
        let text = ui_string(text, "status text")?;
        let level = ui_string(level, "status level")?;
        validate_ui_text(&text)?;
        let level = match level.as_str() {
            "stable" => crate::api::scripts::ScriptStatusLevel::Stable,
            "problematic" => crate::api::scripts::ScriptStatusLevel::Problematic,
            "experimental" => crate::api::scripts::ScriptStatusLevel::Experimental,
            _ => return Err("status level must be stable, problematic, or experimental".into()),
        };
        self.append(&id, ScriptPanelWidgetKind::Status { text, level })?;
        Ok(self.clone())
    }

    fn checkbox(
        &mut self,
        id: Dynamic,
        label: Dynamic,
        value: bool,
    ) -> Result<Self, Box<rhai::EvalAltResult>> {
        let id = ui_string(id, "widget ID")?;
        let label = ui_string(label, "widget label")?;
        validate_ui_text(&label)?;
        self.append(&id, ScriptPanelWidgetKind::Checkbox { label, value })?;
        Ok(self.clone())
    }

    fn slider(
        &mut self,
        id: Dynamic,
        label: Dynamic,
        value: rhai::FLOAT,
        min: rhai::FLOAT,
        max: rhai::FLOAT,
    ) -> Result<Self, Box<rhai::EvalAltResult>> {
        let id = ui_string(id, "widget ID")?;
        let label = ui_string(label, "widget label")?;
        validate_ui_number(value, min, max)?;
        validate_ui_text(&label)?;
        self.append(
            &id,
            ScriptPanelWidgetKind::Slider {
                label,
                value,
                min,
                max,
            },
        )?;
        Ok(self.clone())
    }

    fn number(
        &mut self,
        id: Dynamic,
        label: Dynamic,
        value: rhai::FLOAT,
        min: rhai::FLOAT,
        max: rhai::FLOAT,
        step: rhai::FLOAT,
    ) -> Result<Self, Box<rhai::EvalAltResult>> {
        let id = ui_string(id, "widget ID")?;
        let label = ui_string(label, "widget label")?;
        validate_ui_number(value, min, max)?;
        if !step.is_finite() || step <= 0.0 {
            return Err("numeric widget step must be finite and positive".into());
        }
        validate_ui_text(&label)?;
        self.append(
            &id,
            ScriptPanelWidgetKind::Number {
                label,
                value,
                min,
                max,
                step,
            },
        )?;
        Ok(self.clone())
    }

    fn select(
        &mut self,
        id: Dynamic,
        label: Dynamic,
        value: Dynamic,
        options: Array,
    ) -> Result<Self, Box<rhai::EvalAltResult>> {
        let id = ui_string(id, "widget ID")?;
        let label = ui_string(label, "widget label")?;
        let value = ui_string(value, "selector value")?;
        validate_ui_text(&label)?;
        validate_ui_text(&value)?;
        if options.is_empty() || options.len() > crate::api::scripts::MAX_PANEL_OPTIONS {
            return Err("selector option limit exceeded".into());
        }
        let mut parsed = Vec::with_capacity(options.len());
        for option in options {
            let option = option
                .try_cast::<rhai::ImmutableString>()
                .ok_or("selector options must be strings")?
                .to_string();
            validate_ui_text(&option)?;
            parsed.push(option);
        }
        if !parsed.iter().any(|option| option == &value) {
            return Err("selector value must match an option".into());
        }
        self.append(
            &id,
            ScriptPanelWidgetKind::Select {
                label,
                value,
                options: parsed,
            },
        )?;
        Ok(self.clone())
    }

    fn button(&mut self, id: Dynamic, label: Dynamic) -> Result<Self, Box<rhai::EvalAltResult>> {
        let id = ui_string(id, "widget ID")?;
        let label = ui_string(label, "widget label")?;
        validate_ui_text(&label)?;
        self.append(&id, ScriptPanelWidgetKind::Button { label })?;
        Ok(self.clone())
    }
}

fn ui_string(value: Dynamic, label: &str) -> Result<String, Box<rhai::EvalAltResult>> {
    value
        .try_cast::<rhai::ImmutableString>()
        .map(|value| value.to_string())
        .ok_or_else(|| format!("{label} must be a string").into())
}

fn validate_ui_id(value: &str, kind: &str) -> Result<(), Box<rhai::EvalAltResult>> {
    if value.is_empty()
        || value.len() > crate::api::scripts::MAX_SCRIPT_ID_BYTES
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
    {
        return Err(format!("invalid {kind} ID").into());
    }
    Ok(())
}

fn validate_ui_text(value: &str) -> Result<(), Box<rhai::EvalAltResult>> {
    if value.len() > crate::api::scripts::MAX_PANEL_TEXT_BYTES {
        return Err("panel text limit exceeded".into());
    }
    Ok(())
}

fn validate_ui_number(value: f64, min: f64, max: f64) -> Result<(), Box<rhai::EvalAltResult>> {
    if !value.is_finite()
        || !min.is_finite()
        || !max.is_finite()
        || min >= max
        || value < min
        || value > max
        || min.abs().max(max.abs()) > 100_000.0
    {
        return Err(
            "widget value must be finite, within its ordered bounds, and within ±100000".into(),
        );
    }
    Ok(())
}
#[derive(Clone)]
struct CopiedObject {
    values: Map,
    write: Option<InvocationVehicleWrite>,
}

const MAX_RAYCAST_TICKETS: usize = 1024;
static NEXT_RAYCAST_TICKET: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Default)]
pub(crate) struct RaycastTicketStore(Arc<Mutex<BTreeMap<u64, RaycastTicketEntry>>>);

#[derive(Clone)]
struct RaycastTicketEntry {
    owner: ScriptId,
    generation: ScriptGeneration,
    expires_at: u64,
    available: Option<bool>,
    state_present: bool,
    fraction: Option<f32>,
    hit_position: Option<[f32; 3]>,
}

impl RaycastTicketStore {
    pub(crate) fn has_capacity(&self) -> bool {
        self.0
            .lock()
            .is_ok_and(|entries| entries.len() < MAX_RAYCAST_TICKETS)
    }

    pub(crate) fn insert_pending(&self, request: &ScriptRaycastRequest) -> bool {
        let Ok(mut entries) = self.0.lock() else {
            return false;
        };
        if entries.len() >= MAX_RAYCAST_TICKETS {
            return false;
        }
        entries.insert(
            request.ticket,
            RaycastTicketEntry {
                owner: request.source.id.clone(),
                generation: request.source.generation,
                expires_at: request.expires_at,
                available: None,
                state_present: false,
                fraction: None,
                hit_position: None,
            },
        );
        true
    }

    pub(crate) fn complete(&self, completion: ScriptRaycastCompletion) -> bool {
        let Ok(mut entries) = self.0.lock() else {
            return false;
        };
        let Some(entry) = entries.get_mut(&completion.ticket) else {
            return false;
        };
        if entry.owner != completion.source.id
            || entry.generation != completion.source.generation
            || entry.available.is_some()
        {
            return false;
        }
        entry.available = Some(completion.available);
        entry.state_present = completion.state_present;
        entry.fraction = completion.fraction;
        entry.hit_position = completion.hit_position;
        true
    }

    pub(crate) fn expire(&self, callback_sequence: u64) {
        if let Ok(mut entries) = self.0.lock() {
            entries.retain(|_, entry| callback_sequence <= entry.expires_at.saturating_add(120));
        }
    }

    pub(crate) fn remove_owner(&self, owner: &ScriptId) {
        if let Ok(mut entries) = self.0.lock() {
            entries.retain(|_, entry| &entry.owner != owner);
        }
    }

    pub(crate) fn remove_ticket(&self, ticket: u64) {
        if let Ok(mut entries) = self.0.lock() {
            entries.remove(&ticket);
        }
    }

    pub(crate) fn latest_result_for(&self, owner: &str) -> Option<Option<[f32; 3]>> {
        let entries = self.0.lock().ok()?;
        entries
            .iter()
            .rev()
            .find(|(_, entry)| entry.owner.as_str() == owner && entry.available.is_some())
            .map(|(_, entry)| entry.hit_position)
    }

    fn result(&self, ticket: i64, source: &ScriptEvent, callback_sequence: u64) -> Map {
        let Some(ticket) = u64::try_from(ticket).ok() else {
            return raycast_result_map("unknown", false, None, None, false);
        };
        let Ok(mut entries) = self.0.lock() else {
            return raycast_result_map("unavailable", false, None, None, false);
        };
        let Some(entry) = entries.get(&ticket) else {
            return raycast_result_map("unknown", false, None, None, false);
        };
        if entry.owner != source.id || entry.generation != source.generation {
            return raycast_result_map("unknown", false, None, None, false);
        }
        if callback_sequence > entry.expires_at {
            entries.remove(&ticket);
            return raycast_result_map("expired", false, None, None, false);
        }
        let is_terminal = entry.available.is_some();
        let result = match entry.available {
            None => raycast_result_map("pending", false, None, None, false),
            Some(false) => raycast_result_map("unavailable", false, None, None, false),
            Some(true) => raycast_result_map(
                "complete",
                entry.fraction.is_some_and(|fraction| fraction < 1.0)
                    && entry.hit_position.is_some(),
                entry.hit_position,
                entry.fraction,
                entry.state_present,
            ),
        };
        if is_terminal {
            entries.remove(&ticket);
        }
        result
    }
}

fn raycast_result_map(
    state: &str,
    has_hit: bool,
    hit_position: Option<[f32; 3]>,
    fraction: Option<f32>,
    state_present: bool,
) -> Map {
    let mut result = Map::new();
    result.insert("state".into(), Dynamic::from(state.to_owned()));
    result.insert("has_hit".into(), Dynamic::from(has_hit));
    result.insert("hit_position".into(), vector(hit_position));
    result.insert(
        "fraction".into(),
        fraction.map_or(Dynamic::UNIT, |value| Dynamic::from(f64::from(value))),
    );
    result.insert("state_present".into(), Dynamic::from(state_present));
    result
}

#[derive(Clone)]
pub(crate) struct InvocationRaycast {
    active: Arc<AtomicBool>,
    request: Arc<Mutex<Option<ScriptRaycastRequest>>>,
    allowed: bool,
    capacity: bool,
    source: ScriptEvent,
    callback_sequence: u64,
    tickets: RaycastTicketStore,
    camera_position: Option<[f32; 3]>,
    camera_direction: Option<[f32; 3]>,
}

impl InvocationRaycast {
    #[cfg(test)]
    fn disabled() -> Self {
        Self::new(
            false,
            false,
            ScriptEvent {
                request_id: None,
                id: ScriptId::parse("invalid").expect("valid static script ID"),
                generation: ScriptGeneration(0),
                phase: ScriptPhaseId::GamePostSim,
                callback: "on_post_sim".into(),
                kind: ScriptEventKind::Log,
                status: None,
                auto_reload: false,
                reload_error: None,
                line: None,
                column: None,
                text: String::new(),
            },
            0,
            RaycastTicketStore::default(),
            None,
            None,
        )
    }

    pub(crate) fn new(
        allowed: bool,
        capacity: bool,
        source: ScriptEvent,
        callback_sequence: u64,
        tickets: RaycastTicketStore,
        camera_position: Option<[f32; 3]>,
        camera_direction: Option<[f32; 3]>,
    ) -> Self {
        Self {
            active: Arc::new(AtomicBool::new(true)),
            request: Arc::new(Mutex::new(None)),
            allowed,
            capacity,
            source,
            callback_sequence,
            tickets,
            camera_position,
            camera_direction,
        }
    }

    pub(crate) fn finish(&self) -> Option<ScriptRaycastRequest> {
        self.active.store(false, Ordering::Relaxed);
        self.request.lock().ok()?.take()
    }

    fn request(
        &mut self,
        origin: Map,
        direction: Map,
        max_distance: Dynamic,
        filter_name: &str,
    ) -> Result<i64, Box<rhai::EvalAltResult>> {
        require_capability(self.allowed, "game.raycast")?;
        if !self.active.load(Ordering::Relaxed) {
            return Err("raycast context has expired".into());
        }
        if !self.capacity || !self.tickets.has_capacity() {
            return Err("raycast request queue full; retry on a later callback".into());
        }
        let mut request = self
            .request
            .lock()
            .map_err(|_| Box::<rhai::EvalAltResult>::from("raycast request lock unavailable"))?;
        if request.is_some() {
            return Err("only one raycast request is allowed per invocation".into());
        }
        let origin = parse_script_vector(origin, "raycast origin")?;
        let direction = parse_script_vector(direction, "raycast direction")?;
        let length = (direction[0] * direction[0]
            + direction[1] * direction[1]
            + direction[2] * direction[2])
            .sqrt();
        if !length.is_finite() || length <= f32::EPSILON {
            return Err("raycast direction must have nonzero length".into());
        }
        let distance = numeric_value(max_distance, "raycast distance must be numeric")?;
        if !distance.is_finite() || distance <= 0.0 || distance > 500.0 {
            return Err("raycast distance must be greater than 0 and at most 500".into());
        }
        let filter = match filter_name {
            "static_only" => ScriptRaycastFilter::StaticOnly,
            "player_aim" => ScriptRaycastFilter::PlayerAim,
            _ => return Err("unknown raycast filter; use static_only or player_aim".into()),
        };
        if origin.iter().any(|value| value.abs() > 100_000.0) {
            return Err("raycast coordinates must be within [-100000,100000]".into());
        }
        let ticket = NEXT_RAYCAST_TICKET
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
                (next > 0 && next < i64::MAX as u64).then_some(next + 1)
            })
            .map_err(|_| Box::<rhai::EvalAltResult>::from("raycast ticket space exhausted"))?;
        let direction = direction.map(|value| value / length);
        *request = Some(ScriptRaycastRequest {
            ticket,
            origin,
            direction,
            max_distance: distance as f32,
            filter,
            source: self.source.clone(),
            expires_at: self.callback_sequence.saturating_add(120),
        });
        Ok(ticket as i64)
    }

    fn result(&mut self, ticket: rhai::INT) -> Result<Map, Box<rhai::EvalAltResult>> {
        require_capability(self.allowed, "game.raycast")?;
        Ok(self
            .tickets
            .result(ticket, &self.source, self.callback_sequence))
    }

    fn request_camera(
        &mut self,
        max_distance: Dynamic,
        filter_name: &str,
    ) -> Result<i64, Box<rhai::EvalAltResult>> {
        require_capability(self.allowed, "game.raycast")?;
        if !self.active.load(Ordering::Relaxed) {
            return Err("raycast context has expired".into());
        }
        let origin = self
            .camera_position
            .ok_or_else(|| Box::<rhai::EvalAltResult>::from("camera position unavailable"))?;
        let direction = self
            .camera_direction
            .ok_or_else(|| Box::<rhai::EvalAltResult>::from("camera direction unavailable"))?;
        self.request(
            vector_map(origin),
            vector_map(direction),
            max_distance,
            filter_name,
        )
    }
}

fn vector_map(value: [f32; 3]) -> Map {
    let mut map = Map::new();
    for (name, value) in ["x", "y", "z"].into_iter().zip(value) {
        map.insert(name.into(), Dynamic::from(f64::from(value)));
    }
    map
}

fn parse_script_vector(
    map: Map,
    label: &'static str,
) -> Result<[f32; 3], Box<rhai::EvalAltResult>> {
    if map.len() != 3 {
        return Err(format!("{label} requires exactly x, y, z components").into());
    }
    let mut components = [0.0; 3];
    for (index, name) in ["x", "y", "z"].into_iter().enumerate() {
        let value = map.get(name).cloned().ok_or_else(|| {
            Box::<rhai::EvalAltResult>::from(format!("{label} is missing {name}"))
        })?;
        let value = numeric_value(value, "raycast vector components must be numeric")?;
        if !value.is_finite() || value.abs() > 100_000.0 {
            return Err(format!("{label} components must be finite and within ±100000").into());
        }
        components[index] = value as f32;
    }
    Ok(components)
}

pub(crate) const MAX_VELOCITY_COMPONENT: f64 = 250.0;

/// Owned invocation-local intent. It never contains an engine pointer or entity.
#[derive(Clone)]
pub(crate) struct InvocationVehicleWrite {
    active: Arc<AtomicBool>,
    value: Arc<Mutex<Option<[f32; 3]>>>,
    allowed: bool,
    available: bool,
    capacity: bool,
}

impl InvocationVehicleWrite {
    pub(crate) fn new(allowed: bool, available: bool, capacity: bool) -> Self {
        Self {
            active: Arc::new(AtomicBool::new(true)),
            value: Arc::new(Mutex::new(None)),
            allowed,
            available,
            capacity,
        }
    }
    pub(crate) fn finish(&self) -> Option<[f32; 3]> {
        self.active.store(false, Ordering::Relaxed);
        self.value.lock().ok()?.take()
    }
    fn set_velocity(&self, map: Map) -> Result<(), Box<rhai::EvalAltResult>> {
        require_capability(self.allowed, "game.vehicle.write")?;
        if !self.active.load(Ordering::Relaxed) {
            return Err("vehicle write context has expired".into());
        }
        if !self.available {
            return Err("current vehicle velocity is unavailable".into());
        }
        if !self.capacity {
            return Err("script velocity effect queue full (32); retry later".into());
        }
        if map.len() != 3 {
            return Err("velocity requires exactly x, y, z numeric components".into());
        }
        let mut velocity = [0.0; 3];
        for (index, name) in ["x", "y", "z"].into_iter().enumerate() {
            let value = map.get(name).ok_or_else(|| {
                Box::<rhai::EvalAltResult>::from(format!("velocity component {name} is missing"))
            })?;
            let value = value
                .clone()
                .try_cast::<rhai::FLOAT>()
                .or_else(|| value.clone().try_cast::<rhai::INT>().map(|v| v as f64))
                .ok_or_else(|| {
                    Box::<rhai::EvalAltResult>::from(format!(
                        "velocity component {name} must be numeric"
                    ))
                })?;
            if !value.is_finite() || value.abs() > MAX_VELOCITY_COMPONENT {
                return Err(format!(
                    "velocity component {name} must be finite and within [-250,250] units/s"
                )
                .into());
            }
            velocity[index] = value as f32;
        }
        let mut pending = self
            .value
            .lock()
            .map_err(|_| Box::<rhai::EvalAltResult>::from("vehicle effect lock unavailable"))?;
        if pending.is_some() {
            return Err("only one vehicle velocity request is allowed per invocation".into());
        }
        *pending = Some(velocity);
        Ok(())
    }
}

#[derive(Clone)]
pub(crate) struct InvocationLog {
    sink: ScriptEventSink,
    template: ScriptEvent,
    active: Arc<AtomicBool>,
    count: Arc<AtomicUsize>,
}
impl InvocationLog {
    pub(crate) fn new(sink: ScriptEventSink, template: ScriptEvent) -> Self {
        Self {
            sink,
            template,
            active: Arc::new(AtomicBool::new(true)),
            count: Arc::new(AtomicUsize::new(0)),
        }
    }
    pub(crate) fn finish(&self) {
        self.active.store(false, Ordering::Relaxed);
    }
    fn info(&mut self, text: &str) {
        if !self.active.load(Ordering::Relaxed) {
            return;
        }
        if self.count.fetch_add(1, Ordering::Relaxed) >= MAX_LOGS_PER_INVOCATION {
            return;
        }
        let mut event = self.template.clone();
        event.text = bounded_text(text, MAX_EVENT_BYTES);
        self.sink.emit(event);
    }
}
fn vector(value: Option<[f32; 3]>) -> Dynamic {
    value.map_or(Dynamic::UNIT, |v| {
        let mut map = Map::new();
        for (name, value) in ["x", "y", "z"].into_iter().zip(v) {
            map.insert(name.into(), Dynamic::from(f64::from(value)));
        }
        Dynamic::from_map(map)
    })
}
fn matrix_values(value: Option<Matrix4x4>) -> Dynamic {
    value.map_or(Dynamic::UNIT, |matrix| {
        Dynamic::from_array(
            matrix
                .values
                .into_iter()
                .flatten()
                .map(|value| Dynamic::from(f64::from(value)))
                .collect(),
        )
    })
}
fn object(
    position: Option<[f32; 3]>,
    transform: Option<Matrix4x4>,
    forward: Option<[f32; 3]>,
    rotation: Option<[f32; 3]>,
    velocity: Option<[f32; 3]>,
    acceleration: Option<[f32; 3]>,
    angular_velocity: Option<[f32; 3]>,
) -> CopiedObject {
    let mut values = Map::new();
    values.insert(
        "available".into(),
        Dynamic::from(
            position.is_some()
                || rotation.is_some()
                || transform.is_some()
                || forward.is_some()
                || velocity.is_some()
                || acceleration.is_some()
                || angular_velocity.is_some(),
        ),
    );
    for (name, value) in [
        ("position", position),
        ("forward", forward),
        ("rotation", rotation),
        ("velocity", velocity),
        ("acceleration", acceleration),
        ("angular_velocity", angular_velocity),
    ] {
        values.insert(name.into(), vector(value));
    }
    values.insert("transform".into(), matrix_values(transform));
    CopiedObject {
        values,
        write: None,
    }
}
impl GamePostSimContext {
    #[cfg(test)]
    pub(crate) fn new(
        snapshot: GameSnapshot,
        log: InvocationLog,
        capabilities: &[String],
        write: InvocationVehicleWrite,
        debug: InvocationDebugDraw,
    ) -> Self {
        Self::new_with_raycast(
            snapshot,
            log,
            capabilities,
            write,
            InvocationRaycast::disabled(),
            debug,
        )
    }

    #[cfg(test)]
    pub(crate) fn new_with_raycast(
        snapshot: GameSnapshot,
        log: InvocationLog,
        capabilities: &[String],
        write: InvocationVehicleWrite,
        raycast: InvocationRaycast,
        debug: InvocationDebugDraw,
    ) -> Self {
        Self::new_with_ui(
            snapshot,
            log,
            capabilities,
            write,
            raycast,
            debug,
            InvocationScriptUi::disabled(),
        )
    }

    #[cfg(test)]
    pub(crate) fn new_with_ui(
        snapshot: GameSnapshot,
        log: InvocationLog,
        capabilities: &[String],
        write: InvocationVehicleWrite,
        raycast: InvocationRaycast,
        debug: InvocationDebugDraw,
        ui: InvocationScriptUi,
    ) -> Self {
        Self::new_with_dependencies(
            snapshot,
            log,
            capabilities,
            write,
            raycast,
            debug,
            ui,
            BTreeMap::new(),
        )
    }

    pub(crate) fn new_with_dependencies(
        snapshot: GameSnapshot,
        log: InvocationLog,
        capabilities: &[String],
        write: InvocationVehicleWrite,
        raycast: InvocationRaycast,
        debug: InvocationDebugDraw,
        ui: InvocationScriptUi,
        dependencies: BTreeMap<String, bool>,
    ) -> Self {
        let mut frame = Map::new();
        frame.insert("phase".into(), Dynamic::from("game.post_sim"));
        frame.insert(
            "sequence".into(),
            Dynamic::from(snapshot.callback_sequence.min(i64::MAX as u64) as i64),
        );
        frame.insert(
            "sample_sequence".into(),
            Dynamic::from(snapshot.sample_sequence.min(i64::MAX as u64) as i64),
        );
        frame.insert(
            "delta_seconds".into(),
            Dynamic::from(f64::from(snapshot.callback_delta_seconds)),
        );
        frame.insert(
            "elapsed_seconds".into(),
            Dynamic::from(snapshot.callback_elapsed_seconds),
        );
        frame.insert(
            "sample_age_seconds".into(),
            snapshot
                .sample_age_seconds
                .map_or(Dynamic::UNIT, |v| Dynamic::from(f64::from(v))),
        );
        Self {
            frame,
            math: ScriptMath,
            camera: camera_object(snapshot.camera_position, snapshot.camera_direction),
            camera_control: InvocationCameraControl::new(false, snapshot.callback_sequence),
            can_read: capabilities.iter().any(|id| id == "game.read"),
            can_log: capabilities.iter().any(|id| id == "log"),
            raycast,
            spawn: InvocationSpawn::disabled(),
            ui,
            dependencies: InvocationScriptDependencies::new(dependencies),
            player: object(
                snapshot.player_position,
                snapshot.player_transform,
                snapshot.player_forward,
                snapshot.player_rotation,
                snapshot.player_velocity,
                snapshot.player_acceleration,
                None,
            ),
            vehicle: {
                let mut vehicle = object(
                    snapshot.vehicle_position,
                    snapshot.vehicle_transform,
                    snapshot.vehicle_forward,
                    snapshot.vehicle_rotation,
                    snapshot.vehicle_velocity,
                    snapshot.vehicle_acceleration,
                    snapshot.vehicle_angular_velocity,
                );
                vehicle.write = Some(write);
                vehicle
            },
            input: CopiedInput {
                frame: snapshot.input_frame,
                allowed: capabilities.iter().any(|id| id == "input.read"),
            },
            debug,
            log,
        }
    }

    pub(crate) fn with_spawn(mut self, spawn: InvocationSpawn) -> Self {
        self.spawn = spawn;
        self
    }

    pub(crate) fn with_camera_control(mut self, control: InvocationCameraControl) -> Self {
        self.camera_control = control;
        self
    }
}
pub(crate) fn register_v1(engine: &mut Engine) {
    ScriptMath::register(engine);
    register_context_properties(engine);
    register_basic_debug_draw(engine);
    register_transformed_debug_draw(engine);
    register_value_helpers(engine);
    register_copied_object_properties(engine);
}

fn register_context_properties(engine: &mut Engine) {
    engine
        .register_type_with_name::<GamePostSimContext>("MadMaxPostSimContext")
        .register_type_with_name::<CopiedObject>("MadMaxCopiedObject")
        .register_type_with_name::<InvocationRaycast>("MadMaxRaycast")
        .register_type_with_name::<InvocationCameraControl>("MadMaxCameraControl")
        .register_type_with_name::<InvocationSpawn>("MadMaxEntitySpawner")
        .register_type_with_name::<InvocationScriptUi>("MadMaxScriptUi")
        .register_type_with_name::<InvocationScriptDependencies>("MadMaxScriptDependencies")
        .register_type_with_name::<ScriptPanelBuilder>("MadMaxPanelBuilder")
        .register_type_with_name::<CopiedInput>("MadMaxInputFrame")
        .register_type_with_name::<ScriptFormatter>("MadMaxFormatter")
        .register_type_with_name::<InvocationLog>("MadMaxLog")
        .register_type_with_name::<InvocationDebugDraw>("MadMaxDebugDraw")
        .register_get("frame", |ctx: &mut GamePostSimContext| ctx.frame.clone())
        .register_get("math", |ctx: &mut GamePostSimContext| ctx.math)
        .register_get("camera", |ctx: &mut GamePostSimContext| {
            require_capability(ctx.can_read, "game.read")?;
            Ok::<_, Box<rhai::EvalAltResult>>(ctx.camera.clone())
        })
        .register_get("camera_control", |ctx: &mut GamePostSimContext| {
            Ok::<_, Box<rhai::EvalAltResult>>(ctx.camera_control.clone())
        })
        .register_get("player", |ctx: &mut GamePostSimContext| {
            require_capability(ctx.can_read, "game.read")?;
            Ok::<_, Box<rhai::EvalAltResult>>(ctx.player.clone())
        })
        .register_get("vehicle", |ctx: &mut GamePostSimContext| {
            require_capability(ctx.can_read, "game.read")?;
            Ok::<_, Box<rhai::EvalAltResult>>(ctx.vehicle.clone())
        })
        .register_get("raycast", |ctx: &mut GamePostSimContext| {
            require_capability(ctx.raycast.allowed, "game.raycast")?;
            Ok::<_, Box<rhai::EvalAltResult>>(ctx.raycast.clone())
        })
        .register_get("entities", |ctx: &mut GamePostSimContext| {
            require_capability(ctx.spawn.allowed, "game.spawn")?;
            Ok::<_, Box<rhai::EvalAltResult>>(ctx.spawn.clone())
        })
        .register_get("ui", |ctx: &mut GamePostSimContext| {
            require_capability(ctx.ui.allowed, "ui.panels")?;
            Ok::<_, Box<rhai::EvalAltResult>>(ctx.ui.clone())
        })
        .register_get("dependencies", |ctx: &mut GamePostSimContext| {
            Ok::<_, Box<rhai::EvalAltResult>>(ctx.dependencies.clone())
        })
        .register_get("input", |ctx: &mut GamePostSimContext| {
            require_capability(ctx.input.allowed, "input.read")?;
            Ok::<_, Box<rhai::EvalAltResult>>(ctx.input.clone())
        })
        .register_get("log", |ctx: &mut GamePostSimContext| {
            require_capability(ctx.can_log, "log")?;
            Ok::<_, Box<rhai::EvalAltResult>>(ctx.log.clone())
        })
        .register_get("gizmos", |ctx: &mut GamePostSimContext| {
            require_capability(ctx.debug.allowed, "debug.draw")?;
            Ok::<_, Box<rhai::EvalAltResult>>(ctx.debug.clone())
        })
        .register_get("format", |_ctx: &mut GamePostSimContext| ScriptFormatter)
        .register_fn("info", InvocationLog::info);
    engine
        .register_fn(
            "request",
            |raycast: &mut InvocationRaycast,
             origin: Map,
             direction: Map,
             distance: Dynamic,
             filter: &str| raycast.request(origin, direction, distance, filter),
        )
        .register_fn(
            "camera",
            |raycast: &mut InvocationRaycast, distance: Dynamic, filter: &str| {
                raycast.request_camera(distance, filter)
            },
        )
        .register_fn("result", InvocationRaycast::result)
        .register_fn("ready", InvocationSpawn::ready)
        .register_fn("request", InvocationSpawn::request)
        .register_fn("set_offset", InvocationCameraControl::set_offset)
        .register_fn("clear", InvocationCameraControl::clear)
        .register_fn("available", InvocationCameraControl::available)
        .register_fn("external_calls", InvocationCameraControl::external_calls)
        .register_fn(
            "external_override_calls",
            InvocationCameraControl::external_override_calls,
        )
        .register_fn("panel", InvocationScriptUi::panel)
        .register_fn("events", InvocationScriptUi::events)
        .register_fn("publish", InvocationScriptUi::publish)
        .register_fn("available", InvocationScriptDependencies::available)
        .register_fn("heading", ScriptPanelBuilder::heading)
        .register_fn("label", ScriptPanelBuilder::label)
        .register_fn("status", ScriptPanelBuilder::status)
        .register_fn("checkbox", ScriptPanelBuilder::checkbox)
        .register_fn("slider", ScriptPanelBuilder::slider)
        .register_fn("number", ScriptPanelBuilder::number)
        .register_fn("select", ScriptPanelBuilder::select)
        .register_fn("button", ScriptPanelBuilder::button);
}

fn register_basic_debug_draw(engine: &mut Engine) {
    engine
        .register_fn(
            "axes",
            |draw: &mut InvocationDebugDraw, position: Map, length: rhai::FLOAT| {
                draw.axes(position, length)
            },
        )
        .register_fn(
            "transformed_axes",
            |draw: &mut InvocationDebugDraw, transform: Array, length: rhai::FLOAT| {
                draw.transformed_axes(transform, length)
            },
        )
        .register_fn(
            "line",
            |draw: &mut InvocationDebugDraw, start: Map, end: Map, color: Map| {
                draw.line(start, end, color)
            },
        )
        .register_fn(
            "arrow",
            |draw: &mut InvocationDebugDraw, start: Map, end: Map, color: Map| {
                draw.arrow(start, end, color)
            },
        )
        .register_fn(
            "wire_sphere",
            |d: &mut InvocationDebugDraw, c: Map, r: rhai::FLOAT, color: Map| {
                d.wire_sphere(c, r, color)
            },
        )
        .register_fn(
            "xray_sphere",
            |d: &mut InvocationDebugDraw, c: Map, r: rhai::FLOAT, color: Map| {
                d.xray_sphere(c, r, color)
            },
        )
        .register_fn(
            "surface_wire_sphere",
            |d: &mut InvocationDebugDraw, c: Map, r: rhai::FLOAT, color: Map| {
                d.surface_wire_sphere(c, r, color)
            },
        )
        .register_fn(
            "wire_aabb",
            |d: &mut InvocationDebugDraw, min: Map, max: Map, color: Map| {
                d.wire_aabb(min, max, color)
            },
        );
}

fn register_transformed_debug_draw(engine: &mut Engine) {
    engine
        .register_fn(
            "wire_box",
            |d: &mut InvocationDebugDraw, transform: Array, min: Map, max: Map, color: Map| {
                d.wire_box(transform, min, max, color)
            },
        )
        .register_fn(
            "xray_wire_box",
            |d: &mut InvocationDebugDraw, transform: Array, min: Map, max: Map, color: Map| {
                d.xray_wire_box(transform, min, max, color)
            },
        )
        .register_fn(
            "solid_cube",
            |d: &mut InvocationDebugDraw, transform: Array, min: Map, max: Map, color: Map| {
                d.solid_shape(false, transform, min, max, color)
            },
        )
        .register_fn(
            "solid_arrow",
            |d: &mut InvocationDebugDraw, transform: Array, min: Map, max: Map, color: Map| {
                d.solid_shape(true, transform, min, max, color)
            },
        );
}

fn register_value_helpers(engine: &mut Engine) {
    engine
        .register_fn("fixed", ScriptFormatter::fixed)
        .register_fn("vector", ScriptFormatter::vector)
        .register_fn("down", |input: &mut CopiedInput, key: &str| {
            input.query(key, 0)
        })
        .register_fn("just_pressed", |input: &mut CopiedInput, key: &str| {
            input.query(key, 1)
        })
        .register_fn("just_released", |input: &mut CopiedInput, key: &str| {
            input.query(key, 2)
        })
        .register_fn("available", CopiedInput::available)
        .register_fn("set_velocity", |object: &mut CopiedObject, value: Map| {
            let write = object.write.as_ref().ok_or_else(|| {
                Box::<rhai::EvalAltResult>::from("set_velocity is only available on ctx.vehicle")
            })?;
            write.set_velocity(value)
        });
}

fn register_copied_object_properties(engine: &mut Engine) {
    for name in [
        "available",
        "position",
        "direction",
        "query_available",
        "state_present",
        "position",
        "origin",
        "direction",
        "hit_fraction",
        "hit_position",
        "has_hit",
        "transform",
        "forward",
        "rotation",
        "velocity",
        "acceleration",
        "angular_velocity",
    ] {
        engine.register_get(name, move |object: &mut CopiedObject| {
            object.values.get(name).cloned().unwrap_or(Dynamic::UNIT)
        });
    }
}

fn camera_object(position: Option<[f32; 3]>, direction: Option<[f32; 3]>) -> CopiedObject {
    let mut values = Map::new();
    values.insert(
        "available".into(),
        Dynamic::from(position.is_some() && direction.is_some()),
    );
    values.insert("position".into(), vector(position));
    values.insert("direction".into(), vector(direction));
    CopiedObject {
        values,
        write: None,
    }
}
fn require_capability(allowed: bool, id: &str) -> Result<(), Box<rhai::EvalAltResult>> {
    if allowed {
        Ok(())
    } else {
        Err(format!("capability {id} was not declared").into())
    }
}

#[cfg(test)]
mod math_integration_tests {
    use super::{GamePostSimContext, InvocationDebugDraw, InvocationLog, InvocationVehicleWrite};
    use crate::api::ecs::GameSnapshot;
    use crate::api::game::abi::types::Matrix4x4;
    use crate::api::scripts::engine::RhaiEngineFactory;
    use crate::api::scripts::*;
    use rhai::{Array, Map};

    #[test]
    fn ctx_math_is_pure_and_preserves_negative_z_forward() {
        let engine = RhaiEngineFactory::build();
        let (sink, _receiver) = crate::api::scripts::console::event_channel();
        let log = InvocationLog::new(
            ScriptEventSink(sink),
            ScriptEvent {
                request_id: None,
                id: ScriptId::parse("math_integration").expect("valid test script ID"),
                generation: ScriptGeneration(1),
                phase: ScriptPhaseId::GamePostSim,
                callback: "on_post_sim".into(),
                kind: ScriptEventKind::Log,
                status: None,
                auto_reload: false,
                reload_error: None,
                line: None,
                column: None,
                text: String::new(),
            },
        );
        let mut scope = rhai::Scope::new();
        scope.push(
            "ctx",
            GamePostSimContext::new(
                GameSnapshot::default(),
                log.clone(),
                &[],
                InvocationVehicleWrite::new(false, false, false),
                InvocationDebugDraw::new(false, 32, 256),
            ),
        );
        let results = engine
            .eval_with_scope::<Array>(
                &mut scope,
                r#"
                    let origin = ctx.math.vec3(0.0, 0.0, 0.0);
                    let forward = ctx.math.vec3(0.0, 0.0, -2.0);
                    let added = ctx.math.add(forward, #{x: 1, y: 2.0, z: -3});
                    [forward, added, ctx.math.normalize(forward),
                     ctx.math.direction(origin, forward),
                     ctx.math.subtract(added, forward), ctx.math.scale(forward, 2.0),
                     ctx.math.dot(forward, forward),
                     ctx.math.cross(#{x: 1, y: 0, z: 0}, #{x: 0, y: 1, z: 0}),
                     ctx.math.length(forward), ctx.math.distance(origin, forward),
                     ctx.math.lerp(origin, forward, 0.5),
                     ctx.math.normalize(origin), ctx.math.direction(origin, origin)]
                "#,
            )
            .expect("math should resolve through the v1 engine without capabilities");
        for (index, expected) in [
            (0, [0.0, 0.0, -2.0]),
            (1, [1.0, 2.0, -5.0]),
            (2, [0.0, 0.0, -1.0]),
            (3, [0.0, 0.0, -1.0]),
            (4, [1.0, 2.0, -3.0]),
            (5, [0.0, 0.0, -4.0]),
            (7, [0.0, 0.0, 1.0]),
            (10, [0.0, 0.0, -1.0]),
        ] {
            let vector = results[index].clone().cast::<Map>();
            assert_eq!(vector.len(), 3);
            for (component, expected) in ["x", "y", "z"].into_iter().zip(expected) {
                assert_eq!(vector[component].as_float().unwrap(), expected);
            }
        }
        assert_eq!(results[6].as_float().unwrap(), 4.0);
        assert_eq!(results[8].as_float().unwrap(), 2.0);
        assert_eq!(results[9].as_float().unwrap(), 2.0);
        assert!(results[11].is_unit());
        assert!(results[12].is_unit());
        let identity = Matrix4x4::from_rows([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]);
        // The native snapshot carries the observed horizontal -row-2 forward.
        // Check that the scripting adapter copies it without reflecting Z.
        scope.push(
            "native_ctx",
            GamePostSimContext::new(
                GameSnapshot {
                    player_transform: Some(identity),
                    player_forward: Some([0.0, 0.0, -1.0]),
                    ..GameSnapshot::default()
                },
                log.clone(),
                &["game.read".to_owned()],
                InvocationVehicleWrite::new(false, false, false),
                InvocationDebugDraw::new(false, 32, 256),
            ),
        );
        let copied = engine
            .eval_with_scope::<Array>(
                &mut scope,
                r#"
            let forward = native_ctx.player.forward;
            [native_ctx.player.forward, native_ctx.math.scale(forward, 2.0),
             native_ctx.player.forward]
        "#,
            )
            .expect("copied native forward should work with ctx.math");
        for (index, expected) in [(0, -1.0), (1, -2.0), (2, -1.0)] {
            let vector = copied[index].clone().cast::<Map>();
            assert_eq!(vector["x"].as_float().unwrap(), 0.0);
            assert_eq!(vector["y"].as_float().unwrap(), 0.0);
            assert_eq!(vector["z"].as_float().unwrap(), expected);
        }
    }
}

#[cfg(test)]
mod debug_draw_tests {
    use super::{
        DebugPrimitive, GamePostSimContext, InvocationDebugDraw, InvocationLog, InvocationRaycast,
        InvocationScriptUi, InvocationVehicleWrite, RaycastTicketStore, register_v1,
    };
    use crate::api::ecs::GameSnapshot;
    use crate::api::game::abi::types::Matrix4x4;
    use crate::api::scripts::*;
    use rhai::{CallFnOptions, Dynamic, Engine, Map};

    #[test]
    fn debug_line_is_available_only_during_its_bounded_invocation() {
        let draw = InvocationDebugDraw::new(true, 1, 256);
        draw.line(
            vector_map([0.0, 1.0, 2.0]),
            vector_map([3.0, 4.0, 5.0]),
            color_map([0.0, 1.0, 1.0, 1.0]),
        )
        .expect("valid line should queue");
        assert_eq!(draw.finish().len(), 1);
        assert!(
            draw.line(
                vector_map([0.0, 1.0, 2.0]),
                vector_map([3.0, 4.0, 5.0]),
                color_map([0.0, 1.0, 1.0, 1.0]),
            )
            .is_err()
        );
    }

    #[test]
    fn debug_line_rejects_non_finite_or_out_of_range_coordinates() {
        let draw = InvocationDebugDraw::new(true, 2, 256);
        assert!(
            draw.line(
                vector_map([f64::NAN, 0.0, 0.0]),
                vector_map([1.0, 0.0, 0.0]),
                color_map([1.0, 0.0, 0.0, 1.0]),
            )
            .is_err()
        );
        assert!(
            draw.line(
                vector_map([100_001.0, 0.0, 0.0]),
                vector_map([1.0, 0.0, 0.0]),
                color_map([1.0, 0.0, 0.0, 1.0]),
            )
            .is_err()
        );
    }

    #[test]
    fn debug_axes_preserve_player_relative_position_and_scale() {
        let draw = InvocationDebugDraw::new(true, 1, 256);
        draw.axes(vector_map([10.0, 12.0, 14.0]), 40.0)
            .expect("valid axes should queue");
        assert_eq!(
            draw.finish(),
            vec![DebugPrimitive::Axes {
                position: [10.0, 12.0, 14.0],
                length: 40.0,
            }]
        );
    }

    #[test]
    fn debug_axes_are_registered_as_a_rhai_method() {
        let mut engine = Engine::new();
        register_v1(&mut engine);
        let mut scope = rhai::Scope::new();
        let (sink, _receiver) = crate::api::scripts::console::event_channel();
        let script_id = ScriptId::parse("debug_gizmo").expect("test script ID is valid");
        let log = InvocationLog::new(
            crate::api::scripts::ScriptEventSink(sink),
            crate::api::scripts::ScriptEvent {
                request_id: None,
                id: script_id,
                generation: ScriptGeneration(1),
                phase: ScriptPhaseId::GamePostSim,
                callback: "on_post_sim".into(),
                kind: ScriptEventKind::Log,
                status: None,
                auto_reload: false,
                reload_error: None,
                line: None,
                column: None,
                text: String::new(),
            },
        );
        let capabilities = vec!["debug.draw".to_owned(), "game.read".to_owned()];
        let mut snapshot = GameSnapshot::default();
        snapshot.player_position = Some([10.0, 12.0, 14.0]);
        scope.push(
            "ctx",
            GamePostSimContext::new(
                snapshot,
                log.clone(),
                &capabilities,
                InvocationVehicleWrite::new(false, false, false),
                InvocationDebugDraw::new(true, 32, 256),
            ),
        );

        engine
            .eval_with_scope::<()>(
                &mut scope,
                "ctx.gizmos.axes(ctx.player.position, 40.0); ctx.gizmos.line(#{x: 0.0, y: 1.0, z: 2.0}, #{x: 3.0, y: 4.0, z: 5.0}, #{r: 1.0, g: 0.0, b: 0.0, a: 1.0}); ctx.gizmos.arrow(#{x: 1.0, y: 2.0, z: 3.0}, #{x: 4.0, y: 5.0, z: 6.0}, #{r: 0.0, g: 1.0, b: 0.0, a: 1.0})",
            )
            .expect("axes should resolve through the registered Rhai API");
        let ctx = scope
            .get_value::<GamePostSimContext>("ctx")
            .expect("context should remain in the scope");
        assert_eq!(
            ctx.debug.finish(),
            vec![
                DebugPrimitive::Axes {
                    position: [10.0, 12.0, 14.0],
                    length: 40.0,
                },
                DebugPrimitive::Line {
                    start: [0.0, 1.0, 2.0],
                    end: [3.0, 4.0, 5.0],
                    color: [1.0, 0.0, 0.0, 1.0],
                },
                DebugPrimitive::Arrow {
                    start: [1.0, 2.0, 3.0],
                    end: [4.0, 5.0, 6.0],
                    color: [0.0, 1.0, 0.0, 1.0],
                },
            ]
        );
    }

    #[test]
    fn camera_snapshot_is_available_as_copied_script_data() {
        let mut engine = Engine::new();
        register_v1(&mut engine);
        let mut scope = rhai::Scope::new();
        let (sink, _receiver) = crate::api::scripts::console::event_channel();
        let log = InvocationLog::new(
            crate::api::scripts::ScriptEventSink(sink),
            crate::api::scripts::ScriptEvent {
                request_id: None,
                id: ScriptId::parse("reticle_ray").expect("test script ID is valid"),
                generation: ScriptGeneration(1),
                phase: ScriptPhaseId::GamePostSim,
                callback: "on_post_sim".into(),
                kind: ScriptEventKind::Log,
                status: None,
                auto_reload: false,
                reload_error: None,
                line: None,
                column: None,
                text: String::new(),
            },
        );
        let snapshot = GameSnapshot {
            camera_position: Some([1.0, 2.0, 3.0]),
            camera_direction: Some([0.0, 0.0, -1.0]),
            ..GameSnapshot::default()
        };
        scope.push(
            "ctx",
            GamePostSimContext::new(
                snapshot,
                log,
                &["game.read".to_owned()],
                InvocationVehicleWrite::new(false, false, false),
                InvocationDebugDraw::new(false, 32, 256),
            ),
        );
        assert_eq!(
            engine
                .eval_with_scope::<f64>(&mut scope, "ctx.camera.position.x")
                .expect("camera position should be readable"),
            1.0
        );
        assert_eq!(
            engine
                .eval_with_scope::<f64>(&mut scope, "ctx.camera.direction.z")
                .expect("camera direction should be readable"),
            -1.0
        );
    }

    #[test]
    fn reticle_ray_script_uses_the_general_raycast_request_api() {
        let script = include_str!("../../../examples/scripts/reticle_ray/main.rhai");
        let engine = crate::api::scripts::engine::RhaiEngineFactory::build();
        let ast = engine
            .compile(script)
            .expect("reticle script should compile");
        let (sink, _receiver) = crate::api::scripts::console::event_channel();
        let event = crate::api::scripts::ScriptEvent {
            request_id: None,
            id: ScriptId::parse("reticle_ray").expect("test script ID is valid"),
            generation: ScriptGeneration(1),
            phase: ScriptPhaseId::GamePostSim,
            callback: "on_post_sim".into(),
            kind: ScriptEventKind::Log,
            status: None,
            auto_reload: false,
            reload_error: None,
            line: None,
            column: None,
            text: String::new(),
        };
        let event_sink = crate::api::scripts::ScriptEventSink(sink);
        let log = InvocationLog::new(event_sink.clone(), event.clone());
        let tickets = RaycastTicketStore::default();
        let raycast = InvocationRaycast::new(
            true,
            true,
            event.clone(),
            1,
            tickets.clone(),
            Some([0.0, 1.0, 2.0]),
            Some([0.0, 0.0, -1.0]),
        );
        let snapshot = GameSnapshot {
            callback_sequence: 1,
            sample_sequence: 1,
            camera_position: Some([0.0, 1.0, 2.0]),
            camera_direction: Some([0.0, 0.0, -1.0]),
            ..GameSnapshot::default()
        };
        let context = GamePostSimContext::new_with_ui(
            snapshot,
            log,
            &[
                "debug.draw".to_owned(),
                "game.read".to_owned(),
                "game.raycast".to_owned(),
                "ui.panels".to_owned(),
            ],
            InvocationVehicleWrite::new(false, false, false),
            raycast.clone(),
            InvocationDebugDraw::new(true, 32, 256),
            InvocationScriptUi::new(true, Vec::new()),
        );
        let mut scope = rhai::Scope::new();
        let mut state = Map::new();
        state.insert("ticket".into(), Dynamic::UNIT);
        state.insert("callback_sequence".into(), Dynamic::from(0_i64));
        scope.push("state", state);
        engine
            .call_fn_with_options::<()>(
                CallFnOptions::new().eval_ast(false),
                &mut scope,
                &ast,
                "on_post_sim",
                (context,),
            )
            .expect("reticle callback should request the camera ray");
        let request = raycast
            .finish()
            .expect("camera ray request should be queued");
        assert_eq!(request.origin, [0.0, 1.0, 2.0]);
        assert_eq!(request.direction, [0.0, 0.0, -1.0]);

        let next_raycast = InvocationRaycast::new(
            true,
            true,
            event.clone(),
            2,
            tickets,
            Some([0.0, 1.0, 2.0]),
            Some([0.0, 0.0, -1.0]),
        );
        let next_context = GamePostSimContext::new_with_ui(
            GameSnapshot {
                callback_sequence: 2,
                sample_sequence: 1,
                camera_position: Some([0.0, 1.0, 2.0]),
                camera_direction: Some([0.0, 0.0, -1.0]),
                ..GameSnapshot::default()
            },
            InvocationLog::new(event_sink, event),
            &[
                "debug.draw".to_owned(),
                "game.read".to_owned(),
                "game.raycast".to_owned(),
                "ui.panels".to_owned(),
            ],
            InvocationVehicleWrite::new(false, false, false),
            next_raycast.clone(),
            InvocationDebugDraw::new(true, 32, 256),
            InvocationScriptUi::new(true, Vec::new()),
        );
        engine
            .call_fn_with_options::<()>(
                CallFnOptions::new().eval_ast(false),
                &mut scope,
                &ast,
                "on_post_sim",
                (next_context,),
            )
            .expect("reticle callback should refresh when callback sequence advances");
        assert!(
            next_raycast.finish().is_some(),
            "a fresh callback should request a fresh camera ray even if telemetry sample sequence did not change"
        );
    }

    #[test]
    fn copied_player_transform_is_available_as_row_major_values() {
        let mut engine = Engine::new();
        register_v1(&mut engine);
        let mut scope = rhai::Scope::new();
        let (sink, _receiver) = crate::api::scripts::console::event_channel();
        let script_id = ScriptId::parse("debug_orientation").expect("valid script ID");
        let log = InvocationLog::new(
            crate::api::scripts::ScriptEventSink(sink),
            crate::api::scripts::ScriptEvent {
                request_id: None,
                id: script_id,
                generation: ScriptGeneration(1),
                phase: ScriptPhaseId::GamePostSim,
                callback: "on_post_sim".into(),
                kind: ScriptEventKind::Log,
                status: None,
                auto_reload: false,
                reload_error: None,
                line: None,
                column: None,
                text: String::new(),
            },
        );
        let capabilities = vec!["debug.draw".to_owned(), "game.read".to_owned()];
        let mut snapshot = GameSnapshot::default();
        snapshot.player_transform = Some(Matrix4x4::from_rows([
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, -1.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            [12.0, 34.0, 56.0, 1.0],
        ]));
        snapshot.vehicle_transform = Some(Matrix4x4::from_rows([
            [0.0, -1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [78.0, 90.0, 123.0, 1.0],
        ]));
        scope.push(
            "ctx",
            GamePostSimContext::new(
                snapshot,
                log,
                &capabilities,
                InvocationVehicleWrite::new(false, false, false),
                InvocationDebugDraw::new(true, 32, 256),
            ),
        );

        let forward_y = engine
            .eval_with_scope::<f64>(&mut scope, "ctx.player.transform[5]")
            .expect("player transform should be exposed to Rhai");
        let position_x = engine
            .eval_with_scope::<f64>(&mut scope, "ctx.player.transform[12]")
            .expect("translation should retain Mad Max matrix ordering");
        let vehicle_position_z = engine
            .eval_with_scope::<f64>(&mut scope, "ctx.vehicle.transform[14]")
            .expect("vehicle transform should be exposed with the same matrix ordering");
        assert_eq!(forward_y, 0.0);
        assert_eq!(position_x, 12.0);
        assert_eq!(vehicle_position_z, 123.0);
    }

    #[test]
    fn transformed_axes_are_registered_as_a_rhai_method() {
        let mut engine = Engine::new();
        register_v1(&mut engine);
        let mut scope = rhai::Scope::new();
        let (sink, _receiver) = crate::api::scripts::console::event_channel();
        let script_id = ScriptId::parse("debug_orientation").expect("valid script ID");
        let log = InvocationLog::new(
            crate::api::scripts::ScriptEventSink(sink),
            crate::api::scripts::ScriptEvent {
                request_id: None,
                id: script_id,
                generation: ScriptGeneration(1),
                phase: ScriptPhaseId::GamePostSim,
                callback: "on_post_sim".into(),
                kind: ScriptEventKind::Log,
                status: None,
                auto_reload: false,
                reload_error: None,
                line: None,
                column: None,
                text: String::new(),
            },
        );
        let capabilities = vec!["debug.draw".to_owned()];
        scope.push(
            "ctx",
            GamePostSimContext::new(
                GameSnapshot::default(),
                log,
                &capabilities,
                InvocationVehicleWrite::new(false, false, false),
                InvocationDebugDraw::new(true, 32, 256),
            ),
        );

        engine
            .eval_with_scope::<()>(
                &mut scope,
                "ctx.gizmos.transformed_axes([1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 12.0, 34.0, 56.0, 1.0], 8.0)",
            )
            .expect("transformed axes should resolve through the registered Rhai API");
        let ctx = scope
            .get_value::<GamePostSimContext>("ctx")
            .expect("context should remain in the scope");
        assert_eq!(ctx.debug.finish().len(), 1);
    }

    fn vector_map(values: [f64; 3]) -> Map {
        ["x", "y", "z"]
            .into_iter()
            .zip(values)
            .map(|(name, value)| (name.into(), Dynamic::from(value)))
            .collect()
    }

    fn color_map(values: [f64; 4]) -> Map {
        ["r", "g", "b", "a"]
            .into_iter()
            .zip(values)
            .map(|(name, value)| (name.into(), Dynamic::from(value)))
            .collect()
    }
}
