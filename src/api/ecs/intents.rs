//! Validated spawn/vehicle intent contracts and request constructors.
use crate::api::ecs::config::{MAX_SPAWN_ASSET_KEY_BYTES, NEXT_INTENT_ID};
use crate::api::game::abi::types::Matrix4x4;
use bevy_ecs::prelude::*;
use std::sync::atomic::Ordering;

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
    /// Validate and own a normalized exported-entity key.
    ///
    /// # Errors
    ///
    /// Returns [`SpawnRequestError::InvalidAssetKey`] when the key is empty,
    /// too long, non-ASCII, outside `editor/entities/`, or contains an invalid
    /// path segment.
    ///
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
    /// Accept a finite row-major transform for a spawn request.
    ///
    /// # Errors
    ///
    /// Returns [`SpawnRequestError::InvalidTransform`] if any matrix element is
    /// non-finite.
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

/// Build a queued spawn bundle from a validated asset key and transform.
///
/// # Errors
///
/// Returns [`SpawnRequestError::InvalidAssetKey`] or
/// [`SpawnRequestError::InvalidTransform`] when either input fails validation.
pub fn spawn_entity_intent(
    asset_key: impl Into<String>,
    matrix: Matrix4x4,
) -> Result<impl Bundle, SpawnRequestError> {
    let asset_key = SpawnAssetKey::parse(asset_key)?;
    let transform = SpawnTransform::new(matrix)?;
    let request_id = SpawnRequestId(NEXT_INTENT_ID.fetch_add(1, Ordering::Relaxed));
    Ok((request_id, asset_key, transform, SpawnLifecycle::Queued))
}

/// Build an effect for the vehicle occupied when the native adapter consumes it.
///
/// This constructor does not validate the velocity. The game adapter validates
/// finite components and the supported speed range before applying it.
pub fn player_vehicle_velocity_intent(velocity: [f32; 3]) -> impl Bundle {
    (
        TargetCurrentPlayerVehicle,
        IntentId(
            crate::api::ecs::config::NEXT_INTENT_ID
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        ),
        SetPlayerVehicleVelocity(velocity),
    )
}
