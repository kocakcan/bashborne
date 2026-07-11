use crate::game::chapter::ChapterId;
use crate::game::item::{
    chainmail_hauberk, coinwrought_blade, coinwrought_plate, copper_band, ember_of_return, ether,
    greater_ether, greater_potion, iron_loop, iron_sword, merchants_blessing, potion,
    purging_stone, rangers_cloak, ring_of_vigor, sovereign_elixir, sunken_relic_blade,
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

/// Item stock the shop carries, paired with its gold price. The Chapter-One
/// floor (`purging_stone`/etc.) never goes away; from Chapter Two onward the
/// shop also surfaces `sovereign_elixir` — always in `item.rs`, but with
/// nowhere to buy it until now.
pub fn shop_item_stock(chapter: ChapterId) -> Vec<(ItemFactory, u32)> {
    let mut stock = vec![
        (potion as ItemFactory, 15),
        (ether as ItemFactory, 20),
        (greater_potion as ItemFactory, 40),
        (greater_ether as ItemFactory, 45),
        (purging_stone as ItemFactory, 40),
        (ember_of_return as ItemFactory, 60),
    ];
    if chapter != ChapterId::One {
        stock.push((sovereign_elixir as ItemFactory, 90));
    }
    stock
}

/// Weapon stock, paired with its gold price. Chapter One is capped at Rare —
/// Epic weapons stay something you have to earn out in the field or win from
/// a tough enemy, not something gold can shortcut, until later chapters make
/// a dedicated shop-exclusive Epic (`coinwrought_blade`) available; Legendary
/// stays boss-exclusive at every chapter.
pub fn shop_weapon_stock(chapter: ChapterId) -> Vec<(WeaponFactory, u32)> {
    let mut stock = vec![
        (iron_sword as WeaponFactory, 20),
        (travelers_spear as WeaponFactory, 50),
        (sunlit_straightsword as WeaponFactory, 50),
        (sunken_relic_blade as WeaponFactory, 120),
    ];
    if chapter != ChapterId::One {
        stock.push((coinwrought_blade as WeaponFactory, 300));
    }
    stock
}

/// Armor stock, paired with its gold price. Same chapter-gated Epic unlock
/// as `shop_weapon_stock`.
pub fn shop_armor_stock(chapter: ChapterId) -> Vec<(ArmorFactory, u32)> {
    let mut stock = vec![
        (travelers_chestguard as ArmorFactory, 20),
        (rangers_cloak as ArmorFactory, 50),
        (chainmail_hauberk as ArmorFactory, 50),
    ];
    if chapter != ChapterId::One {
        stock.push((coinwrought_plate as ArmorFactory, 300));
    }
    stock
}

/// Ring stock, paired with its gold price. Same chapter-gated Epic unlock as
/// `shop_weapon_stock`.
pub fn shop_ring_stock(chapter: ChapterId) -> Vec<(RingFactory, u32)> {
    let mut stock = vec![
        (copper_band as RingFactory, 20),
        (iron_loop as RingFactory, 20),
        (ring_of_vigor as RingFactory, 50),
        (warded_loop as RingFactory, 120),
    ];
    if chapter != ChapterId::One {
        stock.push((merchants_blessing as RingFactory, 300));
    }
    stock
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_CHAPTERS: [ChapterId; 3] = [ChapterId::One, ChapterId::Two, ChapterId::Three];

    #[test]
    fn item_stock_prices_match_their_catalog_value() {
        for chapter in ALL_CHAPTERS {
            for (factory, price) in shop_item_stock(chapter) {
                assert_eq!(factory().value, price);
            }
        }
    }

    #[test]
    fn weapon_stock_prices_match_their_rarity_value() {
        for chapter in ALL_CHAPTERS {
            for (factory, price) in shop_weapon_stock(chapter) {
                assert_eq!(factory().rarity.base_value(), price);
            }
        }
    }

    #[test]
    fn armor_stock_prices_match_their_rarity_value() {
        for chapter in ALL_CHAPTERS {
            for (factory, price) in shop_armor_stock(chapter) {
                assert_eq!(factory().rarity.base_value(), price);
            }
        }
    }

    #[test]
    fn ring_stock_prices_match_their_rarity_value() {
        for chapter in ALL_CHAPTERS {
            for (factory, price) in shop_ring_stock(chapter) {
                assert_eq!(factory().rarity.base_value(), price);
            }
        }
    }

    #[test]
    fn chapter_one_stock_never_carries_epic_or_legendary_gear() {
        for (factory, _) in shop_weapon_stock(ChapterId::One) {
            let rarity = factory().rarity;
            assert!(
                rarity < Rarity::Epic,
                "Chapter One shop stock should stop at Rare; found {rarity}"
            );
        }
        for (factory, _) in shop_armor_stock(ChapterId::One) {
            assert!(factory().rarity < Rarity::Epic);
        }
        for (factory, _) in shop_ring_stock(ChapterId::One) {
            assert!(factory().rarity < Rarity::Epic);
        }
    }

    #[test]
    fn chapters_two_and_three_unlock_exactly_one_epic_entry_per_tab_and_never_legendary() {
        for chapter in [ChapterId::Two, ChapterId::Three] {
            let epic_weapons = shop_weapon_stock(chapter)
                .into_iter()
                .filter(|(f, _)| f().rarity == Rarity::Epic)
                .count();
            let epic_armor = shop_armor_stock(chapter)
                .into_iter()
                .filter(|(f, _)| f().rarity == Rarity::Epic)
                .count();
            let epic_rings = shop_ring_stock(chapter)
                .into_iter()
                .filter(|(f, _)| f().rarity == Rarity::Epic)
                .count();
            assert_eq!(epic_weapons, 1);
            assert_eq!(epic_armor, 1);
            assert_eq!(epic_rings, 1);

            for (factory, _) in shop_weapon_stock(chapter) {
                assert!(factory().rarity < Rarity::Legendary, "Legendary stays boss-exclusive");
            }
            for (factory, _) in shop_armor_stock(chapter) {
                assert!(factory().rarity < Rarity::Legendary);
            }
            for (factory, _) in shop_ring_stock(chapter) {
                assert!(factory().rarity < Rarity::Legendary);
            }
        }
    }

    #[test]
    fn sovereign_elixir_is_only_sold_from_chapter_two_onward() {
        assert!(shop_item_stock(ChapterId::One)
            .iter()
            .all(|(f, _)| f().name != "Sovereign Elixir"));
        for chapter in [ChapterId::Two, ChapterId::Three] {
            assert!(shop_item_stock(chapter)
                .iter()
                .any(|(f, _)| f().name == "Sovereign Elixir"));
        }
    }
}
