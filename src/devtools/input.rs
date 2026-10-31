//! Translate the stable mod-owned input snapshot to and from ImGui key IDs.
//!
//! The mapping isolates the public [`crate::api::ecs::input_key::InputKey`]
//! identifiers from backend-specific ImGui enums. Input capture is owned by
//! the overlay thread; the resulting snapshot is copied for game and script
//! consumers. Input suppression is a separate overlay-capture policy and is
//! not performed by this mapping layer.
use crate::api::ecs::input_key::InputKey;
use hudhook::imgui;

const IMGUI_KEYS: [imgui::Key; 77] = [
    imgui::Key::Tab,
    imgui::Key::LeftArrow,
    imgui::Key::RightArrow,
    imgui::Key::UpArrow,
    imgui::Key::DownArrow,
    imgui::Key::PageUp,
    imgui::Key::PageDown,
    imgui::Key::Home,
    imgui::Key::End,
    imgui::Key::Insert,
    imgui::Key::Delete,
    imgui::Key::Backspace,
    imgui::Key::Space,
    imgui::Key::Enter,
    imgui::Key::Escape,
    imgui::Key::LeftCtrl,
    imgui::Key::LeftShift,
    imgui::Key::LeftAlt,
    imgui::Key::RightCtrl,
    imgui::Key::RightShift,
    imgui::Key::RightAlt,
    imgui::Key::LeftSuper,
    imgui::Key::RightSuper,
    imgui::Key::Menu,
    imgui::Key::Alpha0,
    imgui::Key::Alpha1,
    imgui::Key::Alpha2,
    imgui::Key::Alpha3,
    imgui::Key::Alpha4,
    imgui::Key::Alpha5,
    imgui::Key::Alpha6,
    imgui::Key::Alpha7,
    imgui::Key::Alpha8,
    imgui::Key::Alpha9,
    imgui::Key::A,
    imgui::Key::B,
    imgui::Key::C,
    imgui::Key::D,
    imgui::Key::E,
    imgui::Key::F,
    imgui::Key::G,
    imgui::Key::H,
    imgui::Key::I,
    imgui::Key::J,
    imgui::Key::K,
    imgui::Key::L,
    imgui::Key::M,
    imgui::Key::N,
    imgui::Key::O,
    imgui::Key::P,
    imgui::Key::Q,
    imgui::Key::R,
    imgui::Key::S,
    imgui::Key::T,
    imgui::Key::U,
    imgui::Key::V,
    imgui::Key::W,
    imgui::Key::X,
    imgui::Key::Y,
    imgui::Key::Z,
    imgui::Key::F1,
    imgui::Key::F2,
    imgui::Key::F3,
    imgui::Key::F4,
    imgui::Key::F5,
    imgui::Key::F6,
    imgui::Key::F7,
    imgui::Key::F8,
    imgui::Key::F9,
    imgui::Key::F10,
    imgui::Key::F11,
    imgui::Key::F12,
    imgui::Key::MouseLeft,
    imgui::Key::MouseRight,
    imgui::Key::MouseMiddle,
    imgui::Key::MouseX1,
    imgui::Key::MouseX2,
];

pub(crate) fn from_imgui(key: imgui::Key) -> Option<InputKey> {
    InputKey::ALL
        .iter()
        .copied()
        .find(|candidate| to_imgui(*candidate) == key)
}

pub(crate) fn to_imgui(key: InputKey) -> imgui::Key {
    IMGUI_KEYS[key.id() as usize]
}
