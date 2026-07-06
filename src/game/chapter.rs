use serde::{Deserialize, Serialize};

use crate::game::character::{ashen_sovereign, barrow_knight, wyrmscale_warden, Character};
use crate::game::map::{Map, Position};
use crate::game::npc::NpcId;

/// Identifies one of the game's chapters — each a full map with its own
/// boss lair. A plain enum + exhaustive `chapter_def` match, so adding a
/// new chapter is a compile error everywhere until it's wired up, rather
/// than a silent gap (the same reasoning CLAUDE.md gives for `GameState`
/// avoiding trait objects).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChapterId {
    One,
    Two,
    Three,
}

/// Identifies which boss a `Character` is, so `combat::resolve_enemy_action`
/// can dispatch scripted moves by matching this enum instead of comparing
/// display-name strings (which is how the Barrow Knight was special-cased
/// before this refactor — fragile, and it only gets more so with more bosses).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BossKind {
    BarrowKnight,
    WyrmscaleWarden,
    AshenSovereign,
}

/// Static data describing a chapter: its map, where the player spawns into
/// it, its boss, the NPCs placed on it, and which chapter (if any) follows
/// once its boss is defeated.
pub struct ChapterDef {
    pub id: ChapterId,
    pub name: &'static str,
    pub map: fn() -> Map,
    pub spawn: Position,
    pub boss: fn(&str) -> Character,
    pub boss_display_name: &'static str,
    /// The level every regular (non-boss) monster on this chapter's map is
    /// scaled up to via `Character::scale_to_level` — the knob that makes
    /// each chapter's tall grass genuinely harder than the last chapter's.
    pub enemy_level: u32,
    pub npcs: Vec<(Position, NpcId)>,
    /// The chapter that follows once this one's boss is defeated. `None`
    /// only for the final chapter.
    pub next: Option<ChapterId>,
}

pub fn chapter_def(id: ChapterId) -> ChapterDef {
    match id {
        ChapterId::One => ChapterDef {
            id,
            name: "The Barrow Fields",
            map: Map::starting_area,
            spawn: Position { x: 4, y: 2 },
            boss: barrow_knight,
            boss_display_name: "The Barrow Knight",
            enemy_level: 1,
            npcs: vec![
                (Position { x: 12, y: 5 }, NpcId::OldHerbalist),
                (Position { x: 2, y: 3 }, NpcId::Blacksmith),
            ],
            next: Some(ChapterId::Two),
        },
        ChapterId::Two => ChapterDef {
            id,
            name: "The Wyrmscale Marsh",
            map: Map::chapter_two,
            spawn: Position { x: 4, y: 7 },
            boss: wyrmscale_warden,
            boss_display_name: "Wyrmscale Warden",
            enemy_level: 4,
            npcs: vec![
                (Position { x: 10, y: 5 }, NpcId::WoundedScout),
                (Position { x: 3, y: 7 }, NpcId::Blacksmith),
            ],
            next: Some(ChapterId::Three),
        },
        ChapterId::Three => ChapterDef {
            id,
            name: "The Ashen Approach",
            map: Map::chapter_three,
            spawn: Position { x: 4, y: 2 },
            boss: ashen_sovereign,
            boss_display_name: "The Ashen Sovereign",
            enemy_level: 7,
            npcs: vec![
                (Position { x: 5, y: 5 }, NpcId::AshenPilgrim),
                (Position { x: 3, y: 2 }, NpcId::Blacksmith),
            ],
            next: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chapters_chain_one_to_two_to_three_and_then_end() {
        assert_eq!(chapter_def(ChapterId::One).next, Some(ChapterId::Two));
        assert_eq!(chapter_def(ChapterId::Two).next, Some(ChapterId::Three));
        assert!(chapter_def(ChapterId::Three).next.is_none());
    }

    #[test]
    fn each_chapters_boss_factory_sets_the_matching_boss_kind() {
        for id in [ChapterId::One, ChapterId::Two, ChapterId::Three] {
            let def = chapter_def(id);
            let boss = (def.boss)(def.boss_display_name);
            let expected = match id {
                ChapterId::One => BossKind::BarrowKnight,
                ChapterId::Two => BossKind::WyrmscaleWarden,
                ChapterId::Three => BossKind::AshenSovereign,
            };
            assert_eq!(boss.boss_kind, Some(expected));
        }
    }

    #[test]
    fn regular_enemy_levels_escalate_chapter_over_chapter() {
        let one = chapter_def(ChapterId::One).enemy_level;
        let two = chapter_def(ChapterId::Two).enemy_level;
        let three = chapter_def(ChapterId::Three).enemy_level;
        assert!(one < two && two < three);
    }

    #[test]
    fn every_chapters_npcs_and_spawn_sit_on_walkable_tiles() {
        for id in [ChapterId::One, ChapterId::Two, ChapterId::Three] {
            let def = chapter_def(id);
            let map = (def.map)();
            assert!(
                map.is_walkable(def.spawn),
                "{:?}'s spawn point should be walkable",
                id
            );
            for (pos, npc) in &def.npcs {
                assert!(
                    map.is_walkable(*pos),
                    "{:?}'s NPC {:?} should sit on a walkable tile",
                    id,
                    npc
                );
            }
        }
    }

    #[test]
    fn every_chapter_has_at_least_one_npc() {
        for id in [ChapterId::One, ChapterId::Two, ChapterId::Three] {
            assert!(
                !chapter_def(id).npcs.is_empty(),
                "{:?} should have an NPC",
                id
            );
        }
    }

    #[test]
    fn andre_is_reachable_in_every_chapter() {
        for id in [ChapterId::One, ChapterId::Two, ChapterId::Three] {
            let def = chapter_def(id);
            assert!(
                def.npcs.iter().any(|(_, npc)| *npc == NpcId::Blacksmith),
                "{:?} should place the blacksmith somewhere on its map",
                id
            );
        }
    }

    #[test]
    fn chapter_one_places_the_blacksmith_on_a_town_tile() {
        use crate::game::map::Tile;
        let def = chapter_def(ChapterId::One);
        let pos = Position { x: 2, y: 3 };
        assert!(
            def.npcs.contains(&(pos, NpcId::Blacksmith)),
            "Chapter One should place the blacksmith at {:?}",
            pos
        );
        let map = (def.map)();
        assert_eq!(
            map.tile_at(pos),
            Tile::Town,
            "the blacksmith should stand on a Town tile, available from the start"
        );
    }
}
