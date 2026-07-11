use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ItemKind {
    /// Heals a fraction of the target's max HP, so potency scales with the
    /// target's stats (a leveled-up character with more max HP heals for
    /// more) rather than staying a flat number forever.
    Potion { heal_percent: f32 },
    /// Restores a fraction of the target's max MP — same reasoning as `Potion`.
    Ether { mp_percent: f32 },
    /// Fully restores the target's HP and MP in one draught.
    Elixir,
    /// Brings a fallen ally back at this fraction of max HP. Useless on the
    /// living. (A member revived mid-fight rejoins the turn order next fight.)
    Revive { heal_percent: f32 },
    /// Strips every active curse (negative status effect) from the party.
    /// Party-wide by nature, so it never asks for a target.
    CureCurse,
}

impl ItemKind {
    /// What this item actually does, in combat-menu-appropriate terms —
    /// used where the player is deciding whether to use it (the combat Item
    /// menu), as opposed to `Item::description`'s flavor text (used where
    /// the player is just browsing, e.g. the inventory screen).
    pub fn purpose_description(&self) -> String {
        match self {
            ItemKind::Potion { heal_percent } => format!("Heals {:.0}% HP", heal_percent * 100.0),
            ItemKind::Ether { mp_percent } => format!("Restores {:.0}% MP", mp_percent * 100.0),
            ItemKind::Elixir => "Fully restores HP and MP".into(),
            ItemKind::Revive { heal_percent } => {
                format!("Revives a fallen ally at {:.0}% HP", heal_percent * 100.0)
            }
            ItemKind::CureCurse => "Removes all active curses".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub name: String,
    pub kind: ItemKind,
    /// Base gold value. The shop buys at this price and sells for half.
    pub value: u32,
    /// Short flavor line shown wherever the item is listed.
    pub description: String,
}

pub fn potion() -> Item {
    Item {
        name: "Potion".into(),
        kind: ItemKind::Potion { heal_percent: 0.35 },
        value: 15,
        description: "A murky red draught that knits shallow wounds shut.".into(),
    }
}

pub fn ether() -> Item {
    Item {
        name: "Ether".into(),
        kind: ItemKind::Ether { mp_percent: 0.35 },
        value: 20,
        description: "Bitter and metallic; steadies the mind enough to cast again.".into(),
    }
}

pub fn greater_potion() -> Item {
    Item {
        name: "Greater Potion".into(),
        kind: ItemKind::Potion { heal_percent: 0.6 },
        value: 40,
        description: "Thicker and richer than a common potion — it mends deep.".into(),
    }
}

pub fn greater_ether() -> Item {
    Item {
        name: "Greater Ether".into(),
        kind: ItemKind::Ether { mp_percent: 0.6 },
        value: 45,
        description: "A potent tonic that floods the mind with restored focus.".into(),
    }
}

pub fn sovereign_elixir() -> Item {
    Item {
        name: "Sovereign Elixir".into(),
        kind: ItemKind::Elixir,
        value: 90,
        description: "A regal cure-all, brewed for a king who never earned it.".into(),
    }
}

pub fn ember_of_return() -> Item {
    Item {
        name: "Ember of Return".into(),
        kind: ItemKind::Revive { heal_percent: 0.5 },
        value: 60,
        description: "A single coal that refuses to go out. It remembers being alive.".into(),
    }
}

pub fn purging_stone() -> Item {
    Item {
        name: "Purging Stone".into(),
        kind: ItemKind::CureCurse,
        value: 40,
        description: "Smooth and cold; it drinks in whatever curse clings to you.".into(),
    }
}

/// Factory function types shared by loot tables and the shop's fixed stock —
/// both just need "a function that conjures a fresh Item/Weapon/Armor/Ring"
/// paired with a probability or a price.
pub type ItemFactory = fn() -> Item;
pub type WeaponFactory = fn() -> Weapon;
pub type ArmorFactory = fn() -> Armor;
pub type RingFactory = fn() -> Ring;

/// Weapon rarity tier. Declared in ascending power order so `Rarity::Legendary`
/// is greater than `Rarity::Common` etc. via the derived `Ord` — the rarer a
/// weapon, the stronger it is, full stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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

/// Where a piece of gear came from — purely descriptive, shown in the
/// inventory screen so the player knows how they got it. Shared by
/// `Weapon`, `Armor`, and `Ring` alike.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GearSource {
    /// Carried from the start of the game.
    Starting,
    /// Found while exploring the field (hidden caches, buried relics).
    World,
    /// Dropped by a specific enemy species after combat. An owned `String`
    /// (rather than `&'static str`) so saved games can deserialize it.
    EnemyDrop(String),
}

impl fmt::Display for GearSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GearSource::Starting => write!(f, "Starting gear"),
            GearSource::World => write!(f, "Found in the world"),
            GearSource::EnemyDrop(species) => write!(f, "Dropped by {species}"),
        }
    }
}

/// A unique passive effect carried by a specific Legendary weapon — these
/// are hand-assigned one-to-one in the factory functions below, not a
/// generic rarity-tier bonus, so each Legendary feels distinct rather than
/// just "bigger numbers." Read by `game::combat`'s damage resolution.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum WeaponPassive {
    /// Basic Attacks heal the wielder for this fraction of the damage dealt.
    Lifesteal(f32),
    /// Basic Attacks deal this much bonus damage (as a fraction of the roll)
    /// against boss enemies.
    BossSlayer(f32),
    /// This fraction of incoming damage is shaved off whenever the wielder
    /// is attacked, regardless of who's attacking.
    DamageReduction(f32),
    /// Basic Attacks ignore this fraction of the target's defense.
    ArmorPierce(f32),
}

impl WeaponPassive {
    /// Short flavor line shown next to the weapon in UI lists.
    pub fn description(&self) -> String {
        match self {
            WeaponPassive::Lifesteal(pct) => {
                format!(
                    "Passive: attacks heal the wielder for {:.0}% of damage dealt",
                    pct * 100.0
                )
            }
            WeaponPassive::BossSlayer(pct) => {
                format!(
                    "Passive: attacks deal {:.0}% bonus damage to bosses",
                    pct * 100.0
                )
            }
            WeaponPassive::DamageReduction(pct) => {
                format!("Passive: wielder takes {:.0}% less damage", pct * 100.0)
            }
            WeaponPassive::ArmorPierce(pct) => {
                format!(
                    "Passive: attacks ignore {:.0}% of the target's defense",
                    pct * 100.0
                )
            }
        }
    }
}

/// A wieldable weapon. Every playable character always has exactly one
/// equipped (`Character::equipped_weapon`); better ones are found while
/// exploring the field or won by defeating certain enemies. Rarity is a
/// straightforward power ladder — the rarer the weapon, the stronger it is.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Weapon {
    pub name: String,
    pub rarity: Rarity,
    pub attack_bonus: i32,
    pub defense_bonus: i32,
    pub description: String,
    pub source: GearSource,
    /// How many times the blacksmith (`game::blacksmith`) has upgraded this
    /// specific weapon, `0..=MAX_UPGRADE_LEVEL`. Every weapon starts at 0.
    pub upgrade_level: u8,
    /// A unique combat passive, currently only ever set on the game's four
    /// Legendary signature weapons (see their factory functions below).
    pub passive: Option<WeaponPassive>,
}

/// The highest upgrade tier the blacksmith can apply to a weapon.
pub const MAX_UPGRADE_LEVEL: u8 = 5;

impl Weapon {
    /// Applies one upgrade tier: attack always grows; defense only grows if
    /// this weapon already grants some (a weapon with 0 base defense_bonus,
    /// like the Worn Shortsword, never gains defense from upgrading).
    pub fn apply_upgrade(&mut self, atk_inc: i32, def_inc: i32) {
        self.upgrade_level += 1;
        self.attack_bonus += atk_inc;
        if self.defense_bonus > 0 {
            self.defense_bonus += def_inc;
        }
    }

    /// The name as shown in UI: `"Iron Sword"` at tier 0, `"Iron Sword +3"`
    /// once upgraded. `.name` itself is left untouched so name-equality
    /// checks elsewhere (loot, tests) keep working regardless of upgrades.
    pub fn display_name(&self) -> String {
        if self.upgrade_level > 0 {
            format!("{} +{}", self.name, self.upgrade_level)
        } else {
            self.name.clone()
        }
    }
}

// --- Weapon factory functions, grouped by rarity tier. ---
// Bonuses scale with rarity so "rarer = better" holds at a glance:
// Common ~1-2, Uncommon ~3, Rare ~4-5, Epic ~6-7, Legendary ~10-12.

pub fn worn_shortsword() -> Weapon {
    Weapon {
        name: "Worn Shortsword".into(),
        rarity: Rarity::Common,
        attack_bonus: 1,
        defense_bonus: 0,
        description: "A nicked, well-used blade. Better than fists.".into(),
        source: GearSource::Starting,
        upgrade_level: 0,
        passive: None,
    }
}

pub fn apprentice_wand() -> Weapon {
    Weapon {
        name: "Apprentice's Wand".into(),
        rarity: Rarity::Common,
        attack_bonus: 1,
        defense_bonus: 0,
        description: "A simple wand every mage starts with.".into(),
        source: GearSource::Starting,
        upgrade_level: 0,
        passive: None,
    }
}

pub fn acolytes_mace() -> Weapon {
    Weapon {
        name: "Acolyte's Mace".into(),
        rarity: Rarity::Common,
        attack_bonus: 1,
        defense_bonus: 1,
        description: "Blunt and humble, blessed only lightly.".into(),
        source: GearSource::Starting,
        upgrade_level: 0,
        passive: None,
    }
}

pub fn thieves_dirk() -> Weapon {
    Weapon {
        name: "Thief's Dirk".into(),
        rarity: Rarity::Common,
        attack_bonus: 2,
        defense_bonus: 0,
        description: "Slim, quick, and honest about none of its history.".into(),
        source: GearSource::Starting,
        upgrade_level: 0,
        passive: None,
    }
}

pub fn iron_sword() -> Weapon {
    Weapon {
        name: "Iron Sword".into(),
        rarity: Rarity::Common,
        attack_bonus: 1,
        defense_bonus: 0,
        description: "A sturdy, mass-produced blade left behind by a traveler.".into(),
        source: GearSource::World,
        upgrade_level: 0,
        passive: None,
    }
}

pub fn goblin_shiv() -> Weapon {
    Weapon {
        name: "Goblin Shiv".into(),
        rarity: Rarity::Common,
        attack_bonus: 2,
        defense_bonus: 0,
        description: "Crude but wickedly sharp.".into(),
        source: GearSource::EnemyDrop("Goblin".into()),
        upgrade_level: 0,
        passive: None,
    }
}

pub fn hollow_soldiers_blade() -> Weapon {
    Weapon {
        name: "Hollow Soldier's Blade".into(),
        rarity: Rarity::Common,
        attack_bonus: 2,
        defense_bonus: 0,
        description: "Its owner forgot everything but how to swing it.".into(),
        source: GearSource::EnemyDrop("Hollow".into()),
        upgrade_level: 0,
        passive: None,
    }
}

pub fn travelers_spear() -> Weapon {
    Weapon {
        name: "Traveler's Spear".into(),
        rarity: Rarity::Uncommon,
        attack_bonus: 3,
        defense_bonus: 1,
        description: "Light, well-balanced, and easy to keep an enemy at range.".into(),
        source: GearSource::World,
        upgrade_level: 0,
        passive: None,
    }
}

pub fn bone_blade() -> Weapon {
    Weapon {
        name: "Bone Blade".into(),
        rarity: Rarity::Uncommon,
        attack_bonus: 3,
        defense_bonus: 0,
        description: "Carved from a fallen skeleton's own femur.".into(),
        source: GearSource::EnemyDrop("Skeleton".into()),
        upgrade_level: 0,
        passive: None,
    }
}

pub fn bandits_falchion() -> Weapon {
    Weapon {
        name: "Bandit's Falchion".into(),
        rarity: Rarity::Uncommon,
        attack_bonus: 3,
        defense_bonus: 0,
        description: "Curved for cutting purses and throats alike.".into(),
        source: GearSource::EnemyDrop("Bandit".into()),
        upgrade_level: 0,
        passive: None,
    }
}

pub fn sunlit_straightsword() -> Weapon {
    Weapon {
        name: "Sunlit Straightsword".into(),
        rarity: Rarity::Uncommon,
        attack_bonus: 3,
        defense_bonus: 1,
        description: "Kept bright by an oath someone else abandoned.".into(),
        source: GearSource::World,
        upgrade_level: 0,
        passive: None,
    }
}

pub fn orcish_greataxe() -> Weapon {
    Weapon {
        name: "Orcish Greataxe".into(),
        rarity: Rarity::Rare,
        attack_bonus: 5,
        defense_bonus: 0,
        description: "Heavy enough to fell a tree in one swing.".into(),
        source: GearSource::EnemyDrop("Orc".into()),
        upgrade_level: 0,
        passive: None,
    }
}

pub fn wraithbane_edge() -> Weapon {
    Weapon {
        name: "Wraithbane Edge".into(),
        rarity: Rarity::Rare,
        attack_bonus: 4,
        defense_bonus: 1,
        description: "Etched with wards that flare hot near the restless dead.".into(),
        source: GearSource::EnemyDrop("Wraith".into()),
        upgrade_level: 0,
        passive: None,
    }
}

pub fn sunken_relic_blade() -> Weapon {
    Weapon {
        name: "Sunken Relic Blade".into(),
        rarity: Rarity::Rare,
        attack_bonus: 5,
        defense_bonus: 0,
        description: "Pulled from somewhere it should never have been found.".into(),
        source: GearSource::World,
        upgrade_level: 0,
        passive: None,
    }
}

pub fn fell_censer() -> Weapon {
    Weapon {
        name: "Fell Censer".into(),
        rarity: Rarity::Rare,
        attack_bonus: 4,
        defense_bonus: 1,
        description: "Swings heavy; the smoke it sheds still prays.".into(),
        source: GearSource::EnemyDrop("Fell Acolyte".into()),
        upgrade_level: 0,
        passive: None,
    }
}

pub fn forsaken_longsword() -> Weapon {
    Weapon {
        name: "Forsaken Longsword".into(),
        rarity: Rarity::Rare,
        attack_bonus: 5,
        defense_bonus: 1,
        description: "Its crest was filed off long before its wielder fell.".into(),
        source: GearSource::EnemyDrop("Forsaken Knight".into()),
        upgrade_level: 0,
        passive: None,
    }
}

pub fn sentinels_greathammer() -> Weapon {
    Weapon {
        name: "Sentinel's Greathammer".into(),
        rarity: Rarity::Epic,
        attack_bonus: 6,
        defense_bonus: 1,
        description: "A pillar with a grip. The barrow shook when it fell.".into(),
        source: GearSource::EnemyDrop("Barrow Sentinel".into()),
        upgrade_level: 0,
        passive: None,
    }
}

pub fn moonlit_greatsword() -> Weapon {
    Weapon {
        name: "Moonlit Greatsword".into(),
        rarity: Rarity::Epic,
        attack_bonus: 7,
        defense_bonus: 0,
        description: "A pale arc of light given an edge. It hums when drawn.".into(),
        source: GearSource::World,
        upgrade_level: 0,
        passive: None,
    }
}

pub fn mimics_fang() -> Weapon {
    Weapon {
        name: "Mimic's Fang".into(),
        rarity: Rarity::Epic,
        attack_bonus: 7,
        defense_bonus: 1,
        description: "Still faintly warm. It was a tooth a moment ago.".into(),
        source: GearSource::EnemyDrop("Mimic".into()),
        upgrade_level: 0,
        passive: None,
    }
}

/// Shop-exclusive Epic weapon, unlocked from Chapter Two onward
/// (`game/shop.rs::shop_weapon_stock`) — commissioned rather than looted, so
/// it deliberately doesn't share a name or `GearSource::EnemyDrop` tag with
/// anything in `combat::loot_profile` or the treasure-tile roll.
pub fn coinwrought_blade() -> Weapon {
    Weapon {
        name: "Coinwrought Blade".into(),
        rarity: Rarity::Epic,
        attack_bonus: 7,
        defense_bonus: 1,
        description: "Forged wherever the gold ran thickest. It doesn't care whose war it ends.".into(),
        source: GearSource::World,
        upgrade_level: 0,
        passive: None,
    }
}

pub fn dragonslayers_oath() -> Weapon {
    Weapon {
        name: "Dragonslayer's Oath".into(),
        rarity: Rarity::Legendary,
        attack_bonus: 10,
        defense_bonus: 1,
        description: "A greatsword said to have ended an age. Absurdly rare to find.".into(),
        source: GearSource::World,
        upgrade_level: 0,
        passive: Some(WeaponPassive::BossSlayer(0.5)),
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
        attack_bonus: 11,
        defense_bonus: 2,
        description: "Forged in the barrow's cold fire; still hums with the Knight's fury.".into(),
        source: GearSource::EnemyDrop("The Barrow Knight".into()),
        upgrade_level: 0,
        passive: Some(WeaponPassive::Lifesteal(0.15)),
    }
}

/// The Wyrmscale Warden's signature weapon — chapter two's guaranteed boss
/// drop. On par with Knightsbane; the Ashen Sovereign's drop below is what
/// finally outclasses both.
pub fn wardens_fang() -> Weapon {
    Weapon {
        name: "Warden's Fang".into(),
        rarity: Rarity::Legendary,
        attack_bonus: 11,
        defense_bonus: 2,
        description: "Torn from the Warden's own jaw; the scales along its edge still shed.".into(),
        source: GearSource::EnemyDrop("Wyrmscale Warden".into()),
        upgrade_level: 0,
        passive: Some(WeaponPassive::DamageReduction(0.15)),
    }
}

/// The Ashen Sovereign's signature weapon — the single strongest item in
/// the game, and the only reward for beating the final chapter's boss.
pub fn sovereigns_reckoning() -> Weapon {
    Weapon {
        name: "Sovereign's Reckoning".into(),
        rarity: Rarity::Legendary,
        attack_bonus: 12,
        defense_bonus: 2,
        description: "What's left of a throne, reforged into something that only takes.".into(),
        source: GearSource::EnemyDrop("The Ashen Sovereign".into()),
        upgrade_level: 0,
        passive: Some(WeaponPassive::ArmorPierce(0.5)),
    }
}

/// A suit of armor, worn in a character's dedicated armor slot. Unlike
/// weapons, armor only ever bolsters defense — no attack bonus — so it's
/// the clearest way to build a tankier frontline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Armor {
    pub name: String,
    pub rarity: Rarity,
    pub defense_bonus: i32,
    pub description: String,
    pub source: GearSource,
}

// --- Armor factory functions, grouped by rarity tier. ---
// Defense scales with rarity, same ladder as weapons:
// Common ~1-2, Uncommon ~2-3, Rare ~4-5, Epic ~6-7, Legendary ~11.

pub fn padded_vest() -> Armor {
    Armor {
        name: "Padded Vest".into(),
        rarity: Rarity::Common,
        defense_bonus: 1,
        description: "Quilted cloth over leather. Better than bare skin.".into(),
        source: GearSource::World,
    }
}

pub fn worn_leather_jerkin() -> Armor {
    Armor {
        name: "Worn Leather Jerkin".into(),
        rarity: Rarity::Common,
        defense_bonus: 1,
        description: "Cracked with age, but the stitching still holds.".into(),
        source: GearSource::World,
    }
}

pub fn travelers_chestguard() -> Armor {
    Armor {
        name: "Traveler's Chestguard".into(),
        rarity: Rarity::Common,
        defense_bonus: 2,
        description: "Boiled leather plates, favored by wandering merchants.".into(),
        source: GearSource::World,
    }
}

pub fn rangers_cloak() -> Armor {
    Armor {
        name: "Ranger's Cloak".into(),
        rarity: Rarity::Uncommon,
        defense_bonus: 2,
        description: "Woven to shed rain, blades, and notice alike.".into(),
        source: GearSource::World,
    }
}

pub fn chainmail_hauberk() -> Armor {
    Armor {
        name: "Chainmail Hauberk".into(),
        rarity: Rarity::Uncommon,
        defense_bonus: 3,
        description: "Interlocked rings stripped from a fallen skeleton.".into(),
        source: GearSource::EnemyDrop("Skeleton".into()),
    }
}

pub fn acolytes_vestment() -> Armor {
    Armor {
        name: "Acolyte's Vestment".into(),
        rarity: Rarity::Uncommon,
        defense_bonus: 2,
        description: "Ash-gray robes, hemmed with prayers best left unread.".into(),
        source: GearSource::EnemyDrop("Fell Acolyte".into()),
    }
}

pub fn brigand_leathers() -> Armor {
    Armor {
        name: "Brigand Leathers".into(),
        rarity: Rarity::Uncommon,
        defense_bonus: 3,
        description: "Patched a dozen times, each patch a bad story.".into(),
        source: GearSource::EnemyDrop("Bandit".into()),
    }
}

pub fn warded_chainmail() -> Armor {
    Armor {
        name: "Warded Chainmail".into(),
        rarity: Rarity::Rare,
        defense_bonus: 4,
        description: "Cold to the touch, etched with wards against the restless dead.".into(),
        source: GearSource::EnemyDrop("Wraith".into()),
    }
}

pub fn knights_plate() -> Armor {
    Armor {
        name: "Knight's Plate".into(),
        rarity: Rarity::Rare,
        defense_bonus: 5,
        description: "Dented but unbroken, stripped from a brute of an orc.".into(),
        source: GearSource::EnemyDrop("Orc".into()),
    }
}

pub fn elite_knights_armor() -> Armor {
    Armor {
        name: "Elite Knight's Armor".into(),
        rarity: Rarity::Rare,
        defense_bonus: 5,
        description: "Proud plate that outlived its order and its knight.".into(),
        source: GearSource::EnemyDrop("Forsaken Knight".into()),
    }
}

pub fn sentinels_bulwark() -> Armor {
    Armor {
        name: "Sentinel's Bulwark".into(),
        rarity: Rarity::Epic,
        defense_bonus: 6,
        description: "Stone-set plate that stood watch longer than its wearer lived.".into(),
        source: GearSource::EnemyDrop("Barrow Sentinel".into()),
    }
}

pub fn barrow_touched_plate() -> Armor {
    Armor {
        name: "Barrow-Touched Plate".into(),
        rarity: Rarity::Epic,
        defense_bonus: 7,
        description: "Unearthed from the barrow's outer vaults, still faintly humming.".into(),
        source: GearSource::World,
    }
}

/// Shop-exclusive Epic armor, unlocked from Chapter Two onward — see
/// `coinwrought_blade`'s doc comment for why it stays out of the loot tables.
pub fn coinwrought_plate() -> Armor {
    Armor {
        name: "Coinwrought Plate".into(),
        rarity: Rarity::Epic,
        defense_bonus: 7,
        description: "Bought, sold, and bought again. Still holds.".into(),
        source: GearSource::World,
    }
}

pub fn dragonscale_aegis() -> Armor {
    Armor {
        name: "Dragonscale Aegis".into(),
        rarity: Rarity::Legendary,
        defense_bonus: 11,
        description: "Overlapping scales said to have turned aside a dragon's breath.".into(),
        source: GearSource::World,
    }
}

/// A ring, worn in one of a character's two ring slots. Rings lean toward
/// attack, defense, or a bit of both — the flexible build-around slot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ring {
    pub name: String,
    pub rarity: Rarity,
    pub attack_bonus: i32,
    pub defense_bonus: i32,
    pub description: String,
    pub source: GearSource,
}

// --- Ring factory functions, grouped by rarity tier. ---
// Bonuses are roughly half the weapon/armor ladder, since two rings can be
// worn at once: Common ~1, Uncommon ~1, Rare ~1-2, Epic ~2-3, Legendary ~2-3.

pub fn copper_band() -> Ring {
    Ring {
        name: "Copper Band".into(),
        rarity: Rarity::Common,
        attack_bonus: 1,
        defense_bonus: 0,
        description: "A plain band, faintly warm to the touch.".into(),
        source: GearSource::World,
    }
}

pub fn iron_loop() -> Ring {
    Ring {
        name: "Iron Loop".into(),
        rarity: Rarity::Common,
        attack_bonus: 0,
        defense_bonus: 1,
        description: "Heavy and unadorned. Purely functional.".into(),
        source: GearSource::World,
    }
}

pub fn travelers_ring() -> Ring {
    Ring {
        name: "Traveler's Ring".into(),
        rarity: Rarity::Common,
        attack_bonus: 1,
        defense_bonus: 1,
        description: "Worn smooth by a hundred roads.".into(),
        source: GearSource::World,
    }
}

pub fn ring_of_vigor() -> Ring {
    Ring {
        name: "Ring of Vigor".into(),
        rarity: Rarity::Uncommon,
        attack_bonus: 1,
        defense_bonus: 0,
        description: "Pried from a goblin's clenched, still-twitching fist.".into(),
        source: GearSource::EnemyDrop("Goblin".into()),
    }
}

pub fn ring_of_warding() -> Ring {
    Ring {
        name: "Ring of Warding".into(),
        rarity: Rarity::Uncommon,
        attack_bonus: 0,
        defense_bonus: 1,
        description: "Threaded with a ward-sigil, dulled but still active.".into(),
        source: GearSource::EnemyDrop("Skeleton".into()),
    }
}

pub fn ghouls_knucklebone() -> Ring {
    Ring {
        name: "Ghoul's Knucklebone".into(),
        rarity: Rarity::Uncommon,
        attack_bonus: 1,
        defense_bonus: 1,
        description: "A finger bone lashed into a crude loop. Still hungry.".into(),
        source: GearSource::EnemyDrop("Grave Ghoul".into()),
    }
}

pub fn wolfsbane_signet() -> Ring {
    Ring {
        name: "Wolfsbane Signet".into(),
        rarity: Rarity::Rare,
        attack_bonus: 2,
        defense_bonus: 0,
        description: "Carved with a snarling wolf's-head, still sharp-edged.".into(),
        source: GearSource::EnemyDrop("Wolf".into()),
    }
}

pub fn warded_loop() -> Ring {
    Ring {
        name: "Warded Loop".into(),
        rarity: Rarity::Rare,
        attack_bonus: 0,
        defense_bonus: 2,
        description: "A closed circuit of old wards, humming under strain.".into(),
        source: GearSource::World,
    }
}

pub fn ring_of_favor() -> Ring {
    Ring {
        name: "Ring of Favor".into(),
        rarity: Rarity::Rare,
        attack_bonus: 1,
        defense_bonus: 1,
        description: "A goddess's favor, or a very good forgery of it.".into(),
        source: GearSource::World,
    }
}

pub fn sentinels_seal() -> Ring {
    Ring {
        name: "Sentinel's Seal".into(),
        rarity: Rarity::Epic,
        attack_bonus: 0,
        defense_bonus: 3,
        description: "The badge of the barrow watch, heavy as duty.".into(),
        source: GearSource::EnemyDrop("Barrow Sentinel".into()),
    }
}

/// Shop-exclusive Epic ring, unlocked from Chapter Two onward — see
/// `coinwrought_blade`'s doc comment for why it stays out of the loot tables.
pub fn merchants_blessing() -> Ring {
    Ring {
        name: "Merchant's Blessing".into(),
        rarity: Rarity::Epic,
        attack_bonus: 1,
        defense_bonus: 3,
        description: "A trade charm, worn smooth by a hundred desperate haggles.".into(),
        source: GearSource::World,
    }
}

pub fn sovereigns_signet() -> Ring {
    Ring {
        name: "Sovereign's Signet".into(),
        rarity: Rarity::Legendary,
        attack_bonus: 3,
        defense_bonus: 2,
        description: "The seal of a throne of ash. It still expects obedience.".into(),
        source: GearSource::EnemyDrop("The Ashen Sovereign".into()),
    }
}

pub fn mimics_coil() -> Ring {
    Ring {
        name: "Mimic's Coil".into(),
        rarity: Rarity::Epic,
        attack_bonus: 2,
        defense_bonus: 2,
        description: "Still faintly shifting, as if unsure what shape to hold.".into(),
        source: GearSource::EnemyDrop("Mimic".into()),
    }
}

pub fn band_of_the_barrow() -> Ring {
    Ring {
        name: "Band of the Barrow".into(),
        rarity: Rarity::Legendary,
        attack_bonus: 3,
        defense_bonus: 3,
        description: "Buried alongside the Knight himself. It has not gone cold.".into(),
        source: GearSource::World,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    // (item, quantity)
    pub items: Vec<(Item, u32)>,
    /// Weapons currently carried but not equipped by anyone.
    pub weapons: Vec<Weapon>,
    /// Armor currently carried but not equipped by anyone.
    pub armors: Vec<Armor>,
    /// Rings currently carried but not equipped by anyone.
    pub rings: Vec<Ring>,
    /// "Titanite Shards" — the blacksmith's upgrade material, spent
    /// alongside gold in `game::blacksmith::upgrade_cost`. A bare counter
    /// rather than an `ItemKind`, mirroring `Party::gold`: a fungible
    /// resource that's never equipped/sold/used like a consumable item.
    pub upgrade_materials: u32,
}

impl Inventory {
    pub fn starting() -> Self {
        Self {
            items: vec![(potion(), 3), (ether(), 2)],
            weapons: Vec::new(),
            armors: Vec::new(),
            rings: Vec::new(),
            upgrade_materials: 0,
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

    pub fn add_armor(&mut self, armor: Armor) {
        self.armors.push(armor);
    }

    pub fn add_ring(&mut self, ring: Ring) {
        self.rings.push(ring);
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
    fn sovereigns_reckoning_is_the_single_strongest_weapon() {
        let contenders = [
            worn_shortsword(),
            iron_sword(),
            goblin_shiv(),
            hollow_soldiers_blade(),
            travelers_spear(),
            bone_blade(),
            bandits_falchion(),
            sunlit_straightsword(),
            orcish_greataxe(),
            wraithbane_edge(),
            sunken_relic_blade(),
            fell_censer(),
            forsaken_longsword(),
            mimics_fang(),
            sentinels_greathammer(),
            moonlit_greatsword(),
            dragonslayers_oath(),
            knightsbane(),
            wardens_fang(),
        ];
        let strongest_atk = sovereigns_reckoning().attack_bonus;
        assert!(
            contenders.iter().all(|w| w.attack_bonus < strongest_atk),
            "the final boss's signature weapon should outclass every other weapon in the game"
        );
    }

    #[test]
    fn boss_signature_weapons_are_at_least_as_strong_chapter_over_chapter() {
        assert!(wardens_fang().attack_bonus >= knightsbane().attack_bonus);
        assert!(sovereigns_reckoning().attack_bonus > wardens_fang().attack_bonus);
    }

    #[test]
    fn add_armor_appends_to_inventory() {
        let mut inv = Inventory::starting();
        assert!(inv.armors.is_empty());
        inv.add_armor(padded_vest());
        assert_eq!(inv.armors.len(), 1);
        assert_eq!(inv.armors[0].name, "Padded Vest");
    }

    #[test]
    fn add_ring_appends_to_inventory() {
        let mut inv = Inventory::starting();
        assert!(inv.rings.is_empty());
        inv.add_ring(copper_band());
        assert_eq!(inv.rings.len(), 1);
        assert_eq!(inv.rings[0].name, "Copper Band");
    }

    #[test]
    fn rarer_armor_has_bigger_defense_within_the_world_find_ladder() {
        assert!(padded_vest().defense_bonus < barrow_touched_plate().defense_bonus);
        assert!(barrow_touched_plate().defense_bonus < dragonscale_aegis().defense_bonus);
    }

    #[test]
    fn rarer_rings_have_bigger_combined_bonus() {
        let combined = |r: &Ring| r.attack_bonus + r.defense_bonus;
        assert!(combined(&copper_band()) < combined(&mimics_coil()));
        assert!(combined(&mimics_coil()) < combined(&band_of_the_barrow()));
    }

    #[test]
    fn apply_upgrade_grows_attack_and_defense_when_the_weapon_has_some() {
        let mut weapon = wraithbane_edge(); // starts with attack_bonus 9, defense_bonus 2
        let (atk_before, def_before) = (weapon.attack_bonus, weapon.defense_bonus);
        weapon.apply_upgrade(3, 2);
        assert_eq!(weapon.upgrade_level, 1);
        assert_eq!(weapon.attack_bonus, atk_before + 3);
        assert_eq!(weapon.defense_bonus, def_before + 2);
    }

    #[test]
    fn apply_upgrade_never_grants_defense_to_a_weapon_that_started_with_none() {
        let mut weapon = worn_shortsword(); // defense_bonus 0
        weapon.apply_upgrade(1, 1);
        assert_eq!(weapon.defense_bonus, 0);
    }

    #[test]
    fn upgrade_level_reaches_the_cap_after_max_applications() {
        let mut weapon = worn_shortsword();
        for _ in 0..MAX_UPGRADE_LEVEL {
            weapon.apply_upgrade(1, 0);
        }
        assert_eq!(weapon.upgrade_level, MAX_UPGRADE_LEVEL);
    }

    #[test]
    fn display_name_only_shows_a_suffix_once_upgraded() {
        let mut weapon = worn_shortsword();
        assert_eq!(weapon.display_name(), "Worn Shortsword");
        weapon.apply_upgrade(1, 0);
        assert_eq!(weapon.display_name(), "Worn Shortsword +1");
    }
}
