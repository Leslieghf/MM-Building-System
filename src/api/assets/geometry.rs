use super::AssetKey;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Defensive geometry limits applied by constructors and `validate`.
pub struct GeometryLimits;

impl GeometryLimits {
    pub const MAX_PRIMITIVES: usize = 256;
    pub const MAX_VERTICES_PER_PRIMITIVE: usize = 65_536;
    pub const MAX_INDICES_PER_PRIMITIVE: usize = 196_608;
    pub const MAX_VERTICES_PER_ASSET: usize = 1_000_000;
    pub const MAX_INDICES_PER_ASSET: usize = 3_000_000;
    pub const MAX_MATERIAL_SLOTS: usize = 64;
}

/// A named material reference local to one primitive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialSlot {
    pub key: String,
}

impl MaterialSlot {
    pub fn new(key: impl Into<String>) -> Result<Self, GeometryError> {
        let key = key.into();
        if key.is_empty() || key.len() > 128 {
            return Err(GeometryError::InvalidMaterialKey);
        }
        Ok(Self { key })
    }
}

/// Axis-aligned bounds in the containing geometry asset's local space.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LocalBounds {
    min: [f32; 3],
    max: [f32; 3],
}

impl LocalBounds {
    pub fn min(&self) -> [f32; 3] {
        self.min
    }

    pub fn max(&self) -> [f32; 3] {
        self.max
    }

    fn from_points(points: impl Iterator<Item = [f32; 3]>) -> Result<Self, GeometryError> {
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        let mut any = false;
        for point in points {
            any = true;
            for axis in 0..3 {
                min[axis] = min[axis].min(point[axis]);
                max[axis] = max[axis].max(point[axis]);
            }
        }
        if any {
            Ok(Self { min, max })
        } else {
            Err(GeometryError::EmptyVertices)
        }
    }

    fn union(self, other: Self) -> Self {
        let mut min = self.min;
        let mut max = self.max;
        for axis in 0..3 {
            min[axis] = min[axis].min(other.min[axis]);
            max[axis] = max[axis].max(other.max[axis]);
        }
        Self { min, max }
    }
}

/// Indexed triangle mesh in asset-local space.
///
/// Transform storage follows the game's copied 16-float transform convention:
/// basis columns use indices `0..3`, `4..7`, `8..11` and translation is at
/// `12..14`. The last row/column values are required to describe an affine
/// transform (`3, 7, 11 = 0`, `15 = 1`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GeometryPrimitive {
    vertices: Vec<[f32; 3]>,
    indices: Vec<u32>,
    local_transform: [f32; 16],
    material_slots: Vec<MaterialSlot>,
    /// One material-slot index per triangle, in index-buffer order.
    triangle_materials: Vec<u16>,
    bounds: LocalBounds,
}

#[derive(Deserialize)]
struct GeometryPrimitiveWire {
    vertices: Vec<[f32; 3]>,
    indices: Vec<u32>,
    local_transform: [f32; 16],
    material_slots: Vec<MaterialSlot>,
    triangle_materials: Vec<u16>,
    // Accept older serialized values while never trusting their cached bounds.
    #[serde(default)]
    bounds: Option<LocalBounds>,
}

impl<'de> Deserialize<'de> for GeometryPrimitive {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = GeometryPrimitiveWire::deserialize(deserializer)?;
        let _ignored_cached_bounds = wire.bounds;
        Self::new(
            wire.vertices,
            wire.indices,
            wire.local_transform,
            wire.material_slots,
            wire.triangle_materials,
        )
        .map_err(serde::de::Error::custom)
    }
}

impl GeometryPrimitive {
    pub fn new(
        vertices: Vec<[f32; 3]>,
        indices: Vec<u32>,
        local_transform: [f32; 16],
        material_slots: Vec<MaterialSlot>,
        triangle_materials: Vec<u16>,
    ) -> Result<Self, GeometryError> {
        let mut primitive = Self {
            vertices,
            indices,
            local_transform,
            material_slots,
            triangle_materials,
            bounds: LocalBounds {
                min: [0.0; 3],
                max: [0.0; 3],
            },
        };
        primitive.validate_and_compute_bounds()?;
        Ok(primitive)
    }

    pub fn vertices(&self) -> &[[f32; 3]] {
        &self.vertices
    }
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }
    pub fn local_transform(&self) -> &[f32; 16] {
        &self.local_transform
    }
    pub fn material_slots(&self) -> &[MaterialSlot] {
        &self.material_slots
    }
    pub fn triangle_materials(&self) -> &[u16] {
        &self.triangle_materials
    }
    pub fn bounds(&self) -> LocalBounds {
        self.bounds
    }

    /// Revalidate the primitive and recompute its cached transformed bounds.
    pub fn validate(&mut self) -> Result<(), GeometryError> {
        self.validate_and_compute_bounds()
    }

    fn validate_and_compute_bounds(&mut self) -> Result<(), GeometryError> {
        if self.vertices.is_empty() {
            return Err(GeometryError::EmptyVertices);
        }
        if self.vertices.len() > GeometryLimits::MAX_VERTICES_PER_PRIMITIVE {
            return Err(GeometryError::PrimitiveVertexLimit);
        }
        if self.indices.is_empty() || self.indices.len() % 3 != 0 {
            return Err(GeometryError::IndicesNotTriangles);
        }
        if self.indices.len() > GeometryLimits::MAX_INDICES_PER_PRIMITIVE {
            return Err(GeometryError::PrimitiveIndexLimit);
        }
        for vertex in &self.vertices {
            if vertex.iter().any(|value| !value.is_finite()) {
                return Err(GeometryError::NonFiniteCoordinate);
            }
        }
        for &index in &self.indices {
            if index as usize >= self.vertices.len() {
                return Err(GeometryError::IndexOutOfRange);
            }
        }
        validate_transform(&self.local_transform)?;
        if self.material_slots.is_empty()
            || self.material_slots.len() > GeometryLimits::MAX_MATERIAL_SLOTS
        {
            return Err(GeometryError::MaterialSlotLimit);
        }
        for slot in &self.material_slots {
            if slot.key.is_empty() || slot.key.len() > 128 {
                return Err(GeometryError::InvalidMaterialKey);
            }
        }
        if self.triangle_materials.len() != self.indices.len() / 3 {
            return Err(GeometryError::TriangleMaterialCountMismatch);
        }
        if self
            .triangle_materials
            .iter()
            .any(|&slot| slot as usize >= self.material_slots.len())
        {
            return Err(GeometryError::MaterialReferenceOutOfRange);
        }
        let transformed = self
            .vertices
            .iter()
            .map(|point| transform_point(&self.local_transform, *point));
        let transformed: Vec<_> = transformed.collect();
        if transformed.iter().flatten().any(|value| !value.is_finite()) {
            return Err(GeometryError::NonFiniteComputedBounds);
        }
        self.bounds = LocalBounds::from_points(transformed.into_iter())?;
        Ok(())
    }
}

/// Owned, validated procedural geometry description. Bounds are recomputed
/// from vertices and primitive transforms rather than trusted from input.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GeometryAsset {
    schema_version: u32,
    key: AssetKey,
    primitives: Vec<GeometryPrimitive>,
    bounds: LocalBounds,
}

#[derive(Deserialize)]
struct GeometryAssetWire {
    schema_version: u32,
    key: AssetKey,
    primitives: Vec<GeometryPrimitive>,
    // Cached serialized bounds are ignored and recomputed on deserialization.
    #[serde(default)]
    bounds: Option<LocalBounds>,
}

impl<'de> Deserialize<'de> for GeometryAsset {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = GeometryAssetWire::deserialize(deserializer)?;
        let _ignored_cached_bounds = wire.bounds;
        if wire.schema_version != crate::api::assets::ASSET_SCHEMA_VERSION {
            return Err(serde::de::Error::custom(
                GeometryError::UnsupportedSchemaVersion,
            ));
        }
        Self::new(wire.key, wire.primitives).map_err(serde::de::Error::custom)
    }
}

impl GeometryAsset {
    pub fn new(key: AssetKey, primitives: Vec<GeometryPrimitive>) -> Result<Self, GeometryError> {
        let mut asset = Self {
            schema_version: crate::api::assets::ASSET_SCHEMA_VERSION,
            key,
            primitives,
            bounds: LocalBounds {
                min: [0.0; 3],
                max: [0.0; 3],
            },
        };
        asset.validate_and_compute_bounds()?;
        Ok(asset)
    }

    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }
    pub fn key(&self) -> &AssetKey {
        &self.key
    }
    pub fn primitives(&self) -> &[GeometryPrimitive] {
        &self.primitives
    }
    pub fn bounds(&self) -> LocalBounds {
        self.bounds
    }

    /// Revalidate deserialized data and recompute all cached bounds.
    pub fn validate(&mut self) -> Result<(), GeometryError> {
        self.validate_and_compute_bounds()
    }

    fn validate_and_compute_bounds(&mut self) -> Result<(), GeometryError> {
        if self.schema_version != crate::api::assets::ASSET_SCHEMA_VERSION {
            return Err(GeometryError::UnsupportedSchemaVersion);
        }
        AssetKey::new(self.key.as_str().to_owned()).map_err(|_| GeometryError::InvalidAssetKey)?;
        if self.primitives.is_empty() || self.primitives.len() > GeometryLimits::MAX_PRIMITIVES {
            return Err(GeometryError::PrimitiveCountLimit);
        }
        let mut vertex_count = 0usize;
        let mut index_count = 0usize;
        let mut aggregate: Option<LocalBounds> = None;
        for primitive in &mut self.primitives {
            primitive.validate_and_compute_bounds()?;
            vertex_count = vertex_count.saturating_add(primitive.vertices.len());
            index_count = index_count.saturating_add(primitive.indices.len());
            aggregate = Some(match aggregate {
                Some(bounds) => bounds.union(primitive.bounds),
                None => primitive.bounds,
            });
        }
        if vertex_count > GeometryLimits::MAX_VERTICES_PER_ASSET {
            return Err(GeometryError::AssetVertexLimit);
        }
        if index_count > GeometryLimits::MAX_INDICES_PER_ASSET {
            return Err(GeometryError::AssetIndexLimit);
        }
        self.bounds = aggregate.expect("non-empty primitive list was validated");
        Ok(())
    }
}

fn validate_transform(matrix: &[f32; 16]) -> Result<(), GeometryError> {
    if matrix.iter().any(|value| !value.is_finite()) {
        return Err(GeometryError::NonFiniteTransform);
    }
    if matrix[3].abs() > 1e-6
        || matrix[7].abs() > 1e-6
        || matrix[11].abs() > 1e-6
        || (matrix[15] - 1.0).abs() > 1e-6
    {
        return Err(GeometryError::NonAffineTransform);
    }
    Ok(())
}

fn transform_point(matrix: &[f32; 16], point: [f32; 3]) -> [f32; 3] {
    [
        matrix[0] * point[0] + matrix[4] * point[1] + matrix[8] * point[2] + matrix[12],
        matrix[1] * point[0] + matrix[5] * point[1] + matrix[9] * point[2] + matrix[13],
        matrix[2] * point[0] + matrix[6] * point[1] + matrix[10] * point[2] + matrix[14],
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometryError {
    UnsupportedSchemaVersion,
    PrimitiveCountLimit,
    PrimitiveVertexLimit,
    PrimitiveIndexLimit,
    AssetVertexLimit,
    AssetIndexLimit,
    EmptyVertices,
    IndicesNotTriangles,
    IndexOutOfRange,
    NonFiniteCoordinate,
    NonFiniteComputedBounds,
    NonFiniteTransform,
    NonAffineTransform,
    InvalidAssetKey,
    MaterialSlotLimit,
    InvalidMaterialKey,
    TriangleMaterialCountMismatch,
    MaterialReferenceOutOfRange,
}

impl fmt::Display for GeometryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::UnsupportedSchemaVersion => "unsupported geometry schema version",
            Self::PrimitiveCountLimit => "geometry asset must contain 1..=256 primitives",
            Self::PrimitiveVertexLimit => "primitive exceeds vertex limit",
            Self::PrimitiveIndexLimit => "primitive exceeds index limit",
            Self::AssetVertexLimit => "geometry asset exceeds total vertex limit",
            Self::AssetIndexLimit => "geometry asset exceeds total index limit",
            Self::EmptyVertices => "primitive has no vertices",
            Self::IndicesNotTriangles => "index buffer must contain complete triangles",
            Self::IndexOutOfRange => "index refers to a vertex outside the primitive",
            Self::NonFiniteCoordinate => "vertex coordinates must be finite",
            Self::NonFiniteComputedBounds => "transformed coordinates must remain finite",
            Self::NonFiniteTransform => "transform components must be finite",
            Self::NonAffineTransform => "transform must be affine in the documented matrix layout",
            Self::InvalidAssetKey => "geometry asset key is invalid",
            Self::MaterialSlotLimit => "primitive must contain 1..=64 material slots",
            Self::InvalidMaterialKey => "material slot key is empty or exceeds 128 bytes",
            Self::TriangleMaterialCountMismatch => "each triangle must reference one material slot",
            Self::MaterialReferenceOutOfRange => "triangle references a missing material slot",
        };
        f.write_str(message)
    }
}

impl std::error::Error for GeometryError {}
