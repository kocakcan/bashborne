use crate::game::item::{
    chainmail_hauberk, copper_band, ember_of_return, ether, greater_ether, greater_potion,
    iron_loop, iron_sword, potion, purging_stone, rangers_cloak, ring_of_vigor, sunken_relic_blade,
    sunlit_straightsword, travelers_chestguard, travelers_spear, warded_loop, ArmorFactory,
    ItemFactory, Rarity, RingFactory, WeaponFactory,
};
use crate::game::map::Position;

/// Sell-back price for a piece of gear of the given rarity — half its
/// catalog value, the one formula both the shop and the sell-confirmation
/// UI must agree on.
pub fn sell_price(rarity: Rarity) -> u32 {
    rarity.base_value() / 2
}

/// The shop's own tab split — conceptually the same "Items vs Weapons vs
/// Armor vs Rings" categories as the out-of-combat inventory screen, but
/// kept as a separate enum (rather than reusing `InventoryTab`) since the
/// shop has no Materials tab: Titanite Shards aren't bought or sold here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShopTab {
    Items,
    Weapons,
    Armor,
    Rings,
}

impl ShopTab {
    pub fn next(self) -> Self {
        match self {
            ShopTab::Items => ShopTab::Weapons,
            ShopTab::Weapons => ShopTab::Armor,
            ShopTab::Armor => ShopTab::Rings,
            ShopTab::Rings => ShopTab::Items,
        }
    }
}

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
    /// Set when a Rare+ sell is armed and waiting on a second Enter to
    /// confirm — see `World::apply_shop_sell`. Any other key (navigation,
    /// mode/tab switch) clears it rather than letting a stale confirmation
    /// silently apply to a different item.
    pub pending_sell: Option<(ShopTab, usize)>,
}

impl ShopUiState {
    pub fn new(return_pos: Position) -> Self {
        Self {
            mode: ShopMode::Buy,
            tab: ShopTab::Items,
            cursor: 0,
            message: None,
            return_pos,
            pending_sell: None,
        }
    }
}

/// Fixed item stock the shop always carries, paired with its gold price.
pub fn shop_item_stock() -> Vec<(ItemFactory, u32)> {
    vec![
        (potion as ItemFactory, 15),
        (ether as ItemFactory, 20),
        (greater_potion as ItemFactory, 40),
        (greater_ether as ItemFactory, 45),
        (purging_stone as ItemFactory, 40),
        (ember_of_return as ItemFactory, 60),
    ]
}

/// Fixed weapon stock, paired with its gold price. Deliberately capped at
/// Rare — Epic and Legendary weapons stay something you have to earn out in
/// the field or win from a tough enemy, not something gold can shortcut.
pub fn shop_weapon_stock() -> Vec<(WeaponFactory, u32)> {
    vec![
        (iron_sword as WeaponFactory, 20),
        (travelers_spear as WeaponFactory, 50),
        (sunlit_straightsword as WeaponFactory, 50),
        (sunken_relic_blade as WeaponFactory, 120),
    ]
}

/// Fixed armor stock, paired with its gold price. Same Rare cap as weapons.
pub fn shop_armor_stock() -> Vec<(ArmorFactory, u32)> {
    vec![
        (travelers_chestguard as ArmorFactory, 20),
        (rangers_cloak as ArmorFactory, 50),
        (chainmail_hauberk as ArmorFactory, 50),
    ]
}

/// Fixed ring stock, paired with its gold price. Same Rare cap as weapons.
pub fn shop_ring_stock() -> Vec<(RingFactory, u32)> {
    vec![
        (copper_band as RingFactory, 20),
        (iron_loop as RingFactory, 20),
        (ring_of_vigor as RingFactory, 50),
        (warded_loop as RingFactory, 120),
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
