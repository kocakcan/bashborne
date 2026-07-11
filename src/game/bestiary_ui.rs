use crate::game::character::{
    ashen_sovereign, bandit, barrow_knight, barrow_sentinel, bat, carrion_crow, fell_acolyte,
    forsaken_knight, goblin, grave_ghoul, hollow, mimic, orc, rat, skeleton, slime, wolf, wraith,
    wyrmscale_warden, Character,
};
use crate::game::map::Position;

/// Read-only bestiary/codex screen: browses every species and boss the
/// player has faced at least once (`World.bestiary_seen`).
pub struct BestiaryUiState {
    pub cursor: usize,
    /// Where to place the player back on the map once this screen is closed.
    pub return_pos: Position,
}

impl BestiaryUiState {
    pub fn new(return_pos: Position) -> Self {
        Self {
            cursor: 0,
            return_pos,
        }
    }
}

/// One codex row. `name` doubles as the species tag (`Character.name`, the
/// same raw key `loot_profile`/`bestiary_seen` use) and as the argument the
/// detail panel passes to `factory` to get baseline level-1 stats.
pub struct BestiaryEntry {
    pub name: &'static str,
    pub factory: fn(&str) -> Character,
    /// One-line blurb paraphrasing the species' signature move (or lack of one).
    pub signature: &'static str,
    pub is_boss: bool,
}

/// The full codex: every species `roll_encounter` can field, the Mimic
/// (treasure ambushes only), and the three chapter bosses, in the rough
/// order the player is likely to meet them.
pub fn bestiary_entries() -> Vec<BestiaryEntry> {
    fn species(name: &'static str, factory: fn(&str) -> Character, signature: &'static str) -> BestiaryEntry {
        BestiaryEntry {
            name,
            factory,
            signature,
            is_boss: false,
        }
    }
    fn boss(name: &'static str, factory: fn(&str) -> Character, signature: &'static str) -> BestiaryEntry {
        BestiaryEntry {
            name,
            factory,
            signature,
            is_boss: true,
        }
    }
    vec![
        species("Slime", slime, "A quivering ooze with no tricks - it simply keeps coming."),
        species("Rat", rat, "A scrawny scavenger. Barely worth the swing that fells it."),
        species("Bat", bat, "Quick but frail; it darts in, bites, and hopes."),
        species("Goblin", goblin, "Picks on the weak: most attacks seek whoever's closest to death."),
        species("Wolf", wolf, "Hunts the frailest prey, favoring the member with the thinnest hide."),
        species("Hollow", hollow, "An emptied husk that swings out of habit more than hunger."),
        species("Carrion Crow", carrion_crow, "Circles the dying. Strikes without pattern or mercy."),
        species("Skeleton", skeleton, "Bone Guard: may forgo its attack to lock its joints for +3 defense."),
        species("Orc", orc, "Reckless Swing: a brutal 1.6x blow that tears at the orc's own arms."),
        species("Bandit", bandit, "Coin Grab: may snatch your gold instead of drawing blood."),
        species("Wraith", wraith, "Whispers a curse over the whole party instead of striking."),
        species("Grave Ghoul", grave_ghoul, "A patient corpse-eater; tough, hungry, and unhurried."),
        species("Fell Acolyte", fell_acolyte, "Withering Prayer: drains a member's mana to knit its own wounds."),
        species("Barrow Sentinel", barrow_sentinel, "Warcry: a bellow that saps the entire party's defense."),
        species("Forsaken Knight", forsaken_knight, "Knight's Judgment: a flawless 1.7x strike with no drawback."),
        species("Mimic", mimic, "Never roams the wilds. It waits inside treasure - one chest in five bites back."),
        boss("The Barrow Knight", barrow_knight, "Rending Cleave; near death it draws a Second Wind, once."),
        boss("Wyrmscale Warden", wyrmscale_warden, "Tail Sweep rakes the whole party; when pressed it rallies its scales, once."),
        boss("The Ashen Sovereign", ashen_sovereign, "Cinder Nova sears everyone; it rises from its own ashes twice."),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn entries_cover_every_species_and_boss_with_no_duplicates() {
        let entries = bestiary_entries();
        assert_eq!(entries.len(), 19, "16 species + 3 bosses");
        let names: HashSet<&str> = entries.iter().map(|e| e.name).collect();
        assert_eq!(names.len(), entries.len(), "entry names must be unique");
        // Every species the encounter table can roll, plus the treasure-only
        // Mimic, plus each chapter's boss under its display name.
        for expected in [
            "Slime", "Goblin", "Bat", "Wolf", "Skeleton", "Orc", "Wraith", "Mimic", "Hollow",
            "Rat", "Carrion Crow", "Bandit", "Fell Acolyte", "Grave Ghoul", "Barrow Sentinel",
            "Forsaken Knight", "The Barrow Knight", "Wyrmscale Warden", "The Ashen Sovereign",
        ] {
            assert!(names.contains(expected), "missing bestiary entry: {expected}");
        }
        assert_eq!(entries.iter().filter(|e| e.is_boss).count(), 3);
    }

    #[test]
    fn entry_factories_produce_the_matching_character() {
        for entry in bestiary_entries() {
            let c = (entry.factory)(entry.name);
            assert_eq!(c.name, entry.name);
            assert_eq!(c.boss_kind.is_some(), entry.is_boss, "{}", entry.name);
        }
    }
}
