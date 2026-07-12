use serde::{Deserialize, Serialize};

use crate::game::character::Character;
use crate::game::status::{StatEffectTarget, StatusEffect};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Party {
    pub members: Vec<Character>,
    pub gold: u32,
    pub effects: Vec<StatusEffect>,
    /// Recruited characters not currently in the active `members` roster.
    /// Combat/rendering only ever look at `members`, so the bench is purely
    /// a holding area — see `Party::swap` to bring someone in from it.
    /// Defaults to empty for saves predating recruitable NPCs.
    #[serde(default)]
    pub bench: Vec<Character>,
}

impl Party {
    pub fn new(members: Vec<Character>) -> Self {
        Self {
            members,
            gold: 50,
            effects: Vec::new(),
            bench: Vec::new(),
        }
    }

    /// Adds a newly recruited character to the bench. Recruits always land
    /// here rather than being auto-inserted into the active roster, so
    /// which four (or fewer) members are actually fighting stays a
    /// deliberate player choice made via `swap`.
    pub fn recruit(&mut self, character: Character) {
        self.bench.push(character);
    }

    /// Swaps an active member out for a benched one, in place. Returns
    /// whether the swap happened — `false` (a no-op) if either index is out
    /// of range, e.g. the bench is empty.
    pub fn swap(&mut self, active_idx: usize, bench_idx: usize) -> bool {
        if active_idx >= self.members.len() || bench_idx >= self.bench.len() {
            return false;
        }
        std::mem::swap(&mut self.members[active_idx], &mut self.bench[bench_idx]);
        true
    }

    pub fn alive_members(&self) -> impl Iterator<Item = &Character> {
        self.members.iter().filter(|c| c.is_alive())
    }

    pub fn alive_members_mut(&mut self) -> impl Iterator<Item = &mut Character> {
        self.members.iter_mut().filter(|c| c.is_alive())
    }

    pub fn is_wiped(&self) -> bool {
        self.alive_members().count() == 0
    }

    /// Mean level across all party members, rounded — the input to
    /// `Character::scale_boss_to_party`, so a boss reacts to the whole
    /// roster's progress rather than just whoever happens to be lowest/highest.
    pub fn average_level(&self) -> u32 {
        if self.members.is_empty() {
            return 0;
        }
        let total: u32 = self.members.iter().map(|m| m.level).sum();
        ((total as f64 / self.members.len() as f64).round()) as u32
    }

    /// Net bonus (positive) or penalty (negative) currently active for a given stat,
    /// summed across all active blessings/curses.
    pub fn stat_delta(&self, target: StatEffectTarget) -> i32 {
        self.effects
            .iter()
            .filter(|e| e.target == target)
            .map(|e| e.delta)
            .sum()
    }

    /// Adds a new buff/curse. If an effect with the same name is already
    /// active, its magnitude stacks and its duration refreshes to whichever
    /// is longer, rather than cluttering the list with duplicate entries.
    pub fn add_effect(&mut self, effect: StatusEffect) {
        if let Some(existing) = self.effects.iter_mut().find(|e| e.name == effect.name) {
            existing.delta += effect.delta;
            existing.encounters_remaining = existing
                .encounters_remaining
                .max(effect.encounters_remaining);
        } else {
            self.effects.push(effect);
        }
    }

    /// Strips every active curse (negative-delta effect), leaving blessings
    /// untouched. Returns how many were lifted — the Purging Stone's effect.
    pub fn cure_curses(&mut self) -> usize {
        let before = self.effects.len();
        self.effects.retain(|e| e.delta >= 0);
        before - self.effects.len()
    }

    /// Called once per concluded encounter (victory or successful flee) to count
    /// down and expire effects whose duration has run out.
    pub fn tick_effects(&mut self) {
        for e in &mut self.effects {
            e.encounters_remaining = e.encounters_remaining.saturating_sub(1);
        }
        self.effects.retain(|e| e.encounters_remaining > 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::character::{rogue, warrior};

    #[test]
    fn recruit_always_lands_on_the_bench() {
        let mut party = Party::new(vec![warrior("Bram")]);
        party.recruit(rogue("Wren"));
        assert_eq!(party.members.len(), 1, "the active roster is untouched");
        assert_eq!(party.bench.len(), 1);
        assert_eq!(party.bench[0].name, "Wren");
    }

    #[test]
    fn swap_exchanges_an_active_and_benched_member() {
        let mut party = Party::new(vec![warrior("Bram")]);
        party.recruit(rogue("Wren"));

        assert!(party.swap(0, 0));
        assert_eq!(party.members[0].name, "Wren", "the recruit is now active");
        assert_eq!(party.bench[0].name, "Bram", "the displaced member lands on the bench");
    }

    #[test]
    fn swap_with_an_out_of_range_index_is_a_no_op() {
        let mut party = Party::new(vec![warrior("Bram")]);
        assert!(!party.swap(0, 0), "the bench is empty");
        assert!(!party.swap(1, 0), "no active member at index 1");
        assert_eq!(party.members[0].name, "Bram");
    }

    #[test]
    fn cure_curses_lifts_only_the_negative_effects() {
        let mut party = Party::new(vec![warrior("Bram")]);
        party.add_effect(StatusEffect {
            name: "Warrior's Blessing".into(),
            target: StatEffectTarget::Attack,
            delta: 5,
            encounters_remaining: 2,
        });
        party.add_effect(StatusEffect {
            name: "Curse of Frailty".into(),
            target: StatEffectTarget::Defense,
            delta: -4,
            encounters_remaining: 2,
        });
        assert_eq!(party.cure_curses(), 1);
        assert_eq!(party.effects.len(), 1);
        assert_eq!(party.effects[0].name, "Warrior's Blessing");
        // A second stone finds nothing left to purge.
        assert_eq!(party.cure_curses(), 0);
    }

    #[test]
    fn average_level_rounds_to_the_nearest_whole_level() {
        let mut party = Party::new(vec![warrior("Bram"), warrior("Elle"), warrior("Rook")]);
        party.members[0].level = 1;
        party.members[1].level = 2;
        party.members[2].level = 2;
        assert_eq!(party.average_level(), 2); // mean 1.667 rounds to 2

        party.members[0].level = 1;
        party.members[1].level = 1;
        party.members[2].level = 2;
        assert_eq!(party.average_level(), 1); // mean 1.333 rounds to 1
    }
}
