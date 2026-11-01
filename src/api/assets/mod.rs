//! Engine-independent asset descriptions shared by host-side tools.
//!
//! This module contains owned data only. It does not load game resources,
//! construct native objects, or expose handles into Mad Max. Rhai currently
//! has no constructors or accessors for these types.

mod descriptor;
mod geometry;

pub use descriptor::{
    AssetDependency, AssetDescriptor, AssetKey, AssetSource, DecodeState, DescriptorError,
    EvidenceConfidence, PreviewState, ResourceFamily, SourceKind,
};
pub use geometry::{
    GeometryAsset, GeometryError, GeometryLimits, GeometryPrimitive, LocalBounds, MaterialSlot,
};

/// Version of the serialized host-side asset/geometry schema.
pub const ASSET_SCHEMA_VERSION: u32 = 1;
