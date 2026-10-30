//! Thread-safe publication of copied telemetry across hook and UI boundaries.
use super::*;

pub(in crate::api::game::probe) struct PlayerSampleState {
    pub(in crate::api::game::probe) status: u8,
    pub(in crate::api::game::probe) position: Option<[f32; 3]>,
    pub(in crate::api::game::probe) transform: Option<Matrix4x4>,
    pub(in crate::api::game::probe) forward: Option<[f32; 3]>,
    pub(in crate::api::game::probe) rotation: Option<[f32; 3]>,
    pub(in crate::api::game::probe) view_rotation: Option<[f32; 3]>,
    pub(in crate::api::game::probe) reticle_ray_origin: Option<[f32; 3]>,
    pub(in crate::api::game::probe) reticle_ray_direction: Option<[f32; 3]>,
    pub(in crate::api::game::probe) reticle_query_available: bool,
    pub(in crate::api::game::probe) reticle_query_state_present: bool,
    pub(in crate::api::game::probe) reticle_hit_fraction: Option<f32>,
    pub(in crate::api::game::probe) reticle_candidate_position: Option<[f32; 3]>,
    pub(in crate::api::game::probe) reticle_candidate_shape: Option<RaycastCandidateShape>,
    pub(in crate::api::game::probe) player_aim_query_available: bool,
    pub(in crate::api::game::probe) player_aim_query_state_present: bool,
    pub(in crate::api::game::probe) player_aim_hit_fraction: Option<f32>,
    pub(in crate::api::game::probe) player_aim_candidate_position: Option<[f32; 3]>,
    pub(in crate::api::game::probe) player_aim_candidate_shape: Option<RaycastCandidateShape>,
    pub(in crate::api::game::probe) ignore_characters_vehicles_query_available: bool,
    pub(in crate::api::game::probe) ignore_characters_vehicles_query_state_present: bool,
    pub(in crate::api::game::probe) ignore_characters_vehicles_hit_fraction: Option<f32>,
    pub(in crate::api::game::probe) ignore_characters_vehicles_candidate_position: Option<[f32; 3]>,
    pub(in crate::api::game::probe) ignore_characters_vehicles_candidate_shape:
        Option<RaycastCandidateShape>,
    pub(in crate::api::game::probe) ignore_player_owner_query_available: bool,
    pub(in crate::api::game::probe) ignore_player_owner_query_state_present: bool,
    pub(in crate::api::game::probe) ignore_player_owner_fraction: Option<f32>,
    pub(in crate::api::game::probe) ignore_player_owner_candidate_position: Option<[f32; 3]>,
    pub(in crate::api::game::probe) ignore_player_owner_candidate_shape:
        Option<RaycastCandidateShape>,
    pub(in crate::api::game::probe) velocity: Option<[f32; 3]>,
    pub(in crate::api::game::probe) acceleration: Option<[f32; 3]>,
    pub(in crate::api::game::probe) sampled_at: Option<Instant>,
}

pub(in crate::api::game::probe) struct VehicleSampleState {
    pub(in crate::api::game::probe) position: Option<[f32; 3]>,
    pub(in crate::api::game::probe) forward: Option<[f32; 3]>,
    pub(in crate::api::game::probe) transform: Option<Matrix4x4>,
    pub(in crate::api::game::probe) rotation: Option<[f32; 3]>,
    pub(in crate::api::game::probe) velocity: Option<[f32; 3]>,
    pub(in crate::api::game::probe) acceleration: Option<[f32; 3]>,
    pub(in crate::api::game::probe) angular_velocity: Option<[f32; 3]>,
    pub(in crate::api::game::probe) velocity_last_frame: Option<[f32; 3]>,
    pub(in crate::api::game::probe) sampled_at: Option<Instant>,
}

pub(in crate::api::game::probe) static PLAYER_SAMPLE: Mutex<PlayerSampleState> =
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
pub(in crate::api::game::probe) static LAST_RETICLE_RAY_SAMPLE: Mutex<Option<Instant>> =
    Mutex::new(None);

#[repr(align(16))]
pub(in crate::api::game::probe) struct RaycastFilterContextStorage(
    pub(in crate::api::game::probe) [u8; 0xD0],
);
const _: () = assert!(
    std::mem::size_of::<RaycastFilterContextStorage>()
        >= C_STATIC_ONLY_FILTER_CATEGORY_MASK_OFFSET + std::mem::size_of::<u32>()
        && std::mem::size_of::<RaycastFilterContextStorage>()
            >= C_IGNORE_FILTER_OWNER_VECTOR_CAPACITY_OFFSET + std::mem::size_of::<usize>()
);

#[derive(Clone, Copy)]
pub(in crate::api::game::probe) struct ReticleRaySample {
    pub(in crate::api::game::probe) origin: Option<[f32; 3]>,
    pub(in crate::api::game::probe) direction: Option<[f32; 3]>,
    pub(in crate::api::game::probe) query_available: bool,
    pub(in crate::api::game::probe) query_state_present: bool,
    pub(in crate::api::game::probe) fraction: Option<f32>,
    pub(in crate::api::game::probe) candidate_position: Option<[f32; 3]>,
    pub(in crate::api::game::probe) candidate_shape: Option<RaycastCandidateShape>,
    pub(in crate::api::game::probe) player_aim_query_available: bool,
    pub(in crate::api::game::probe) player_aim_query_state_present: bool,
    pub(in crate::api::game::probe) player_aim_fraction: Option<f32>,
    pub(in crate::api::game::probe) player_aim_candidate_position: Option<[f32; 3]>,
    pub(in crate::api::game::probe) player_aim_candidate_shape: Option<RaycastCandidateShape>,
    pub(in crate::api::game::probe) ignore_characters_vehicles_query_available: bool,
    pub(in crate::api::game::probe) ignore_characters_vehicles_query_state_present: bool,
    pub(in crate::api::game::probe) ignore_characters_vehicles_fraction: Option<f32>,
    pub(in crate::api::game::probe) ignore_characters_vehicles_candidate_position: Option<[f32; 3]>,
    pub(in crate::api::game::probe) ignore_characters_vehicles_candidate_shape:
        Option<RaycastCandidateShape>,
    pub(in crate::api::game::probe) ignore_player_owner_query_available: bool,
    pub(in crate::api::game::probe) ignore_player_owner_query_state_present: bool,
    pub(in crate::api::game::probe) ignore_player_owner_fraction: Option<f32>,
    pub(in crate::api::game::probe) ignore_player_owner_candidate_position: Option<[f32; 3]>,
    pub(in crate::api::game::probe) ignore_player_owner_candidate_shape:
        Option<RaycastCandidateShape>,
}

pub(in crate::api::game::probe) static VEHICLE_SAMPLE: Mutex<VehicleSampleState> =
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
