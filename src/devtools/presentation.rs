//! UI-side schedule, readout projection, and UI intent collection.
use crate::api::ecs::*;
use crate::api::format::{
    angles as format_angles, fixed as format_fixed, magnitude as format_magnitude,
    vector as format_vector,
};
use crate::api::game::abi::build::steam::IMAGE_BASE as STEAM_IMAGE_BASE;
use crate::api::game::abi::game::world_query::C_CUSTOM_SAMPLED_HEIGHT_FIELD_SHAPE_VFTABLE_RVA;
use crate::api::scripts::console::ConsoleOutput;
use crate::devtools::model::*;
use bevy_app::App;
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{IntoScheduleConfigs, Schedule, SingleThreadedExecutor};

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
    crate::api::ecs::modding::register_ui_systems(&mut app);
    app
}

/// Transfer UI-world intent entities into the bounded game-world transport.
/// Full queues leave the entities in the UI world to retry on a later render.
pub fn flush_ui_intents(app: &mut App, endpoint: &UiEndpoint) {
    let world = app.world_mut();
    let mut pending: Vec<_> = {
        let mut query = world.query_filtered::<
            (Entity, &IntentId, &SetPlayerVehicleVelocity),
            Without<IntentOutcomeRecord>,
        >();
        query
            .iter(world)
            .map(|(entity, id, velocity)| (entity, id.0, velocity.0))
            .collect()
    };
    pending.sort_by_key(|(_, intent_id, _)| *intent_id);

    let mut sent = 0;
    for (entity, intent_id, velocity) in pending {
        if sent >= MAX_INTENTS_PER_RENDER {
            break;
        }
        match endpoint.intent_tx.try_send(IntentEnvelope {
            intent_id,
            velocity,
        }) {
            Ok(()) => {
                world.entity_mut(entity).despawn();
                sent += 1;
            }
            Err(std::sync::mpsc::TrySendError::Full(_)) => {
                let mut diagnostics = world.resource_mut::<RuntimeDiagnostics>();
                diagnostics.intent_transport_full_count =
                    diagnostics.intent_transport_full_count.saturating_add(1);
                break;
            }
            Err(std::sync::mpsc::TrySendError::Disconnected(_)) => break,
        }
    }

    let mut pending_spawns: Vec<_> = {
        let mut query = world.query_filtered::<
            (Entity, &SpawnRequestId, &SpawnAssetKey, &SpawnTransform),
            Without<SpawnOutcomeRecord>,
        >();
        query
            .iter(world)
            .map(|(entity, id, key, transform)| {
                (entity, id.0, key.as_str().to_owned(), transform.matrix)
            })
            .collect()
    };
    pending_spawns.sort_by_key(|(_, request_id, _, _)| *request_id);
    for (entity, request_id, asset_key, matrix) in
        pending_spawns.into_iter().take(MAX_SPAWNS_PER_RENDER)
    {
        match endpoint.spawn_tx.try_send(SpawnIntentEnvelope {
            request_id,
            asset_key,
            matrix,
        }) {
            Ok(()) => {
                world.entity_mut(entity).despawn();
            }
            Err(std::sync::mpsc::TrySendError::Full(_)) => break,
            Err(std::sync::mpsc::TrySendError::Disconnected(_)) => break,
        }
    }
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
    rows.reserve(18);
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
    rows.push(row(
        DebugSection::Camera,
        "Rotation (view)",
        format_angles(data.view_rotation),
    ));
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
