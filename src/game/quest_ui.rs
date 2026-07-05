use crate::game::map::Position;

/// Read-only quest log screen: browses active and completed quests.
pub struct QuestLogUiState {
    pub cursor: usize,
    /// Where to place the player back on the map once this screen is closed.
    pub return_pos: Position,
}

impl QuestLogUiState {
    pub fn new(return_pos: Position) -> Self {
        Self {
            cursor: 0,
            return_pos,
        }
    }
}
