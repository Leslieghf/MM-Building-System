use super::api_catalog::{API_CALLBACKS, API_CAPABILITIES};
use super::vector::ScriptMath;
use super::*;
use crate::{
    runtime::GameSnapshot,
    sdk::core::input::{self, InputFrame},
    sdk::types::Matrix4x4,
};
use rhai::{Array, Dynamic, Engine, Map};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

pub struct ScriptApiRegistry {
    pub descriptors: Vec<ScriptApiDescriptor>,
    pub capabilities: &'static [&'static str],
}
impl ScriptApiRegistry {
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
pub struct GamePostSimContext {
    frame: Map,
    math: ScriptMath,
    player: CopiedObject,
    vehicle: CopiedObject,
    input: CopiedInput,
    debug: InvocationDebugDraw,
    log: InvocationLog,
    can_read: bool,
    can_log: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum DebugPrimitive {
    Axes {
        position: [f32; 3],
        length: f32,
    },
    TransformedAxes {
        transform: [f32; 16],
        length: f32,
    },
    Line {
        start: [f32; 3],
        end: [f32; 3],
        color: [f32; 4],
    },
    Arrow {
        start: [f32; 3],
        end: [f32; 3],
        color: [f32; 4],
    },
    Sphere {
        center: [f32; 3],
        radius: f32,
        color: [f32; 4],
    },
    SparseSphere {
        center: [f32; 3],
        radius: f32,
        color: [f32; 4],
    },
    TessellatedSphere {
        center: [f32; 3],
        radius: f32,
        color: [f32; 4],
    },
    Aabb {
        minimum: [f32; 3],
        maximum: [f32; 3],
        color: [f32; 4],
    },
    TransformedAabb {
        transform: [f32; 16],
        minimum: [f32; 3],
        maximum: [f32; 3],
        color: [f32; 4],
    },
    QueuedTransformedAabb {
        transform: [f32; 16],
        minimum: [f32; 3],
        maximum: [f32; 3],
        color: [f32; 4],
    },
    BoundsSphere {
        transform: [f32; 16],
        minimum: [f32; 3],
        maximum: [f32; 3],
        color: [f32; 4],
    },
    BoundsCylinder {
        transform: [f32; 16],
        minimum: [f32; 3],
        maximum: [f32; 3],
        color: [f32; 4],
    },
    BoundsCapsule {
        transform: [f32; 16],
        minimum: [f32; 3],
        maximum: [f32; 3],
        color: [f32; 4],
    },
    BoundsConvexVertices {
        transform: [f32; 16],
        minimum: [f32; 3],
        maximum: [f32; 3],
        color: [f32; 4],
    },
}

impl DebugPrimitive {
    pub(crate) fn work_cost(self) -> usize {
        match self {
            Self::Axes { .. } | Self::TransformedAxes { .. } => 3,
            Self::Line { .. } => 1,
            Self::Arrow { .. } => 8,
            Self::Sphere { .. } | Self::SparseSphere { .. } => 9,
            Self::TessellatedSphere { .. } => 60,
            Self::Aabb { .. }
            | Self::TransformedAabb { .. }
            | Self::QueuedTransformedAabb { .. } => 12,
            Self::BoundsSphere { .. }
            | Self::BoundsCylinder { .. }
            | Self::BoundsCapsule { .. }
            | Self::BoundsConvexVertices { .. } => 1,
        }
    }
}

#[derive(Clone)]
pub(crate) struct InvocationDebugDraw {
    active: Arc<AtomicBool>,
    queued: Arc<Mutex<Vec<DebugPrimitive>>>,
    allowed: bool,
    capacity: usize,
    work_capacity: usize,
    work_used: Arc<AtomicUsize>,
}

impl InvocationDebugDraw {
    pub(crate) fn new(allowed: bool, capacity: usize, work_capacity: usize) -> Self {
        Self {
            active: Arc::new(AtomicBool::new(true)),
            queued: Arc::new(Mutex::new(Vec::new())),
            allowed,
            capacity,
            work_capacity,
            work_used: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub(crate) fn finish(&self) -> Vec<DebugPrimitive> {
        self.active.store(false, Ordering::Relaxed);
        self.queued
            .lock()
            .map(|mut queued| std::mem::take(&mut *queued))
            .unwrap_or_default()
    }

    fn axes(&self, position: Map, length: rhai::FLOAT) -> Result<(), Box<rhai::EvalAltResult>> {
        let position = parse_debug_vector(position)?;
        if !length.is_finite() || !(0.1..=1_000.0).contains(&length) {
            return Err("debug axes length must be finite and within [0.1,1000]".into());
        }
        self.push(DebugPrimitive::Axes {
            position,
            length: length as f32,
        })
    }

    fn transformed_axes(
        &self,
        transform: Array,
        length: rhai::FLOAT,
    ) -> Result<(), Box<rhai::EvalAltResult>> {
        if !length.is_finite() || !(0.1..=1_000.0).contains(&length) {
            return Err("debug axes length must be finite and within [0.1,1000]".into());
        }
        self.push(DebugPrimitive::TransformedAxes {
            transform: parse_debug_transform(transform)?,
            length: length as f32,
        })
    }

    fn line(&self, start: Map, end: Map, color: Map) -> Result<(), Box<rhai::EvalAltResult>> {
        self.push(DebugPrimitive::Line {
            start: parse_debug_vector(start)?,
            end: parse_debug_vector(end)?,
            color: parse_debug_color(color)?,
        })
    }

    fn arrow(&self, start: Map, end: Map, color: Map) -> Result<(), Box<rhai::EvalAltResult>> {
        self.push(DebugPrimitive::Arrow {
            start: parse_debug_vector(start)?,
            end: parse_debug_vector(end)?,
            color: parse_debug_color(color)?,
        })
    }

    fn sphere(
        &self,
        center: Map,
        radius: rhai::FLOAT,
        color: Map,
    ) -> Result<(), Box<rhai::EvalAltResult>> {
        self.push(DebugPrimitive::Sphere {
            center: parse_debug_vector(center)?,
            radius: parse_debug_radius(radius)?,
            color: parse_debug_color(color)?,
        })
    }

    fn sparse_sphere(
        &self,
        center: Map,
        radius: rhai::FLOAT,
        color: Map,
    ) -> Result<(), Box<rhai::EvalAltResult>> {
        self.push(DebugPrimitive::SparseSphere {
            center: parse_debug_vector(center)?,
            radius: parse_debug_radius(radius)?,
            color: parse_debug_color(color)?,
        })
    }

    fn tessellated_sphere(
        &self,
        center: Map,
        radius: rhai::FLOAT,
        color: Map,
    ) -> Result<(), Box<rhai::EvalAltResult>> {
        self.push(DebugPrimitive::TessellatedSphere {
            center: parse_debug_vector(center)?,
            radius: parse_debug_radius(radius)?,
            color: parse_debug_color(color)?,
        })
    }

    fn aabb(&self, minimum: Map, maximum: Map, color: Map) -> Result<(), Box<rhai::EvalAltResult>> {
        self.push(DebugPrimitive::Aabb {
            minimum: parse_debug_vector(minimum)?,
            maximum: parse_debug_vector(maximum)?,
            color: parse_debug_color(color)?,
        })
    }

    fn transformed_aabb(
        &self,
        transform: Array,
        minimum: Map,
        maximum: Map,
        color: Map,
    ) -> Result<(), Box<rhai::EvalAltResult>> {
        self.push(DebugPrimitive::TransformedAabb {
            transform: parse_debug_transform(transform)?,
            minimum: parse_debug_vector(minimum)?,
            maximum: parse_debug_vector(maximum)?,
            color: parse_debug_color(color)?,
        })
    }

    fn queued_transformed_aabb(
        &self,
        transform: Array,
        minimum: Map,
        maximum: Map,
        color: Map,
    ) -> Result<(), Box<rhai::EvalAltResult>> {
        self.push(DebugPrimitive::QueuedTransformedAabb {
            transform: parse_debug_transform(transform)?,
            minimum: parse_debug_vector(minimum)?,
            maximum: parse_debug_vector(maximum)?,
            color: parse_debug_color(color)?,
        })
    }

    fn bounds_shape(
        &self,
        kind: u8,
        transform: Array,
        minimum: Map,
        maximum: Map,
        color: Map,
    ) -> Result<(), Box<rhai::EvalAltResult>> {
        let transform = parse_debug_transform(transform)?;
        let minimum = parse_debug_vector(minimum)?;
        let maximum = parse_debug_vector(maximum)?;
        let color = parse_debug_color(color)?;
        self.push(match kind {
            0 => DebugPrimitive::BoundsSphere {
                transform,
                minimum,
                maximum,
                color,
            },
            1 => DebugPrimitive::BoundsCylinder {
                transform,
                minimum,
                maximum,
                color,
            },
            4 => DebugPrimitive::BoundsCapsule {
                transform,
                minimum,
                maximum,
                color,
            },
            _ => DebugPrimitive::BoundsConvexVertices {
                transform,
                minimum,
                maximum,
                color,
            },
        })
    }

    fn push(&self, primitive: DebugPrimitive) -> Result<(), Box<rhai::EvalAltResult>> {
        require_capability(self.allowed, "debug.draw")?;
        if !self.active.load(Ordering::Relaxed) {
            return Err("debug draw context has expired".into());
        }
        let mut queued = self
            .queued
            .lock()
            .map_err(|_| Box::<rhai::EvalAltResult>::from("debug draw queue unavailable"))?;
        if queued.len() >= self.capacity {
            return Err("debug draw limit reached for this callback".into());
        }
        let cost = primitive.work_cost();
        let current = self.work_used.load(Ordering::Relaxed);
        if current.saturating_add(cost) > self.work_capacity {
            return Err("debug draw geometry budget reached for this callback".into());
        }
        self.work_used.store(current + cost, Ordering::Relaxed);
        queued.push(primitive);
        Ok(())
    }
}

fn parse_debug_radius(radius: rhai::FLOAT) -> Result<f32, Box<rhai::EvalAltResult>> {
    if !radius.is_finite() || !(0.01..=10_000.0).contains(&radius) {
        return Err("debug sphere radius must be finite and within [0.01,10000]".into());
    }
    Ok(radius as f32)
}

fn parse_debug_transform(values: Array) -> Result<[f32; 16], Box<rhai::EvalAltResult>> {
    if values.len() != 16 {
        return Err("debug transform requires exactly 16 numbers in Mad Max matrix order".into());
    }
    let mut transform = [0.0_f32; 16];
    for (index, value) in values.into_iter().enumerate() {
        let value = numeric_value(value, "debug transform values must be numbers")?;
        if !value.is_finite() || value.abs() > 100_000.0 {
            return Err("debug transform values must be finite and within [-100000,100000]".into());
        }
        transform[index] = value as f32;
    }
    Ok(transform)
}

fn parse_debug_vector(map: Map) -> Result<[f32; 3], Box<rhai::EvalAltResult>> {
    if map.len() != 3 {
        return Err("debug vector requires exactly x, y, z components".into());
    }
    let mut vector = [0.0; 3];
    for (index, name) in ["x", "y", "z"].into_iter().enumerate() {
        let value = map.get(name).cloned().ok_or_else(|| {
            Box::<rhai::EvalAltResult>::from(format!("debug vector component {name} is missing"))
        })?;
        let value = numeric_value(value, "debug vector components must be numbers")?;
        if !value.is_finite() || value.abs() > 100_000.0 {
            return Err(
                "debug vector components must be finite and within [-100000,100000]".into(),
            );
        }
        vector[index] = value as f32;
    }
    Ok(vector)
}

fn parse_debug_color(map: Map) -> Result<[f32; 4], Box<rhai::EvalAltResult>> {
    if map.len() != 4 {
        return Err("debug color requires exactly r, g, b, a components".into());
    }
    let mut color = [0.0; 4];
    for (index, name) in ["r", "g", "b", "a"].into_iter().enumerate() {
        let value = map.get(name).cloned().ok_or_else(|| {
            Box::<rhai::EvalAltResult>::from(format!("debug color component {name} is missing"))
        })?;
        let value = numeric_value(value, "debug color components must be numbers")?;
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err("debug color components must be finite and within [0,1]".into());
        }
        color[index] = value as f32;
    }
    Ok(color)
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
        Ok(crate::display::fixed(value))
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
        Ok(crate::display::vector_components(components))
    }
}

fn numeric_value(value: Dynamic, error: &'static str) -> Result<f64, Box<rhai::EvalAltResult>> {
    value
        .clone()
        .try_cast::<rhai::FLOAT>()
        .or_else(|| value.try_cast::<rhai::INT>().map(|value| value as f64))
        .ok_or_else(|| error.into())
}
impl CopiedInput {
    fn query(&self, key: &str, state: u8) -> Result<bool, Box<rhai::EvalAltResult>> {
        require_capability(self.allowed, "input.read")?;
        let key = input::parse_key(key).ok_or_else(|| {
            Box::<rhai::EvalAltResult>::from("unknown input key; use show_keys for supported names")
        })?;
        let key = key as u32;
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
    pub(crate) fn new(
        snapshot: GameSnapshot,
        log: InvocationLog,
        capabilities: &[String],
        write: InvocationVehicleWrite,
        debug: InvocationDebugDraw,
    ) -> Self {
        let mut frame = Map::new();
        frame.insert("phase".into(), Dynamic::from("game.post_sim"));
        frame.insert(
            "sequence".into(),
            Dynamic::from(snapshot.callback_sequence.min(i64::MAX as u64) as i64),
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
            can_read: capabilities.iter().any(|id| id == "game.read"),
            can_log: capabilities.iter().any(|id| id == "log"),
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
    engine
        .register_type_with_name::<GamePostSimContext>("MadMaxPostSimContext")
        .register_type_with_name::<CopiedObject>("MadMaxCopiedObject")
        .register_type_with_name::<CopiedInput>("MadMaxInputFrame")
        .register_type_with_name::<ScriptFormatter>("MadMaxFormatter")
        .register_type_with_name::<InvocationLog>("MadMaxLog")
        .register_type_with_name::<InvocationDebugDraw>("MadMaxDebugDraw")
        .register_get("frame", |ctx: &mut GamePostSimContext| ctx.frame.clone())
        .register_get("math", |ctx: &mut GamePostSimContext| ctx.math)
        .register_get("player", |ctx: &mut GamePostSimContext| {
            require_capability(ctx.can_read, "game.read")?;
            Ok::<_, Box<rhai::EvalAltResult>>(ctx.player.clone())
        })
        .register_get("vehicle", |ctx: &mut GamePostSimContext| {
            require_capability(ctx.can_read, "game.read")?;
            Ok::<_, Box<rhai::EvalAltResult>>(ctx.vehicle.clone())
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
        .register_fn("info", InvocationLog::info)
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
            "sphere",
            |d: &mut InvocationDebugDraw, c: Map, r: rhai::FLOAT, color: Map| d.sphere(c, r, color),
        )
        .register_fn(
            "sparse_sphere",
            |d: &mut InvocationDebugDraw, c: Map, r: rhai::FLOAT, color: Map| {
                d.sparse_sphere(c, r, color)
            },
        )
        .register_fn(
            "tessellated_sphere",
            |d: &mut InvocationDebugDraw, c: Map, r: rhai::FLOAT, color: Map| {
                d.tessellated_sphere(c, r, color)
            },
        )
        .register_fn(
            "aabb",
            |d: &mut InvocationDebugDraw, min: Map, max: Map, color: Map| d.aabb(min, max, color),
        )
        .register_fn(
            "transformed_aabb",
            |d: &mut InvocationDebugDraw, transform: Array, min: Map, max: Map, color: Map| {
                d.transformed_aabb(transform, min, max, color)
            },
        )
        .register_fn(
            "queued_transformed_aabb",
            |d: &mut InvocationDebugDraw, transform: Array, min: Map, max: Map, color: Map| {
                d.queued_transformed_aabb(transform, min, max, color)
            },
        )
        .register_fn(
            "bounds_sphere",
            |d: &mut InvocationDebugDraw, transform: Array, min: Map, max: Map, color: Map| {
                d.bounds_shape(0, transform, min, max, color)
            },
        )
        .register_fn(
            "bounds_cylinder",
            |d: &mut InvocationDebugDraw, transform: Array, min: Map, max: Map, color: Map| {
                d.bounds_shape(1, transform, min, max, color)
            },
        )
        .register_fn(
            "bounds_capsule",
            |d: &mut InvocationDebugDraw, transform: Array, min: Map, max: Map, color: Map| {
                d.bounds_shape(4, transform, min, max, color)
            },
        )
        .register_fn(
            "bounds_convex_vertices",
            |d: &mut InvocationDebugDraw, transform: Array, min: Map, max: Map, color: Map| {
                d.bounds_shape(5, transform, min, max, color)
            },
        )
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
    for name in [
        "available",
        "position",
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
fn require_capability(allowed: bool, id: &str) -> Result<(), Box<rhai::EvalAltResult>> {
    if allowed {
        Ok(())
    } else {
        Err(format!("capability {id} was not declared").into())
    }
}

#[cfg(test)]
mod math_integration_tests {
    use super::*;
    use crate::scripting::engine::RhaiEngineFactory;

    #[test]
    fn ctx_math_is_pure_and_preserves_negative_z_forward() {
        let engine = RhaiEngineFactory::build();
        let (sink, _receiver) = crate::scripting::console::event_channel();
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
                log,
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
    use super::*;
    use crate::sdk::types::Matrix4x4;

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
        let (sink, _receiver) = crate::scripting::console::event_channel();
        let script_id = ScriptId::parse("debug_gizmo").expect("test script ID is valid");
        let log = InvocationLog::new(
            crate::scripting::ScriptEventSink(sink),
            crate::scripting::ScriptEvent {
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
                log,
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
    fn copied_player_transform_is_available_as_row_major_values() {
        let mut engine = Engine::new();
        register_v1(&mut engine);
        let mut scope = rhai::Scope::new();
        let (sink, _receiver) = crate::scripting::console::event_channel();
        let script_id = ScriptId::parse("debug_orientation").expect("valid script ID");
        let log = InvocationLog::new(
            crate::scripting::ScriptEventSink(sink),
            crate::scripting::ScriptEvent {
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
        let (sink, _receiver) = crate::scripting::console::event_channel();
        let script_id = ScriptId::parse("debug_orientation").expect("valid script ID");
        let log = InvocationLog::new(
            crate::scripting::ScriptEventSink(sink),
            crate::scripting::ScriptEvent {
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
