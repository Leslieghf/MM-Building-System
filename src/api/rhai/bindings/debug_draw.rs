//! Bounded debug geometry accepted from one Rhai callback invocation.
use super::{numeric_value, require_capability};
use rhai::{Array, Map};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

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

/// Invocation-local, owned debug requests. `finish` closes the capability and
/// transfers the validated primitives to the host for deferred rendering.
#[derive(Clone)]
pub(crate) struct InvocationDebugDraw {
    active: Arc<AtomicBool>,
    queued: Arc<Mutex<Vec<DebugPrimitive>>>,
    pub(super) allowed: bool,
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

    pub(super) fn axes(
        &self,
        position: Map,
        length: rhai::FLOAT,
    ) -> Result<(), Box<rhai::EvalAltResult>> {
        let position = parse_debug_vector(position)?;
        if !length.is_finite() || !(0.1..=1_000.0).contains(&length) {
            return Err("debug axes length must be finite and within [0.1,1000]".into());
        }
        self.push(DebugPrimitive::Axes {
            position,
            length: length as f32,
        })
    }

    pub(super) fn transformed_axes(
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

    pub(super) fn line(
        &self,
        start: Map,
        end: Map,
        color: Map,
    ) -> Result<(), Box<rhai::EvalAltResult>> {
        self.push(DebugPrimitive::Line {
            start: parse_debug_vector(start)?,
            end: parse_debug_vector(end)?,
            color: parse_debug_color(color)?,
        })
    }

    pub(super) fn arrow(
        &self,
        start: Map,
        end: Map,
        color: Map,
    ) -> Result<(), Box<rhai::EvalAltResult>> {
        self.push(DebugPrimitive::Arrow {
            start: parse_debug_vector(start)?,
            end: parse_debug_vector(end)?,
            color: parse_debug_color(color)?,
        })
    }

    pub(super) fn sphere(
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

    pub(super) fn sparse_sphere(
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

    pub(super) fn tessellated_sphere(
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

    pub(super) fn aabb(
        &self,
        minimum: Map,
        maximum: Map,
        color: Map,
    ) -> Result<(), Box<rhai::EvalAltResult>> {
        self.push(DebugPrimitive::Aabb {
            minimum: parse_debug_vector(minimum)?,
            maximum: parse_debug_vector(maximum)?,
            color: parse_debug_color(color)?,
        })
    }

    pub(super) fn transformed_aabb(
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

    pub(super) fn queued_transformed_aabb(
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

    pub(super) fn bounds_shape(
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
