//! Game-state discriminants declared by the pinned `game/game.h`.
//!
//! The enum records source values for layout and decoding work; it is not a
//! live process-state reader. The accompanying function alias has no resolved
//! address or validated call path and must not be invoked directly.

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Game lifecycle states and their SDK discriminants.
pub enum GameState {
    /// Installation state.
    Install = 0,
    /// Initialization state.
    Init = 1,
    /// Content loading state.
    Load = 2,
    /// Active gameplay state.
    Run = 3,
    /// Startup transition state.
    Startup = 4,
}

#[cfg(test)]
mod tests {
    use super::GameState;

    #[test]
    fn game_state_discriminants_match_the_sdk_header() {
        assert_eq!(GameState::Install as i32, 0);
        assert_eq!(GameState::Init as i32, 1);
        assert_eq!(GameState::Load as i32, 2);
        assert_eq!(GameState::Run as i32, 3);
        assert_eq!(GameState::Startup as i32, 4);
    }
}
