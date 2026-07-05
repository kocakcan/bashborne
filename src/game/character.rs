use std::fmt;

use crate::game::item::{acolytes_mace, apprentice_wand, worn_shortsword, Weapon};

#[derive(Debug, Clone, Copy)]
pub struct Stats {
    pub max_hp: i32,
    pub hp: i32,
    pub max_mp: i32,
    pub mp: i32,
    pub attack: i32,
    pub defense: i32,
    pub speed: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    Warrior,
    Mage,
    #[allow(dead_code)] // reserved for a future playable class
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

#[derive(Debug, Clone, Copy)]
pub enum AbilityKind {
    PhysicalDamage,
    MagicDamage,
    Heal,
}

#[derive(Debug, Clone)]
pub struct Ability {
    pub name: String,
    pub mp_cost: i32,
    pub power: i32,
    pub kind: AbilityKind,
}

#[derive(Debug, Clone)]
pub struct Character {
    pub name: String,
    pub class: Class,
    pub level: u32,
    pub stats: Stats,
    pub abilities: Vec<Ability>,
    /// The weapon this character currently wields. Always populated for
    /// playable characters (see the class factory functions below); monsters
    /// leave this `None` since they fight bare-handed/claw/fang.
    pub equipped_weapon: Option<Weapon>,
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

    /// Whether the ability at `idx` targets an ally (heal) rather than an enemy.
    /// Returns false (i.e. "targets an enemy") if the index is out of range,
    /// so callers can default to enemy-targeting UI safely.
    pub fn ability_is_heal(&self, idx: usize) -> bool {
        self.abilities
            .get(idx)
            .map(|a| matches!(a.kind, AbilityKind::Heal))
            .unwrap_or(false)
    }

    /// Attack stat plus whatever bonus the equipped weapon grants. This is
    /// what basic "Attack" actions roll damage from — abilities scale off the
    /// raw stat instead, since spells don't care which sword you're holding.
    pub fn total_attack(&self) -> i32 {
        self.stats.attack
            + self
                .equipped_weapon
                .as_ref()
                .map(|w| w.attack_bonus)
                .unwrap_or(0)
    }

    /// Defense stat plus whatever bonus the equipped weapon grants.
    pub fn total_defense(&self) -> i32 {
        self.stats.defense
            + self
                .equipped_weapon
                .as_ref()
                .map(|w| w.defense_bonus)
                .unwrap_or(0)
    }

    /// Equips `weapon`, returning whatever was previously equipped (if
    /// anything) so the caller can return it to the party's inventory.
    pub fn equip_weapon(&mut self, weapon: Weapon) -> Option<Weapon> {
        self.equipped_weapon.replace(weapon)
    }
}

// --- Starting party factory functions ---

pub fn warrior(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Warrior,
        level: 1,
        stats: Stats {
            max_hp: 42,
            hp: 42,
            max_mp: 10,
            mp: 10,
            attack: 12,
            defense: 8,
            speed: 6,
        },
        abilities: vec![
            Ability {
                name: "Power Strike".into(),
                mp_cost: 3,
                power: 10,
                kind: AbilityKind::PhysicalDamage,
            },
            Ability {
                name: "Crushing Blow".into(),
                mp_cost: 6,
                power: 18,
                kind: AbilityKind::PhysicalDamage,
            },
        ],
        equipped_weapon: Some(worn_shortsword()),
    }
}

pub fn mage(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Mage,
        level: 1,
        stats: Stats {
            max_hp: 26,
            hp: 26,
            max_mp: 30,
            mp: 30,
            attack: 5,
            defense: 4,
            speed: 8,
        },
        abilities: vec![
            Ability {
                name: "Firebolt".into(),
                mp_cost: 6,
                power: 16,
                kind: AbilityKind::MagicDamage,
            },
            Ability {
                name: "Lightning Bolt".into(),
                mp_cost: 12,
                power: 26,
                kind: AbilityKind::MagicDamage,
            },
        ],
        equipped_weapon: Some(apprentice_wand()),
    }
}

pub fn cleric(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Cleric,
        level: 1,
        stats: Stats {
            max_hp: 32,
            hp: 32,
            max_mp: 24,
            mp: 24,
            attack: 6,
            defense: 6,
            speed: 7,
        },
        abilities: vec![
            Ability {
                name: "Mend".into(),
                mp_cost: 8,
                power: 18,
                kind: AbilityKind::Heal,
            },
            Ability {
                name: "Smite".into(),
                mp_cost: 5,
                power: 12,
                kind: AbilityKind::PhysicalDamage,
            },
        ],
        equipped_weapon: Some(acolytes_mace()),
    }
}

pub fn slime(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Monster,
        level: 1,
        stats: Stats {
            max_hp: 18,
            hp: 18,
            max_mp: 0,
            mp: 0,
            attack: 7,
            defense: 3,
            speed: 4,
        },
        abilities: vec![],
        equipped_weapon: None,
    }
}

pub fn goblin(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Monster,
        level: 1,
        stats: Stats {
            max_hp: 24,
            hp: 24,
            max_mp: 0,
            mp: 0,
            attack: 9,
            defense: 5,
            speed: 9,
        },
        abilities: vec![],
        equipped_weapon: None,
    }
}

pub fn bat(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Monster,
        level: 1,
        stats: Stats {
            max_hp: 14,
            hp: 14,
            max_mp: 0,
            mp: 0,
            attack: 6,
            defense: 2,
            speed: 12,
        },
        abilities: vec![],
        equipped_weapon: None,
    }
}

pub fn wolf(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Monster,
        level: 1,
        stats: Stats {
            max_hp: 22,
            hp: 22,
            max_mp: 0,
            mp: 0,
            attack: 10,
            defense: 4,
            speed: 10,
        },
        abilities: vec![],
        equipped_weapon: None,
    }
}

pub fn skeleton(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Monster,
        level: 1,
        stats: Stats {
            max_hp: 30,
            hp: 30,
            max_mp: 0,
            mp: 0,
            attack: 8,
            defense: 9,
            speed: 3,
        },
        abilities: vec![],
        equipped_weapon: None,
    }
}

pub fn orc(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Monster,
        level: 1,
        stats: Stats {
            max_hp: 36,
            hp: 36,
            max_mp: 0,
            mp: 0,
            attack: 13,
            defense: 7,
            speed: 5,
        },
        abilities: vec![],
        equipped_weapon: None,
    }
}

/// Its attack occasionally curses the party instead of dealing damage —
/// see `CombatState::resolve_enemy_action`.
pub fn wraith(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Monster,
        level: 1,
        stats: Stats {
            max_hp: 20,
            hp: 20,
            max_mp: 0,
            mp: 0,
            attack: 7,
            defense: 5,
            speed: 7,
        },
        abilities: vec![],
        equipped_weapon: None,
    }
}

/// Disguised as treasure until it ambushes you — see `FieldEvent` in `game/state.rs`.
pub fn mimic(name: &str) -> Character {
    Character {
        name: name.to_string(),
        class: Class::Monster,
        level: 1,
        stats: Stats {
            max_hp: 30,
            hp: 30,
            max_mp: 0,
            mp: 0,
            attack: 12,
            defense: 8,
            speed: 4,
        },
        abilities: vec![],
        equipped_weapon: None,
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
        stats: Stats {
            max_hp: 90,
            hp: 90,
            max_mp: 0,
            mp: 0,
            attack: 16,
            defense: 10,
            speed: 6,
        },
        abilities: vec![],
        equipped_weapon: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::item::{dragonslayers_oath, iron_sword};

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
}
