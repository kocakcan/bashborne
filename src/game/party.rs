use serde::{Deserialize, Serialize};

use crate::game::character::Character;
use crate::game::status::{StatEffectTarget, StatusEffect};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Party {
    pub members: Vec<Character>,
    pub gold: u32,
    pub effects: Vec<StatusEffect>,
}

impl Party {
    pub fn new(members: Vec<Character>) -> Self {
        Self {
            members,
            gold: 50,
            effects: Vec::new(),
        }
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
            existing.encounters_remaining =
                existing.encounters_remaining.max(effect.encounters_remaining);
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
    use crate::game::character::warrior;

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
}
