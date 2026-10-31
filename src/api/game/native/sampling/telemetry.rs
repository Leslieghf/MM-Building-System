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
    pub(in crate::api::game::native) view_rotation: Option<[f32; 3]>,
    pub(in crate::api::game::native) reticle_ray_origin: Option<[f32; 3]>,
    pub(in crate::api::game::native) reticle_ray_direction: Option<[f32; 3]>,
    pub(in crate::api::game::native) reticle_query_available: bool,
    pub(in crate::api::game::native) reticle_query_state_present: bool,
    pub(in crate::api::game::native) reticle_hit_fraction: Option<f32>,
    pub(in crate::api::game::native) reticle_candidate_position: Option<[f32; 3]>,
    pub(in crate::api::game::native) reticle_candidate_shape: Option<RaycastCandidateShape>,
    pub(in crate::api::game::native) player_aim_query_available: bool,
    pub(in crate::api::game::native) player_aim_query_state_present: bool,
    pub(in crate::api::game::native) player_aim_hit_fraction: Option<f32>,
    pub(in crate::api::game::native) player_aim_candidate_position: Option<[f32; 3]>,
    pub(in crate::api::game::native) player_aim_candidate_shape: Option<RaycastCandidateShape>,
    pub(in crate::api::game::native) ignore_characters_vehicles_query_available: bool,
    pub(in crate::api::game::native) ignore_characters_vehicles_query_state_present: bool,
    pub(in crate::api::game::native) ignore_characters_vehicles_hit_fraction: Option<f32>,
    pub(in crate::api::game::native) ignore_characters_vehicles_candidate_position:
        Option<[f32; 3]>,
    pub(in crate::api::game::native) ignore_characters_vehicles_candidate_shape:
        Option<RaycastCandidateShape>,
    pub(in crate::api::game::native) ignore_player_owner_query_available: bool,
    pub(in crate::api::game::native) ignore_player_owner_query_state_present: bool,
    pub(in crate::api::game::native) ignore_player_owner_fraction: Option<f32>,
    pub(in crate::api::game::native) ignore_player_owner_candidate_position: Option<[f32; 3]>,
    pub(in crate::api::game::native) ignore_player_owner_candidate_shape:
        Option<RaycastCandidateShape>,
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

pub(in crate::api::game::native) static PLAYER_SAMPLE: Mutex<PlayerSampleState> =
    Mutex::new(PlayerSampleState {
        status: 0,
        position: None,
        transform: None,
        forward: None,
        rotation: None,
        view_rotation: None,
        reticle_ray_origin: None,
        reticle_ray_direction: None,
        reticle_query_available: false,
        reticle_query_state_present: false,
        reticle_hit_fraction: None,
        reticle_candidate_position: None,
        reticle_candidate_shape: None,
        player_aim_query_available: false,
        player_aim_query_state_present: false,
        player_aim_hit_fraction: None,
        player_aim_candidate_position: None,
        player_aim_candidate_shape: None,
        ignore_characters_vehicles_query_available: false,
        ignore_characters_vehicles_query_state_present: false,
        ignore_characters_vehicles_hit_fraction: None,
        ignore_characters_vehicles_candidate_position: None,
        ignore_characters_vehicles_candidate_shape: None,
        ignore_player_owner_query_available: false,
        ignore_player_owner_query_state_present: false,
        ignore_player_owner_fraction: None,
        ignore_player_owner_candidate_position: None,
        ignore_player_owner_candidate_shape: None,
        velocity: None,
        acceleration: None,
        sampled_at: None,
    });
pub(in crate::api::game::native) static LAST_RETICLE_RAY_SAMPLE: Mutex<Option<Instant>> =
    Mutex::new(None);

#[repr(align(16))]
pub(in crate::api::game::native) struct RaycastFilterContextStorage(
    pub(in crate::api::game::native) [u8; 0xD0],
);
const _: () = assert!(
    std::mem::size_of::<RaycastFilterContextStorage>()
        >= C_STATIC_ONLY_FILTER_CATEGORY_MASK_OFFSET + std::mem::size_of::<u32>()
        && std::mem::size_of::<RaycastFilterContextStorage>()
            >= C_IGNORE_FILTER_OWNER_VECTOR_CAPACITY_OFFSET + std::mem::size_of::<usize>()
);

#[derive(Clone, Copy, Default)]
pub(in crate::api::game::native) struct ReticleRaySample {
    pub(in crate::api::game::native) origin: Option<[f32; 3]>,
    pub(in crate::api::game::native) direction: Option<[f32; 3]>,
    pub(in crate::api::game::native) query_available: bool,
    pub(in crate::api::game::native) query_state_present: bool,
    pub(in crate::api::game::native) fraction: Option<f32>,
    pub(in crate::api::game::native) candidate_position: Option<[f32; 3]>,
    pub(in crate::api::game::native) candidate_shape: Option<RaycastCandidateShape>,
    pub(in crate::api::game::native) player_aim_query_available: bool,
    pub(in crate::api::game::native) player_aim_query_state_present: bool,
    pub(in crate::api::game::native) player_aim_fraction: Option<f32>,
    pub(in crate::api::game::native) player_aim_candidate_position: Option<[f32; 3]>,
    pub(in crate::api::game::native) player_aim_candidate_shape: Option<RaycastCandidateShape>,
    pub(in crate::api::game::native) ignore_characters_vehicles_query_available: bool,
    pub(in crate::api::game::native) ignore_characters_vehicles_query_state_present: bool,
    pub(in crate::api::game::native) ignore_characters_vehicles_fraction: Option<f32>,
    pub(in crate::api::game::native) ignore_characters_vehicles_candidate_position:
        Option<[f32; 3]>,
    pub(in crate::api::game::native) ignore_characters_vehicles_candidate_shape:
        Option<RaycastCandidateShape>,
    pub(in crate::api::game::native) ignore_player_owner_query_available: bool,
    pub(in crate::api::game::native) ignore_player_owner_query_state_present: bool,
    pub(in crate::api::game::native) ignore_player_owner_fraction: Option<f32>,
    pub(in crate::api::game::native) ignore_player_owner_candidate_position: Option<[f32; 3]>,
    pub(in crate::api::game::native) ignore_player_owner_candidate_shape:
        Option<RaycastCandidateShape>,
}

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
