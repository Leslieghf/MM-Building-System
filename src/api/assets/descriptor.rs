use serde::{Deserialize, Serialize};
use std::fmt;

/// Maximum encoded length for a stable mod-owned asset key.
pub const MAX_ASSET_KEY_BYTES: usize = 192;
/// Maximum encoded length for a provenance locator (path, URI, or index key).
pub const MAX_SOURCE_LOCATOR_BYTES: usize = 512;
/// Maximum dependency edges stored on one descriptor.
pub const MAX_ASSET_DEPENDENCIES: usize = 128;

/// Stable, mod-owned identity. This is not a game object ID or native handle.
///
/// Keys use `namespace:path` syntax. Both parts are lowercase ASCII and may
/// contain digits, `.`, `_`, `-`, and `/`; each part must start with a letter
/// or digit. Examples: `base:models/scrap_heap` and `author:meshes/wall`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct AssetKey(String);

impl<'de> Deserialize<'de> for AssetKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

impl AssetKey {
    /// Validate and create a stable key.
    pub fn new(value: impl Into<String>) -> Result<Self, DescriptorError> {
        let value = value.into();
        if value.is_empty() || value.len() > MAX_ASSET_KEY_BYTES {
            return Err(DescriptorError::InvalidAssetKey);
        }
        let Some((namespace, path)) = value.split_once(':') else {
            return Err(DescriptorError::InvalidAssetKey);
        };
        if path.is_empty()
            || namespace.is_empty()
            || path.contains(':')
            || !valid_key_part(namespace)
            || !valid_key_part(path)
        {
            return Err(DescriptorError::InvalidAssetKey);
        }
        Ok(Self(value))
    }

    /// Borrow the canonical key string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn valid_key_part(part: &str) -> bool {
    let mut chars = part.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_lowercase() || first.is_ascii_digit())
        && chars.all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-' | '/')
        })
}

/// Where an asset description came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    /// Authored by a mod or tooling project.
    Mod,
    /// Identified in the installed game's content/index files.
    GameContent,
    /// Derived from another asset or a conversion step.
    Generated,
    /// Origin has not been classified.
    Unknown,
}

/// Provenance information; `locator` is descriptive data, never opened here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AssetSource {
    pub kind: SourceKind,
    pub locator: String,
}

#[derive(Deserialize)]
struct AssetSourceWire {
    kind: SourceKind,
    locator: String,
}

impl<'de> Deserialize<'de> for AssetSource {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = AssetSourceWire::deserialize(deserializer)?;
        Self::new(wire.kind, wire.locator).map_err(serde::de::Error::custom)
    }
}

impl AssetSource {
    pub fn new(kind: SourceKind, locator: impl Into<String>) -> Result<Self, DescriptorError> {
        let locator = locator.into();
        if locator.is_empty() || locator.len() > MAX_SOURCE_LOCATOR_BYTES {
            return Err(DescriptorError::InvalidSourceLocator);
        }
        Ok(Self { kind, locator })
    }
}

/// Broad content family. This classification does not imply runtime support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceFamily {
    Model,
    Geometry,
    Material,
    Texture,
    Shader,
    Entity,
    Animation,
    Skeleton,
    Audio,
    Effect,
    Script,
    Bundle,
    Other,
}

/// State of an offline/host decoder for the referenced resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecodeState {
    NotAttempted,
    Decoded,
    Partial,
    Unsupported,
    Failed,
}

/// State of preview generation, separate from game rendering support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewState {
    NotAttempted,
    Available,
    Partial,
    Unsupported,
    Failed,
}

/// Confidence in the semantic identification, not confidence that an asset works in game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceConfidence {
    Confirmed,
    StrongCandidate,
    Tentative,
    Unknown,
}

/// A bounded reference to another mod-owned asset descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetDependency {
    pub key: AssetKey,
    pub required: bool,
}

/// Owned metadata for a resource; contains no native handles or pointers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AssetDescriptor {
    pub key: AssetKey,
    pub source: AssetSource,
    pub family: ResourceFamily,
    pub dependencies: Vec<AssetDependency>,
    pub decode: DecodeState,
    pub preview: PreviewState,
    pub confidence: EvidenceConfidence,
}

#[derive(Deserialize)]
struct AssetDescriptorWire {
    key: AssetKey,
    source: AssetSource,
    family: ResourceFamily,
    dependencies: Vec<AssetDependency>,
    decode: DecodeState,
    preview: PreviewState,
    confidence: EvidenceConfidence,
}

impl<'de> Deserialize<'de> for AssetDescriptor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = AssetDescriptorWire::deserialize(deserializer)?;
        Self::new(
            wire.key,
            wire.source,
            wire.family,
            wire.dependencies,
            wire.decode,
            wire.preview,
            wire.confidence,
        )
        .map_err(serde::de::Error::custom)
    }
}

impl AssetDescriptor {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        key: AssetKey,
        source: AssetSource,
        family: ResourceFamily,
        dependencies: Vec<AssetDependency>,
        decode: DecodeState,
        preview: PreviewState,
        confidence: EvidenceConfidence,
    ) -> Result<Self, DescriptorError> {
        let value = Self {
            key,
            source,
            family,
            dependencies,
            decode,
            preview,
            confidence,
        };
        value.validate()?;
        Ok(value)
    }

    /// Recheck data after deserialization or mutation by a host caller.
    pub fn validate(&self) -> Result<(), DescriptorError> {
        AssetKey::new(self.key.0.clone())?;
        AssetSource::new(self.source.kind, self.source.locator.clone())?;
        if self.dependencies.len() > MAX_ASSET_DEPENDENCIES {
            return Err(DescriptorError::TooManyDependencies);
        }
        for dependency in &self.dependencies {
            AssetKey::new(dependency.key.0.clone())?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescriptorError {
    InvalidAssetKey,
    InvalidSourceLocator,
    TooManyDependencies,
}

impl fmt::Display for DescriptorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAssetKey => f.write_str("asset key is invalid or exceeds its byte limit"),
            Self::InvalidSourceLocator => {
                f.write_str("source locator is empty or exceeds its byte limit")
            }
            Self::TooManyDependencies => f.write_str("asset exceeds the dependency limit"),
        }
    }
}

impl std::error::Error for DescriptorError {}
