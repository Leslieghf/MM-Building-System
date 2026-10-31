//! Labels and ordered sets for the host-driven ECS schedules.
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ScheduleLabel;

#[derive(ScheduleLabel, Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Game phase executed at the host's accepted post-simulation callback.
pub struct MadMaxPostSim;

#[derive(ScheduleLabel, Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Mod-owned render/presentation phase executed from the render hook.
pub struct MadMaxRender;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Ordered sets within [`MadMaxPostSim`].
pub enum MadMaxPostSimSet {
    /// Import current frame data and pending requests.
    Import,
    /// Run compiled Rust systems and managed script callbacks.
    ModSystems,
    /// Validate and export bounded effects and outcomes.
    Export,
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Ordered sets within [`MadMaxRender`].
pub enum MadMaxRenderSet {
    /// Run mod-owned UI preparation systems.
    ModSystems,
    /// Build and present developer-tools UI.
    Presentation,
}
