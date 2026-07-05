use std::fmt;

use crate::game::chapter::BossKind;
use crate::game::item::{acolytes_mace, apprentice_wand, worn_shortsword, Armor, Ring, Weapon};

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

/// Which of a character's two ring slots is being addressed. Two slots
/// (rather than a bare index) so equip/unequip call sites read as intent,
/// not magic numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RingSlot {
    First,
    Second,
}

#[derive(Debug, Clone)]
pub struct Character {
    pub name: String,
    pub class: Class,
    pub level: u32,
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
        stats: Stats {
            max_hp: 130,
            hp: 130,
            max_mp: 0,
            mp: 0,
            attack: 20,
            defense: 13,
            speed: 8,
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
        stats: Stats {
            max_hp: 170,
            hp: 170,
            max_mp: 0,
            mp: 0,
            attack: 24,
            defense: 15,
            speed: 10,
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
        assert_eq!(
            hero.equipped_rings[0].as_ref().unwrap().name,
            "Copper Band"
        );
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
}
