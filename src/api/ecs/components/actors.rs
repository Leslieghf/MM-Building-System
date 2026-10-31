//! ECS components projected from the latest copied game frame.
use crate::api::game::abi::types::Matrix4x4;
use bevy_ecs::prelude::Component;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
/// Marker on the ECS entity representing the sampled player character.
pub struct PlayerCharacterProxy;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
/// Marker on the ECS entity representing the current player vehicle.
pub struct CurrentPlayerVehicleProxy;

#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
/// Optional player values projected from the current [`crate::api::ecs::GameSnapshot`].
pub struct PlayerFrameData {
    /// World position in game units.
    pub position: Option<[f32; 3]>,
    /// Horizontal normalized forward direction when available.
    pub forward: Option<[f32; 3]>,
    /// Copied row-major transform.
    pub transform: Option<Matrix4x4>,
    /// Sampled rotation values.
    pub rotation: Option<[f32; 3]>,
    /// Sampled camera/view rotation values.
    pub view_rotation: Option<[f32; 3]>,
    /// Sampled linear velocity.
    pub velocity: Option<[f32; 3]>,
    /// Sampled acceleration.
    pub acceleration: Option<[f32; 3]>,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
/// Optional current-vehicle values projected from the current [`crate::api::ecs::GameSnapshot`].
pub struct VehicleFrameData {
    /// World position in game units.
    pub position: Option<[f32; 3]>,
    /// Horizontal normalized forward direction when available.
    pub forward: Option<[f32; 3]>,
    /// Copied row-major transform.
    pub transform: Option<Matrix4x4>,
    /// Sampled rotation values.
    pub rotation: Option<[f32; 3]>,
    /// Sampled linear velocity.
    pub velocity: Option<[f32; 3]>,
    /// Sampled acceleration.
    pub acceleration: Option<[f32; 3]>,
    /// Sampled angular velocity.
    pub angular_velocity: Option<[f32; 3]>,
    /// Linear velocity from the previous sample, when available.
    pub velocity_last_frame: Option<[f32; 3]>,
}
