//! UI-world schedule, readout projection, and request handoff.
//!
//! # Data flow
//!
//! The render schedule consumes the latest copied [`crate::api::ecs::GameSnapshot`]
//! and diagnostics, then stores display-ready rows in `UiPresentation`. It does
//! not sample the game or dereference native pointers. [`flush_ui_intents`]
//! forwards bounded UI requests to the game callback; a full queue leaves the
//! corresponding UI intent available for retry.
//!
//! # Ordering
//!
//! Registered UI mod systems run before presentation systems. Keep systems
//! that need to affect the current frame in the mod-system set.
mod intent_transport;
use crate::api::ecs::*;
use crate::api::format::{
    angles as format_angles, fixed as format_fixed, magnitude as format_magnitude,
    vector as format_vector,
};
use crate::api::game::abi::build::steam::IMAGE_BASE as STEAM_IMAGE_BASE;
use crate::api::game::abi::game::world_query::C_CUSTOM_SAMPLED_HEIGHT_FIELD_SHAPE_VFTABLE_RVA;
use crate::api::scripts::console::ConsoleOutput;
use crate::devtools::state::*;
use bevy_app::App;
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{IntoScheduleConfigs, Schedule, SingleThreadedExecutor};
pub(crate) use intent_transport::flush_ui_intents;

pub fn create_ui_app() -> App {
    let mut app = App::empty();
    let mut schedule = Schedule::new(MadMaxRender);
    schedule.set_executor(SingleThreadedExecutor::new());
    app.add_schedule(schedule);
    app.configure_sets(
        MadMaxRender,
        (MadMaxRenderSet::ModSystems, MadMaxRenderSet::Presentation).chain(),
    );
    app.init_resource::<UiGameSnapshot>();
    app.init_resource::<crate::api::ecs::input::InputFrame>();
    app.init_resource::<UiPreferences>();
    app.init_resource::<UiPresentation>();
    app.init_resource::<RuntimeDiagnostics>();
    app.init_resource::<SpawnTestUiState>();
    app.init_resource::<ConsoleOutput>();
    app.init_resource::<ScriptConsoleUiState>();
    app.add_systems(
        MadMaxRender,
        (count_render_schedule, prepare_presentation)
            .chain()
            .in_set(MadMaxRenderSet::Presentation),
    );
    crate::api::ecs::systems::register_ui_systems(&mut app);
    app
}

fn count_render_schedule(mut diagnostics: ResMut<RuntimeDiagnostics>) {
    diagnostics.render_schedule_runs = diagnostics.render_schedule_runs.saturating_add(1);
}

fn prepare_presentation(
    snapshot: Res<UiGameSnapshot>,
    diagnostics: Res<RuntimeDiagnostics>,
    mut presentation: ResMut<UiPresentation>,
) {
    let data = snapshot.0;
    let rows = &mut presentation.0;
    rows.clear();
    rows.reserve(31);
    append_player_rows(&data, rows);
    append_camera_rows(&data, rows);
    append_static_query_rows(&data, rows);
    append_player_aim_rows(&data, rows);
    append_ignore_actor_rows(&data, rows);
    append_ignore_owner_rows(&data, rows);
    append_vehicle_rows(&data, rows);
    append_runtime_rows(&data, &diagnostics, rows);
}

fn append_player_rows(data: &GameSnapshot, rows: &mut Vec<PresentationRow>) {
    rows.push(row(
        DebugSection::Player,
        "Position",
        format_vector(data.player_position),
    ));
    rows.push(row(
        DebugSection::Player,
        "Rotation (player/model)",
        format_angles(data.player_rotation),
    ));
    rows.push(row(
        DebugSection::Player,
        "Speed magnitude (estimated)",
        format_magnitude(data.player_velocity, " units/s"),
    ));
    rows.push(row(
        DebugSection::Player,
        "Acceleration magnitude (estimated)",
        format_magnitude(data.player_acceleration, " units/s²"),
    ));
}

fn append_camera_rows(data: &GameSnapshot, rows: &mut Vec<PresentationRow>) {
    rows.push(row(
        DebugSection::Camera,
        "Rotation (view)",
        format_angles(data.view_rotation),
    ));
}

fn append_static_query_rows(data: &GameSnapshot, rows: &mut Vec<PresentationRow>) {
    append_reticle_query_status(data, rows);
    append_reticle_ray(data, rows);
    append_reticle_hit(data, rows);
}

fn append_reticle_query_status(data: &GameSnapshot, rows: &mut Vec<PresentationRow>) {
    rows.push(row(
        DebugSection::Raycast,
        "Static-only query",
        if data.reticle_query_available {
            format!(
                "active / {}",
                if data.reticle_query_state_present {
                    "result state present"
                } else {
                    "no result state"
                }
            )
        } else {
            "unavailable".to_owned()
        },
    ));
}

fn append_reticle_ray(data: &GameSnapshot, rows: &mut Vec<PresentationRow>) {
    rows.push(row(
        DebugSection::Raycast,
        "Ray origin",
        format_vector(data.reticle_ray_origin),
    ));
    rows.push(row(
        DebugSection::Raycast,
        "Ray direction",
        format_vector(data.reticle_ray_direction),
    ));
}

fn append_reticle_hit(data: &GameSnapshot, rows: &mut Vec<PresentationRow>) {
    rows.push(row(
        DebugSection::Raycast,
        "Static-only hit fraction",
        data.reticle_hit_fraction.map_or_else(
            || "unavailable".to_owned(),
            |value| {
                if value >= 1.0 {
                    "no hit".to_owned()
                } else {
                    format_fixed(f64::from(value))
                }
            },
        ),
    ));
    rows.push(row(
        DebugSection::Raycast,
        "Static-only hit position",
        data.reticle_candidate_position.map_or_else(
            || {
                if !data.reticle_query_available {
                    "unavailable".to_owned()
                } else if data
                    .reticle_hit_fraction
                    .is_some_and(|fraction| fraction >= 1.0)
                {
                    "no hit".to_owned()
                } else {
                    "unknown".to_owned()
                }
            },
            |position| format_vector(Some(position)),
        ),
    ));
    rows.push(row(
        DebugSection::Raycast,
        "Static-only hit shape",
        format_raycast_candidate_shape(
            data.reticle_candidate_shape,
            data.reticle_query_available,
            data.reticle_hit_fraction,
        ),
    ));
}

fn append_player_aim_rows(data: &GameSnapshot, rows: &mut Vec<PresentationRow>) {
    rows.push(row(
        DebugSection::Raycast,
        "Player-aim query",
        if data.player_aim_query_available {
            format!(
                "active / {}",
                if data.player_aim_query_state_present {
                    "result state present"
                } else {
                    "no result state"
                }
            )
        } else {
            "unavailable".to_owned()
        },
    ));
    rows.push(row(
        DebugSection::Raycast,
        "Player-aim filtered fraction",
        data.player_aim_hit_fraction.map_or_else(
            || {
                if !data.player_aim_query_available {
                    "unavailable".to_owned()
                } else {
                    "no hit".to_owned()
                }
            },
            |value| {
                if value >= 1.0 {
                    "no hit".to_owned()
                } else {
                    format_fixed(f64::from(value))
                }
            },
        ),
    ));
    rows.push(row(
        DebugSection::Raycast,
        "Player-aim filtered position",
        data.player_aim_candidate_position.map_or_else(
            || {
                if !data.player_aim_query_available {
                    "unavailable".to_owned()
                } else {
                    "no hit".to_owned()
                }
            },
            |position| format_vector(Some(position)),
        ),
    ));
    rows.push(row(
        DebugSection::Raycast,
        "Player-aim hit shape",
        format_raycast_candidate_shape(
            data.player_aim_candidate_shape,
            data.player_aim_query_available,
            data.player_aim_hit_fraction,
        ),
    ));
}

fn append_ignore_actor_rows(data: &GameSnapshot, rows: &mut Vec<PresentationRow>) {
    rows.push(row(
        DebugSection::Raycast,
        "Ignore characters/vehicles query",
        if data.ignore_characters_vehicles_query_available {
            if data.ignore_characters_vehicles_query_state_present {
                "active / result state present".to_owned()
            } else {
                "active / no result state".to_owned()
            }
        } else {
            "unavailable".to_owned()
        },
    ));
    rows.push(row(
        DebugSection::Raycast,
        "Ignore characters/vehicles fraction",
        data.ignore_characters_vehicles_hit_fraction.map_or_else(
            || {
                if data.ignore_characters_vehicles_query_available {
                    "no hit".to_owned()
                } else {
                    "unavailable".to_owned()
                }
            },
            |value| {
                if value >= 1.0 {
                    "no hit".to_owned()
                } else {
                    format_fixed(f64::from(value))
                }
            },
        ),
    ));
    rows.push(row(
        DebugSection::Raycast,
        "Ignore characters/vehicles position",
        data.ignore_characters_vehicles_candidate_position
            .map_or_else(
                || {
                    if data.ignore_characters_vehicles_query_available {
                        "no hit".to_owned()
                    } else {
                        "unavailable".to_owned()
                    }
                },
                |position| format_vector(Some(position)),
            ),
    ));
    rows.push(row(
        DebugSection::Raycast,
        "Ignore characters/vehicles hit shape",
        format_raycast_candidate_shape(
            data.ignore_characters_vehicles_candidate_shape,
            data.ignore_characters_vehicles_query_available,
            data.ignore_characters_vehicles_hit_fraction,
        ),
    ));
}

fn append_ignore_owner_rows(data: &GameSnapshot, rows: &mut Vec<PresentationRow>) {
    rows.push(row(
        DebugSection::Raycast,
        "Ignore-player-owner query",
        if data.ignore_player_owner_query_available {
            if data.ignore_player_owner_query_state_present {
                "active / result state present".to_owned()
            } else {
                "active / no result state".to_owned()
            }
        } else {
            "unavailable".to_owned()
        },
    ));
    rows.push(row(
        DebugSection::Raycast,
        "Ignore-player-owner fraction",
        data.ignore_player_owner_hit_fraction.map_or_else(
            || {
                if data.ignore_player_owner_query_available {
                    "no hit".to_owned()
                } else {
                    "unavailable".to_owned()
                }
            },
            |value| {
                if value >= 1.0 {
                    "no hit".to_owned()
                } else {
                    format_fixed(f64::from(value))
                }
            },
        ),
    ));
    rows.push(row(
        DebugSection::Raycast,
        "Ignore-player-owner position",
        data.ignore_player_owner_candidate_position.map_or_else(
            || {
                if data.ignore_player_owner_query_available {
                    "no hit".to_owned()
                } else {
                    "unavailable".to_owned()
                }
            },
            |position| format_vector(Some(position)),
        ),
    ));
    rows.push(row(
        DebugSection::Raycast,
        "Ignore-player-owner hit shape",
        format_raycast_candidate_shape(
            data.ignore_player_owner_candidate_shape,
            data.ignore_player_owner_query_available,
            data.ignore_player_owner_hit_fraction,
        ),
    ));
}

fn append_vehicle_rows(data: &GameSnapshot, rows: &mut Vec<PresentationRow>) {
    let vehicle_status = if data.vehicle_position.is_some()
        || data.vehicle_velocity.is_some()
        || data.vehicle_acceleration.is_some()
        || data.vehicle_angular_velocity.is_some()
        || data.vehicle_velocity_last_frame.is_some()
    {
        "vehicle found".to_owned()
    } else {
        "no vehicle available".to_owned()
    };
    rows.push(row(DebugSection::Vehicle, "Status", vehicle_status));
    rows.push(row(
        DebugSection::Vehicle,
        "Position",
        format_vector(data.vehicle_position),
    ));
    rows.push(row(
        DebugSection::Vehicle,
        "Rotation",
        format_angles(data.vehicle_rotation),
    ));
    rows.push(row(
        DebugSection::Vehicle,
        "Speed magnitude (physics)",
        format_magnitude(data.vehicle_velocity, " units/s"),
    ));
    rows.push(row(
        DebugSection::Vehicle,
        "Acceleration magnitude (physics)",
        format_magnitude(data.vehicle_acceleration, " units/s²"),
    ));
    rows.push(row(
        DebugSection::Vehicle,
        "Angular velocity magnitude",
        format_magnitude(data.vehicle_angular_velocity, ""),
    ));
    rows.push(row(
        DebugSection::Vehicle,
        "Previous-frame speed magnitude",
        format_magnitude(data.vehicle_velocity_last_frame, ""),
    ));
}

fn append_runtime_rows(
    data: &GameSnapshot,
    diagnostics: &RuntimeDiagnostics,
    rows: &mut Vec<PresentationRow>,
) {
    rows.push(row(
        DebugSection::Runtime,
        "Frame delta",
        format!(
            "{} ms",
            format_fixed(f64::from(data.callback_delta_seconds) * 1000.0)
        ),
    ));
    if let Some(outcome) = data.last_intent_outcome.or(diagnostics.last_intent_outcome) {
        rows.push(row(
            DebugSection::Runtime,
            "Last velocity intent",
            format!("{}: {:?}", outcome.intent_id, outcome.result),
        ));
    }
    if let Some(outcome) = data.last_spawn_outcome.or(diagnostics.last_spawn_outcome) {
        rows.push(row(
            DebugSection::Runtime,
            "Last spawn request",
            format!(
                "{}: {:?} ({:?})",
                outcome.request_id, outcome.result, outcome.native_return
            ),
        ));
    }
}

fn row(section: DebugSection, label: &'static str, value: String) -> PresentationRow {
    PresentationRow {
        section,
        label,
        value,
    }
}

fn format_raycast_candidate_shape(
    shape: Option<RaycastCandidateShape>,
    query_available: bool,
    hit_fraction: Option<f32>,
) -> String {
    let Some(shape) = shape else {
        if !query_available {
            return "unavailable".to_owned();
        }
        return if hit_fraction.is_some_and(|fraction| fraction >= 1.0) {
            "no hit".to_owned()
        } else {
            "unknown".to_owned()
        };
    };
    let height_field_vtable = STEAM_IMAGE_BASE + C_CUSTOM_SAMPLED_HEIGHT_FIELD_SHAPE_VFTABLE_RVA;
    let kind = if shape.vtable == height_field_vtable {
        "sampled heightfield terrain"
    } else {
        "unknown shape"
    };
    format!("{kind} 0x{:X} (vtable 0x{:X})", shape.address, shape.vtable)
}
