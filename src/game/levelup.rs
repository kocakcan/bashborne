use crate::game::character::AllocStat;
use crate::game::map::Position;

/// The level-up / point-allocation screen: pick a party member, pick a
/// stat, spend a banked point on it. Reachable any time from Explore via
/// `'u'`, not just right after a level-up, so the player can check stats
/// even with nothing to spend.
pub struct LevelUpUiState {
    pub member_cursor: usize,
    pub stat_cursor: usize,
    pub message: Option<String>,
    /// Where to place the player back on the map once this screen is closed.
    pub return_pos: Position,
    /// Points spent during this visit to the screen, in spend order, so
    /// Backspace can undo them one at a time via `Character::deallocate_point`.
    /// Reset every time the screen is (re)opened — undo is scoped to a single
    /// session, not persisted.
    pub history: Vec<(usize, AllocStat, i32)>,
}

impl LevelUpUiState {
    pub fn new(return_pos: Position) -> Self {
        Self {
            member_cursor: 0,
            stat_cursor: 0,
            message: None,
            return_pos,
            history: Vec::new(),
        }
    }
}
