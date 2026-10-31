//! ECS schedule construction, first-party systems, and registration.
pub(crate) mod registration;
pub mod schedule;
pub mod schedule_labels;
pub(crate) mod script_dispatch;

pub(crate) use registration::{register_game_systems, register_ui_systems};
