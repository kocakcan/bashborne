use std::fmt;

use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusEffect {
    pub name: String,
    pub target: StatEffectTarget,
    pub delta: i32,
    pub encounters_remaining: u32,
}

/// Curated pool so blessings/curses read as flavorful events rather than a
/// generic "+N stat" toast. The last entry (`min_ng_plus`) tier-gates the
/// stronger options: 4 are always available, a 5th unlocks at NG+1, and a
/// 6th at NG+2 — so New Game+ genuinely offers new blessings, not just
/// bigger monster stats.
type BlessingOption = (&'static str, StatEffectTarget, i32, u32);

const BLESSING_OPTIONS: [BlessingOption; 6] = [
    ("Warrior's Blessing", StatEffectTarget::Attack, 5, 0),
    ("Blessing of Swiftness", StatEffectTarget::Speed, 4, 0),
    ("Blessing of Iron Skin", StatEffectTarget::Defense, 5, 0),
    ("Blessing of the Berserker", StatEffectTarget::Attack, 3, 0),
    ("Blessing of the Ancients", StatEffectTarget::Attack, 8, 1),
    ("Blessing of Transcendence", StatEffectTarget::Speed, 7, 2),
];

const CURSE_OPTIONS: [BlessingOption; 6] = [
    ("Curse of Frailty", StatEffectTarget::Defense, -4, 0),
    ("Curse of Sloth", StatEffectTarget::Speed, -4, 0),
    ("Curse of Weakness", StatEffectTarget::Attack, -4, 0),
    ("Curse of Brittleness", StatEffectTarget::Defense, -3, 0),
    ("Curse of Ruin", StatEffectTarget::Attack, -7, 1),
    ("Curse of the Abyss", StatEffectTarget::Defense, -7, 2),
];

fn roll_from(options: &[BlessingOption], ng_plus: u32, rng: &mut impl Rng) -> StatusEffect {
    let available: Vec<&BlessingOption> =
        options.iter().filter(|(_, _, _, tier)| *tier <= ng_plus).collect();
    let &(name, target, delta, _) = available[rng.gen_range(0..available.len())];
    StatusEffect {
        name: name.to_string(),
        target,
        delta,
        encounters_remaining: 2,
    }
}

pub fn roll_blessing(rng: &mut impl Rng, ng_plus: u32) -> StatusEffect {
    roll_from(&BLESSING_OPTIONS, ng_plus, rng)
}

pub fn roll_curse(rng: &mut impl Rng, ng_plus: u32) -> StatusEffect {
    roll_from(&CURSE_OPTIONS, ng_plus, rng)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn every_blessing_delta_is_positive_and_every_curse_delta_is_negative() {
        for seed in 0..100u64 {
            let blessing = roll_blessing(&mut StdRng::seed_from_u64(seed), 2);
            assert!(blessing.delta > 0, "{}", blessing.name);
            let curse = roll_curse(&mut StdRng::seed_from_u64(seed), 2);
            assert!(curse.delta < 0, "{}", curse.name);
        }
    }

    #[test]
    fn tier_gated_blessing_is_absent_below_its_unlock_tier_and_present_at_it() {
        let never_seen_below = (0..200)
            .all(|seed| roll_blessing(&mut StdRng::seed_from_u64(seed), 0).name != "Blessing of the Ancients");
        assert!(
            never_seen_below,
            "the NG+1 blessing shouldn't appear at NG+0"
        );

        let seen_at_tier = (0..200)
            .any(|seed| roll_blessing(&mut StdRng::seed_from_u64(seed), 1).name == "Blessing of the Ancients");
        assert!(seen_at_tier, "the NG+1 blessing should appear once unlocked");
    }

    #[test]
    fn tier_gated_curse_is_absent_below_its_unlock_tier_and_present_at_it() {
        let never_seen_below = (0..200)
            .all(|seed| roll_curse(&mut StdRng::seed_from_u64(seed), 1).name != "Curse of the Abyss");
        assert!(never_seen_below, "the NG+2 curse shouldn't appear before NG+2");

        let seen_at_tier = (0..200)
            .any(|seed| roll_curse(&mut StdRng::seed_from_u64(seed), 2).name == "Curse of the Abyss");
        assert!(seen_at_tier, "the NG+2 curse should appear once unlocked");
    }
}
