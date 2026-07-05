use std::fmt;

#[derive(Debug, Clone, Copy)]
pub enum ItemKind {
    Potion { heal: i32 },
    Ether { mp: i32 },
}

#[derive(Debug, Clone)]
pub struct Item {
    pub name: String,
    pub kind: ItemKind,
    /// Base gold value. The shop buys at this price and sells for half.
    pub value: u32,
}

pub fn potion() -> Item {
    Item {
        name: "Potion".into(),
        kind: ItemKind::Potion { heal: 20 },
        value: 15,
    }
}

pub fn ether() -> Item {
    Item {
        name: "Ether".into(),
        kind: ItemKind::Ether { mp: 15 },
        value: 20,
    }
}

/// Factory function types shared by loot tables and the shop's fixed stock —
/// both just need "a function that conjures a fresh Item/Weapon" paired with
/// a probability or a price.
pub type ItemFactory = fn() -> Item;
pub type WeaponFactory = fn() -> Weapon;

/// Weapon rarity tier. Declared in ascending power order so `Rarity::Legendary`
/// is greater than `Rarity::Common` etc. via the derived `Ord` — the rarer a
/// weapon, the stronger it is, full stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

impl fmt::Display for Rarity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Rarity::Common => "Common",
            Rarity::Uncommon => "Uncommon",
            Rarity::Rare => "Rare",
            Rarity::Epic => "Epic",
            Rarity::Legendary => "Legendary",
        };
        write!(f, "{s}")
    }
}

impl Rarity {
    /// Gold value used by the shop: this is roughly what a weapon of this
    /// rarity costs to buy, and the shop pays half of it back on a sale.
    /// Epic/Legendary weapons are intentionally not part of the shop's fixed
    /// stock (see `game::shop::shop_weapon_stock`), but a found one can still
    /// be sold at this rate.
    pub fn base_value(&self) -> u32 {
        match self {
            Rarity::Common => 20,
            Rarity::Uncommon => 50,
            Rarity::Rare => 120,
            Rarity::Epic => 300,
            Rarity::Legendary => 800,
        }
    }
}

/// Where a weapon came from — purely descriptive, shown in the inventory
/// screen so the player knows how they got it.
#[derive(Debug, Clone)]
pub enum WeaponSource {
    /// Carried from the start of the game.
    Starting,
    /// Found while exploring the field (hidden caches, buried relics).
    World,
    /// Dropped by a specific enemy species after combat.
    EnemyDrop(&'static str),
}

impl fmt::Display for WeaponSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WeaponSource::Starting => write!(f, "Starting gear"),
            WeaponSource::World => write!(f, "Found in the world"),
            WeaponSource::EnemyDrop(species) => write!(f, "Dropped by {species}"),
        }
    }
}

/// A wieldable weapon. Every playable character always has exactly one
/// equipped (`Character::equipped_weapon`); better ones are found while
/// exploring the field or won by defeating certain enemies. Rarity is a
/// straightforward power ladder — the rarer the weapon, the stronger it is.
#[derive(Debug, Clone)]
pub struct Weapon {
    pub name: String,
    pub rarity: Rarity,
    pub attack_bonus: i32,
    pub defense_bonus: i32,
    pub description: String,
    pub source: WeaponSource,
}

// --- Weapon factory functions, grouped by rarity tier. ---
// Bonuses scale with rarity so "rarer = better" holds at a glance:
// Common ~2-4, Uncommon ~5-7, Rare ~9-11, Epic ~15, Legendary ~22.

pub fn worn_shortsword() -> Weapon {
    Weapon {
        name: "Worn Shortsword".into(),
        rarity: Rarity::Common,
        attack_bonus: 2,
        defense_bonus: 0,
        description: "A nicked, well-used blade. Better than fists.".into(),
        source: WeaponSource::Starting,
    }
}

pub fn apprentice_wand() -> Weapon {
    Weapon {
        name: "Apprentice's Wand".into(),
        rarity: Rarity::Common,
        attack_bonus: 1,
        defense_bonus: 0,
        description: "A simple wand every mage starts with.".into(),
        source: WeaponSource::Starting,
    }
}

pub fn acolytes_mace() -> Weapon {
    Weapon {
        name: "Acolyte's Mace".into(),
        rarity: Rarity::Common,
        attack_bonus: 2,
        defense_bonus: 1,
        description: "Blunt and humble, blessed only lightly.".into(),
        source: WeaponSource::Starting,
    }
}

pub fn iron_sword() -> Weapon {
    Weapon {
        name: "Iron Sword".into(),
        rarity: Rarity::Common,
        attack_bonus: 3,
        defense_bonus: 0,
        description: "A sturdy, mass-produced blade left behind by a traveler.".into(),
        source: WeaponSource::World,
    }
}

pub fn goblin_shiv() -> Weapon {
    Weapon {
        name: "Goblin Shiv".into(),
        rarity: Rarity::Common,
        attack_bonus: 4,
        defense_bonus: 0,
        description: "Crude but wickedly sharp.".into(),
        source: WeaponSource::EnemyDrop("Goblin"),
    }
}

pub fn travelers_spear() -> Weapon {
    Weapon {
        name: "Traveler's Spear".into(),
        rarity: Rarity::Uncommon,
        attack_bonus: 6,
        defense_bonus: 1,
        description: "Light, well-balanced, and easy to keep an enemy at range.".into(),
        source: WeaponSource::World,
    }
}

pub fn bone_blade() -> Weapon {
    Weapon {
        name: "Bone Blade".into(),
        rarity: Rarity::Uncommon,
        attack_bonus: 7,
        defense_bonus: 0,
        description: "Carved from a fallen skeleton's own femur.".into(),
        source: WeaponSource::EnemyDrop("Skeleton"),
    }
}

pub fn orcish_greataxe() -> Weapon {
    Weapon {
        name: "Orcish Greataxe".into(),
        rarity: Rarity::Rare,
        attack_bonus: 10,
        defense_bonus: 0,
        description: "Heavy enough to fell a tree in one swing.".into(),
        source: WeaponSource::EnemyDrop("Orc"),
    }
}

pub fn wraithbane_edge() -> Weapon {
    Weapon {
        name: "Wraithbane Edge".into(),
        rarity: Rarity::Rare,
        attack_bonus: 9,
        defense_bonus: 2,
        description: "Etched with wards that flare hot near the restless dead.".into(),
        source: WeaponSource::EnemyDrop("Wraith"),
    }
}

pub fn sunken_relic_blade() -> Weapon {
    Weapon {
        name: "Sunken Relic Blade".into(),
        rarity: Rarity::Rare,
        attack_bonus: 11,
        defense_bonus: 0,
        description: "Pulled from somewhere it should never have been found.".into(),
        source: WeaponSource::World,
    }
}

pub fn mimics_fang() -> Weapon {
    Weapon {
        name: "Mimic's Fang".into(),
        rarity: Rarity::Epic,
        attack_bonus: 15,
        defense_bonus: 1,
        description: "Still faintly warm. It was a tooth a moment ago.".into(),
        source: WeaponSource::EnemyDrop("Mimic"),
    }
}

pub fn dragonslayers_oath() -> Weapon {
    Weapon {
        name: "Dragonslayer's Oath".into(),
        rarity: Rarity::Legendary,
        attack_bonus: 22,
        defense_bonus: 3,
        description: "A greatsword said to have ended an age. Absurdly rare to find.".into(),
        source: WeaponSource::World,
    }
}

/// The Barrow Knight's signature weapon — the single strongest item in the
/// game, and the only reward for beating the boss lair. Unlike everything
/// else in `loot_profile`, this drops every time; the boss fight itself is
/// the gate, not a dice roll on top of it.
pub fn knightsbane() -> Weapon {
    Weapon {
        name: "Knightsbane".into(),
        rarity: Rarity::Legendary,
        attack_bonus: 24,
        defense_bonus: 4,
        description: "Forged in the barrow's cold fire; still hums with the Knight's fury.".into(),
        source: WeaponSource::EnemyDrop("The Barrow Knight"),
    }
}

pub struct Inventory {
    // (item, quantity)
    pub items: Vec<(Item, u32)>,
    /// Weapons currently carried but not equipped by anyone.
    pub weapons: Vec<Weapon>,
}

impl Inventory {
    pub fn starting() -> Self {
        Self {
            items: vec![(potion(), 3), (ether(), 2)],
            weapons: Vec::new(),
        }
    }

    pub fn add(&mut self, item: Item, qty: u32) {
        if let Some(existing) = self.items.iter_mut().find(|(i, _)| i.name == item.name) {
            existing.1 += qty;
        } else {
            self.items.push((item, qty));
        }
    }

    pub fn add_weapon(&mut self, weapon: Weapon) {
        self.weapons.push(weapon);
    }

    /// Decrements the count of the item at `idx`, removing the slot if it hits zero.
    /// Returns the item's kind if one was consumed.
    pub fn use_at(&mut self, idx: usize) -> Option<ItemKind> {
        let (item, qty) = self.items.get_mut(idx)?;
        let kind = item.kind;
        *qty -= 1;
        if *qty == 0 {
            self.items.remove(idx);
        }
        Some(kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rarity_ordering_ranks_legendary_highest() {
        assert!(Rarity::Legendary > Rarity::Epic);
        assert!(Rarity::Epic > Rarity::Rare);
        assert!(Rarity::Rare > Rarity::Uncommon);
        assert!(Rarity::Uncommon > Rarity::Common);
    }

    #[test]
    fn add_weapon_appends_to_inventory() {
        let mut inv = Inventory::starting();
        assert!(inv.weapons.is_empty());
        inv.add_weapon(iron_sword());
        assert_eq!(inv.weapons.len(), 1);
        assert_eq!(inv.weapons[0].name, "Iron Sword");
    }

    #[test]
    fn rarer_weapons_have_bigger_bonuses_within_their_family() {
        // Not a universal law across every hand-authored weapon, but the
        // common -> legendary world-find ladder should clearly climb.
        assert!(iron_sword().attack_bonus < sunken_relic_blade().attack_bonus);
        assert!(sunken_relic_blade().attack_bonus < dragonslayers_oath().attack_bonus);
    }

    #[test]
    fn base_value_climbs_with_rarity() {
        assert!(Rarity::Common.base_value() < Rarity::Uncommon.base_value());
        assert!(Rarity::Uncommon.base_value() < Rarity::Rare.base_value());
        assert!(Rarity::Rare.base_value() < Rarity::Epic.base_value());
        assert!(Rarity::Epic.base_value() < Rarity::Legendary.base_value());
    }

    #[test]
    fn knightsbane_is_the_single_strongest_weapon() {
        let contenders = [
            worn_shortsword(),
            iron_sword(),
            goblin_shiv(),
            travelers_spear(),
            bone_blade(),
            orcish_greataxe(),
            wraithbane_edge(),
            sunken_relic_blade(),
            mimics_fang(),
            dragonslayers_oath(),
        ];
        let knightsbane_atk = knightsbane().attack_bonus;
        assert!(
            contenders.iter().all(|w| w.attack_bonus < knightsbane_atk),
            "the boss's signature weapon should outclass every other weapon in the game"
        );
    }
}
