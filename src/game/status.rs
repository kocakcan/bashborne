use std::fmt;

use rand::Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatEffectTarget {
    Attack,
    Defense,
    Speed,
}

impl fmt::Display for StatEffectTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            StatEffectTarget::Attack => "Attack",
            StatEffectTarget::Defense => "Defense",
            StatEffectTarget::Speed => "Speed",
        };
        write!(f, "{s}")
    }
}

/// A party-wide buff (positive delta) or curse (negative delta) that lasts a fixed
/// number of *encounters* (not turns) — it survives until the party has fought
/// through (or fled) that many battles, then wears off.
#[derive(Debug, Clone)]
pub struct StatusEffect {
    pub name: String,
    pub target: StatEffectTarget,
    pub delta: i32,
    pub encounters_remaining: u32,
}

/// Small curated pool so blessings/curses read as flavorful events rather than
/// a generic "+N stat" toast. Add more entries here to expand variety.
pub fn roll_blessing(rng: &mut impl Rng) -> StatusEffect {
    let options: [(&str, StatEffectTarget, i32); 2] = [
        ("Warrior's Blessing", StatEffectTarget::Attack, 5),
        ("Blessing of Swiftness", StatEffectTarget::Speed, 4),
    ];
    let (name, target, delta) = options[rng.gen_range(0..options.len())];
    StatusEffect {
        name: name.to_string(),
        target,
        delta,
        encounters_remaining: 2,
    }
}

pub fn roll_curse(rng: &mut impl Rng) -> StatusEffect {
    let options: [(&str, StatEffectTarget, i32); 2] = [
        ("Curse of Frailty", StatEffectTarget::Defense, -4),
        ("Curse of Sloth", StatEffectTarget::Speed, -4),
    ];
    let (name, target, delta) = options[rng.gen_range(0..options.len())];
    StatusEffect {
        name: name.to_string(),
        target,
        delta,
        encounters_remaining: 2,
    }
}
