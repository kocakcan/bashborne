use crate::game::chapter::ChapterId;
use crate::game::item::{ArmorFactory, Inventory, ItemFactory, RingFactory, WeaponFactory};
use crate::game::npc::NpcId;

/// Identifies a specific quest. Each variant has a fixed registry entry in
/// `quest_def` — a plain enum + exhaustive match, the same pattern used
/// throughout this codebase instead of string-keyed lookups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuestId {
    HerbalistsRequest,
    ScoutsCommendation,
    PilgrimsBlessing,
}

/// What must be true for a quest to be turned in. Deliberately single-stage
/// — no multi-step quests.
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
    }
}

/// Whether `objective` is currently satisfied, given what's in the party's
/// bag and which chapters' bosses have fallen so far.
pub fn objective_satisfied(
    objective: &QuestObjective,
    inventory: &Inventory,
    bosses_defeated: &std::collections::HashSet<ChapterId>,
) -> bool {
    match objective {
        QuestObjective::DeliverItem { item_name, qty } => inventory
            .items
            .iter()
            .any(|(item, have)| item.name == *item_name && *have >= *qty),
        QuestObjective::DefeatBoss(chapter) => bosses_defeated.contains(chapter),
    }
}

/// Tracks which quests the party has accepted and finished. A quest can
/// only ever be in one of "not yet offered", `active`, or `completed`.
pub struct QuestLog {
    pub active: Vec<QuestId>,
    pub completed: Vec<QuestId>,
}

impl QuestLog {
    pub fn new() -> Self {
        Self {
            active: Vec::new(),
            completed: Vec::new(),
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
        let objective = QuestObjective::DeliverItem {
            item_name: "Potion",
            qty: 4,
        };
        assert!(
            !objective_satisfied(&objective, &inv, &bosses_defeated),
            "the starting stock alone shouldn't satisfy the quest"
        );

        inv.add(crate::game::item::potion(), 1);
        assert!(objective_satisfied(&objective, &inv, &bosses_defeated));
    }

    #[test]
    fn defeat_boss_objective_checks_the_bosses_defeated_set() {
        let inv = Inventory::starting();
        let mut bosses_defeated = std::collections::HashSet::new();
        let objective = QuestObjective::DefeatBoss(ChapterId::One);
        assert!(!objective_satisfied(&objective, &inv, &bosses_defeated));

        bosses_defeated.insert(ChapterId::One);
        assert!(objective_satisfied(&objective, &inv, &bosses_defeated));
    }

    #[test]
    fn every_quest_registry_entry_has_a_unique_id_matching_its_lookup_key() {
        for id in [
            QuestId::HerbalistsRequest,
            QuestId::ScoutsCommendation,
            QuestId::PilgrimsBlessing,
        ] {
            assert_eq!(quest_def(id).id, id);
            assert!(!quest_def(id).rewards.is_empty());
        }
    }
}
