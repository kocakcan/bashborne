use crate::game::item::{Inventory, Rarity, Weapon, MAX_UPGRADE_LEVEL};
use crate::game::map::Position;
use crate::game::party::Party;

/// A weapon's current location: still in the shared bag, or equipped on a
/// specific party member. Lets the blacksmith list and upgrade a weapon
/// "wherever it lives" without duplicating logic — shared by `app.rs` (which
/// needs mutable access to apply an upgrade) and `ui/blacksmith.rs` (which
/// only needs to render the list), so both agree on the same ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponRef {
    Bag(usize),
    Equipped(usize),
}

/// Every weapon currently available to upgrade: every spare weapon in the
/// bag, followed by every party member's equipped weapon (if any).
/// Rebuilt fresh on every render/key-handle call rather than cached,
/// matching the shop's fixed-stock convention of never storing derived state.
pub fn weapon_refs(inventory: &Inventory, party: &Party) -> Vec<WeaponRef> {
    let mut refs: Vec<WeaponRef> = (0..inventory.weapons.len()).map(WeaponRef::Bag).collect();
    for (i, m) in party.members.iter().enumerate() {
        if m.equipped_weapon.is_some() {
            refs.push(WeaponRef::Equipped(i));
        }
    }
    refs
}

pub fn weapon_for<'a>(r: WeaponRef, inventory: &'a Inventory, party: &'a Party) -> Option<&'a Weapon> {
    match r {
        WeaponRef::Bag(i) => inventory.weapons.get(i),
        WeaponRef::Equipped(i) => party.members.get(i)?.equipped_weapon.as_ref(),
    }
}

pub fn weapon_for_mut<'a>(
    r: WeaponRef,
    inventory: &'a mut Inventory,
    party: &'a mut Party,
) -> Option<&'a mut Weapon> {
    match r {
        WeaponRef::Bag(i) => inventory.weapons.get_mut(i),
        WeaponRef::Equipped(i) => party.members.get_mut(i)?.equipped_weapon.as_mut(),
    }
}

/// A label describing where a `WeaponRef` currently lives, for display next
/// to the weapon's name in the blacksmith screen.
pub fn weapon_ref_label(r: WeaponRef, party: &Party) -> String {
    match r {
        WeaponRef::Bag(_) => "(spare)".to_string(),
        WeaponRef::Equipped(i) => party
            .members
            .get(i)
            .map(|m| format!("({}'s weapon)", m.name))
            .unwrap_or_default(),
    }
}

/// Andre of Astora's upgrade screen: pick a weapon (from the bag or
/// currently equipped on a party member — see `WeaponRef`) and spend gold +
/// Titanite Shards to raise its upgrade tier. No stored stock like the
/// shop's — the weapon list is rebuilt fresh from the party's current gear
/// every time it's rendered/handled.
pub struct BlacksmithUiState {
    pub cursor: usize,
    pub message: Option<String>,
    /// Where to place the player back on the map once this screen is closed.
    pub return_pos: Position,
}

impl BlacksmithUiState {
    pub fn new(return_pos: Position) -> Self {
        Self {
            cursor: 0,
            message: None,
            return_pos,
        }
    }
}

/// How much a weapon's attack_bonus grows per upgrade tier, by rarity.
fn upgrade_attack_increment(rarity: Rarity) -> i32 {
    match rarity {
        Rarity::Common => 1,
        Rarity::Uncommon => 2,
        Rarity::Rare => 3,
        Rarity::Epic => 4,
        Rarity::Legendary => 5,
    }
}

/// How much a weapon's defense_bonus grows per upgrade tier, by rarity
/// (only applied if the weapon already grants some — see `Weapon::apply_upgrade`).
fn upgrade_defense_increment(rarity: Rarity) -> i32 {
    (upgrade_attack_increment(rarity) + 1) / 2
}

/// The (attack, defense) increment `apply_upgrade` should use for a weapon
/// of this rarity.
pub fn upgrade_increments(rarity: Rarity) -> (i32, i32) {
    (upgrade_attack_increment(rarity), upgrade_defense_increment(rarity))
}

/// Cost (gold, shards) to raise a weapon of `rarity` from `current_tier` to
/// `current_tier + 1`. `None` once the weapon is already at the cap.
/// Gold grows quadratically in the target tier (a real late-game sink);
/// shards grow linearly (the scarcer, pacing-setting currency). Both scale
/// with rarity so a Legendary costs substantially more to fully upgrade
/// than a Common of the same tier.
pub fn upgrade_cost(rarity: Rarity, current_tier: u8) -> Option<(u32, u32)> {
    if current_tier >= MAX_UPGRADE_LEVEL {
        return None;
    }
    let next_tier = (current_tier as u32) + 1;
    let rarity_mult = match rarity {
        Rarity::Common => 1,
        Rarity::Uncommon => 2,
        Rarity::Rare => 3,
        Rarity::Epic => 5,
        Rarity::Legendary => 8,
    };
    let gold = 10 * rarity_mult * next_tier * next_tier;
    let shards = (next_tier * rarity_mult).max(1);
    Some((gold, shards))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upgrade_cost_climbs_with_rarity_at_a_fixed_tier() {
        let (common_gold, common_shards) = upgrade_cost(Rarity::Common, 0).unwrap();
        let (legendary_gold, legendary_shards) = upgrade_cost(Rarity::Legendary, 0).unwrap();
        assert!(legendary_gold > common_gold);
        assert!(legendary_shards > common_shards);
    }

    #[test]
    fn upgrade_cost_climbs_with_tier_at_a_fixed_rarity() {
        let (tier1_gold, _) = upgrade_cost(Rarity::Rare, 0).unwrap();
        let (tier2_gold, _) = upgrade_cost(Rarity::Rare, 1).unwrap();
        assert!(tier2_gold > tier1_gold);
    }

    #[test]
    fn upgrade_cost_is_none_at_or_above_the_cap() {
        assert!(upgrade_cost(Rarity::Common, MAX_UPGRADE_LEVEL).is_none());
    }

    #[test]
    fn weapon_refs_lists_bag_weapons_then_equipped_weapons() {
        use crate::game::character::warrior;
        use crate::game::item::iron_sword;

        let mut inventory = Inventory::starting();
        inventory.add_weapon(iron_sword());
        let party = Party::new(vec![warrior("Bram")]);

        let refs = weapon_refs(&inventory, &party);
        assert_eq!(refs, vec![WeaponRef::Bag(0), WeaponRef::Equipped(0)]);
    }

    #[test]
    fn weapon_for_mut_upgrades_the_correct_weapon_in_either_location() {
        use crate::game::character::warrior;
        use crate::game::item::iron_sword;

        let mut inventory = Inventory::starting();
        inventory.add_weapon(iron_sword());
        let mut party = Party::new(vec![warrior("Bram")]);

        let bag_weapon = weapon_for_mut(WeaponRef::Bag(0), &mut inventory, &mut party).unwrap();
        assert_eq!(bag_weapon.name, "Iron Sword");
        bag_weapon.apply_upgrade(1, 0);
        assert_eq!(inventory.weapons[0].upgrade_level, 1);

        let equipped_weapon =
            weapon_for_mut(WeaponRef::Equipped(0), &mut inventory, &mut party).unwrap();
        assert_eq!(equipped_weapon.name, "Worn Shortsword");
        equipped_weapon.apply_upgrade(1, 0);
        assert_eq!(
            party.members[0].equipped_weapon.as_ref().unwrap().upgrade_level,
            1
        );
    }
}
