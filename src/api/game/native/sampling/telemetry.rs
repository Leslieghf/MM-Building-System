//! Fixed-capacity telemetry records shared across native callback and UI threads.
//!
//! Writers publish copied scalar/array values with release ordering; readers
//! acquire readiness before constructing owned snapshots. Records never
//! transfer ownership of game memory, and invalid samples remain explicitly
//! unavailable rather than reusing a previous frame's value.
use super::*;

pub(in crate::api::game::native) struct PlayerSampleState {
    pub(in crate::api::game::native) status: u8,
    pub(in crate::api::game::native) position: Option<[f32; 3]>,
    pub(in crate::api::game::native) transform: Option<Matrix4x4>,
    pub(in crate::api::game::native) forward: Option<[f32; 3]>,
    pub(in crate::api::game::native) rotation: Option<[f32; 3]>,
    pub(in crate::api::game::native) velocity: Option<[f32; 3]>,
    pub(in crate::api::game::native) acceleration: Option<[f32; 3]>,
    pub(in crate::api::game::native) sampled_at: Option<Instant>,
}

pub(in crate::api::game::native) struct VehicleSampleState {
    pub(in crate::api::game::native) position: Option<[f32; 3]>,
    pub(in crate::api::game::native) forward: Option<[f32; 3]>,
    pub(in crate::api::game::native) transform: Option<Matrix4x4>,
    pub(in crate::api::game::native) rotation: Option<[f32; 3]>,
    pub(in crate::api::game::native) velocity: Option<[f32; 3]>,
    pub(in crate::api::game::native) acceleration: Option<[f32; 3]>,
    pub(in crate::api::game::native) angular_velocity: Option<[f32; 3]>,
    pub(in crate::api::game::native) velocity_last_frame: Option<[f32; 3]>,
    pub(in crate::api::game::native) sampled_at: Option<Instant>,
}

/// Camera pose has its own cadence because reticle queries need current view
/// geometry even while the heavier player/vehicle telemetry remains throttled.
#[derive(Default)]
pub(in crate::api::game::native) struct CameraSampleState {
    pub(in crate::api::game::native) position: Option<[f32; 3]>,
    pub(in crate::api::game::native) direction: Option<[f32; 3]>,
    pub(in crate::api::game::native) view_rotation: Option<[f32; 3]>,
    pub(in crate::api::game::native) sampled_at: Option<Instant>,
}

pub(in crate::api::game::native) static CAMERA_SAMPLE: Mutex<CameraSampleState> =
    Mutex::new(CameraSampleState {
        position: None,
        direction: None,
        view_rotation: None,
        sampled_at: None,
    });

pub(in crate::api::game::native) static PLAYER_SAMPLE: Mutex<PlayerSampleState> =
    Mutex::new(PlayerSampleState {
        status: 0,
        position: None,
        transform: None,
        forward: None,
        rotation: None,
        velocity: None,
        acceleration: None,
        sampled_at: None,
    });
#[repr(align(16))]
pub(in crate::api::game::native) struct RaycastFilterContextStorage(
    pub(in crate::api::game::native) [u8; 0xD0],
);
const _: () = assert!(
    std::mem::size_of::<RaycastFilterContextStorage>()
        >= C_STATIC_ONLY_FILTER_CATEGORY_MASK_OFFSET + std::mem::size_of::<u32>()
);

pub(in crate::api::game::native) static VEHICLE_SAMPLE: Mutex<VehicleSampleState> =
    Mutex::new(VehicleSampleState {
        position: None,
        forward: None,
        transform: None,
        rotation: None,
        velocity: None,
        acceleration: None,
        angular_velocity: None,
        velocity_last_frame: None,
        sampled_at: None,
    });
