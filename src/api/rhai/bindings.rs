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
    player: CopiedObject,
    vehicle: CopiedObject,
    input: CopiedInput,
    raycast: InvocationRaycast,
    debug: InvocationDebugDraw,
    log: InvocationLog,
    can_read: bool,
    can_log: bool,
}

#[derive(Clone)]
struct CopiedInput {
    frame: InputFrame,
    allowed: bool,
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
        let Ok(entries) = self.0.lock() else {
            return raycast_result_map("unavailable", false, None, None, false);
        };
        let Some(entry) = entries.get(&ticket) else {
            return raycast_result_map("unknown", false, None, None, false);
        };
        if entry.owner != source.id || entry.generation != source.generation {
            return raycast_result_map("unknown", false, None, None, false);
        }
        if callback_sequence > entry.expires_at {
            return raycast_result_map("expired", false, None, None, false);
        }
        match entry.available {
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
        }
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
        )
    }

    pub(crate) fn new(
        allowed: bool,
        capacity: bool,
        source: ScriptEvent,
        callback_sequence: u64,
        tickets: RaycastTicketStore,
    ) -> Self {
        Self {
            active: Arc::new(AtomicBool::new(true)),
            request: Arc::new(Mutex::new(None)),
            allowed,
            capacity,
            source,
            callback_sequence,
            tickets,
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

    pub(crate) fn new_with_raycast(
        snapshot: GameSnapshot,
        log: InvocationLog,
        capabilities: &[String],
        write: InvocationVehicleWrite,
        raycast: InvocationRaycast,
        debug: InvocationDebugDraw,
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
            can_read: capabilities.iter().any(|id| id == "game.read"),
            can_log: capabilities.iter().any(|id| id == "log"),
            raycast,
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
        .register_fn("result", InvocationRaycast::result);
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
        InvocationVehicleWrite, RaycastTicketStore, register_v1,
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
        let mut engine = Engine::new();
        register_v1(&mut engine);
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
        let raycast = InvocationRaycast::new(true, true, event.clone(), 1, tickets.clone());
        let snapshot = GameSnapshot {
            callback_sequence: 1,
            sample_sequence: 1,
            camera_position: Some([0.0, 1.0, 2.0]),
            camera_direction: Some([0.0, 0.0, -1.0]),
            ..GameSnapshot::default()
        };
        let context = GamePostSimContext::new_with_raycast(
            snapshot,
            log,
            &[
                "debug.draw".to_owned(),
                "game.read".to_owned(),
                "game.raycast".to_owned(),
            ],
            InvocationVehicleWrite::new(false, false, false),
            raycast.clone(),
            InvocationDebugDraw::new(true, 32, 256),
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

        let next_raycast = InvocationRaycast::new(true, true, event.clone(), 2, tickets);
        let next_context = GamePostSimContext::new_with_raycast(
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
            ],
            InvocationVehicleWrite::new(false, false, false),
            next_raycast.clone(),
            InvocationDebugDraw::new(true, 32, 256),
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
