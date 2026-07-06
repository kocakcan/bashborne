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

/// How one allocated point behaves for one stat: the full-value gain, the
/// soft cap past which the gain is halved (rounded up), and the hard cap at
/// which allocation is refused outright. Caps compare against the *base*
/// stat (`stats.attack`, not `total_attack()`), so gear never eats headroom;
/// automatic `level_growth` also ignores caps entirely — they only govern
/// hand-spent points.
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
    AllocRule {
        gain,
        soft_cap,
        hard_cap,
    }
}

/// Each class's allocation table. Hard caps are tuned to always clear the
/// level-14 automatic-growth trajectory (asserted by a test below), so
/// leveling alone can never lock a stat out of further investment.
pub fn alloc_profile(class: Class) -> AllocProfile {
    match class {
        // Off-tank: the best HP-per-point in the party *and* a real attack
        // gain, at the price of mediocre MP/speed/luck payoff.
        Class::Warrior => AllocProfile {
            max_hp: rule(7, 170, 220),
            max_mp: rule(2, 30, 40),
            attack: rule(2, 45, 60),
            defense: rule(2, 40, 50),
            speed: rule(1, 15, 20),
            luck: rule(1, 15, 20),
        },
        // Glass cannon: top-tier attack and MP payoff, the frailest HP/defense.
        Class::Mage => AllocProfile {
            max_hp: rule(3, 80, 100),
            max_mp: rule(4, 110, 140),
            attack: rule(3, 40, 55),
            defense: rule(1, 22, 30),
            speed: rule(1, 25, 32),
            luck: rule(1, 25, 32),
        },
        // Support: balanced, durable-leaning, with no standout damage payoff.
        Class::Cleric => AllocProfile {
            max_hp: rule(5, 120, 150),
            max_mp: rule(3, 90, 115),
            attack: rule(2, 35, 45),
            defense: rule(2, 38, 48),
            speed: rule(1, 16, 22),
            luck: rule(1, 20, 26),
        },
        // Striker: the highest attack ceiling plus double-value speed/luck.
        Class::Rogue => AllocProfile {
            max_hp: rule(4, 100, 125),
            max_mp: rule(2, 45, 60),
            attack: rule(3, 48, 62),
            defense: rule(1, 24, 32),
            speed: rule(2, 42, 52),
            luck: rule(2, 34, 44),
        },
        // Monsters never allocate; keep the old flat, uncapped table so
        // nothing changes if one ever does.
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
    /// At/past the hard cap: allocation refused, the point is kept.
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
#[derive(Debug, Clone, Copy)]
pub struct LevelGrowth {
    pub max_hp: i32,
    pub max_mp: i32,
    pub attack: i32,
    pub defense: i32,
    pub speed: i32,
    pub luck: i32,
}

/// Each class's growth profile. `Monster` gets a flat, modest curve — it's
/// what `Character::scale_to_level` uses to toughen enemies up for later
/// chapters, not something the player ever sees on a party member.
pub fn level_growth(class: Class) -> LevelGrowth {
    match class {
        Class::Warrior => LevelGrowth {
            max_hp: 7,
            max_mp: 1,
            attack: 2,
            defense: 2,
            speed: 0,
            luck: 0,
        },
        Class::Mage => LevelGrowth {
            max_hp: 3,
            max_mp: 5,
            attack: 1,
            defense: 1,
            speed: 1,
            luck: 1,
        },
        Class::Rogue => LevelGrowth {
            max_hp: 4,
            max_mp: 2,
            attack: 2,
            defense: 1,
            speed: 2,
            luck: 2,
        },
        Class::Cleric => LevelGrowth {
            max_hp: 5,
            max_mp: 4,
            attack: 1,
            defense: 2,
            speed: 0,
            luck: 1,
        },
        Class::Monster => LevelGrowth {
            max_hp: 5,
            max_mp: 0,
            attack: 1,
            defense: 1,
            speed: 0,
            luck: 0,
        },
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
}

impl Character {
    pub fn is_alive(&self) -> bool {
        self.stats.hp > 0
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

    /// Adds `amount` XP, applying as many level-ups as it covers (a single
    /// large award — e.g. a boss kill — can cross several thresholds at
    /// once). Returns the number of levels gained, 0 if none. Any level gained
    /// fully restores HP and MP, so leveling up always feels like a reward
    /// rather than leaving the party mid-fight at whatever HP it was at.
    pub fn gain_xp(&mut self, amount: u32) -> u32 {
        self.xp += amount;
        let mut levels = 0;
        while self.xp >= xp_to_next_level(self.level) {
            self.xp -= xp_to_next_level(self.level);
            self.level += 1;
            self.unspent_points += POINTS_PER_LEVEL;
            self.apply_level_growth();
            levels += 1;
        }
        if levels > 0 {
            self.stats.hp = self.stats.max_hp;
            self.stats.mp = self.stats.max_mp;
        }
        levels
    }

    /// Applies one level's worth of this class's automatic growth (see
    /// `level_growth`). Current HP/MP grow alongside their maxima so the
    /// bump is never a phantom gain that still has to be healed up to.
    fn apply_level_growth(&mut self) {
        let g = level_growth(self.class);
        self.stats.max_hp += g.max_hp;
        self.stats.hp += g.max_hp;
        self.stats.max_mp += g.max_mp;
        self.stats.mp += g.max_mp;
        self.stats.attack += g.attack;
        self.stats.defense += g.defense;
        self.stats.speed += g.speed;
        self.stats.luck += g.luck;
    }

    /// Raises this character straight to `level` (if higher than its current
    /// one), applying its class's automatic growth for every level gained
    /// and topping HP/MP off. This is how later chapters toughen up their
    /// regular monsters — same species, more dangerous specimen — and since
    /// `combat::xp_value` reads the resulting stats, XP rewards scale along
    /// with the threat automatically.
    pub fn scale_to_level(&mut self, level: u32) {
        while self.level < level {
            self.level += 1;
            self.apply_level_growth();
        }
        self.stats.hp = self.stats.max_hp;
        self.stats.mp = self.stats.max_mp;
    }

    /// The current *base* value of an allocatable stat — what the caps in
    /// `alloc_profile` compare against (deliberately not `total_attack()`
    /// etc., so gear bonuses never eat allocation headroom).
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

    /// What spending one point on `stat` would do right now, given this
    /// class's `alloc_profile` and the stat's current base value.
    pub fn alloc_preview(&self, stat: AllocStat) -> AllocPreview {
        let rule = alloc_profile(self.class).rule(stat);
        let current = self.base_stat(stat);
        if current >= rule.hard_cap {
            AllocPreview::Capped
        } else if current >= rule.soft_cap {
            AllocPreview::Diminished((rule.gain + 1) / 2)
        } else {
            AllocPreview::Full(rule.gain)
        }
    }

    /// Spends one banked point on `stat`, if any are available and the stat
    /// isn't hard-capped. Returns whether a point was spent — a refusal
    /// (no points, or capped) leaves `unspent_points` untouched.
    pub fn allocate_point(&mut self, stat: AllocStat) -> bool {
        if self.unspent_points == 0 {
            return false;
        }
        let gain = match self.alloc_preview(stat) {
            AllocPreview::Capped => return false,
            AllocPreview::Full(n) | AllocPreview::Diminished(n) => n,
        };
        self.unspent_points -= 1;
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
        true
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

pub fn warrior(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Warrior,
        level: 1,
        xp: 0,
        unspent_points: 0,
        stats: Stats {
            max_hp: 42,
            hp: 42,
            max_mp: 10,
            mp: 10,
            attack: 12,
            defense: 8,
            speed: 6,
            luck: 5,
        },
        abilities: vec![
            Ability {
                name: "Power Strike".into(),
                mp_cost: 3,
                base_power: 10,
                kind: AbilityKind::PhysicalDamage,
            },
            Ability {
                name: "Crushing Blow".into(),
                mp_cost: 6,
                base_power: 18,
                kind: AbilityKind::PhysicalDamage,
            },
        ],
        equipped_weapon: Some(worn_shortsword()),
        equipped_armor: None,
        equipped_rings: [None, None],
        boss_kind: None,
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
            max_hp: 26,
            hp: 26,
            max_mp: 30,
            mp: 30,
            attack: 5,
            defense: 4,
            speed: 8,
            luck: 6,
        },
        abilities: vec![
            Ability {
                name: "Firebolt".into(),
                mp_cost: 6,
                base_power: 16,
                kind: AbilityKind::MagicDamage,
            },
            Ability {
                name: "Lightning Bolt".into(),
                mp_cost: 12,
                base_power: 26,
                kind: AbilityKind::MagicDamage,
            },
        ],
        equipped_weapon: Some(apprentice_wand()),
        equipped_armor: None,
        equipped_rings: [None, None],
        boss_kind: None,
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
            max_hp: 32,
            hp: 32,
            max_mp: 24,
            mp: 24,
            attack: 6,
            defense: 6,
            speed: 7,
            luck: 4,
        },
        abilities: vec![
            Ability {
                name: "Mend".into(),
                mp_cost: 8,
                base_power: 18,
                kind: AbilityKind::Heal,
            },
            Ability {
                name: "Smite".into(),
                mp_cost: 5,
                base_power: 12,
                kind: AbilityKind::PhysicalDamage,
            },
        ],
        equipped_weapon: Some(acolytes_mace()),
        equipped_armor: None,
        equipped_rings: [None, None],
        boss_kind: None,
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
            max_hp: 30,
            hp: 30,
            max_mp: 14,
            mp: 14,
            attack: 9,
            defense: 5,
            speed: 11,
            luck: 12,
        },
        abilities: vec![
            Ability {
                name: "Backstab".into(),
                mp_cost: 4,
                base_power: 12,
                kind: AbilityKind::PhysicalDamage,
            },
            Ability {
                name: "Fan of Knives".into(),
                mp_cost: 9,
                base_power: 22,
                kind: AbilityKind::PhysicalDamage,
            },
        ],
        equipped_weapon: Some(thieves_dirk()),
        equipped_armor: None,
        equipped_rings: [None, None],
        boss_kind: None,
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
    }
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
            max_hp: 130,
            hp: 130,
            max_mp: 0,
            mp: 0,
            attack: 20,
            defense: 13,
            speed: 8,
            luck: 10,
        },
        abilities: vec![],
        equipped_weapon: None,
        equipped_armor: None,
        equipped_rings: [None, None],
        boss_kind: Some(BossKind::WyrmscaleWarden),
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
            max_hp: 170,
            hp: 170,
            max_mp: 0,
            mp: 0,
            attack: 24,
            defense: 15,
            speed: 10,
            luck: 14,
        },
        abilities: vec![],
        equipped_weapon: None,
        equipped_armor: None,
        equipped_rings: [None, None],
        boss_kind: Some(BossKind::AshenSovereign),
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
        hero.equip_weapon(iron_sword()); // +3 attack
        assert_eq!(hero.total_attack(), base + 3);
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
        hero.equip_armor(padded_vest()); // +2 defense
        hero.equip_ring(RingSlot::First, copper_band()); // +0 defense
        hero.equip_ring(RingSlot::Second, band_of_the_barrow()); // +6 defense
        assert_eq!(hero.total_defense(), base + 2 + 6);
    }

    #[test]
    fn total_attack_folds_in_both_rings() {
        let mut hero = warrior("Bram");
        let base = hero.total_attack();
        hero.equip_ring(RingSlot::First, copper_band()); // +2 attack
        hero.equip_ring(RingSlot::Second, band_of_the_barrow()); // +6 attack
        assert_eq!(hero.total_attack(), base + 2 + 6);
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
        assert_eq!(hero.stats.max_hp, base_hp + 7);
        assert_eq!(hero.stats.hp, base_hp + 7, "current HP follows max");
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
        hero.stats.max_hp = rule.soft_cap;
        hero.stats.hp = rule.soft_cap;
        hero.unspent_points = 1;
        assert_eq!(
            hero.alloc_preview(AllocStat::MaxHp),
            AllocPreview::Diminished((rule.gain + 1) / 2)
        );
        assert!(hero.allocate_point(AllocStat::MaxHp));
        assert_eq!(hero.stats.max_hp, rule.soft_cap + (rule.gain + 1) / 2);
    }

    #[test]
    fn the_hard_cap_refuses_the_point_and_keeps_it() {
        let mut hero = rogue("Wren");
        let rule = alloc_profile(Class::Rogue).rule(AllocStat::Attack);
        hero.stats.attack = rule.hard_cap;
        hero.unspent_points = 2;
        assert_eq!(hero.alloc_preview(AllocStat::Attack), AllocPreview::Capped);
        assert!(!hero.allocate_point(AllocStat::Attack));
        assert_eq!(hero.stats.attack, rule.hard_cap);
        assert_eq!(hero.unspent_points, 2, "a refused point must not be spent");
        // The kept point is still spendable elsewhere.
        assert!(hero.allocate_point(AllocStat::Speed));
        assert_eq!(hero.unspent_points, 1);
    }

    #[test]
    fn every_hard_cap_clears_the_level_14_auto_growth_trajectory() {
        // Automatic level growth ignores caps, but if it could carry a base
        // stat past its hard cap on its own, leveling would silently lock
        // that stat out of allocation. Guard the tuning against that.
        for hero in [warrior("B"), mage("S"), cleric("I"), rogue("W")] {
            let mut grown = hero.clone();
            grown.scale_to_level(14);
            let profile = alloc_profile(hero.class);
            for stat in ALLOC_STATS {
                assert!(
                    grown.base_stat(stat) < profile.rule(stat).hard_cap,
                    "{:?} {stat} reaches its hard cap by level 14 via growth alone",
                    hero.class
                );
            }
        }
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
        assert_eq!(vex.abilities.len(), 2);
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
        let level_once = |mut c: Character| {
            c.gain_xp(xp_to_next_level(1));
            c
        };
        let bram = level_once(warrior("Bram"));
        let sella = level_once(mage("Sella"));
        let wren = level_once(rogue("Wren"));

        // Same level-up, different automatic growth per class.
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
        hero.gain_xp(xp_to_next_level(1));
        assert!(
            hero.stats.max_hp > hp_before,
            "growth applies without spending points"
        );
        assert!(hero.stats.attack > atk_before);
        assert_eq!(
            hero.unspent_points, POINTS_PER_LEVEL,
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
    fn effective_power_grows_with_a_stronger_weapon() {
        let mut hero = warrior("Bram");
        let ability = hero.abilities[0].clone();
        let before = ability.effective_power(&hero);
        hero.equip_weapon(dragonslayers_oath());
        let after = ability.effective_power(&hero);
        assert!(after > before);
    }
}
