use crate::game::character::Character;
use crate::game::status::{StatEffectTarget, StatusEffect};

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

    #[allow(dead_code)] // reserved for status-effect ticking, out-of-combat rest, etc.
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

    /// Called once per concluded encounter (victory or successful flee) to count
    /// down and expire effects whose duration has run out.
    pub fn tick_effects(&mut self) {
        for e in &mut self.effects {
            e.encounters_remaining = e.encounters_remaining.saturating_sub(1);
        }
        self.effects.retain(|e| e.encounters_remaining > 0);
    }
}
