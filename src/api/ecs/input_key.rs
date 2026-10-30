//! Stable, UI-independent key names and IDs used by scripts and console commands.

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Stable identifiers for keys observed by the mod-owned UI input backend.
///
/// IDs and lowercase names are independent of the game's keyboard API. The
/// variants remain in a fixed order because their discriminants are shared
/// with Rhai input helpers and console key bindings.
pub enum InputKey {
    /// Tab key.
    Tab = 0,
    /// Left Arrow key.
    LeftArrow = 1,
    /// Right Arrow key.
    RightArrow = 2,
    /// Up Arrow key.
    UpArrow = 3,
    /// Down Arrow key.
    DownArrow = 4,
    /// Page Up key.
    PageUp = 5,
    /// Page Down key.
    PageDown = 6,
    /// Home key.
    Home = 7,
    /// End key.
    End = 8,
    /// Insert key.
    Insert = 9,
    /// Delete key.
    Delete = 10,
    /// Backspace key.
    Backspace = 11,
    /// Space key.
    Space = 12,
    /// Enter key.
    Enter = 13,
    /// Escape key.
    Escape = 14,
    /// Left Ctrl key.
    LeftCtrl = 15,
    /// Left Shift key.
    LeftShift = 16,
    /// Left Alt key.
    LeftAlt = 17,
    /// Right Ctrl key.
    RightCtrl = 18,
    /// Right Shift key.
    RightShift = 19,
    /// Right Alt key.
    RightAlt = 20,
    /// Left Super key.
    LeftSuper = 21,
    /// Right Super key.
    RightSuper = 22,
    /// Menu key.
    Menu = 23,
    /// Digit 0 key.
    Alpha0 = 24,
    /// Digit 1 key.
    Alpha1 = 25,
    /// Digit 2 key.
    Alpha2 = 26,
    /// Digit 3 key.
    Alpha3 = 27,
    /// Digit 4 key.
    Alpha4 = 28,
    /// Digit 5 key.
    Alpha5 = 29,
    /// Digit 6 key.
    Alpha6 = 30,
    /// Digit 7 key.
    Alpha7 = 31,
    /// Digit 8 key.
    Alpha8 = 32,
    /// Digit 9 key.
    Alpha9 = 33,
    /// A key.
    A = 34,
    /// B key.
    B = 35,
    /// C key.
    C = 36,
    /// D key.
    D = 37,
    /// E key.
    E = 38,
    /// F key.
    F = 39,
    /// G key.
    G = 40,
    /// H key.
    H = 41,
    /// I key.
    I = 42,
    /// J key.
    J = 43,
    /// K key.
    K = 44,
    /// L key.
    L = 45,
    /// M key.
    M = 46,
    /// N key.
    N = 47,
    /// O key.
    O = 48,
    /// P key.
    P = 49,
    /// Q key.
    Q = 50,
    /// R key.
    R = 51,
    /// S key.
    S = 52,
    /// T key.
    T = 53,
    /// U key.
    U = 54,
    /// V key.
    V = 55,
    /// W key.
    W = 56,
    /// X key.
    X = 57,
    /// Y key.
    Y = 58,
    /// Z key.
    Z = 59,
    /// Function key F1.
    F1 = 60,
    /// Function key F2.
    F2 = 61,
    /// Function key F3.
    F3 = 62,
    /// Function key F4.
    F4 = 63,
    /// Function key F5.
    F5 = 64,
    /// Function key F6.
    F6 = 65,
    /// Function key F7.
    F7 = 66,
    /// Function key F8.
    F8 = 67,
    /// Function key F9.
    F9 = 68,
    /// Function key F10.
    F10 = 69,
    /// Function key F11.
    F11 = 70,
    /// Function key F12.
    F12 = 71,
    /// left mouse button.
    MouseLeft = 72,
    /// right mouse button.
    MouseRight = 73,
    /// middle mouse button.
    MouseMiddle = 74,
    /// extended 1 mouse button.
    MouseX1 = 75,
    /// extended 2 mouse button.
    MouseX2 = 76,
}

impl InputKey {
    const NAMES: [&'static str; 77] = [
        "tab",
        "left",
        "right",
        "up",
        "down",
        "pageup",
        "pagedown",
        "home",
        "end",
        "insert",
        "delete",
        "backspace",
        "space",
        "enter",
        "escape",
        "lctrl",
        "lshift",
        "lalt",
        "rctrl",
        "rshift",
        "ralt",
        "lsuper",
        "rsuper",
        "menu",
        "0",
        "1",
        "2",
        "3",
        "4",
        "5",
        "6",
        "7",
        "8",
        "9",
        "a",
        "b",
        "c",
        "d",
        "e",
        "f",
        "g",
        "h",
        "i",
        "j",
        "k",
        "l",
        "m",
        "n",
        "o",
        "p",
        "q",
        "r",
        "s",
        "t",
        "u",
        "v",
        "w",
        "x",
        "y",
        "z",
        "f1",
        "f2",
        "f3",
        "f4",
        "f5",
        "f6",
        "f7",
        "f8",
        "f9",
        "f10",
        "f11",
        "f12",
        "mouse1",
        "mouse2",
        "mouse3",
        "mouse4",
        "mouse5",
    ];
    /// Every supported key in stable numeric ID order.
    pub const ALL: [Self; 77] = [
        InputKey::Tab,
        InputKey::LeftArrow,
        InputKey::RightArrow,
        InputKey::UpArrow,
        InputKey::DownArrow,
        InputKey::PageUp,
        InputKey::PageDown,
        InputKey::Home,
        InputKey::End,
        InputKey::Insert,
        InputKey::Delete,
        InputKey::Backspace,
        InputKey::Space,
        InputKey::Enter,
        InputKey::Escape,
        InputKey::LeftCtrl,
        InputKey::LeftShift,
        InputKey::LeftAlt,
        InputKey::RightCtrl,
        InputKey::RightShift,
        InputKey::RightAlt,
        InputKey::LeftSuper,
        InputKey::RightSuper,
        InputKey::Menu,
        InputKey::Alpha0,
        InputKey::Alpha1,
        InputKey::Alpha2,
        InputKey::Alpha3,
        InputKey::Alpha4,
        InputKey::Alpha5,
        InputKey::Alpha6,
        InputKey::Alpha7,
        InputKey::Alpha8,
        InputKey::Alpha9,
        InputKey::A,
        InputKey::B,
        InputKey::C,
        InputKey::D,
        InputKey::E,
        InputKey::F,
        InputKey::G,
        InputKey::H,
        InputKey::I,
        InputKey::J,
        InputKey::K,
        InputKey::L,
        InputKey::M,
        InputKey::N,
        InputKey::O,
        InputKey::P,
        InputKey::Q,
        InputKey::R,
        InputKey::S,
        InputKey::T,
        InputKey::U,
        InputKey::V,
        InputKey::W,
        InputKey::X,
        InputKey::Y,
        InputKey::Z,
        InputKey::F1,
        InputKey::F2,
        InputKey::F3,
        InputKey::F4,
        InputKey::F5,
        InputKey::F6,
        InputKey::F7,
        InputKey::F8,
        InputKey::F9,
        InputKey::F10,
        InputKey::F11,
        InputKey::F12,
        InputKey::MouseLeft,
        InputKey::MouseRight,
        InputKey::MouseMiddle,
        InputKey::MouseX1,
        InputKey::MouseX2,
    ];

    /// Parse a supported name case-insensitively.
    pub fn parse(name: &str) -> Option<Self> {
        let name = name.to_ascii_lowercase();
        Self::NAMES
            .iter()
            .position(|candidate| *candidate == name)
            .and_then(|index| Self::from_id(index as u32))
    }

    /// Resolve a stable numeric key ID.
    pub fn from_id(id: u32) -> Option<Self> {
        Self::ALL
            .get(id as usize)
            .copied()
            .filter(|key| *key as u32 == id)
    }

    /// Return the stable numeric ID shared by runtime input consumers.
    pub const fn id(self) -> u32 {
        self as u32
    }

    /// Return the canonical lowercase name used by Rhai and console commands.
    pub const fn name(self) -> &'static str {
        Self::NAMES[self as usize]
    }
}
