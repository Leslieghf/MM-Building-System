//! Snapshot and display rows shared by developer-tool presentation systems.
use crate::api::ecs::{GameSnapshot, input::InputFrame};
use bevy_ecs::prelude::Resource;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugSection {
    Player,
    Camera,
    Raycast,
    Vehicle,
    Runtime,
}

#[derive(Debug, Clone)]
pub struct PresentationRow {
    pub section: DebugSection,
    pub label: &'static str,
    pub value: String,
}

#[derive(Resource, Debug, Default)]
pub struct UiPresentation(pub Vec<PresentationRow>);

#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct UiGameSnapshot(pub GameSnapshot);

#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct PendingUiInput(pub InputFrame);
