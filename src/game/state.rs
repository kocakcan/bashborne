use rand::Rng;

use crate::game::blacksmith::BlacksmithUiState;
use crate::game::chapter::{chapter_def, ChapterId};
use crate::game::character::Character;
use crate::game::combat::CombatState;
use crate::game::inventory_ui::InventoryUiState;
use crate::game::item::{
    dragonslayers_oath, ether, iron_sword, potion, sunken_relic_blade, travelers_spear, Item,
    Weapon,
};
use crate::game::levelup::LevelUpUiState;
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
    LevelUp(LevelUpUiState),
    Blacksmith(BlacksmithUiState),
    GameOver { victory: bool },
}

/// Simple encounter table used when the player steps into tall grass and a
/// fight occurs. Every rolled enemy is then scaled up to `enemy_level` (the
/// current chapter's `ChapterDef::enemy_level`), so the same species table
/// gets genuinely harder chapter over chapter.
pub fn roll_encounter(rng: &mut impl Rng, enemy_level: u32) -> Vec<Character> {
    use crate::game::character::{bat, goblin, orc, skeleton, slime, wolf, wraith};
    let mut enemies = match rng.gen_range(0..10) {
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
    };
    for enemy in &mut enemies {
        enemy.scale_to_level(enemy_level);
    }
    enemies
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
        /// Bonus Titanite Shards buried alongside the treasure — see
        /// `Inventory::upgrade_materials`.
        materials: u32,
    },
}

/// Weighted table for grass encounters: 55% combat, 15% blessing, 15% curse,
/// 15% treasure. Tune the ranges below to rebalance. `enemy_level` scales
/// any combat outcome (including a Mimic ambush) to the current chapter.
pub fn roll_field_event(rng: &mut impl Rng, enemy_level: u32) -> FieldEvent {
    match rng.gen_range(0..20) {
        0..=10 => FieldEvent::Combat(roll_encounter(rng, enemy_level)), // 11/20
        11..=13 => FieldEvent::Blessing(roll_blessing(rng)), // 3/20
        14..=16 => FieldEvent::Curse(roll_curse(rng)),       // 3/20
        _ => {
            // 3/20: usually real treasure, but occasionally it bites back.
            if rng.gen_ratio(1, 6) {
                let mut mimic = crate::game::character::mimic("Mimic");
                mimic.scale_to_level(enemy_level);
                FieldEvent::Combat(vec![mimic])
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
                // Independent chance of bonus Titanite Shards buried
                // alongside the treasure, so exploring (not just fighting)
                // feeds the blacksmith too.
                let materials = if rng.gen_ratio(1, 5) { rng.gen_range(1..=3) } else { 0 };
                FieldEvent::Treasure { gold, item, weapon, materials }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn later_chapters_field_stronger_versions_of_the_same_species() {
        // Same seed rolls the same composition; only the scaling differs.
        for seed in 0..20u64 {
            let ch1 = roll_encounter(&mut StdRng::seed_from_u64(seed), 1);
            let ch3 = roll_encounter(&mut StdRng::seed_from_u64(seed), 7);
            for (weak, strong) in ch1.iter().zip(ch3.iter()) {
                assert_eq!(weak.name, strong.name);
                assert_eq!(weak.level, 1);
                assert_eq!(strong.level, 7);
                assert!(strong.stats.max_hp > weak.stats.max_hp);
                assert!(strong.stats.attack > weak.stats.attack);
                assert!(strong.stats.defense > weak.stats.defense);
            }
        }
    }

    #[test]
    fn a_mimic_ambush_is_scaled_to_the_chapters_enemy_level() {
        // Sweep seeds until the treasure arm rolls a Mimic, then check its level.
        let mut saw_a_mimic = false;
        for seed in 0..500u64 {
            let mut rng = StdRng::seed_from_u64(seed);
            if let FieldEvent::Combat(enemies) = roll_field_event(&mut rng, 7) {
                if enemies.iter().any(|e| e.name == "Mimic") {
                    assert!(enemies.iter().all(|e| e.level == 7));
                    saw_a_mimic = true;
                    break;
                }
            }
        }
        assert!(saw_a_mimic, "some seed should roll a Mimic ambush");
    }
}
