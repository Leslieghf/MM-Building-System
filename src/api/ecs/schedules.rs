//! Game-side Bevy schedule setup and projection of snapshots into mod-facing entities.
use super::{
    CurrentGameFrame, CurrentPlayerVehicleProxy, GameSnapshot, MadMaxPostSim, MadMaxPostSimSet,
    PlayerCharacterProxy, PlayerFrameData, RuntimeDiagnostics, VehicleFrameData,
};
use bevy_app::App;
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{IntoScheduleConfigs, Schedule, SingleThreadedExecutor};

/// Create the single-threaded [`App`] and its ordered game callback schedule.
///
/// The returned app owns copied frame resources and mod-facing entities. Native
/// pointers and game calls remain outside this schedule.
///
pub fn create_game_app() -> App {
    let mut app = App::empty();
    let mut schedule = Schedule::new(MadMaxPostSim);
    schedule.set_executor(SingleThreadedExecutor::new());
    app.add_schedule(schedule);
    app.configure_sets(
        MadMaxPostSim,
        (
            MadMaxPostSimSet::Import,
            MadMaxPostSimSet::ModSystems,
            MadMaxPostSimSet::Export,
        )
            .chain(),
    );
    app.init_resource::<CurrentGameFrame>();
    app.init_resource::<crate::api::ecs::input::InputFrame>();
    app.init_resource::<crate::devtools::model::PendingUiInput>();
    app.init_resource::<crate::api::scripts::manager::ScriptManager>();
    app.init_resource::<crate::api::ecs::script_dispatch::PendingScriptDebugDraws>();
    app.init_resource::<RuntimeDiagnostics>();
    app.add_systems(
        MadMaxPostSim,
        project_frame_to_proxies.in_set(MadMaxPostSimSet::Import),
    );

    app.world_mut()
        .spawn((PlayerCharacterProxy, PlayerFrameData::default()));
    app.world_mut()
        .spawn((CurrentPlayerVehicleProxy, VehicleFrameData::default()));
    app
}

fn project_frame_to_proxies(
    frame: Res<CurrentGameFrame>,
    mut players: Query<&mut PlayerFrameData, With<PlayerCharacterProxy>>,
    mut vehicles: Query<&mut VehicleFrameData, With<CurrentPlayerVehicleProxy>>,
) {
    let snapshot = frame.0;
    for mut player in &mut players {
        *player = PlayerFrameData {
            position: snapshot.player_position,
            forward: snapshot.player_forward,
            transform: snapshot.player_transform,
            rotation: snapshot.player_rotation,
            view_rotation: snapshot.view_rotation,
            velocity: snapshot.player_velocity,
            acceleration: snapshot.player_acceleration,
        };
    }
    for mut vehicle in &mut vehicles {
        *vehicle = VehicleFrameData {
            position: snapshot.vehicle_position,
            forward: snapshot.vehicle_forward,
            transform: snapshot.vehicle_transform,
            rotation: snapshot.vehicle_rotation,
            velocity: snapshot.vehicle_velocity,
            acceleration: snapshot.vehicle_acceleration,
            angular_velocity: snapshot.vehicle_angular_velocity,
            velocity_last_frame: snapshot.vehicle_velocity_last_frame,
        };
    }
}

/// Replace the current frame resource with an owned [`GameSnapshot`].
///
/// Systems observe the replacement when the schedule next runs.
pub fn update_game_frame(app: &mut App, snapshot: GameSnapshot) {
    app.world_mut().insert_resource(CurrentGameFrame(snapshot));
}
