use rand::Rng;

use crate::game::blacksmith::BlacksmithUiState;
use crate::game::chapter::{chapter_def, ChapterId};
use crate::game::character::Character;
use crate::game::combat::CombatState;
use crate::game::inventory_ui::InventoryUiState;
use crate::game::item::{
    dragonslayers_oath, ember_of_return, ether, iron_loop, iron_sword, moonlit_greatsword,
    padded_vest, potion, rangers_cloak, ring_of_favor, sovereign_elixir, sunken_relic_blade,
    sunlit_straightsword, travelers_chestguard, travelers_ring, travelers_spear, warded_loop,
    worn_leather_jerkin, Armor, Item, Ring, Weapon,
};
use crate::game::levelup::LevelUpUiState;
use crate::game::map::{Map, Position};
use crate::game::npc::NpcId;
use crate::game::quest_ui::QuestLogUiState;
use crate::game::shop::ShopUiState;
use crate::game::status::{roll_blessing, roll_curse, StatusEffect};

/// Same cap as `CombatState::push_log` — keeps a long session's log from
/// growing unbounded while still holding plenty of scrollback.
const MAX_LOG_LINES: usize = 200;

pub struct ExploreState {
    pub map: Map,
    pub player_pos: Position,
    pub log: Vec<String>,
    pub steps_in_grass: u32,
    /// Lines of scrollback from the bottom of `log` currently displayed.
    pub log_scroll: usize,
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
            log: vec![format!(
                "You arrive at {}. Watch the tall grass...",
                def.name
            )],
            steps_in_grass: 0,
            log_scroll: 0,
        }
    }

    /// Appends a line to the log, capping it at `MAX_LOG_LINES` and
    /// resetting scrollback so the newest line is always visible.
    pub fn push_log(&mut self, line: impl Into<String>) {
        self.log.push(line.into());
        if self.log.len() > MAX_LOG_LINES {
            self.log.remove(0);
        }
        self.log_scroll = 0;
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

/// The title screen shown at startup, before any world state matters.
pub struct MainMenuState {
    pub cursor: usize,
    /// Whether a valid save exists — gates the Continue entry.
    pub has_save: bool,
    /// True while "New Game" is waiting for the player to confirm
    /// abandoning the existing save.
    pub confirm_overwrite: bool,
}

/// An entry on the main menu — which ones appear depends on `has_save`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainMenuEntry {
    Continue,
    NewGame,
    Quit,
}

impl MainMenuEntry {
    pub fn label(self) -> &'static str {
        match self {
            MainMenuEntry::Continue => "Continue",
            MainMenuEntry::NewGame => "New Game",
            MainMenuEntry::Quit => "Quit",
        }
    }
}

impl MainMenuState {
    pub fn new(has_save: bool) -> Self {
        Self {
            cursor: 0,
            has_save,
            confirm_overwrite: false,
        }
    }

    pub fn entries(&self) -> Vec<MainMenuEntry> {
        if self.has_save {
            vec![
                MainMenuEntry::Continue,
                MainMenuEntry::NewGame,
                MainMenuEntry::Quit,
            ]
        } else {
            vec![MainMenuEntry::NewGame, MainMenuEntry::Quit]
        }
    }
}

pub enum GameState {
    MainMenu(MainMenuState),
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
/// gets genuinely harder chapter over chapter. `ng_plus` layers the New
/// Game+ multiplier on top for players replaying a harder cycle.
pub fn roll_encounter(rng: &mut impl Rng, enemy_level: u32, ng_plus: u32) -> Vec<Character> {
    use crate::game::character::{
        bandit, barrow_sentinel, bat, carrion_crow, fell_acolyte, forsaken_knight, goblin,
        grave_ghoul, hollow, orc, rat, skeleton, slime, wolf, wraith,
    };
    let mut enemies = match rng.gen_range(0..20) {
        0 => vec![slime("Slime")],
        1 => vec![slime("Slime"), slime("Slime")],
        2 => vec![goblin("Goblin")],
        3 => vec![bat("Bat"), bat("Bat")],
        4 => vec![wolf("Wolf")],
        5 => vec![wolf("Wolf"), bat("Bat")],
        6 => vec![skeleton("Skeleton")],
        7 => vec![orc("Orc")],
        8 => vec![wraith("Wraith")],
        9 => vec![goblin("Goblin"), bat("Bat")],
        10 => vec![hollow("Hollow")],
        11 => vec![hollow("Hollow"), hollow("Hollow")],
        12 => vec![rat("Rat"), rat("Rat"), rat("Rat")],
        13 => vec![carrion_crow("Carrion Crow"), bat("Bat")],
        14 => vec![bandit("Bandit"), bandit("Bandit")],
        15 => vec![fell_acolyte("Fell Acolyte"), hollow("Hollow")],
        16 => vec![grave_ghoul("Grave Ghoul")],
        17 => vec![grave_ghoul("Grave Ghoul"), hollow("Hollow")],
        18 => vec![barrow_sentinel("Barrow Sentinel")],
        // The toughest single regular composition there is.
        _ => vec![forsaken_knight("Forsaken Knight")],
    };
    for enemy in &mut enemies {
        enemy.scale_to_level(enemy_level);
        enemy.apply_ng_plus(ng_plus);
    }
    // Promote exactly one enemy in the composition to an Elite variant,
    // with rising odds each NG+ cycle — never a Mimic or boss, since
    // neither ever appears in this table.
    if rng.gen_bool(crate::game::character::Character::elite_chance(ng_plus) as f64) {
        let idx = rng.gen_range(0..enemies.len());
        enemies[idx].apply_elite();
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
        armor: Option<Armor>,
        ring: Option<Ring>,
        /// Bonus Titanite Shards buried alongside the treasure — see
        /// `Inventory::upgrade_materials`.
        materials: u32,
    },
}

/// Weighted table for grass encounters: 50% combat, 15% blessing, 20% curse,
/// 15% treasure. Tune the ranges below to rebalance. `enemy_level` scales
/// any combat outcome (including a Mimic ambush) to the current chapter;
/// `ng_plus` layers the New Game+ multiplier on top.
pub fn roll_field_event(rng: &mut impl Rng, enemy_level: u32, ng_plus: u32) -> FieldEvent {
    match rng.gen_range(0..20) {
        0..=9 => FieldEvent::Combat(roll_encounter(rng, enemy_level, ng_plus)), // 10/20
        10..=12 => FieldEvent::Blessing(roll_blessing(rng, ng_plus)),           // 3/20
        13..=16 => FieldEvent::Curse(roll_curse(rng, ng_plus)),                 // 4/20
        _ => {
            // 3/20: usually real treasure, but occasionally it bites back.
            if rng.gen_ratio(1, 5) {
                let mut mimic = crate::game::character::mimic("Mimic");
                mimic.scale_to_level(enemy_level);
                mimic.apply_ng_plus(ng_plus);
                FieldEvent::Combat(vec![mimic])
            } else {
                let gold = rng.gen_range(10..=25);
                // Mostly basic consumables, with rarer restoratives at the
                // tail so caches occasionally feel like a real find.
                let item = if rng.gen_bool(0.5) {
                    Some(match rng.gen_range(0..12) {
                        0..=4 => potion(),
                        5..=8 => ether(),
                        9..=10 => ember_of_return(),
                        _ => sovereign_elixir(),
                    })
                } else {
                    None
                };
                // Rare on top of that: the cache also held a weapon. Weighted
                // toward common/uncommon finds, with a vanishingly small
                // chance (roughly 1-in-100 overall) of a legendary relic.
                let weapon = if rng.gen_ratio(1, 8) {
                    Some(match rng.gen_range(0..12) {
                        0..=3 => iron_sword(),
                        4..=6 => travelers_spear(),
                        7..=8 => sunlit_straightsword(),
                        9 => sunken_relic_blade(),
                        10 => moonlit_greatsword(),
                        _ => dragonslayers_oath(),
                    })
                } else {
                    None
                };
                // Independent rolls for buried armor and rings — combat
                // isn't the only way to round out the party's gear.
                let armor = if rng.gen_ratio(1, 10) {
                    Some(match rng.gen_range(0..4) {
                        0 => padded_vest(),
                        1 => worn_leather_jerkin(),
                        2 => travelers_chestguard(),
                        _ => rangers_cloak(),
                    })
                } else {
                    None
                };
                let ring = if rng.gen_ratio(1, 12) {
                    Some(match rng.gen_range(0..4) {
                        0 => iron_loop(),
                        1 => travelers_ring(),
                        2 => warded_loop(),
                        _ => ring_of_favor(),
                    })
                } else {
                    None
                };
                // Independent chance of bonus Titanite Shards buried
                // alongside the treasure, so exploring (not just fighting)
                // feeds the blacksmith too.
                let materials = if rng.gen_ratio(1, 5) {
                    rng.gen_range(1..=3)
                } else {
                    0
                };
                FieldEvent::Treasure {
                    gold,
                    item,
                    weapon,
                    armor,
                    ring,
                    materials,
                }
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
            let ch1 = roll_encounter(&mut StdRng::seed_from_u64(seed), 1, 0);
            let ch3 = roll_encounter(&mut StdRng::seed_from_u64(seed), 7, 0);
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
    fn ng_plus_makes_the_same_encounter_hit_harder() {
        // Same seed, same composition/level; only the NG+ multiplier differs.
        for seed in 0..20u64 {
            let base = roll_encounter(&mut StdRng::seed_from_u64(seed), 1, 0);
            let buffed = roll_encounter(&mut StdRng::seed_from_u64(seed), 1, 3);
            for (weak, strong) in base.iter().zip(buffed.iter()) {
                assert_eq!(weak.name, strong.name);
                assert_eq!(weak.level, strong.level);
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
            if let FieldEvent::Combat(enemies) = roll_field_event(&mut rng, 7, 0) {
                if enemies.iter().any(|e| e.name == "Mimic") {
                    assert!(enemies.iter().all(|e| e.level == 7));
                    saw_a_mimic = true;
                    break;
                }
            }
        }
        assert!(saw_a_mimic, "some seed should roll a Mimic ambush");
    }

    #[test]
    fn at_most_one_enemy_per_encounter_is_promoted_to_elite() {
        // NG+7 maxes the elite odds (24%), so a seed sweep should turn up
        // plenty of promotions to check the "at most one" invariant against.
        let mut saw_an_elite = false;
        for seed in 0..300u64 {
            let enemies = roll_encounter(&mut StdRng::seed_from_u64(seed), 1, 7);
            let elite_count = enemies.iter().filter(|e| e.is_elite).count();
            assert!(elite_count <= 1, "seed {seed}: more than one elite in a single encounter");
            if elite_count == 1 {
                saw_an_elite = true;
            }
        }
        assert!(saw_an_elite, "some seed should promote an enemy to elite");
    }

    #[test]
    fn field_event_weights_keep_all_four_outcomes_reachable() {
        let (mut saw_combat, mut saw_blessing, mut saw_curse, mut saw_treasure) =
            (false, false, false, false);
        for seed in 0..200u64 {
            match roll_field_event(&mut StdRng::seed_from_u64(seed), 1, 0) {
                FieldEvent::Combat(_) => saw_combat = true,
                FieldEvent::Blessing(_) => saw_blessing = true,
                FieldEvent::Curse(_) => saw_curse = true,
                FieldEvent::Treasure { .. } => saw_treasure = true,
            }
        }
        assert!(saw_combat && saw_blessing && saw_curse && saw_treasure);
    }
}
