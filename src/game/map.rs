#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    Floor,
    Wall,
    TallGrass, // random encounter zone
    Town,
    /// A fixed, one-time boss encounter — unlike TallGrass, stepping here
    /// always triggers a fight (until the boss has been defeated).
    BossLair,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

pub struct Map {
    pub width: i32,
    pub height: i32,
    pub tiles: Vec<Tile>,
    /// Fixed-position NPCs on this map. Kept alongside the tile grid rather
    /// than as a `Tile` variant, since NPCs are per-map "furniture" with
    /// identity (which NPC, and later which dialogue/quest state), not a
    /// terrain type — the underlying tile they stand on is ordinary floor.
    pub npcs: Vec<(Position, crate::game::npc::NpcId)>,
}

impl Map {
    /// Shared parser for the hand-authored ASCII layouts below: '#' wall,
    /// '.' floor, ',' tall grass, 'T' town floor, 'B' boss lair. Every row
    /// must be the same length.
    fn from_layout(layout: &[&str]) -> Self {
        let height = layout.len() as i32;
        let width = layout[0].len() as i32;
        let mut tiles = Vec::with_capacity((width * height) as usize);
        for row in layout.iter() {
            for c in row.chars() {
                tiles.push(match c {
                    '#' => Tile::Wall,
                    '.' => Tile::Floor,
                    ',' => Tile::TallGrass,
                    'T' => Tile::Town,
                    'B' => Tile::BossLair,
                    _ => Tile::Floor,
                });
            }
        }
        Self {
            width,
            height,
            tiles,
            npcs: Vec::new(),
        }
    }

    /// Chapter one: a walled town square opening onto tall grass, with the
    /// Barrow Knight's lair tucked in the far corner.
    pub fn starting_area() -> Self {
        Self::from_layout(&[
            "############################",
            "#TTTTTTTT..................#",
            "#TTTTTTTT..................#",
            "#TTTTTTTT........,,,,,,,,,,#",
            "#TTTTTTTT........,,,,,,,,,,#",
            "#........................,,#",
            "#........................,,#",
            "#..............,,,,,,,,,,,,#",
            "#..............,,,,,,,,,,,B#",
            "############################",
        ])
    }

    /// Chapter two: the Wyrmscale Marsh — town tucked in the corner this
    /// time, with tall grass spreading toward the Warden's lair.
    pub fn chapter_two() -> Self {
        Self::from_layout(&[
            "############################",
            "#..............,,,,,,,,,,,B#",
            "#..............,,,,,,,,,,,,#",
            "#..............,,,,,,,,,,,,#",
            "#..............,,,,,,,,,,,,#",
            "#..........................#",
            "#TTTTTTTT..................#",
            "#TTTTTTTT..................#",
            "#TTTTTTTT..................#",
            "############################",
        ])
    }

    /// Chapter three: the approach to the Ashen Sovereign's throne.
    pub fn chapter_three() -> Self {
        Self::from_layout(&[
            "############################",
            "#TTTTTTTT..................#",
            "#TTTTTTTT..................#",
            "#TTTTTTTT...,,,,,,,,,,,,,,,#",
            "#...........,,,,,,,,,,,,,,,#",
            "#...........,,,,,,,,,,,,,,,#",
            "#...........,,,,,,,,,,,,,,,#",
            "#...........,,,,,,,,,,,,,,,#",
            "#...........,,,,,,,,,,,,,,B#",
            "############################",
        ])
    }

    pub fn tile_at(&self, pos: Position) -> Tile {
        if pos.x < 0 || pos.y < 0 || pos.x >= self.width || pos.y >= self.height {
            return Tile::Wall;
        }
        self.tiles[(pos.y * self.width + pos.x) as usize]
    }

    pub fn is_walkable(&self, pos: Position) -> bool {
        self.tile_at(pos) != Tile::Wall
    }

    /// The NPC standing at `pos`, if any.
    pub fn npc_at(&self, pos: Position) -> Option<crate::game::npc::NpcId> {
        self.npcs
            .iter()
            .find(|(p, _)| *p == pos)
            .map(|(_, id)| *id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starting_area_contains_exactly_one_boss_lair() {
        let map = Map::starting_area();
        let count = map.tiles.iter().filter(|&&t| t == Tile::BossLair).count();
        assert_eq!(count, 1, "there should be exactly one boss lair on the map");
    }

    #[test]
    fn the_boss_lair_is_walkable() {
        let map = Map::starting_area();
        let pos = (0..map.height)
            .flat_map(|y| (0..map.width).map(move |x| Position { x, y }))
            .find(|&p| map.tile_at(p) == Tile::BossLair)
            .expect("boss lair should exist");
        assert!(map.is_walkable(pos));
    }

    #[test]
    fn every_chapter_map_contains_exactly_one_walkable_boss_lair() {
        for map in [Map::starting_area(), Map::chapter_two(), Map::chapter_three()] {
            let count = map.tiles.iter().filter(|&&t| t == Tile::BossLair).count();
            assert_eq!(count, 1, "each chapter map should have exactly one boss lair");
            let pos = (0..map.height)
                .flat_map(|y| (0..map.width).map(move |x| Position { x, y }))
                .find(|&p| map.tile_at(p) == Tile::BossLair)
                .expect("boss lair should exist");
            assert!(map.is_walkable(pos));
        }
    }

    #[test]
    fn npc_at_finds_a_placed_npc_and_nothing_elsewhere() {
        use crate::game::npc::NpcId;

        let mut map = Map::starting_area();
        let spot = Position { x: 2, y: 2 };
        map.npcs.push((spot, NpcId::OldHerbalist));

        assert_eq!(map.npc_at(spot), Some(NpcId::OldHerbalist));
        assert_eq!(map.npc_at(Position { x: 0, y: 0 }), None);
    }
}
