use std::fmt;

use serde::{Deserialize, Serialize};

use crate::game::chapter::BossKind;
use crate::game::item::{
    acolytes_mace, apprentice_wand, thieves_dirk, worn_shortsword, Armor, Ring, Weapon,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Stats {
    pub max_hp: i32,
    pub hp: i32,
    pub max_mp: i32,
    pub mp: i32,
    pub attack: i32,
    pub defense: i32,
    pub speed: i32,
    /// Boosts critical-hit chance — see `combat::crit_chance`.
    pub luck: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Class {
    Warrior,
    Mage,
    Rogue,
    Cleric,
    Monster,
}

impl fmt::Display for Class {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Class::Warrior => "Warrior",
            Class::Mage => "Mage",
            Class::Rogue => "Rogue",
            Class::Cleric => "Cleric",
            Class::Monster => "Monster",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AbilityKind {
    PhysicalDamage,
    MagicDamage,
    Heal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ability {
    pub name: String,
    pub mp_cost: i32,
    /// The ability's flat power before the caster's own stats are folded
    /// in — see `effective_power`, which is what combat actually uses.
    pub base_power: i32,
    pub kind: AbilityKind,
    /// Whether this hits every living enemy instead of one chosen target.
    pub targets_all_enemies: bool,
}

impl Ability {
    /// This ability's actual power once the caster's Attack (AP) stat is
    /// folded in, for damage abilities and heals alike — the game has no
    /// separate "magic power" stat, so AP is what all abilities scale from.
    pub fn effective_power(&self, caster: &Character) -> i32 {
        self.base_power + caster.total_attack() / 2
    }
}

/// Which of a character's two ring slots is being addressed. Two slots
/// (rather than a bare index) so equip/unequip call sites read as intent,
/// not magic numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RingSlot {
    First,
    Second,
}

/// A stat a level-up point can be spent on — see `Character::allocate_point`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocStat {
    MaxHp,
    MaxMp,
    Attack,
    Defense,
    Speed,
    Luck,
}

pub const ALLOC_STATS: [AllocStat; 6] = [
    AllocStat::MaxHp,
    AllocStat::MaxMp,
    AllocStat::Attack,
    AllocStat::Defense,
    AllocStat::Speed,
    AllocStat::Luck,
];

impl fmt::Display for AllocStat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            AllocStat::MaxHp => "Max HP",
            AllocStat::MaxMp => "Max MP",
            AllocStat::Attack => "Attack",
            AllocStat::Defense => "Defense",
            AllocStat::Speed => "Speed",
            AllocStat::Luck => "Luck",
        };
        write!(f, "{s}")
    }
}

/// Stat points banked per level-up, to be spent via `Character::allocate_point`.
pub const POINTS_PER_LEVEL: u32 = 3;

/// The highest level a party member can reach via `Character::gain_xp`.
/// Must sit at or above 25 so `growth_multiplier`'s taper (full through
/// level 10, floored at 0.5x by level 25) actually reaches its floor before
/// play stops, rather than capping mid-taper. 40 leaves headroom past that
/// floor — well above the highest level the game's own difficulty curve
/// ever references (boss NG+ scaling tops out at a party average of 34) —
/// for hand-allocated-point progression to keep meaning something after
/// automatic growth bottoms out. Monster/boss scaling (`scale_to_level`) is
/// a separate, already-bounded path and is intentionally not capped here.
pub const MAX_LEVEL: u32 = 40;

/// How one invested rank behaves for one stat: the full-value gain it
/// converts into, the soft cap (in *ranks*) past which the gain is halved
/// (rounded up, floored at +1 — see `AllocPreview::Diminished`), and the hard
/// cap (in ranks) past which the stat refuses to grow at all
/// (`AllocPreview::Capped`). Crucially, `soft_cap`/`hard_cap` bound
/// `Character::alloc_ranks` — the number of ranks invested — not the derived
/// stat value itself (`stats.attack` etc.), which is `gain`-per-rank and
/// therefore differs by class even at the same rank. The hard cap is
/// enforced against *every* source of growth, not just hand-spent points —
/// see `Character::invest_rank`, the single chokepoint both automatic
/// `level_growth` and `allocate_point_tracked` go through — so a stat can
/// never sneak past its cap via leveling alone while allocation still thinks
/// there's headroom.
#[derive(Debug, Clone, Copy)]
pub struct AllocRule {
    pub gain: i32,
    pub soft_cap: i32,
    pub hard_cap: i32,
}

/// Per-class allocation payoff, one `AllocRule` per spendable stat. This is
/// where class identity in point-spending lives: the Warrior is the premier
/// HP investment, the Mage and Rogue get the best attack-per-point.
#[derive(Debug, Clone, Copy)]
pub struct AllocProfile {
    pub max_hp: AllocRule,
    pub max_mp: AllocRule,
    pub attack: AllocRule,
    pub defense: AllocRule,
    pub speed: AllocRule,
    pub luck: AllocRule,
}

impl AllocProfile {
    pub fn rule(&self, stat: AllocStat) -> AllocRule {
        match stat {
            AllocStat::MaxHp => self.max_hp,
            AllocStat::MaxMp => self.max_mp,
            AllocStat::Attack => self.attack,
            AllocStat::Defense => self.defense,
            AllocStat::Speed => self.speed,
            AllocStat::Luck => self.luck,
        }
    }
}

/// Shorthand for the tables below.
const fn rule(gain: i32, soft_cap: i32, hard_cap: i32) -> AllocRule {
    AllocRule { gain, soft_cap, hard_cap }
}

/// Every stat a playable class can allocate into has its *invested rank*
/// hard-capped here — the same ceiling for every class and every stat, so
/// "50" is a uniform cap on how much you've invested, not on what that
/// investment is worth in actual HP/MP/Attack/etc. (that's `gain`, which
/// differs by class — see `AllocRule`'s doc comment for why this is enforced
/// against automatic growth too, not just spent points).
pub const PLAYER_HARD_CAP: i32 = 50;

/// Monster/boss automatic growth (`Character::scale_to_level`) is a
/// percentage of the species' *own* level-1 base stat per level, not a flat
/// shared table — so a tougher species stays proportionally tougher instead
/// of every species converging toward the same numbers at high levels.
/// Speed/luck don't grow this way; NG+'s minor multiplier moves those instead.
const MONSTER_HP_GROWTH_RATE: f32 = 0.10;
const MONSTER_ATTACK_GROWTH_RATE: f32 = 0.03;
// Higher than HP/attack's rates relative to typical base size, since the
// weakest species' defense (as low as 2) needs enough rate to cross a whole
// point within a handful of levels rather than getting stuck at 0 growth
// forever under `growth_carry`'s fractional accumulation.
const MONSTER_DEFENSE_GROWTH_RATE: f32 = 0.09;

/// Each class's allocation table — this is where class identity in
/// point-spending lives (the Warrior is the premier HP investment, the Mage
/// and Rogue get the best attack-per-point), via `gain` (the derived-value
/// payoff per rank) and `soft_cap` (in ranks), all under the shared
/// `PLAYER_HARD_CAP` *rank* ceiling — every class can invest the same 50
/// ranks, but a rank is worth a different amount of HP/MP/Attack/etc.
/// depending on `gain`.
pub fn alloc_profile(class: Class) -> AllocProfile {
    match class {
        // Off-tank: the best HP-per-point in the party, a real attack gain,
        // at the price of mediocre MP/speed/luck payoff.
        Class::Warrior => AllocProfile {
            max_hp: rule(3, 41, PLAYER_HARD_CAP),
            max_mp: rule(1, 24, PLAYER_HARD_CAP),
            attack: rule(2, 28, PLAYER_HARD_CAP),
            defense: rule(2, 30, PLAYER_HARD_CAP),
            speed: rule(1, 24, PLAYER_HARD_CAP),
            luck: rule(1, 24, PLAYER_HARD_CAP),
        },
        // Glass cannon: top-tier attack and MP payoff, the frailest HP/defense.
        Class::Mage => AllocProfile {
            max_hp: rule(1, 24, PLAYER_HARD_CAP),
            max_mp: rule(3, 41, PLAYER_HARD_CAP),
            attack: rule(3, 33, PLAYER_HARD_CAP),
            defense: rule(1, 24, PLAYER_HARD_CAP),
            speed: rule(2, 26, PLAYER_HARD_CAP),
            luck: rule(2, 26, PLAYER_HARD_CAP),
        },
        // Support: balanced, durable-leaning, with no standout damage payoff
        // (deliberately no gain=3 stat — the flattest build in the party).
        Class::Cleric => AllocProfile {
            max_hp: rule(2, 36, PLAYER_HARD_CAP),
            max_mp: rule(2, 38, PLAYER_HARD_CAP),
            attack: rule(1, 24, PLAYER_HARD_CAP),
            defense: rule(2, 28, PLAYER_HARD_CAP),
            speed: rule(1, 24, PLAYER_HARD_CAP),
            luck: rule(1, 24, PLAYER_HARD_CAP),
        },
        // Striker: the highest attack ceiling plus double-value speed/luck
        // (the spikiest build — three gain=3 stats).
        Class::Rogue => AllocProfile {
            max_hp: rule(1, 25, PLAYER_HARD_CAP),
            max_mp: rule(1, 24, PLAYER_HARD_CAP),
            attack: rule(3, 33, PLAYER_HARD_CAP),
            defense: rule(1, 24, PLAYER_HARD_CAP),
            speed: rule(3, 41, PLAYER_HARD_CAP),
            luck: rule(3, 41, PLAYER_HARD_CAP),
        },
        // Monsters never allocate; keep the old flat, effectively-uncapped
        // table so nothing changes if one ever does.
        Class::Monster => AllocProfile {
            max_hp: rule(5, i32::MAX, i32::MAX),
            max_mp: rule(3, i32::MAX, i32::MAX),
            attack: rule(2, i32::MAX, i32::MAX),
            defense: rule(2, i32::MAX, i32::MAX),
            speed: rule(1, i32::MAX, i32::MAX),
            luck: rule(1, i32::MAX, i32::MAX),
        },
    }
}

/// What the next point spent on a stat would actually do — feeds both
/// `allocate_point` and the level-up screen's per-stat gain labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocPreview {
    /// Below the soft cap: the class's full gain.
    Full(i32),
    /// At/past the soft cap: gain halved (rounded up, so never zero).
    Diminished(i32),
    /// Already at the hard cap: no further gain is possible, so the point
    /// stays banked rather than being spent for zero effect.
    Capped,
}

/// XP required to advance from `level` to `level + 1`.
pub fn xp_to_next_level(level: u32) -> u32 {
    20 * level + 30
}

/// Automatic per-level stat growth, applied on every level-up *in addition*
/// to the banked points the player spends by hand — so each class grows into
/// a different shape on its own (the Warrior toughens, the Mage's mana pool
/// deepens, the Rogue gets faster and luckier) even before any allocation.
/// Fields are *ranks* per level (run through `Character::invest_rank`, same
/// as hand-spent points, so `gain` converts them into the actual derived
/// value), fractional rather than flat integers since hitting the right
/// fraction of a 50-rank hard cap by level 40 needs rates like 0.51/level —
/// see `Character::growth_carry`, which accumulates the fractional remainder
/// between levels so a small rate doesn't silently round up to +1/level
/// every single level.
#[derive(Debug, Clone, Copy)]
pub struct LevelGrowth {
    pub max_hp: f32,
    pub max_mp: f32,
    pub attack: f32,
    pub defense: f32,
    pub speed: f32,
    pub luck: f32,
}

/// Each class's growth profile — these rank rates alone (zero points
/// allocated) land every class/stat at or under ~66% of `PLAYER_HARD_CAP`'s
/// rank ceiling by `MAX_LEVEL`, leaving real headroom for allocation to
/// matter. `Monster`'s table is unused: regular monsters/bosses grow via a
/// percentage of their own species base stat instead (see
/// `Character::scale_to_level`'s `monster_base` argument), so their
/// hierarchy doesn't compress at high levels the way a flat shared rate
/// would.
pub fn level_growth(class: Class) -> LevelGrowth {
    match class {
        Class::Warrior => LevelGrowth {
            max_hp: 0.64,
            max_mp: 0.22,
            attack: 0.51,
            defense: 0.56,
            speed: 0.18,
            luck: 0.18,
        },
        Class::Mage => LevelGrowth {
            max_hp: 0.07,
            max_mp: 0.64,
            attack: 0.62,
            defense: 0.15,
            speed: 0.47,
            luck: 0.47,
        },
        Class::Rogue => LevelGrowth {
            max_hp: 0.33,
            max_mp: 0.22,
            attack: 0.62,
            defense: 0.22,
            speed: 0.82,
            luck: 0.82,
        },
        Class::Cleric => LevelGrowth {
            max_hp: 0.55,
            max_mp: 0.58,
            attack: 0.29,
            defense: 0.51,
            speed: 0.18,
            luck: 0.40,
        },
        Class::Monster => LevelGrowth {
            max_hp: 0.0,
            max_mp: 0.0,
            attack: 0.0,
            defense: 0.0,
            speed: 0.0,
            luck: 0.0,
        },
    }
}

/// Tapers automatic per-level growth so it doesn't keep compounding forever:
/// full strength through level 10, linearly down to 0.5x by level 25, then
/// floored at 0.5x beyond. Hand-allocated points (`POINTS_PER_LEVEL`) aren't
/// affected — this only scales the class's automatic `level_growth`.
fn growth_multiplier(level: u32) -> f32 {
    if level <= 10 {
        1.0
    } else if level >= 25 {
        0.5
    } else {
        1.0 - 0.5 * (level - 10) as f32 / 15.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub name: String,
    pub class: Class,
    pub level: u32,
    pub xp: u32,
    pub unspent_points: u32,
    pub stats: Stats,
    pub abilities: Vec<Ability>,
    /// The weapon this character currently wields. Monsters leave this
    /// `None` since they fight bare-handed/claw/fang; playable characters
    /// start with one equipped but, like armor/rings, can be left bare via
    /// `unequip_weapon`.
    pub equipped_weapon: Option<Weapon>,
    /// The armor this character currently wears, if any.
    pub equipped_armor: Option<Armor>,
    /// The two rings this character currently wears, if any.
    pub equipped_rings: [Option<Ring>; 2],
    /// Which boss this is, if it's a boss at all — lets combat dispatch
    /// scripted moves (`combat::resolve_enemy_action`) without comparing
    /// display-name strings.
    pub boss_kind: Option<BossKind>,
    /// Whether this is a rarer, toughened variant of its species — a
    /// display/loot concern only. Every species-keyed comparison (loot
    /// tables, sprite color, AI dispatch) must keep comparing against the
    /// raw `name`; use `display_name()` for anything shown to the player.
    #[serde(default)]
    pub is_elite: bool,
    /// Whether this elite has already fired its guaranteed shot at its
    /// species' signature move (see `apply_elite`). Ignored for non-elites,
    /// which keep rolling their move's normal odds every eligible turn.
    #[serde(default)]
    pub elite_signature_used: bool,
    /// Fractional remainder of automatic per-level growth not yet applied,
    /// one slot per `AllocStat` (indexed in `ALLOC_STATS` order) — see
    /// `LevelGrowth`'s doc comment for why this exists. Defaults to zero for
    /// saves/tests predating this field.
    #[serde(default)]
    pub growth_carry: [f32; 6],
    /// Total ranks invested per stat (indexed in `ALLOC_STATS` order), from
    /// hand-spent points and automatic growth alike — this, not the derived
    /// stat value, is what `PLAYER_HARD_CAP` bounds. See `invest_rank`.
    /// Defaults to zero for saves/tests predating this field.
    #[serde(default)]
    pub alloc_ranks: [i32; 6],
}

impl Character {
    pub fn is_alive(&self) -> bool {
        self.stats.hp > 0
    }

    /// The name to show the player — prefixes "Elite " for elite variants.
    /// Never compare against this for species-based logic; use `name`.
    pub fn display_name(&self) -> String {
        if self.is_elite {
            format!("Elite {}", self.name)
        } else {
            self.name.clone()
        }
    }

    /// Promotes a regular monster to an elite variant: tougher and hits
    /// harder, at a smaller bump to defense. Call *after* `scale_to_level`/
    /// `apply_ng_plus` so it stacks on top of the final stats, and heals to
    /// full so the promotion never leaves the enemy artificially weakened.
    pub fn apply_elite(&mut self) {
        self.is_elite = true;
        self.stats.max_hp = ((self.stats.max_hp as f32) * 1.5).round() as i32;
        self.stats.attack = ((self.stats.attack as f32) * 1.5).round() as i32;
        self.stats.defense = ((self.stats.defense as f32) * 1.3).round() as i32;
        self.stats.hp = self.stats.max_hp;
    }

    /// Odds that a field encounter promotes one of its enemies to elite:
    /// 10% baseline, climbing 2 percentage points per NG+ cycle up to a
    /// 24% ceiling at NG+7.
    pub fn elite_chance(ng_plus: u32) -> f32 {
        0.10 + 0.02 * ng_plus.min(7) as f32
    }

    pub fn take_damage(&mut self, amount: i32) {
        self.stats.hp = (self.stats.hp - amount).max(0);
    }

    pub fn heal(&mut self, amount: i32) {
        self.stats.hp = (self.stats.hp + amount).min(self.stats.max_hp);
    }

    pub fn spend_mp(&mut self, amount: i32) -> bool {
        if self.stats.mp >= amount {
            self.stats.mp -= amount;
            true
        } else {
            false
        }
    }

    pub fn hp_ratio(&self) -> f64 {
        self.stats.hp as f64 / self.stats.max_hp as f64
    }

    pub fn mp_ratio(&self) -> f64 {
        self.stats.mp as f64 / self.stats.max_mp as f64
    }

    /// Adds `amount` XP, applying as many level-ups as it covers (a single
    /// large award — e.g. a boss kill — can cross several thresholds at
    /// once), stopping at `MAX_LEVEL`. Returns the number of levels gained,
    /// 0 if none (including when already at the cap). Any level gained
    /// fully restores HP and MP, so leveling up always feels like a reward
    /// rather than leaving the party mid-fight at whatever HP it was at.
    pub fn gain_xp(&mut self, amount: u32) -> u32 {
        if self.level >= MAX_LEVEL {
            return 0;
        }
        self.xp += amount;
        let mut levels = 0;
        while self.level < MAX_LEVEL && self.xp >= xp_to_next_level(self.level) {
            self.xp -= xp_to_next_level(self.level);
            self.level += 1;
            self.unspent_points += POINTS_PER_LEVEL;
            self.apply_level_growth(None);
            levels += 1;
        }
        if self.level >= MAX_LEVEL {
            // Nothing ever spends banked XP again once capped — keeping it
            // around would just make the XP display lie.
            self.xp = 0;
        }
        if levels > 0 {
            self.stats.hp = self.stats.max_hp;
            self.stats.mp = self.stats.max_mp;
        }
        levels
    }

    /// Applies one level's worth of automatic growth, scaled by
    /// `growth_multiplier` so it doesn't keep compounding forever
    /// (hand-allocated points are untouched by this taper). `monster_base`
    /// is `None` for playable characters (uses this class's `level_growth`
    /// table, interpreted as *ranks* per level and routed through
    /// `invest_rank` so automatic leveling can never push a stat's rank past
    /// its hard cap) or `Some(species' own level-1 stats)` for monsters (see
    /// `scale_to_level`), which grow as a fixed percentage of their own base
    /// stats instead of a shared flat table — that keeps weaker/stronger
    /// species properly separated at high levels instead of compressing
    /// toward the same numbers, and is applied straight to `stats` since
    /// monsters have no rank concept (their `AllocRule` is unbounded).
    /// Fractional remainders that don't add up to a whole unit yet are kept
    /// in `growth_carry` rather than rounded away, so a small rate still adds
    /// up correctly over many levels instead of silently becoming +1/level
    /// every level.
    fn apply_level_growth(&mut self, monster_base: Option<&Stats>) {
        let m = growth_multiplier(self.level);
        match monster_base {
            Some(base) => {
                let rates: [f32; 6] = [
                    base.max_hp as f32 * MONSTER_HP_GROWTH_RATE,
                    0.0,
                    base.attack as f32 * MONSTER_ATTACK_GROWTH_RATE,
                    base.defense as f32 * MONSTER_DEFENSE_GROWTH_RATE,
                    0.0,
                    0.0,
                ];
                for (i, _stat) in ALLOC_STATS.iter().enumerate() {
                    self.growth_carry[i] += rates[i] * m;
                    let whole = self.growth_carry[i].trunc();
                    self.growth_carry[i] -= whole;
                    let whole = whole as i32;
                    match ALLOC_STATS[i] {
                        AllocStat::MaxHp => {
                            self.stats.max_hp += whole;
                            self.stats.hp += whole;
                        }
                        AllocStat::MaxMp => {
                            self.stats.max_mp += whole;
                            self.stats.mp += whole;
                        }
                        AllocStat::Attack => self.stats.attack += whole,
                        AllocStat::Defense => self.stats.defense += whole,
                        AllocStat::Speed => self.stats.speed += whole,
                        AllocStat::Luck => self.stats.luck += whole,
                    }
                }
            }
            None => {
                let g = level_growth(self.class);
                let rates: [f32; 6] = [g.max_hp, g.max_mp, g.attack, g.defense, g.speed, g.luck];
                for (i, stat) in ALLOC_STATS.iter().enumerate() {
                    self.growth_carry[i] += rates[i] * m;
                    let whole = self.growth_carry[i].trunc();
                    self.growth_carry[i] -= whole;
                    self.invest_rank(*stat, whole as i32);
                }
            }
        }
    }

    /// Raises this character straight to `level` (if higher than its current
    /// one), applying its automatic growth for every level gained and
    /// topping HP/MP off. This is how later chapters toughen up their
    /// regular monsters — same species, more dangerous specimen — and since
    /// `combat::xp_value` reads the resulting stats, XP rewards scale along
    /// with the threat automatically. The species' own level-1 stats are
    /// captured once, before any growth is applied, so every subsequent
    /// level's growth is a percentage of the *original* base rather than an
    /// already-grown value (which would compound instead of scaling linearly).
    pub fn scale_to_level(&mut self, level: u32) {
        let monster_base = self.stats;
        while self.level < level {
            self.level += 1;
            self.apply_level_growth(Some(&monster_base));
        }
        self.stats.hp = self.stats.max_hp;
        self.stats.mp = self.stats.max_mp;
    }

    /// Dark-Souls-style New Game+ scaling: multiplies this enemy's
    /// max_hp/attack/defense by `ng_plus_multiplier(ng_plus)` and heals it to
    /// full, stacking on top of whatever `scale_to_level` already applied.
    /// Speed/luck are left untouched so turn order and crit math don't need
    /// re-tuning. A no-op at `ng_plus == 0`.
    pub fn apply_ng_plus(&mut self, ng_plus: u32) {
        if ng_plus == 0 {
            return;
        }
        let mult = ng_plus_multiplier(ng_plus);
        self.stats.max_hp = ((self.stats.max_hp as f32) * mult).round() as i32;
        self.stats.attack = ((self.stats.attack as f32) * mult).round() as i32;
        self.stats.defense = ((self.stats.defense as f32) * mult).round() as i32;
        self.stats.hp = self.stats.max_hp;

        // Speed/luck ride the same curve at half strength — enough that NG+
        // enemies are noticeably nimbler/luckier over several cycles without
        // swinging turn order or crit rates as hard as raw hp/attack/defense.
        let minor_mult = ng_plus_minor_multiplier(ng_plus);
        self.stats.speed = ((self.stats.speed as f32) * minor_mult).round() as i32;
        self.stats.luck = ((self.stats.luck as f32) * minor_mult).round() as i32;
    }

    /// Toughens a boss when the party has significantly overleveled its
    /// chapter — bosses are otherwise hardcoded and never run through
    /// `scale_to_level`, so a party that ground out extra levels before
    /// reaching the lair would trivialize the fight. A no-op at or under
    /// `baseline_level`; otherwise +2.5% max_hp/attack/defense per excess
    /// level, capped at 20 excess levels (+50%), then healed to full.
    pub fn scale_boss_to_party(&mut self, avg_party_level: u32, baseline_level: u32) {
        if avg_party_level <= baseline_level {
            return;
        }
        let excess = (avg_party_level - baseline_level).min(20);
        let mult = 1.0 + 0.025 * excess as f32;
        self.stats.max_hp = ((self.stats.max_hp as f32) * mult).round() as i32;
        self.stats.attack = ((self.stats.attack as f32) * mult).round() as i32;
        self.stats.defense = ((self.stats.defense as f32) * mult).round() as i32;
        self.stats.hp = self.stats.max_hp;
    }

    /// The current *base* value of an allocatable stat — the actual
    /// gameplay number combat reads (deliberately not `total_attack()` etc.,
    /// so gear bonuses aren't counted twice).
    pub fn base_stat(&self, stat: AllocStat) -> i32 {
        match stat {
            AllocStat::MaxHp => self.stats.max_hp,
            AllocStat::MaxMp => self.stats.max_mp,
            AllocStat::Attack => self.stats.attack,
            AllocStat::Defense => self.stats.defense,
            AllocStat::Speed => self.stats.speed,
            AllocStat::Luck => self.stats.luck,
        }
    }

    /// How many ranks are currently invested in `stat` — what the caps in
    /// `alloc_profile` compare against. This is "the stat" the level-up
    /// screen shows (0 up to the shared `PLAYER_HARD_CAP`), distinct from
    /// `base_stat`'s derived gameplay value, which is worth a different
    /// amount per class (see `AllocRule`'s doc comment).
    pub fn rank(&self, stat: AllocStat) -> i32 {
        self.alloc_ranks[ALLOC_STATS.iter().position(|s| *s == stat).unwrap()]
    }

    /// What spending one point on `stat` would do right now, given this
    /// class's `alloc_profile` and the stat's current invested rank. Clips
    /// the gain to whatever rank headroom remains under the hard cap, and
    /// refuses entirely (`Capped`) once there's none left.
    pub fn alloc_preview(&self, stat: AllocStat) -> AllocPreview {
        let rule = alloc_profile(self.class).rule(stat);
        let rank = self.rank(stat);
        let headroom = rule.hard_cap - rank;
        if headroom <= 0 {
            AllocPreview::Capped
        } else if rank >= rule.soft_cap {
            AllocPreview::Diminished((rule.gain + 1) / 2)
        } else {
            AllocPreview::Full(rule.gain)
        }
    }

    /// Spends one banked point on `stat`, if any are banked and it isn't
    /// already at its hard cap. Returns whether a point was spent.
    pub fn allocate_point(&mut self, stat: AllocStat) -> bool {
        self.allocate_point_tracked(stat).is_some()
    }

    /// Same as `allocate_point`, but returns the exact gain applied (or
    /// `None` if no point was spent — either no points are banked, or `stat`
    /// is already at its hard cap) so a caller can record it — e.g. the
    /// level-up screen's undo history — and later reverse it exactly via
    /// `deallocate_point` without recomputing soft-cap math.
    pub fn allocate_point_tracked(&mut self, stat: AllocStat) -> Option<i32> {
        if self.unspent_points == 0 {
            return None;
        }
        if matches!(self.alloc_preview(stat), AllocPreview::Capped) {
            return None;
        }
        self.unspent_points -= 1;
        Some(self.invest_rank(stat, 1))
    }

    /// The single chokepoint every source of stat growth — automatic
    /// `level_growth` and hand-spent allocation alike — goes through to
    /// mutate `stats`. Grants up to `ranks` ranks of `stat`, one at a time
    /// (so a multi-rank grant that crosses the soft cap mid-way still
    /// diminishes correctly partway through), converting each rank into a
    /// derived-value delta via this class's `gain` (halved past the soft
    /// cap), and stopping the moment the rank hard cap is reached. Returns
    /// the total derived-value delta actually applied, which may be less
    /// than `ranks * gain` (or zero) if the cap was reached.
    fn invest_rank(&mut self, stat: AllocStat, ranks: i32) -> i32 {
        let rule = alloc_profile(self.class).rule(stat);
        let mut applied = 0;
        for _ in 0..ranks {
            let rank = self.rank(stat);
            if rank >= rule.hard_cap {
                break;
            }
            let gain = if rank >= rule.soft_cap {
                (rule.gain + 1) / 2
            } else {
                rule.gain
            };
            let idx = ALLOC_STATS.iter().position(|s| *s == stat).unwrap();
            self.alloc_ranks[idx] += 1;
            match stat {
                AllocStat::MaxHp => {
                    self.stats.max_hp += gain;
                    self.stats.hp += gain;
                }
                AllocStat::MaxMp => {
                    self.stats.max_mp += gain;
                    self.stats.mp += gain;
                }
                AllocStat::Attack => self.stats.attack += gain,
                AllocStat::Defense => self.stats.defense += gain,
                AllocStat::Speed => self.stats.speed += gain,
                AllocStat::Luck => self.stats.luck += gain,
            }
            applied += gain;
        }
        applied
    }

    /// Inverse of `allocate_point_tracked`: refunds one banked point,
    /// un-invests one rank, and subtracts exactly `amount` (the gain that
    /// call returned) back off `stat`. Takes the exact amount rather than
    /// recomputing it so undo is always precise regardless of where the soft
    /// cap sits now.
    pub fn deallocate_point(&mut self, stat: AllocStat, amount: i32) {
        self.unspent_points += 1;
        let idx = ALLOC_STATS.iter().position(|s| *s == stat).unwrap();
        self.alloc_ranks[idx] -= 1;
        match stat {
            AllocStat::MaxHp => {
                self.stats.max_hp -= amount;
                self.stats.hp -= amount;
            }
            AllocStat::MaxMp => {
                self.stats.max_mp -= amount;
                self.stats.mp -= amount;
            }
            AllocStat::Attack => self.stats.attack -= amount,
            AllocStat::Defense => self.stats.defense -= amount,
            AllocStat::Speed => self.stats.speed -= amount,
            AllocStat::Luck => self.stats.luck -= amount,
        }
    }

    /// Luck stat, boosting critical-hit chance (`combat::crit_chance`). No
    /// gear grants a luck bonus yet, but this mirrors `total_attack`/
    /// `total_defense`'s shape so gear can add one later without another
    /// `combat.rs` change.
    pub fn total_luck(&self) -> i32 {
        self.stats.luck
    }

    /// Whether the ability at `idx` targets an ally (heal) rather than an enemy.
    /// Returns false (i.e. "targets an enemy") if the index is out of range,
    /// so callers can default to enemy-targeting UI safely.
    pub fn ability_is_heal(&self, idx: usize) -> bool {
        self.abilities
            .get(idx)
            .map(|a| matches!(a.kind, AbilityKind::Heal))
            .unwrap_or(false)
    }

    /// Attack stat plus whatever bonus the equipped weapon and rings grant.
    /// This is what basic "Attack" actions roll damage from — abilities
    /// scale off the raw stat instead, since spells don't care which sword
    /// (or ring) you're holding.
    pub fn total_attack(&self) -> i32 {
        self.stats.attack
            + self
                .equipped_weapon
                .as_ref()
                .map(|w| w.attack_bonus)
                .unwrap_or(0)
            + self
                .equipped_rings
                .iter()
                .flatten()
                .map(|r| r.attack_bonus)
                .sum::<i32>()
    }

    /// Defense stat plus whatever bonus the equipped weapon, armor, and
    /// rings grant.
    pub fn total_defense(&self) -> i32 {
        self.stats.defense
            + self
                .equipped_weapon
                .as_ref()
                .map(|w| w.defense_bonus)
                .unwrap_or(0)
            + self
                .equipped_armor
                .as_ref()
                .map(|a| a.defense_bonus)
                .unwrap_or(0)
            + self
                .equipped_rings
                .iter()
                .flatten()
                .map(|r| r.defense_bonus)
                .sum::<i32>()
    }

    /// Equips `weapon`, returning whatever was previously equipped (if
    /// anything) so the caller can return it to the party's inventory.
    pub fn equip_weapon(&mut self, weapon: Weapon) -> Option<Weapon> {
        self.equipped_weapon.replace(weapon)
    }

    /// Removes and returns the equipped weapon, if any, leaving the slot empty.
    pub fn unequip_weapon(&mut self) -> Option<Weapon> {
        self.equipped_weapon.take()
    }

    /// Equips `armor`, returning whatever was previously equipped (if any).
    pub fn equip_armor(&mut self, armor: Armor) -> Option<Armor> {
        self.equipped_armor.replace(armor)
    }

    /// Removes and returns the equipped armor, if any, leaving the slot empty.
    pub fn unequip_armor(&mut self) -> Option<Armor> {
        self.equipped_armor.take()
    }

    /// Equips `ring` into `slot`, returning whatever was previously there (if any).
    pub fn equip_ring(&mut self, slot: RingSlot, ring: Ring) -> Option<Ring> {
        let slot_ref = match slot {
            RingSlot::First => &mut self.equipped_rings[0],
            RingSlot::Second => &mut self.equipped_rings[1],
        };
        slot_ref.replace(ring)
    }

    /// Removes and returns the ring in `slot`, if any, leaving it empty.
    pub fn unequip_ring(&mut self, slot: RingSlot) -> Option<Ring> {
        let slot_ref = match slot {
            RingSlot::First => &mut self.equipped_rings[0],
            RingSlot::Second => &mut self.equipped_rings[1],
        };
        slot_ref.take()
    }
}

// --- Starting party factory functions ---

/// The full ability kit for a playable class — the single source of truth
/// the party factories below pull from. Kept separate so `World::from_save`
/// can resync loaded members against it: abilities are serialized into the
/// save, so a save written before a class gained a new ability would
/// otherwise never see it.
pub fn class_abilities(class: Class) -> Vec<Ability> {
    match class {
        // mp_cost values are rescaled proportionally to each class's new,
        // much smaller base max_mp (see the base `Stats` in `warrior`/
        // `mage`/`cleric`/`rogue` below) so every ability stays castable at
        // level 1 with zero MP allocated — a flat-percentage translation of
        // the pre-rescale costs, not independently retuned.
        Class::Warrior => vec![
            Ability {
                name: "Power Strike".into(),
                mp_cost: 1,
                base_power: 10,
                kind: AbilityKind::PhysicalDamage,
                targets_all_enemies: false,
            },
            Ability {
                name: "Crushing Blow".into(),
                mp_cost: 2,
                base_power: 18,
                kind: AbilityKind::PhysicalDamage,
                targets_all_enemies: false,
            },
            Ability {
                name: "Quaking Slam".into(),
                mp_cost: 3,
                base_power: 12,
                kind: AbilityKind::PhysicalDamage,
                targets_all_enemies: true,
            },
        ],
        Class::Mage => vec![
            Ability {
                name: "Firebolt".into(),
                mp_cost: 3,
                base_power: 16,
                kind: AbilityKind::MagicDamage,
                targets_all_enemies: false,
            },
            Ability {
                name: "Lightning Bolt".into(),
                mp_cost: 6,
                base_power: 26,
                kind: AbilityKind::MagicDamage,
                targets_all_enemies: false,
            },
            Ability {
                name: "Chaos Storm".into(),
                mp_cost: 9,
                base_power: 20,
                kind: AbilityKind::MagicDamage,
                targets_all_enemies: true,
            },
        ],
        Class::Cleric => vec![
            Ability {
                name: "Mend".into(),
                mp_cost: 5,
                base_power: 18,
                kind: AbilityKind::Heal,
                targets_all_enemies: false,
            },
            Ability {
                name: "Smite".into(),
                mp_cost: 3,
                base_power: 12,
                kind: AbilityKind::PhysicalDamage,
                targets_all_enemies: false,
            },
            Ability {
                name: "Wrath of the Gods".into(),
                mp_cost: 7,
                base_power: 15,
                kind: AbilityKind::MagicDamage,
                targets_all_enemies: true,
            },
        ],
        Class::Rogue => vec![
            Ability {
                name: "Backstab".into(),
                mp_cost: 1,
                base_power: 12,
                kind: AbilityKind::PhysicalDamage,
                targets_all_enemies: false,
            },
            Ability {
                name: "Fan of Knives".into(),
                mp_cost: 3,
                base_power: 14,
                kind: AbilityKind::PhysicalDamage,
                targets_all_enemies: true,
            },
            Ability {
                name: "Hornet Sting".into(),
                mp_cost: 3,
                base_power: 26,
                kind: AbilityKind::PhysicalDamage,
                targets_all_enemies: false,
            },
        ],
        Class::Monster => vec![],
    }
}

pub fn warrior(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Warrior,
        level: 1,
        xp: 0,
        unspent_points: 0,
        stats: Stats {
            max_hp: 15,
            hp: 15,
            max_mp: 4,
            mp: 4,
            attack: 6,
            defense: 7,
            // Kept at (rather than below) the weakest trash monster's speed
            // (Slime, 4) — `CombatState::new`'s turn-order sort is stable
            // and lists party members before enemies, so a tie still goes
            // to the player. Mediocre, not literally the slowest thing in
            // the game.
            speed: 4,
            luck: 2,
        },
        abilities: class_abilities(Class::Warrior),
        equipped_weapon: Some(worn_shortsword()),
        equipped_armor: None,
        equipped_rings: [None, None],
        boss_kind: None,
        is_elite: false,
        elite_signature_used: false,
        growth_carry: [0.0; 6],
        alloc_ranks: [0; 6],
    }
}

pub fn mage(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Mage,
        level: 1,
        xp: 0,
        unspent_points: 0,
        stats: Stats {
            max_hp: 6,
            hp: 6,
            max_mp: 15,
            mp: 15,
            attack: 8,
            defense: 2,
            speed: 5,
            luck: 5,
        },
        abilities: class_abilities(Class::Mage),
        equipped_weapon: Some(apprentice_wand()),
        equipped_armor: None,
        equipped_rings: [None, None],
        boss_kind: None,
        is_elite: false,
        elite_signature_used: false,
        growth_carry: [0.0; 6],
        alloc_ranks: [0; 6],
    }
}

pub fn cleric(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Cleric,
        level: 1,
        xp: 0,
        unspent_points: 0,
        stats: Stats {
            max_hp: 13,
            hp: 13,
            max_mp: 14,
            mp: 14,
            attack: 4,
            defense: 6,
            // See the Warrior's matching comment above.
            speed: 4,
            luck: 4,
        },
        abilities: class_abilities(Class::Cleric),
        equipped_weapon: Some(acolytes_mace()),
        equipped_armor: None,
        equipped_rings: [None, None],
        boss_kind: None,
        is_elite: false,
        elite_signature_used: false,
        growth_carry: [0.0; 6],
        alloc_ranks: [0; 6],
    }
}

pub fn rogue(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Rogue,
        level: 1,
        xp: 0,
        unspent_points: 0,
        stats: Stats {
            max_hp: 8,
            hp: 8,
            max_mp: 4,
            mp: 4,
            attack: 8,
            defense: 2,
            speed: 10,
            luck: 10,
        },
        abilities: class_abilities(Class::Rogue),
        equipped_weapon: Some(thieves_dirk()),
        equipped_armor: None,
        equipped_rings: [None, None],
        boss_kind: None,
        is_elite: false,
        elite_signature_used: false,
        growth_carry: [0.0; 6],
        alloc_ranks: [0; 6],
    }
}

pub fn slime(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Monster,
        level: 1,
        xp: 0,
        unspent_points: 0,
        stats: Stats {
            max_hp: 18,
            hp: 18,
            max_mp: 0,
            mp: 0,
            attack: 7,
            defense: 3,
            speed: 4,
            luck: 2,
        },
        abilities: vec![],
        equipped_weapon: None,
        equipped_armor: None,
        equipped_rings: [None, None],
        boss_kind: None,
        is_elite: false,
        elite_signature_used: false,
        growth_carry: [0.0; 6],
        alloc_ranks: [0; 6],
    }
}

pub fn goblin(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Monster,
        level: 1,
        xp: 0,
        unspent_points: 0,
        stats: Stats {
            max_hp: 24,
            hp: 24,
            max_mp: 0,
            mp: 0,
            attack: 9,
            defense: 5,
            speed: 9,
            luck: 6,
        },
        abilities: vec![],
        equipped_weapon: None,
        equipped_armor: None,
        equipped_rings: [None, None],
        boss_kind: None,
        is_elite: false,
        elite_signature_used: false,
        growth_carry: [0.0; 6],
        alloc_ranks: [0; 6],
    }
}

pub fn bat(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Monster,
        level: 1,
        xp: 0,
        unspent_points: 0,
        stats: Stats {
            max_hp: 14,
            hp: 14,
            max_mp: 0,
            mp: 0,
            attack: 6,
            defense: 2,
            speed: 12,
            luck: 10,
        },
        abilities: vec![],
        equipped_weapon: None,
        equipped_armor: None,
        equipped_rings: [None, None],
        boss_kind: None,
        is_elite: false,
        elite_signature_used: false,
        growth_carry: [0.0; 6],
        alloc_ranks: [0; 6],
    }
}

pub fn wolf(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Monster,
        level: 1,
        xp: 0,
        unspent_points: 0,
        stats: Stats {
            max_hp: 22,
            hp: 22,
            max_mp: 0,
            mp: 0,
            attack: 10,
            defense: 4,
            speed: 10,
            luck: 5,
        },
        abilities: vec![],
        equipped_weapon: None,
        equipped_armor: None,
        equipped_rings: [None, None],
        boss_kind: None,
        is_elite: false,
        elite_signature_used: false,
        growth_carry: [0.0; 6],
        alloc_ranks: [0; 6],
    }
}

pub fn skeleton(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Monster,
        level: 1,
        xp: 0,
        unspent_points: 0,
        stats: Stats {
            max_hp: 30,
            hp: 30,
            max_mp: 0,
            mp: 0,
            attack: 8,
            defense: 9,
            speed: 3,
            luck: 3,
        },
        abilities: vec![],
        equipped_weapon: None,
        equipped_armor: None,
        equipped_rings: [None, None],
        boss_kind: None,
        is_elite: false,
        elite_signature_used: false,
        growth_carry: [0.0; 6],
        alloc_ranks: [0; 6],
    }
}

pub fn orc(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Monster,
        level: 1,
        xp: 0,
        unspent_points: 0,
        stats: Stats {
            max_hp: 36,
            hp: 36,
            max_mp: 0,
            mp: 0,
            attack: 13,
            defense: 7,
            speed: 5,
            luck: 4,
        },
        abilities: vec![],
        equipped_weapon: None,
        equipped_armor: None,
        equipped_rings: [None, None],
        boss_kind: None,
        is_elite: false,
        elite_signature_used: false,
        growth_carry: [0.0; 6],
        alloc_ranks: [0; 6],
    }
}

/// Its attack occasionally curses the party instead of dealing damage —
/// see `CombatState::resolve_enemy_action`.
pub fn wraith(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Monster,
        level: 1,
        xp: 0,
        unspent_points: 0,
        stats: Stats {
            max_hp: 20,
            hp: 20,
            max_mp: 0,
            mp: 0,
            attack: 7,
            defense: 5,
            speed: 7,
            luck: 7,
        },
        abilities: vec![],
        equipped_weapon: None,
        equipped_armor: None,
        equipped_rings: [None, None],
        boss_kind: None,
        is_elite: false,
        elite_signature_used: false,
        growth_carry: [0.0; 6],
        alloc_ranks: [0; 6],
    }
}

/// Disguised as treasure until it ambushes you — see `FieldEvent` in `game/state.rs`.
pub fn mimic(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Monster,
        level: 1,
        xp: 0,
        unspent_points: 0,
        stats: Stats {
            max_hp: 30,
            hp: 30,
            max_mp: 0,
            mp: 0,
            attack: 12,
            defense: 8,
            speed: 4,
            luck: 8,
        },
        abilities: vec![],
        equipped_weapon: None,
        equipped_armor: None,
        equipped_rings: [None, None],
        boss_kind: None,
        is_elite: false,
        elite_signature_used: false,
        growth_carry: [0.0; 6],
        alloc_ranks: [0; 6],
    }
}

pub fn hollow(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Monster,
        level: 1,
        xp: 0,
        unspent_points: 0,
        stats: Stats {
            max_hp: 20,
            hp: 20,
            max_mp: 0,
            mp: 0,
            attack: 8,
            defense: 4,
            speed: 5,
            luck: 3,
        },
        abilities: vec![],
        equipped_weapon: None,
        equipped_armor: None,
        equipped_rings: [None, None],
        boss_kind: None,
        is_elite: false,
        elite_signature_used: false,
        growth_carry: [0.0; 6],
        alloc_ranks: [0; 6],
    }
}

pub fn rat(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Monster,
        level: 1,
        xp: 0,
        unspent_points: 0,
        stats: Stats {
            max_hp: 15,
            hp: 15,
            max_mp: 0,
            mp: 0,
            attack: 7,
            defense: 2,
            speed: 9,
            luck: 6,
        },
        abilities: vec![],
        equipped_weapon: None,
        equipped_armor: None,
        equipped_rings: [None, None],
        boss_kind: None,
        is_elite: false,
        elite_signature_used: false,
        growth_carry: [0.0; 6],
        alloc_ranks: [0; 6],
    }
}

pub fn carrion_crow(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Monster,
        level: 1,
        xp: 0,
        unspent_points: 0,
        stats: Stats {
            max_hp: 16,
            hp: 16,
            max_mp: 0,
            mp: 0,
            attack: 8,
            defense: 3,
            speed: 13,
            luck: 9,
        },
        abilities: vec![],
        equipped_weapon: None,
        equipped_armor: None,
        equipped_rings: [None, None],
        boss_kind: None,
        is_elite: false,
        elite_signature_used: false,
        growth_carry: [0.0; 6],
        alloc_ranks: [0; 6],
    }
}

pub fn bandit(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Monster,
        level: 1,
        xp: 0,
        unspent_points: 0,
        stats: Stats {
            max_hp: 26,
            hp: 26,
            max_mp: 0,
            mp: 0,
            attack: 11,
            defense: 5,
            speed: 8,
            luck: 7,
        },
        abilities: vec![],
        equipped_weapon: None,
        equipped_armor: None,
        equipped_rings: [None, None],
        boss_kind: None,
        is_elite: false,
        elite_signature_used: false,
        growth_carry: [0.0; 6],
        alloc_ranks: [0; 6],
    }
}

/// Has a scripted move: "Withering Prayer" (see `combat::resolve_enemy_action`)
/// drains MP from a party member to heal itself, following the Wraith's
/// name-gated pattern.
pub fn fell_acolyte(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Monster,
        level: 1,
        xp: 0,
        unspent_points: 0,
        stats: Stats {
            max_hp: 22,
            hp: 22,
            max_mp: 0,
            mp: 0,
            attack: 9,
            defense: 4,
            speed: 6,
            luck: 8,
        },
        abilities: vec![],
        equipped_weapon: None,
        equipped_armor: None,
        equipped_rings: [None, None],
        boss_kind: None,
        is_elite: false,
        elite_signature_used: false,
        growth_carry: [0.0; 6],
        alloc_ranks: [0; 6],
    }
}

/// Has a scripted move: "Ravenous Bite" (see `combat::resolve_enemy_action`)
/// heals it for half the damage it deals.
pub fn grave_ghoul(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Monster,
        level: 1,
        xp: 0,
        unspent_points: 0,
        stats: Stats {
            max_hp: 28,
            hp: 28,
            max_mp: 0,
            mp: 0,
            attack: 11,
            defense: 6,
            speed: 8,
            luck: 4,
        },
        abilities: vec![],
        equipped_weapon: None,
        equipped_armor: None,
        equipped_rings: [None, None],
        boss_kind: None,
        is_elite: false,
        elite_signature_used: false,
        growth_carry: [0.0; 6],
        alloc_ranks: [0; 6],
    }
}

/// A slow, heavily-armored elite — the toughest regular spawn, tuned as a
/// wall rather than a striker.
pub fn barrow_sentinel(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Monster,
        level: 1,
        xp: 0,
        unspent_points: 0,
        stats: Stats {
            max_hp: 44,
            hp: 44,
            max_mp: 0,
            mp: 0,
            attack: 12,
            defense: 12,
            speed: 2,
            luck: 2,
        },
        abilities: vec![],
        equipped_weapon: None,
        equipped_armor: None,
        equipped_rings: [None, None],
        boss_kind: None,
        is_elite: false,
        elite_signature_used: false,
        growth_carry: [0.0; 6],
        alloc_ranks: [0; 6],
    }
}

/// The rare elite roll of the encounter table — the hardest-hitting regular
/// enemy in the game.
pub fn forsaken_knight(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Monster,
        level: 1,
        xp: 0,
        unspent_points: 0,
        stats: Stats {
            max_hp: 40,
            hp: 40,
            max_mp: 0,
            mp: 0,
            attack: 15,
            defense: 9,
            speed: 6,
            luck: 5,
        },
        abilities: vec![],
        equipped_weapon: None,
        equipped_armor: None,
        equipped_rings: [None, None],
        boss_kind: None,
        is_elite: false,
        elite_signature_used: false,
        growth_carry: [0.0; 6],
        alloc_ranks: [0; 6],
    }
}

/// Dark-Souls-style NG+ multiplier: every mob and boss hits harder and
/// survives longer each cycle, capping at NG+7. Retuned to a gentler
/// per-cycle rate than the pre-rescale game (0.35/cycle) to match the much
/// smaller HP/attack/defense pools under the new 50-per-stat hard cap.
pub fn ng_plus_multiplier(ng_plus: u32) -> f32 {
    1.0 + 0.10 * ng_plus.min(7) as f32
}

/// Half-strength companion to [`ng_plus_multiplier`], used for speed/luck so
/// NG+ enemies drift nimbler/luckier over successive cycles without swinging
/// turn order or crit rates as hard as the hp/attack/defense curve does.
pub fn ng_plus_minor_multiplier(ng_plus: u32) -> f32 {
    1.0 + 0.05 * ng_plus.min(7) as f32
}

/// The game's one hand-placed boss, guarding the lair in the field's far
/// corner. Clearly outclasses every regular enemy on raw stats alone, and
/// has two scripted moves handled directly in `combat::resolve_enemy_action`
/// (a heavier "Rending Cleave" and a one-time "Second Wind" rally below 30%
/// HP) rather than the generic `abilities` list, matching how the Wraith's
/// curse is implemented.
pub fn barrow_knight(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Monster,
        level: 1,
        xp: 0,
        unspent_points: 0,
        stats: Stats {
            max_hp: 90,
            hp: 90,
            max_mp: 0,
            mp: 0,
            attack: 16,
            defense: 10,
            speed: 6,
            luck: 8,
        },
        abilities: vec![],
        equipped_weapon: None,
        equipped_armor: None,
        equipped_rings: [None, None],
        boss_kind: Some(BossKind::BarrowKnight),
        is_elite: false,
        elite_signature_used: false,
        growth_carry: [0.0; 6],
        alloc_ranks: [0; 6],
    }
}

/// Chapter two's boss. Clearly outclasses the Barrow Knight on raw stats,
/// with two scripted moves handled in `combat::resolve_boss_move`: a
/// party-wide Tail Sweep, and a one-time Molting Rage rally below 40% HP.
pub fn wyrmscale_warden(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Monster,
        level: 1,
        xp: 0,
        unspent_points: 0,
        stats: Stats {
            max_hp: 160,
            hp: 160,
            max_mp: 0,
            mp: 0,
            attack: 25,
            defense: 16,
            speed: 8,
            luck: 10,
        },
        abilities: vec![],
        equipped_weapon: None,
        equipped_armor: None,
        equipped_rings: [None, None],
        boss_kind: Some(BossKind::WyrmscaleWarden),
        is_elite: false,
        elite_signature_used: false,
        growth_carry: [0.0; 6],
        alloc_ranks: [0; 6],
    }
}

/// The final boss, guarding the throne at the end of chapter three.
/// Outclasses every other boss on raw stats, with two scripted moves
/// handled in `combat::resolve_boss_move`: Cinder Nova, and a two-stage
/// Ashen Rebirth rally (once below 50% HP, again below 20%).
pub fn ashen_sovereign(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Monster,
        level: 1,
        xp: 0,
        unspent_points: 0,
        stats: Stats {
            max_hp: 220,
            hp: 220,
            max_mp: 0,
            mp: 0,
            attack: 30,
            defense: 19,
            speed: 10,
            luck: 14,
        },
        abilities: vec![],
        equipped_weapon: None,
        equipped_armor: None,
        equipped_rings: [None, None],
        boss_kind: Some(BossKind::AshenSovereign),
        is_elite: false,
        elite_signature_used: false,
        growth_carry: [0.0; 6],
        alloc_ranks: [0; 6],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::item::{
        band_of_the_barrow, copper_band, dragonscale_aegis, dragonslayers_oath, iron_sword,
        padded_vest,
    };

    #[test]
    fn every_playable_class_has_three_abilities_and_monsters_none() {
        for class in [Class::Warrior, Class::Mage, Class::Cleric, Class::Rogue] {
            let kit = class_abilities(class);
            assert_eq!(kit.len(), 3, "{class:?} should have a 3-ability kit");
        }
        assert!(class_abilities(Class::Monster).is_empty());
    }

    #[test]
    fn party_factories_use_the_class_ability_kit() {
        assert_eq!(warrior("Bram").abilities.len(), 3);
        assert_eq!(mage("Sella").abilities.len(), 3);
        assert_eq!(cleric("Idris").abilities.len(), 3);
        assert_eq!(rogue("Wren").abilities.len(), 3);
    }

    #[test]
    fn starting_characters_always_have_a_weapon_equipped() {
        assert!(warrior("Bram").equipped_weapon.is_some());
        assert!(mage("Sella").equipped_weapon.is_some());
        assert!(cleric("Idris").equipped_weapon.is_some());
    }

    #[test]
    fn monsters_start_unarmed() {
        assert!(slime("Slime").equipped_weapon.is_none());
        assert!(orc("Orc").equipped_weapon.is_none());
    }

    #[test]
    fn equipping_a_weapon_returns_the_previous_one() {
        let mut hero = warrior("Bram");
        let starting_name = hero.equipped_weapon.as_ref().unwrap().name.clone();
        let returned = hero.equip_weapon(iron_sword());
        assert_eq!(returned.unwrap().name, starting_name);
        assert_eq!(hero.equipped_weapon.unwrap().name, "Iron Sword");
    }

    #[test]
    fn total_attack_includes_the_equipped_weapons_bonus() {
        let mut hero = warrior("Bram");
        let base = hero.stats.attack;
        hero.equip_weapon(iron_sword()); // +1 attack
        assert_eq!(hero.total_attack(), base + 1);
    }

    #[test]
    fn rarer_weapon_gives_a_bigger_total_attack() {
        let mut common_hero = warrior("Bram");
        common_hero.equip_weapon(iron_sword());
        let mut legendary_hero = warrior("Bram");
        legendary_hero.equip_weapon(dragonslayers_oath());
        assert!(legendary_hero.total_attack() > common_hero.total_attack());
    }

    #[test]
    fn the_boss_clearly_outclasses_the_toughest_regular_enemy() {
        let boss = barrow_knight("The Barrow Knight");
        let toughest_regular = orc("Orc");
        assert!(boss.stats.max_hp > toughest_regular.stats.max_hp);
        assert!(boss.stats.attack > toughest_regular.stats.attack);
        assert!(boss.stats.defense > toughest_regular.stats.defense);
    }

    #[test]
    fn only_the_boss_factory_sets_boss_kind() {
        assert_eq!(
            barrow_knight("The Barrow Knight").boss_kind,
            Some(BossKind::BarrowKnight)
        );
        assert!(orc("Orc").boss_kind.is_none());
        assert!(warrior("Bram").boss_kind.is_none());
    }

    #[test]
    fn each_chapter_boss_factory_sets_its_own_boss_kind() {
        assert_eq!(
            wyrmscale_warden("Wyrmscale Warden").boss_kind,
            Some(BossKind::WyrmscaleWarden)
        );
        assert_eq!(
            ashen_sovereign("The Ashen Sovereign").boss_kind,
            Some(BossKind::AshenSovereign)
        );
    }

    #[test]
    fn boss_stats_escalate_chapter_over_chapter() {
        let one = barrow_knight("The Barrow Knight");
        let two = wyrmscale_warden("Wyrmscale Warden");
        let three = ashen_sovereign("The Ashen Sovereign");

        assert!(two.stats.max_hp > one.stats.max_hp);
        assert!(two.stats.attack > one.stats.attack);
        assert!(three.stats.max_hp > two.stats.max_hp);
        assert!(three.stats.attack > two.stats.attack);
    }

    #[test]
    fn unequip_weapon_returns_it_and_leaves_the_slot_empty() {
        let mut hero = warrior("Bram");
        let returned = hero.unequip_weapon();
        assert_eq!(returned.unwrap().name, "Worn Shortsword");
        assert!(hero.equipped_weapon.is_none());
    }

    #[test]
    fn equip_armor_returns_the_previous_one() {
        let mut hero = warrior("Bram");
        assert!(hero.equip_armor(padded_vest()).is_none());
        let returned = hero.equip_armor(dragonscale_aegis());
        assert_eq!(returned.unwrap().name, "Padded Vest");
        assert_eq!(hero.equipped_armor.unwrap().name, "Dragonscale Aegis");
    }

    #[test]
    fn unequip_armor_empties_the_slot() {
        let mut hero = warrior("Bram");
        hero.equip_armor(padded_vest());
        let returned = hero.unequip_armor();
        assert_eq!(returned.unwrap().name, "Padded Vest");
        assert!(hero.equipped_armor.is_none());
    }

    #[test]
    fn equip_ring_into_each_slot_independently() {
        let mut hero = warrior("Bram");
        assert!(hero.equip_ring(RingSlot::First, copper_band()).is_none());
        assert!(hero
            .equip_ring(RingSlot::Second, band_of_the_barrow())
            .is_none());
        assert_eq!(hero.equipped_rings[0].as_ref().unwrap().name, "Copper Band");
        assert_eq!(
            hero.equipped_rings[1].as_ref().unwrap().name,
            "Band of the Barrow"
        );
    }

    #[test]
    fn unequip_ring_only_empties_the_targeted_slot() {
        let mut hero = warrior("Bram");
        hero.equip_ring(RingSlot::First, copper_band());
        hero.equip_ring(RingSlot::Second, band_of_the_barrow());
        let returned = hero.unequip_ring(RingSlot::First);
        assert_eq!(returned.unwrap().name, "Copper Band");
        assert!(hero.equipped_rings[0].is_none());
        assert!(hero.equipped_rings[1].is_some());
    }

    #[test]
    fn total_defense_folds_in_armor_and_both_rings() {
        let mut hero = warrior("Bram");
        let base = hero.total_defense();
        hero.equip_armor(padded_vest()); // +1 defense
        hero.equip_ring(RingSlot::First, copper_band()); // +0 defense
        hero.equip_ring(RingSlot::Second, band_of_the_barrow()); // +3 defense
        assert_eq!(hero.total_defense(), base + 1 + 3);
    }

    #[test]
    fn total_attack_folds_in_both_rings() {
        let mut hero = warrior("Bram");
        let base = hero.total_attack();
        hero.equip_ring(RingSlot::First, copper_band()); // +1 attack
        hero.equip_ring(RingSlot::Second, band_of_the_barrow()); // +3 attack
        assert_eq!(hero.total_attack(), base + 1 + 3);
    }

    #[test]
    fn a_fully_gearless_character_does_not_panic_and_uses_base_stats_only() {
        let mut hero = warrior("Bram");
        hero.unequip_weapon();
        assert_eq!(hero.total_attack(), hero.stats.attack);
        assert_eq!(hero.total_defense(), hero.stats.defense);
    }

    #[test]
    fn xp_to_next_level_is_strictly_increasing() {
        for level in 1..20 {
            assert!(xp_to_next_level(level + 1) > xp_to_next_level(level));
        }
    }

    #[test]
    fn gain_xp_below_threshold_grants_no_levels() {
        let mut hero = warrior("Bram");
        let levels = hero.gain_xp(xp_to_next_level(1) - 1);
        assert_eq!(levels, 0);
        assert_eq!(hero.level, 1);
        assert_eq!(hero.unspent_points, 0);
    }

    #[test]
    fn a_large_xp_award_can_grant_multiple_levels_at_once() {
        let mut hero = warrior("Bram");
        let needed = xp_to_next_level(1) + xp_to_next_level(2) + xp_to_next_level(3) + 5;
        let levels = hero.gain_xp(needed);
        assert_eq!(levels, 3);
        assert_eq!(hero.level, 4);
        assert_eq!(hero.unspent_points, POINTS_PER_LEVEL * 3);
        assert_eq!(hero.xp, 5);
    }

    #[test]
    fn gain_xp_stops_at_max_level() {
        let mut hero = warrior("Bram");
        hero.scale_to_level(MAX_LEVEL - 1);

        let levels = hero.gain_xp(1_000_000);
        assert_eq!(hero.level, MAX_LEVEL);
        assert_eq!(hero.xp, 0);
        assert_eq!(levels, 1);

        // Already capped: a further award grants nothing and doesn't
        // silently accumulate banked xp either.
        let levels_again = hero.gain_xp(1_000_000);
        assert_eq!(levels_again, 0);
        assert_eq!(hero.level, MAX_LEVEL);
        assert_eq!(hero.xp, 0);
    }

    #[test]
    fn gaining_a_level_heals_hp_and_mp_to_full() {
        let mut hero = warrior("Bram");
        hero.stats.hp = 1;
        hero.stats.mp = 0;
        hero.gain_xp(xp_to_next_level(1));
        assert_eq!(hero.stats.hp, hero.stats.max_hp);
        assert_eq!(hero.stats.mp, hero.stats.max_mp);
    }

    #[test]
    fn gaining_xp_without_a_level_up_does_not_heal() {
        let mut hero = warrior("Bram");
        hero.stats.hp = 1;
        hero.gain_xp(1);
        assert_eq!(hero.stats.hp, 1);
    }

    #[test]
    fn allocate_point_applies_the_class_gain_and_spends_a_point() {
        let mut hero = warrior("Bram");
        hero.unspent_points = 1;
        let base_hp = hero.stats.max_hp;
        assert!(hero.allocate_point(AllocStat::MaxHp));
        // The Warrior's signature HP payoff, straight from its AllocProfile.
        assert_eq!(hero.stats.max_hp, base_hp + 3);
        assert_eq!(hero.stats.hp, base_hp + 3, "current HP follows max");
        assert_eq!(hero.unspent_points, 0);
    }

    #[test]
    fn the_warrior_gets_more_hp_per_point_than_anyone_else() {
        let hp_gain = |mut c: Character| {
            c.unspent_points = 1;
            let before = c.stats.max_hp;
            assert!(c.allocate_point(AllocStat::MaxHp));
            c.stats.max_hp - before
        };
        let bram = hp_gain(warrior("Bram"));
        for other in [mage("Sella"), cleric("Idris"), rogue("Wren")] {
            assert!(bram > hp_gain(other));
        }
    }

    #[test]
    fn the_damage_dealers_get_more_attack_per_point_than_the_warrior() {
        let attack_gain = |mut c: Character| {
            c.unspent_points = 1;
            let before = c.stats.attack;
            assert!(c.allocate_point(AllocStat::Attack));
            c.stats.attack - before
        };
        let bram = attack_gain(warrior("Bram"));
        assert!(attack_gain(mage("Sella")) > bram);
        assert!(attack_gain(rogue("Wren")) > bram);
    }

    #[test]
    fn gains_are_halved_past_the_soft_cap() {
        let mut hero = warrior("Bram");
        let rule = alloc_profile(Class::Warrior).rule(AllocStat::MaxHp);
        let idx = ALLOC_STATS
            .iter()
            .position(|s| *s == AllocStat::MaxHp)
            .unwrap();
        hero.alloc_ranks[idx] = rule.soft_cap;
        hero.unspent_points = 1;
        let before = hero.stats.max_hp;
        assert_eq!(
            hero.alloc_preview(AllocStat::MaxHp),
            AllocPreview::Diminished((rule.gain + 1) / 2)
        );
        assert!(hero.allocate_point(AllocStat::MaxHp));
        assert_eq!(hero.stats.max_hp, before + (rule.gain + 1) / 2);
    }

    #[test]
    fn allocate_point_refuses_once_a_stat_hits_its_hard_cap() {
        // Every playable class/stat's invested *rank* is hard-capped at
        // `PLAYER_HARD_CAP` (50) — dumping hundreds of banked points into one
        // stat must eventually refuse rather than let it climb forever (the
        // exact bug this rescale fixes: unlimited stat growth from spamming
        // one stat). The derived value itself is not capped at 50 — it's
        // rank * gain, which differs by class.
        for mut hero in [warrior("B"), mage("S"), cleric("I"), rogue("W")] {
            for &stat in &ALLOC_STATS {
                hero.unspent_points = 500;
                while hero.allocate_point(stat) {}
                assert_eq!(
                    hero.rank(stat),
                    PLAYER_HARD_CAP,
                    "{:?}'s {stat} should settle exactly at the hard cap",
                    hero.class
                );
                assert_eq!(
                    hero.alloc_preview(stat),
                    AllocPreview::Capped,
                    "{:?}'s {stat} should refuse further points once capped",
                    hero.class
                );
            }
        }
    }

    #[test]
    fn allocate_point_keeps_the_point_banked_when_already_capped() {
        let mut hero = warrior("Bram");
        let idx = ALLOC_STATS
            .iter()
            .position(|s| *s == AllocStat::MaxHp)
            .unwrap();
        hero.alloc_ranks[idx] = PLAYER_HARD_CAP;
        hero.unspent_points = 3;
        let before = hero.stats.max_hp;
        assert!(!hero.allocate_point(AllocStat::MaxHp));
        assert_eq!(hero.unspent_points, 3, "a refused point must stay banked");
        assert_eq!(hero.stats.max_hp, before);
    }

    #[test]
    fn automatic_growth_alone_never_exceeds_the_hard_cap() {
        // The bug this whole rescale fixes: automatic per-level growth used
        // to be completely unbounded, so a character with zero points ever
        // spent on Attack could still reach a high Attack purely by leveling.
        // Now every stat's *rank* — grown or allocated — is clamped by the
        // same `invest_rank` chokepoint (the derived value it's worth is
        // uncapped and differs by class).
        for factory in [warrior as fn(&str) -> Character, mage, cleric, rogue] {
            let mut hero = factory("Zero-Alloc");
            for _ in 0..(MAX_LEVEL - 1) {
                hero.gain_xp(xp_to_next_level(hero.level));
            }
            for &stat in &ALLOC_STATS {
                assert!(
                    hero.rank(stat) <= PLAYER_HARD_CAP,
                    "{:?}'s {stat} exceeded the hard cap from growth alone: {}",
                    hero.class,
                    hero.rank(stat)
                );
            }
        }
    }

    #[test]
    fn the_same_maxed_rank_is_worth_a_different_derived_value_per_class() {
        // The whole point of the rank/value split: a Warrior and a Mage both
        // fully investing in Max HP (rank 50 for both) must not converge on
        // the same actual HP — that's the bug this rescale fixes.
        let mut warrior = warrior("Bram");
        let mut mage = mage("Sella");
        warrior.unspent_points = 500;
        mage.unspent_points = 500;
        while warrior.allocate_point(AllocStat::MaxHp) {}
        while mage.allocate_point(AllocStat::MaxHp) {}
        assert_eq!(warrior.rank(AllocStat::MaxHp), PLAYER_HARD_CAP);
        assert_eq!(mage.rank(AllocStat::MaxHp), PLAYER_HARD_CAP);
        assert!(
            warrior.stats.max_hp > mage.stats.max_hp,
            "same rank (50) should be worth more HP for the Warrior than the Mage: {} vs {}",
            warrior.stats.max_hp,
            mage.stats.max_hp
        );
    }

    #[test]
    fn allocate_point_with_no_points_does_nothing() {
        let mut hero = warrior("Bram");
        hero.unspent_points = 0;
        let stats_before = hero.stats;
        assert!(!hero.allocate_point(AllocStat::Luck));
        assert_eq!(hero.stats.luck, stats_before.luck);
    }

    #[test]
    fn total_luck_reflects_the_base_stat() {
        let hero = warrior("Bram");
        assert_eq!(hero.total_luck(), hero.stats.luck);
    }

    #[test]
    fn the_rogue_is_a_playable_class_with_a_weapon_and_abilities() {
        let vex = rogue("Wren");
        assert_eq!(vex.class, Class::Rogue);
        assert!(vex.equipped_weapon.is_some());
        assert_eq!(vex.abilities.len(), 3);
        assert!(vex.boss_kind.is_none());
    }

    #[test]
    fn the_rogue_is_faster_and_luckier_than_the_other_starters() {
        let vex = rogue("Wren");
        for other in [warrior("Bram"), mage("Sella"), cleric("Idris")] {
            assert!(vex.stats.speed > other.stats.speed);
            assert!(vex.stats.luck > other.stats.luck);
        }
    }

    #[test]
    fn each_class_grows_into_a_different_shape_on_level_up() {
        // Growth rates are fractional (see `LevelGrowth`'s doc comment) and
        // accumulate in `growth_carry`, so a single level-up often doesn't
        // cross a whole point yet — level up several times to let each
        // class's shape reliably show through.
        let level_up_ten_times = |mut c: Character| {
            for _ in 0..10 {
                c.gain_xp(xp_to_next_level(c.level));
            }
            c
        };
        let bram = level_up_ten_times(warrior("Bram"));
        let sella = level_up_ten_times(mage("Sella"));
        let wren = level_up_ten_times(rogue("Wren"));

        // Same ten level-ups, different automatic growth per class.
        assert!(
            bram.stats.max_hp - warrior("Bram").stats.max_hp
                > sella.stats.max_hp - mage("Sella").stats.max_hp,
            "the warrior should gain more HP per level than the mage"
        );
        assert!(
            sella.stats.max_mp - mage("Sella").stats.max_mp
                > bram.stats.max_mp - warrior("Bram").stats.max_mp,
            "the mage should gain more MP per level than the warrior"
        );
        assert!(
            wren.stats.speed - rogue("Wren").stats.speed
                > bram.stats.speed - warrior("Bram").stats.speed,
            "the rogue should gain more speed per level than the warrior"
        );
    }

    #[test]
    fn level_ups_grow_stats_automatically_on_top_of_banked_points() {
        let mut hero = warrior("Bram");
        let hp_before = hero.stats.max_hp;
        let atk_before = hero.stats.attack;
        // A single level-up's fractional growth (see `LevelGrowth`) doesn't
        // always cross a whole point immediately — level up enough times
        // that `growth_carry` is guaranteed to have produced at least one.
        for _ in 0..10 {
            hero.gain_xp(xp_to_next_level(hero.level));
        }
        assert!(
            hero.stats.max_hp > hp_before,
            "growth applies without spending points"
        );
        assert!(hero.stats.attack > atk_before);
        assert_eq!(
            hero.unspent_points,
            POINTS_PER_LEVEL * 10,
            "banked points still accrue"
        );
    }

    #[test]
    fn scale_to_level_toughens_a_monster_and_heals_it_to_full() {
        let mut enemy = orc("Orc");
        let base = enemy.stats;
        enemy.scale_to_level(5);
        assert_eq!(enemy.level, 5);
        assert!(enemy.stats.max_hp > base.max_hp);
        assert!(enemy.stats.attack > base.attack);
        assert!(enemy.stats.defense > base.defense);
        assert_eq!(enemy.stats.hp, enemy.stats.max_hp);
    }

    #[test]
    fn scale_to_level_never_lowers_a_level() {
        let mut enemy = orc("Orc");
        enemy.scale_to_level(5);
        let stats_at_five = enemy.stats;
        enemy.scale_to_level(2); // no-op: already past level 2
        assert_eq!(enemy.level, 5);
        assert_eq!(enemy.stats.max_hp, stats_at_five.max_hp);
    }

    #[test]
    fn ng_plus_multiplier_climbs_with_cycle_and_caps_at_seven() {
        assert_eq!(ng_plus_multiplier(0), 1.0);
        let six = ng_plus_multiplier(6);
        let seven = ng_plus_multiplier(7);
        let eight = ng_plus_multiplier(8);
        assert!(seven > six);
        assert_eq!(
            seven, eight,
            "NG+ beyond 7 should not keep scaling past the NG+7 cap"
        );
    }

    #[test]
    fn apply_ng_plus_is_a_no_op_at_cycle_zero() {
        let mut enemy = orc("Orc");
        let base = enemy.stats;
        enemy.apply_ng_plus(0);
        assert_eq!(enemy.stats.max_hp, base.max_hp);
        assert_eq!(enemy.stats.attack, base.attack);
        assert_eq!(enemy.stats.defense, base.defense);
    }

    #[test]
    fn apply_ng_plus_toughens_and_heals_a_monster() {
        let mut enemy = orc("Orc");
        let base = enemy.stats;
        enemy.stats.hp = 1; // simulate a near-death enemy before the buff
        enemy.apply_ng_plus(3);
        assert!(enemy.stats.max_hp > base.max_hp);
        assert!(enemy.stats.attack > base.attack);
        assert!(enemy.stats.defense > base.defense);
        assert!(
            enemy.stats.speed >= base.speed,
            "speed scales at half strength, never down"
        );
        assert!(
            enemy.stats.luck >= base.luck,
            "luck scales at half strength, never down"
        );
        assert_eq!(enemy.stats.hp, enemy.stats.max_hp, "heals to full");
    }

    #[test]
    fn apply_ng_plus_scales_speed_and_luck_at_half_strength() {
        let mut enemy = orc("Orc");
        let base = enemy.stats;
        enemy.apply_ng_plus(7);
        let full_mult = ng_plus_multiplier(7);
        let minor_mult = ng_plus_minor_multiplier(7);
        assert!(minor_mult < full_mult, "minor multiplier is the weaker curve");
        assert_eq!(
            enemy.stats.speed,
            ((base.speed as f32) * minor_mult).round() as i32
        );
        assert_eq!(
            enemy.stats.luck,
            ((base.luck as f32) * minor_mult).round() as i32
        );
    }

    #[test]
    fn apply_ng_plus_stacks_on_top_of_chapter_level_scaling() {
        let mut enemy = orc("Orc");
        enemy.scale_to_level(7);
        let after_level_scaling = enemy.stats;
        enemy.apply_ng_plus(2);
        assert!(enemy.stats.max_hp > after_level_scaling.max_hp);
        assert!(enemy.stats.attack > after_level_scaling.attack);
    }

    #[test]
    fn apply_elite_toughens_and_heals_and_flags_the_monster() {
        let mut enemy = orc("Orc");
        let base = enemy.stats;
        enemy.stats.hp = 1;
        enemy.apply_elite();
        assert!(enemy.is_elite);
        assert!(enemy.stats.max_hp > base.max_hp);
        assert!(enemy.stats.attack > base.attack);
        assert!(enemy.stats.defense > base.defense);
        assert_eq!(enemy.stats.hp, enemy.stats.max_hp, "heals to full");
    }

    #[test]
    fn display_name_prefixes_elite_but_name_stays_the_species() {
        let mut enemy = orc("Orc");
        assert_eq!(enemy.display_name(), "Orc");
        enemy.apply_elite();
        assert_eq!(enemy.display_name(), "Elite Orc");
        assert_eq!(enemy.name, "Orc", "raw name must stay the species tag");
    }

    #[test]
    fn elite_chance_climbs_with_ng_plus_and_caps_at_seven() {
        let base = Character::elite_chance(0);
        let mid = Character::elite_chance(3);
        let capped = Character::elite_chance(7);
        let beyond_cap = Character::elite_chance(20);
        assert!(mid > base);
        assert!(capped > mid);
        assert_eq!(capped, beyond_cap, "NG+ beyond 7 shouldn't keep raising the odds");
    }

    #[test]
    fn growth_multiplier_is_full_strength_through_level_ten() {
        assert_eq!(growth_multiplier(1), 1.0);
        assert_eq!(growth_multiplier(10), 1.0);
    }

    #[test]
    fn growth_multiplier_tapers_linearly_then_floors_at_half() {
        assert!((growth_multiplier(17) - 0.7667).abs() < 0.001); // 1.0 - 0.5*(17-10)/15
        assert_eq!(growth_multiplier(25), 0.5);
        assert_eq!(growth_multiplier(40), 0.5);
    }

    #[test]
    fn scaling_to_a_high_level_grows_slower_than_a_flat_curve_would() {
        let base_hp = orc("Orc").stats.max_hp;
        let mut tapered = orc("Orc");
        tapered.scale_to_level(30);
        let untapered_hp = base_hp + (base_hp as f32 * MONSTER_HP_GROWTH_RATE * 30.0).round() as i32;
        assert!(
            tapered.stats.max_hp < untapered_hp,
            "tapered growth should fall behind a naive flat-rate projection at high levels"
        );
    }

    #[test]
    fn scale_boss_to_party_is_a_no_op_at_or_under_baseline() {
        let mut boss = barrow_knight("The Barrow Knight");
        let base = boss.stats;
        boss.scale_boss_to_party(5, 5);
        assert_eq!(boss.stats.max_hp, base.max_hp);
        assert_eq!(boss.stats.attack, base.attack);
        assert_eq!(boss.stats.defense, base.defense);
        boss.scale_boss_to_party(3, 5);
        assert_eq!(boss.stats.max_hp, base.max_hp);
    }

    #[test]
    fn scale_boss_to_party_toughens_an_overleveled_fight() {
        let mut boss = barrow_knight("The Barrow Knight");
        let base = boss.stats;
        boss.stats.hp = 1;
        boss.scale_boss_to_party(10, 5);
        assert!(boss.stats.max_hp > base.max_hp);
        assert!(boss.stats.attack > base.attack);
        assert!(boss.stats.defense > base.defense);
        assert_eq!(boss.stats.hp, boss.stats.max_hp, "heals to full");
    }

    #[test]
    fn scale_boss_to_party_caps_at_twenty_excess_levels() {
        let mut at_cap = barrow_knight("The Barrow Knight");
        at_cap.scale_boss_to_party(25, 5); // exactly 20 excess
        let mut beyond_cap = barrow_knight("The Barrow Knight");
        beyond_cap.scale_boss_to_party(50, 5); // way more than 20 excess
        assert_eq!(at_cap.stats.max_hp, beyond_cap.stats.max_hp);
        assert_eq!(at_cap.stats.attack, beyond_cap.stats.attack);
    }

    #[test]
    fn effective_power_grows_with_a_stronger_weapon() {
        let mut hero = warrior("Bram");
        let ability = hero.abilities[0].clone();
        let before = ability.effective_power(&hero);
        hero.equip_weapon(dragonslayers_oath());
        let after = ability.effective_power(&hero);
        assert!(after > before);
    }
}
