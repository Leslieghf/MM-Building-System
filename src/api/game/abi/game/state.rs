//! Game-state enum declarations from `game/game.h`.

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    Install = 0,
    Init = 1,
    Load = 2,
    Run = 3,
    Startup = 4,
}

/// Candidate signature of `CGame::IsGuiOccludingMainDraw()`.
pub type IsGuiOccludingMainDrawFn = unsafe extern "system" fn() -> bool;

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
