use crate::game::inventory_ui::InventoryTab;
use crate::game::item::{
    chainmail_hauberk, copper_band, ether, iron_sword, potion, ring_of_vigor, rangers_cloak,
    sunken_relic_blade, travelers_spear, ArmorFactory, ItemFactory, RingFactory, WeaponFactory,
};
use crate::game::map::Position;

/// The shop reuses the same "Items vs Weapons" split as the out-of-combat
/// inventory screen — it's the same kind of list either way, just sourced
/// from the shop's stock (Buy) or the party's own bag (Sell).
pub type ShopTab = InventoryTab;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShopMode {
    Buy,
    Sell,
}

impl ShopMode {
    pub fn toggled(self) -> Self {
        match self {
            ShopMode::Buy => ShopMode::Sell,
            ShopMode::Sell => ShopMode::Buy,
        }
    }
}

/// Town shop screen: buy from a fixed stock with party gold, or sell spare
/// items/weapons from the bag for gold. Only reachable while standing on a
/// Town tile during Explore.
pub struct ShopUiState {
    pub mode: ShopMode,
    pub tab: ShopTab,
    pub cursor: usize,
    pub message: Option<String>,
    /// Where to place the player back on the map once the shop is closed.
    pub return_pos: Position,
}

impl ShopUiState {
    pub fn new(return_pos: Position) -> Self {
        Self {
            mode: ShopMode::Buy,
            tab: InventoryTab::Items,
            cursor: 0,
            message: None,
            return_pos,
        }
    }
}

/// Fixed item stock the shop always carries, paired with its gold price.
pub fn shop_item_stock() -> Vec<(ItemFactory, u32)> {
    vec![(potion as ItemFactory, 15), (ether as ItemFactory, 20)]
}

/// Fixed weapon stock, paired with its gold price. Deliberately capped at
/// Rare — Epic and Legendary weapons stay something you have to earn out in
/// the field or win from a tough enemy, not something gold can shortcut.
pub fn shop_weapon_stock() -> Vec<(WeaponFactory, u32)> {
    vec![
        (iron_sword as WeaponFactory, 20),
        (travelers_spear as WeaponFactory, 50),
        (sunken_relic_blade as WeaponFactory, 120),
    ]
}

/// Fixed armor stock, paired with its gold price. Same Rare cap as weapons.
pub fn shop_armor_stock() -> Vec<(ArmorFactory, u32)> {
    vec![
        (rangers_cloak as ArmorFactory, 50),
        (chainmail_hauberk as ArmorFactory, 50),
    ]
}

/// Fixed ring stock, paired with its gold price. Same Rare cap as weapons.
pub fn shop_ring_stock() -> Vec<(RingFactory, u32)> {
    vec![
        (copper_band as RingFactory, 20),
        (ring_of_vigor as RingFactory, 50),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_stock_prices_match_their_catalog_value() {
        for (factory, price) in shop_item_stock() {
            assert_eq!(factory().value, price);
        }
    }

    #[test]
    fn weapon_stock_prices_match_their_rarity_value() {
        for (factory, price) in shop_weapon_stock() {
            assert_eq!(factory().rarity.base_value(), price);
        }
    }

    #[test]
    fn weapon_stock_never_carries_epic_or_legendary_gear() {
        use crate::game::item::Rarity;
        for (factory, _) in shop_weapon_stock() {
            let rarity = factory().rarity;
            assert!(
                rarity < Rarity::Epic,
                "shop stock should stop at Rare; found {rarity}"
            );
        }
    }

    #[test]
    fn armor_stock_prices_match_their_rarity_value() {
        for (factory, price) in shop_armor_stock() {
            assert_eq!(factory().rarity.base_value(), price);
        }
    }

    #[test]
    fn ring_stock_prices_match_their_rarity_value() {
        for (factory, price) in shop_ring_stock() {
            assert_eq!(factory().rarity.base_value(), price);
        }
    }

    #[test]
    fn armor_and_ring_stock_never_carry_epic_or_legendary_gear() {
        use crate::game::item::Rarity;
        for (factory, _) in shop_armor_stock() {
            assert!(factory().rarity < Rarity::Epic);
        }
        for (factory, _) in shop_ring_stock() {
            assert!(factory().rarity < Rarity::Epic);
        }
    }
}
