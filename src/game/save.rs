use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::game::chapter::ChapterId;
use crate::game::item::Inventory;
use crate::game::map::Position;
use crate::game::npc::NpcId;
use crate::game::party::Party;
use crate::game::quest::QuestLog;

/// Bumped whenever `SaveData`'s shape changes incompatibly; `read` treats a
/// mismatched version the same as no save at all rather than half-loading it.
pub const SAVE_VERSION: u32 = 1;

/// Everything that persists between sessions. Deliberately only ever
/// captured while exploring — combat, menus, and dialogue are transient, so
/// a loaded game always resumes standing on the map at `player_pos`.
#[derive(Serialize, Deserialize)]
pub struct SaveData {
    pub version: u32,
    pub party: Party,
    pub inventory: Inventory,
    pub current_chapter: ChapterId,
    pub bosses_defeated: HashSet<ChapterId>,
    pub npc_flags: HashSet<NpcId>,
    pub quest_log: QuestLog,
    pub player_pos: Position,
    /// New Game+ cycle (0 = first playthrough, capped at 7). Older saves
    /// without this field default to 0 rather than failing to load.
    #[serde(default)]
    pub ng_plus: u32,
}

/// How many save slots the main menu offers. Slot 3 is labeled "(Dev)" in
/// the UI by convention only (`render/main_menu.rs`) — no mechanical
/// difference from the other two, just an obvious sandbox to playtest in
/// without touching real progress.
pub const SAVE_SLOTS: u8 = 3;

/// Where slot `slot`'s save file lives: next to wherever the game is run
/// from. `slot` is 1-indexed (1..=SAVE_SLOTS), matching `World.active_slot`
/// and `MainMenuState::slots`' on-screen numbering.
pub fn save_path(slot: u8) -> PathBuf {
    PathBuf::from(format!("bashborne_save_{slot}.json"))
}

/// Where the single save file lived before the 3-slot system existed.
fn legacy_save_path() -> PathBuf {
    PathBuf::from("bashborne_save.json")
}

/// One-time migration: if slot 1 has no file yet but the pre-slot save does,
/// claim it as slot 1 rather than orphaning it. Cheap to call on every read
/// of slot 1 since it's a no-op once the rename has happened.
fn migrate_legacy_save() {
    let legacy = legacy_save_path();
    let slot_one = save_path(1);
    if !slot_one.exists() && legacy.exists() {
        let _ = fs::rename(&legacy, &slot_one);
    }
}

pub fn write(data: &SaveData, slot: u8) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(data)?;
    fs::write(save_path(slot), json)?;
    Ok(())
}

/// Reads `slot`'s save file, if one exists and parses as the current
/// version. Any failure (missing, corrupt, stale version) just means "no
/// save" — the game starts fresh rather than crashing on a bad file.
pub fn read(slot: u8) -> Option<SaveData> {
    if slot == 1 {
        migrate_legacy_save();
    }
    let json = fs::read_to_string(save_path(slot)).ok()?;
    let data: SaveData = serde_json::from_str(&json).ok()?;
    (data.version == SAVE_VERSION).then_some(data)
}

/// Reads every slot at once, for the main menu's slot picker.
pub fn read_all_slots() -> [Option<SaveData>; SAVE_SLOTS as usize] {
    [read(1), read(2), read(3)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::character::{rogue, warrior};
    use crate::game::item::{iron_sword, potion};
    use crate::game::quest::QuestId;

    // No test here touches the real filesystem (`read`/`write`/
    // `migrate_legacy_save`) — `cargo test`'s working directory is the repo
    // root, the same place a real save file lives, and this repo's own
    // `bashborne_save.json` is real player progress. `save_path` is pure
    // (just builds a `PathBuf`), so it's safe to exercise directly.
    #[test]
    fn save_path_is_distinct_per_slot() {
        assert_ne!(save_path(1), save_path(2));
        assert_ne!(save_path(2), save_path(3));
        assert_ne!(save_path(1), save_path(3));
    }

    fn sample_save() -> SaveData {
        let mut party = Party::new(vec![warrior("Bram"), rogue("Wren")]);
        party.gold = 321;
        party.members[0].gain_xp(60); // level 2 with growth applied
        let mut inventory = Inventory::starting();
        inventory.add(potion(), 2);
        inventory.add_weapon(iron_sword());
        inventory.upgrade_materials = 7;
        let mut quest_log = QuestLog::new();
        quest_log.accept(QuestId::HerbalistsRequest);
        SaveData {
            version: SAVE_VERSION,
            party,
            inventory,
            current_chapter: ChapterId::Two,
            bosses_defeated: HashSet::from([ChapterId::One]),
            npc_flags: HashSet::from([NpcId::OldHerbalist]),
            quest_log,
            player_pos: Position { x: 4, y: 7 },
            ng_plus: 0,
        }
    }

    #[test]
    fn save_data_round_trips_through_json() {
        let data = sample_save();
        let json = serde_json::to_string(&data).expect("save should serialize");
        let back: SaveData = serde_json::from_str(&json).expect("save should deserialize");

        assert_eq!(back.version, SAVE_VERSION);
        assert_eq!(back.party.gold, 321);
        assert_eq!(back.party.members.len(), 2);
        assert_eq!(back.party.members[0].level, 2);
        assert_eq!(
            back.party.members[0].stats.max_hp,
            data.party.members[0].stats.max_hp
        );
        assert_eq!(back.party.members[1].name, "Wren");
        assert_eq!(
            back.party.members[1]
                .equipped_weapon
                .as_ref()
                .map(|w| w.name.as_str()),
            Some("Thief's Dirk")
        );
        assert_eq!(back.inventory.upgrade_materials, 7);
        assert!(back
            .inventory
            .weapons
            .iter()
            .any(|w| w.name == "Iron Sword"));
        assert_eq!(back.current_chapter, ChapterId::Two);
        assert!(back.bosses_defeated.contains(&ChapterId::One));
        assert!(back.npc_flags.contains(&NpcId::OldHerbalist));
        assert!(back.quest_log.is_active(QuestId::HerbalistsRequest));
        assert_eq!(back.player_pos, Position { x: 4, y: 7 });
    }

    #[test]
    fn a_save_json_missing_ng_plus_defaults_to_zero() {
        // Simulates a save file written before NG+ existed.
        let mut value: serde_json::Value = serde_json::to_value(sample_save()).unwrap();
        value.as_object_mut().unwrap().remove("ng_plus");
        let back: SaveData = serde_json::from_value(value).expect("old saves should still parse");
        assert_eq!(back.ng_plus, 0);
    }

    #[test]
    fn a_save_json_missing_is_elite_defaults_to_false() {
        // Simulates a save file written before elite variants existed.
        let mut value: serde_json::Value = serde_json::to_value(sample_save()).unwrap();
        value["party"]["members"][0]
            .as_object_mut()
            .unwrap()
            .remove("is_elite");
        let back: SaveData = serde_json::from_value(value).expect("old saves should still parse");
        assert!(!back.party.members[0].is_elite);
    }

    #[test]
    fn a_stale_save_version_reads_as_no_save() {
        let mut data = sample_save();
        data.version = SAVE_VERSION + 1;
        let json = serde_json::to_string(&data).unwrap();
        let back: SaveData = serde_json::from_str(&json).unwrap();
        // `read` itself hits the filesystem, so check its version gate directly.
        assert!(
            (back.version == SAVE_VERSION).then_some(back).is_none(),
            "a future/stale version must be rejected"
        );
    }
}
