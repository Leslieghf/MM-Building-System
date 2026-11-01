//! Copied frame snapshots and display rows shared by UI presentation systems.
//!
//! Snapshot resources are replaced with owned values received from the game
//! endpoint. Display rows are derived for rendering and carry no authority to
//! read or mutate game state.
use crate::api::ecs::{GameSnapshot, input::InputFrame};
use bevy_ecs::prelude::Resource;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugSection {
    Player,
    Camera,
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

#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct OverlayFrameTimings {
    pub sync_ms: f64,
    pub input_ms: f64,
    pub debug_hud_ms: f64,
    pub script_panels_ms: f64,
    pub dev_tools_ms: f64,
    pub spawn_menu_ms: f64,
    pub total_ms: f64,
}

#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct UiGameSnapshot(pub GameSnapshot);

#[derive(Resource, Debug, Default)]
pub struct ScriptPanelsUiState {
    pub publications: Vec<crate::api::scripts::ScriptPanelPublication>,
    pub open: BTreeMap<(crate::api::scripts::ScriptId, String), bool>,
}

impl ScriptPanelsUiState {
    pub fn replace(&mut self, publications: Vec<crate::api::scripts::ScriptPanelPublication>) {
        let active_scripts: std::collections::BTreeSet<_> = publications
            .iter()
            .map(|publication| publication.script_id.clone())
            .collect();
        for publication in &publications {
            for panel in &publication.panels {
                let key = (publication.script_id.clone(), panel.id.clone());
                self.open.entry(key).or_insert(true);
            }
        }
        self.open.retain(|key, _| active_scripts.contains(&key.0));
        self.publications = publications;
    }

    pub fn reopen(&mut self, script_id: &crate::api::scripts::ScriptId, panel_id: &str) {
        if let Some(open) = self.open.get_mut(&(script_id.clone(), panel_id.to_owned())) {
            *open = true;
        }
    }
}

#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct PendingUiInput(pub InputFrame);
