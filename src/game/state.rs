use rand::Rng;

use crate::game::chapter::{chapter_def, ChapterId};
use crate::game::character::Character;
use crate::game::combat::CombatState;
use crate::game::inventory_ui::InventoryUiState;
use crate::game::item::{
    dragonslayers_oath, ether, iron_sword, potion, sunken_relic_blade, travelers_spear, Item,
    Weapon,
};
use crate::game::map::{Map, Position};
use crate::game::npc::NpcId;
use crate::game::quest_ui::QuestLogUiState;
use crate::game::shop::ShopUiState;
use crate::game::status::{roll_blessing, roll_curse, StatusEffect};

pub struct ExploreState {
    pub map: Map,
    pub player_pos: Position,
    pub log: Vec<String>,
    pub steps_in_grass: u32,
}

impl ExploreState {
    /// Builds the explore screen for `chapter`: its map (with that
    /// chapter's NPCs folded in) and its spawn point.
    pub fn for_chapter(chapter: ChapterId) -> Self {
        let def = chapter_def(chapter);
        let mut map = (def.map)();
        map.npcs = def.npcs.clone();
        Self {
            map,
            player_pos: def.spawn,
            log: vec![format!("You arrive at {}. Watch the tall grass...", def.name)],
            steps_in_grass: 0,
        }
    }
}

/// A one-off narrative beat (blessing, curse, treasure find, or NPC
/// dialogue) shown as its own screen before returning to exploration.
/// Combat is handled separately since it needs its own interactive state.
pub struct EventState {
    pub title: String,
    pub lines: Vec<String>,
    /// Where to place the player back on the map once this notice is dismissed.
    pub return_pos: Position,
    /// Which NPC this event is dialogue for, if any — `None` for the
    /// blessing/curse/treasure events below.
    pub npc: Option<NpcId>,
}

pub enum GameState {
    Explore(ExploreState),
    Combat(CombatState),
    Event(EventState),
    Inventory(InventoryUiState),
    Shop(ShopUiState),
    QuestLog(QuestLogUiState),
    GameOver { victory: bool },
}

/// Simple encounter table used when the player steps into tall grass and a fight occurs.
pub fn roll_encounter(rng: &mut impl Rng) -> Vec<Character> {
    use crate::game::character::{bat, goblin, orc, skeleton, slime, wolf, wraith};
    match rng.gen_range(0..10) {
        0 => vec![slime("Slime")],
        1 => vec![slime("Slime"), slime("Slime")],
        2 => vec![goblin("Goblin")],
        3 => vec![bat("Bat"), bat("Bat")],
        4 => vec![wolf("Wolf")],
        5 => vec![wolf("Wolf"), bat("Bat")],
        6 => vec![skeleton("Skeleton")],
        7 => vec![orc("Orc")],
        8 => vec![wraith("Wraith")],
        _ => vec![goblin("Goblin"), bat("Bat")],
    }
}

/// What happens when a tall-grass encounter roll fires: not always a fight.
pub enum FieldEvent {
    Combat(Vec<Character>),
    Blessing(StatusEffect),
    Curse(StatusEffect),
    Treasure {
        gold: u32,
        item: Option<Item>,
        weapon: Option<Weapon>,
    },
}

/// Weighted table for grass encounters: 55% combat, 15% blessing, 15% curse,
/// 15% treasure. Tune the ranges below to rebalance.
pub fn roll_field_event(rng: &mut impl Rng) -> FieldEvent {
    match rng.gen_range(0..20) {
        0..=10 => FieldEvent::Combat(roll_encounter(rng)), // 11/20
        11..=13 => FieldEvent::Blessing(roll_blessing(rng)), // 3/20
        14..=16 => FieldEvent::Curse(roll_curse(rng)),       // 3/20
        _ => {
            // 3/20: usually real treasure, but occasionally it bites back.
            if rng.gen_ratio(1, 6) {
                FieldEvent::Combat(vec![crate::game::character::mimic("Mimic")])
            } else {
                let gold = rng.gen_range(10..=25);
                let item = if rng.gen_bool(0.5) {
                    Some(if rng.gen_bool(0.5) { potion() } else { ether() })
                } else {
                    None
                };
                // Rare on top of that: the cache also held a weapon. Weighted
                // toward common/uncommon finds, with a vanishingly small
                // chance (roughly 1-in-80 overall) of a legendary relic.
                let weapon = if rng.gen_ratio(1, 8) {
                    Some(match rng.gen_range(0..10) {
                        0..=4 => iron_sword(),
                        5..=7 => travelers_spear(),
                        8 => sunken_relic_blade(),
                        _ => dragonslayers_oath(),
                    })
                } else {
                    None
                };
                FieldEvent::Treasure { gold, item, weapon }
            }
        }
    }
}
