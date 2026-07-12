use serde::{Deserialize, Serialize};

use crate::game::chapter::ChapterId;
use crate::game::item::{ArmorFactory, Inventory, ItemFactory, RingFactory, WeaponFactory};
use crate::game::npc::NpcId;

/// Identifies a specific quest. Each variant has a fixed registry entry in
/// `quest_def` — a plain enum + exhaustive match, the same pattern used
/// throughout this codebase instead of string-keyed lookups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QuestId {
    HerbalistsRequest,
    ScoutsCommendation,
    PilgrimsBlessing,
    ExilesVengeance,
}

/// What must be true for a quest to be turned in. One condition per quest —
/// not a multi-step chain — but the condition itself can require ongoing
/// progress (`KillCount`), not just a fact that's already true or false.
pub enum QuestObjective {
    /// The party's shared bag contains at least `qty` of the named item.
    /// `qty` is deliberately set above what the party starts with, so the
    /// quest can't be trivially satisfied the moment it's offered — the
    /// player has to actually go buy or find more.
    DeliverItem { item_name: &'static str, qty: u32 },
    /// The named chapter's boss has already fallen. Since reaching a later
    /// chapter's NPCs requires having beaten every earlier chapter's boss,
    /// this is a retroactive commendation rather than a wait-and-see
    /// objective — it's already true the moment the NPC can be reached.
    DefeatBoss(ChapterId),
    /// The party has defeated at least `count` enemies of the named species
    /// (matched against `Character.name`, the same raw-name convention
    /// `combat::resolve_enemy_action` uses) since `QuestLog::record_kill`
    /// started tracking it — tracked cumulatively, not reset on accept.
    KillCount { species: &'static str, count: u32 },
}

/// One component of a quest's payout. A quest can pay out several of these
/// at once (e.g. gold plus a piece of gear).
pub enum QuestReward {
    Gold(u32),
    Item(ItemFactory),
    Weapon(WeaponFactory),
    Armor(ArmorFactory),
    Ring(RingFactory),
}

pub struct Quest {
    pub id: QuestId,
    pub giver: NpcId,
    pub title: &'static str,
    pub objective: QuestObjective,
    pub rewards: Vec<QuestReward>,
}

pub fn quest_def(id: QuestId) -> Quest {
    match id {
        QuestId::HerbalistsRequest => Quest {
            id,
            giver: NpcId::OldHerbalist,
            title: "A Herbalist's Request",
            objective: QuestObjective::DeliverItem {
                item_name: "Potion",
                qty: 4,
            },
            rewards: vec![
                QuestReward::Gold(30),
                QuestReward::Armor(crate::game::item::rangers_cloak),
            ],
        },
        QuestId::ScoutsCommendation => Quest {
            id,
            giver: NpcId::WoundedScout,
            title: "The Scout's Commendation",
            objective: QuestObjective::DefeatBoss(ChapterId::One),
            rewards: vec![
                QuestReward::Gold(60),
                QuestReward::Weapon(crate::game::item::sunken_relic_blade),
            ],
        },
        QuestId::PilgrimsBlessing => Quest {
            id,
            giver: NpcId::AshenPilgrim,
            title: "The Pilgrim's Blessing",
            objective: QuestObjective::DefeatBoss(ChapterId::Two),
            rewards: vec![
                QuestReward::Gold(100),
                QuestReward::Ring(crate::game::item::band_of_the_barrow),
            ],
        },
        QuestId::ExilesVengeance => Quest {
            id,
            giver: NpcId::ExiledKnight,
            title: "The Exile's Vengeance",
            objective: QuestObjective::KillCount {
                species: "Grave Ghoul",
                count: 5,
            },
            rewards: vec![
                QuestReward::Gold(150),
                QuestReward::Ring(crate::game::item::knights_absolution),
            ],
        },
    }
}

/// Whether `objective` is currently satisfied, given what's in the party's
/// bag, which chapters' bosses have fallen so far, and the quest log's
/// kill-count tracking.
pub fn objective_satisfied(
    objective: &QuestObjective,
    inventory: &Inventory,
    bosses_defeated: &std::collections::HashSet<ChapterId>,
    quest_log: &QuestLog,
) -> bool {
    match objective {
        QuestObjective::DeliverItem { item_name, qty } => inventory
            .items
            .iter()
            .any(|(item, have)| item.name == *item_name && *have >= *qty),
        QuestObjective::DefeatBoss(chapter) => bosses_defeated.contains(chapter),
        QuestObjective::KillCount { species, count } => {
            quest_log.kill_progress.get(*species).copied().unwrap_or(0) >= *count
        }
    }
}

/// Tracks which quests the party has accepted and finished. A quest can
/// only ever be in one of "not yet offered", `active`, or `completed`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestLog {
    pub active: Vec<QuestId>,
    pub completed: Vec<QuestId>,
    /// Cumulative kills per species name, feeding `QuestObjective::KillCount`.
    /// Tracked unconditionally (not just while a relevant quest is active),
    /// so a `KillCount` quest offered after the fact can credit prior kills.
    #[serde(default)]
    pub kill_progress: std::collections::HashMap<String, u32>,
}

impl QuestLog {
    pub fn new() -> Self {
        Self {
            active: Vec::new(),
            completed: Vec::new(),
            kill_progress: std::collections::HashMap::new(),
        }
    }

    pub fn accept(&mut self, id: QuestId) {
        if !self.active.contains(&id) && !self.completed.contains(&id) {
            self.active.push(id);
        }
    }

    pub fn is_active(&self, id: QuestId) -> bool {
        self.active.contains(&id)
    }

    pub fn complete(&mut self, id: QuestId) {
        self.active.retain(|&q| q != id);
        if !self.completed.contains(&id) {
            self.completed.push(id);
        }
    }

    /// Increments the kill counter for `species` by one.
    pub fn record_kill(&mut self, species: &str) {
        *self.kill_progress.entry(species.to_string()).or_insert(0) += 1;
    }
}

impl Default for QuestLog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepting_a_quest_makes_it_active_and_only_once() {
        let mut log = QuestLog::new();
        log.accept(QuestId::HerbalistsRequest);
        log.accept(QuestId::HerbalistsRequest);
        assert_eq!(log.active, vec![QuestId::HerbalistsRequest]);
    }

    #[test]
    fn completing_a_quest_moves_it_from_active_to_completed() {
        let mut log = QuestLog::new();
        log.accept(QuestId::HerbalistsRequest);
        log.complete(QuestId::HerbalistsRequest);
        assert!(log.active.is_empty());
        assert_eq!(log.completed, vec![QuestId::HerbalistsRequest]);
    }

    #[test]
    fn a_completed_quest_cannot_be_re_accepted() {
        let mut log = QuestLog::new();
        log.accept(QuestId::HerbalistsRequest);
        log.complete(QuestId::HerbalistsRequest);
        log.accept(QuestId::HerbalistsRequest);
        assert!(log.active.is_empty());
    }

    #[test]
    fn deliver_item_objective_requires_more_than_the_starting_stock() {
        let mut inv = Inventory::starting(); // starts with 3 Potions
        let bosses_defeated = std::collections::HashSet::new();
        let log = QuestLog::new();
        let objective = QuestObjective::DeliverItem {
            item_name: "Potion",
            qty: 4,
        };
        assert!(
            !objective_satisfied(&objective, &inv, &bosses_defeated, &log),
            "the starting stock alone shouldn't satisfy the quest"
        );

        inv.add(crate::game::item::potion(), 1);
        assert!(objective_satisfied(&objective, &inv, &bosses_defeated, &log));
    }

    #[test]
    fn defeat_boss_objective_checks_the_bosses_defeated_set() {
        let inv = Inventory::starting();
        let mut bosses_defeated = std::collections::HashSet::new();
        let log = QuestLog::new();
        let objective = QuestObjective::DefeatBoss(ChapterId::One);
        assert!(!objective_satisfied(&objective, &inv, &bosses_defeated, &log));

        bosses_defeated.insert(ChapterId::One);
        assert!(objective_satisfied(&objective, &inv, &bosses_defeated, &log));
    }

    #[test]
    fn record_kill_increments_only_the_named_species() {
        let mut log = QuestLog::new();
        log.record_kill("Orc");
        log.record_kill("Orc");
        log.record_kill("Goblin");
        assert_eq!(log.kill_progress.get("Orc"), Some(&2));
        assert_eq!(log.kill_progress.get("Goblin"), Some(&1));
        assert_eq!(log.kill_progress.get("Wolf"), None);
    }

    #[test]
    fn kill_count_objective_is_satisfied_once_the_threshold_is_met() {
        let inv = Inventory::starting();
        let bosses_defeated = std::collections::HashSet::new();
        let mut log = QuestLog::new();
        let objective = QuestObjective::KillCount {
            species: "Orc",
            count: 3,
        };
        assert!(!objective_satisfied(&objective, &inv, &bosses_defeated, &log));

        log.record_kill("Orc");
        log.record_kill("Orc");
        assert!(!objective_satisfied(&objective, &inv, &bosses_defeated, &log));

        log.record_kill("Orc");
        assert!(objective_satisfied(&objective, &inv, &bosses_defeated, &log));
    }

    #[test]
    fn every_quest_registry_entry_has_a_unique_id_matching_its_lookup_key() {
        for id in [
            QuestId::HerbalistsRequest,
            QuestId::ScoutsCommendation,
            QuestId::PilgrimsBlessing,
            QuestId::ExilesVengeance,
        ] {
            assert_eq!(quest_def(id).id, id);
            assert!(!quest_def(id).rewards.is_empty());
        }
    }
}
