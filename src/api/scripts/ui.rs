//! Owned script-authored UI values that cross between the game and render worlds.
//!
//! No Rhai values, ImGui handles, closures, borrowed game data, or native
//! pointers are allowed in these records.

use super::{ScriptGeneration, ScriptId, package::WorkbenchTabMetadata};

pub const MAX_PANELS_PER_SCRIPT: usize = 4;
pub const MAX_WIDGETS_PER_PANEL: usize = 48;
pub const MAX_PANEL_TEXT_BYTES: usize = 160;
pub const MAX_PANEL_OPTIONS: usize = 16;
pub const PANEL_PUBLICATION_QUEUE_CAPACITY: usize = 8;
pub const PANEL_EVENT_QUEUE_CAPACITY: usize = 4096;

#[derive(Debug, Clone, PartialEq)]
pub struct ScriptPanelPublication {
    pub script_id: ScriptId,
    pub generation: ScriptGeneration,
    pub workbench_tab: Option<WorkbenchTabMetadata>,
    pub panels: Vec<ScriptPanelModel>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScriptPanelModel {
    pub id: String,
    pub title: String,
    pub widgets: Vec<ScriptPanelWidget>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScriptPanelWidget {
    pub id: String,
    pub kind: ScriptPanelWidgetKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptStatusLevel {
    Stable,
    Problematic,
    Experimental,
}

impl ScriptStatusLevel {
    pub const fn color(self) -> [f32; 4] {
        match self {
            Self::Stable => [0.32, 0.82, 0.48, 1.0],
            Self::Problematic => [0.94, 0.36, 0.34, 1.0],
            Self::Experimental => [0.96, 0.74, 0.26, 1.0],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScriptPanelWidgetKind {
    Heading {
        text: String,
    },
    Label {
        text: String,
    },
    Status {
        text: String,
        level: ScriptStatusLevel,
    },
    Checkbox {
        label: String,
        value: bool,
    },
    Slider {
        label: String,
        value: f64,
        min: f64,
        max: f64,
    },
    Number {
        label: String,
        value: f64,
        min: f64,
        max: f64,
        step: f64,
    },
    Select {
        label: String,
        value: String,
        options: Vec<String>,
    },
    Button {
        label: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScriptPanelEvent {
    pub script_id: ScriptId,
    pub generation: ScriptGeneration,
    pub panel_id: String,
    pub widget_id: String,
    pub value: ScriptPanelValue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScriptPanelValue {
    Bool(bool),
    Number(f64),
    Text(String),
}
