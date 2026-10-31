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
    append_reticle_query_rows(&data, rows);
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

fn append_reticle_query_rows(data: &GameSnapshot, rows: &mut Vec<PresentationRow>) {
    rows.push(row(
        DebugSection::Raycast,
        "Hit position",
        format_vector(data.raycast_hit_position),
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

#[cfg(test)]
mod raycast_filter_comparison_tests {
    use super::*;

    #[test]
    fn raycast_section_displays_only_the_latest_hit_position() {
        let snapshot = GameSnapshot {
            raycast_hit_position: Some([4.0, 5.0, 6.0]),
            ..GameSnapshot::default()
        };
        let mut rows = Vec::new();

        append_reticle_query_rows(&snapshot, &mut rows);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].label, "Hit position");
        assert_eq!(rows[0].value, "4.000, 5.000, 6.000");
    }
}
