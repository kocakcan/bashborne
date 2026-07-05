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
}

impl Map {
    pub fn starting_area() -> Self {
        // Simple hand-authored layout: a walled town square opening onto tall grass,
        // with a boss lair tucked in the far corner. '#' wall, '.' floor, ',' tall
        // grass, 'T' town floor, 'B' boss lair.
        let layout = [
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
        ];
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
        }
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
}
