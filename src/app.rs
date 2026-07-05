use crossterm::event::KeyCode;
use rand::rngs::ThreadRng;
use rand::Rng;

use crate::game::chapter::{chapter_def, ChapterId};
use crate::game::character::{cleric, mage, warrior, Character, RingSlot};
use crate::game::combat::{ActorRef, CombatAction, CombatPhase, CombatState};
use crate::game::inventory_ui::{EquipSlot, InventoryMode, InventoryTab, InventoryUiState, EQUIP_SLOTS};
use crate::game::item::{Armor, Inventory, Ring, Weapon};
use crate::game::map::{Position, Tile};
use crate::game::npc::{npc_def, NpcId};
use crate::game::party::Party;
use crate::game::shop::{
    shop_armor_stock, shop_item_stock, shop_ring_stock, shop_weapon_stock, ShopMode, ShopTab,
    ShopUiState,
};
use crate::game::state::{roll_field_event, EventState, ExploreState, FieldEvent, GameState};

pub struct World {
    pub party: Party,
    pub inventory: Inventory,
    pub state: GameState,
    pub rng: ThreadRng,
    pub should_quit: bool,
    /// Which chapter the party is currently on — determines the active map,
    /// its boss, and its NPCs.
    pub current_chapter: ChapterId,
    /// Which chapters' bosses have been slain — once a chapter's boss is in
    /// here, its lair tile no longer triggers a fight.
    pub bosses_defeated: std::collections::HashSet<ChapterId>,
    /// Which NPCs the player has already talked to at least once — used to
    /// pick intro vs. repeat dialogue.
    pub npc_flags: std::collections::HashSet<NpcId>,
    pub quest_log: crate::game::quest::QuestLog,
}

impl World {
    pub fn new() -> Self {
        let party = Party::new(vec![
            warrior("Bram"),
            mage("Sella"),
            cleric("Idris"),
        ]);
        Self {
            party,
            inventory: Inventory::starting(),
            state: GameState::Explore(ExploreState::for_chapter(ChapterId::One)),
            rng: rand::thread_rng(),
            should_quit: false,
            current_chapter: ChapterId::One,
            bosses_defeated: std::collections::HashSet::new(),
            npc_flags: std::collections::HashSet::new(),
            quest_log: crate::game::quest::QuestLog::new(),
        }
    }

    pub fn handle_key(&mut self, key: KeyCode) {
        if key == KeyCode::Char('q') && matches!(self.state, GameState::Explore(_)) {
            self.should_quit = true;
            return;
        }
        match &mut self.state {
            GameState::Explore(_) => self.handle_explore_key(key),
            GameState::Combat(_) => self.handle_combat_key(key),
            GameState::Inventory(_) => self.handle_inventory_key(key),
            GameState::Shop(_) => self.handle_shop_key(key),
            GameState::QuestLog(_) => self.handle_quest_log_key(key),
            GameState::Event(ev) => {
                if key == KeyCode::Enter {
                    let return_pos = ev.return_pos;
                    if let Some(id) = ev.npc {
                        self.npc_flags.insert(id);
                    }
                    let mut explore = ExploreState::for_chapter(self.current_chapter);
                    explore.player_pos = return_pos;
                    self.state = GameState::Explore(explore);
                }
            }
            GameState::GameOver { .. } => {
                if key == KeyCode::Enter {
                    self.should_quit = true;
                }
            }
        }
    }

    fn handle_explore_key(&mut self, key: KeyCode) {
        if key == KeyCode::Char('i') {
            let GameState::Explore(explore) = &self.state else {
                return;
            };
            let return_pos = explore.player_pos;
            self.state = GameState::Inventory(InventoryUiState::new(return_pos));
            return;
        }
        if key == KeyCode::Char('l') {
            let GameState::Explore(explore) = &self.state else {
                return;
            };
            let return_pos = explore.player_pos;
            self.state = GameState::QuestLog(crate::game::quest_ui::QuestLogUiState::new(return_pos));
            return;
        }
        if key == KeyCode::Char('e') {
            let GameState::Explore(explore) = &self.state else {
                return;
            };
            let return_pos = explore.player_pos;
            let npc_id = explore.map.npc_at(return_pos);
            let is_town = explore.map.tile_at(return_pos) == Tile::Town;
            // The borrow of self.state (via `explore`) ends here, freeing
            // self up for the mutable interact_with_npc call below.
            if let Some(id) = npc_id {
                self.state = GameState::Event(self.interact_with_npc(id, return_pos));
            } else if is_town {
                self.state = GameState::Shop(ShopUiState::new(return_pos));
            }
            return;
        }
        let GameState::Explore(explore) = &mut self.state else {
            return;
        };
        let delta = match key {
            KeyCode::Up | KeyCode::Char('w') => Some((0, -1)),
            KeyCode::Down | KeyCode::Char('s') => Some((0, 1)),
            KeyCode::Left | KeyCode::Char('a') => Some((-1, 0)),
            KeyCode::Right | KeyCode::Char('d') => Some((1, 0)),
            _ => None,
        };
        let Some((dx, dy)) = delta else { return };
        let next = Position {
            x: explore.player_pos.x + dx,
            y: explore.player_pos.y + dy,
        };
        if !explore.map.is_walkable(next) {
            return;
        }
        explore.player_pos = next;
        let tile = explore.map.tile_at(next);
        if tile == Tile::TallGrass {
            explore.steps_in_grass += 1;
            // ~1 in 4 steps through grass triggers *some* field event (not always a fight).
            if self.rng.gen_ratio(1, 4) {
                match roll_field_event(&mut self.rng) {
                    FieldEvent::Combat(enemies) => {
                        let mut combat = CombatState::new(&self.party, enemies);
                        combat.return_pos = Some(next);
                        self.state = GameState::Combat(combat);
                    }
                    FieldEvent::Blessing(effect) => {
                        let lines = vec![
                            "A warm light washes over your party...".to_string(),
                            format!(
                                "{} takes hold! (+{} {} for the next {} encounters)",
                                effect.name, effect.delta, effect.target, effect.encounters_remaining
                            ),
                        ];
                        self.party.add_effect(effect);
                        self.state = GameState::Event(EventState {
                            title: "A Blessing!".to_string(),
                            lines,
                            return_pos: next,
                            npc: None,
                        });
                    }
                    FieldEvent::Curse(effect) => {
                        let lines = vec![
                            "A cold shiver runs down your spines...".to_string(),
                            format!(
                                "{} takes hold! ({} {} for the next {} encounters)",
                                effect.name, effect.delta, effect.target, effect.encounters_remaining
                            ),
                        ];
                        self.party.add_effect(effect);
                        self.state = GameState::Event(EventState {
                            title: "A Curse...".to_string(),
                            lines,
                            return_pos: next,
                            npc: None,
                        });
                    }
                    FieldEvent::Treasure { gold, item, weapon } => {
                        let mut lines = vec![format!("You found a hidden cache! +{gold} gold.")];
                        self.party.gold += gold;
                        if let Some(item) = item {
                            lines.push(format!("It also held a {}.", item.name));
                            self.inventory.add(item, 1);
                        }
                        if let Some(weapon) = weapon {
                            lines.push(format!(
                                "Buried beneath it: a {} rarity {}!",
                                weapon.rarity, weapon.name
                            ));
                            self.inventory.add_weapon(weapon);
                        }
                        self.state = GameState::Event(EventState {
                            title: "Treasure!".to_string(),
                            lines,
                            return_pos: next,
                            npc: None,
                        });
                    }
                }
            }
        } else if tile == Tile::BossLair && !self.bosses_defeated.contains(&self.current_chapter) {
            let def = chapter_def(self.current_chapter);
            let boss = (def.boss)(def.boss_display_name);
            let mut combat = CombatState::new(&self.party, vec![boss]);
            combat.return_pos = Some(next);
            self.state = GameState::Combat(combat);
        }
    }

    /// Builds the dialogue `EventState` for talking to `id`, picking lines
    /// by a flat match on quest state (intro / reminder / turn-in / after —
    /// see `NpcDef`'s doc comment), and grants the quest's reward plus
    /// marks it completed if this visit is the turn-in.
    fn interact_with_npc(&mut self, id: NpcId, return_pos: Position) -> EventState {
        let def = npc_def(id);
        let first_visit = !self.npc_flags.contains(&id);
        if first_visit {
            if let Some(qid) = def.quest {
                self.quest_log.accept(qid);
            }
        }

        let to_strings = |lines: &[&str]| lines.iter().map(|s| s.to_string()).collect::<Vec<_>>();

        let lines = if let Some(qid) = def.quest {
            if self.quest_log.completed.contains(&qid) {
                to_strings(&def.after)
            } else {
                let quest = crate::game::quest::quest_def(qid);
                if crate::game::quest::objective_satisfied(
                    &quest.objective,
                    &self.inventory,
                    &self.bosses_defeated,
                ) {
                    self.consume_objective_item(&quest.objective);
                    let mut lines = to_strings(&def.turn_in);
                    let reward_msgs = self.grant_rewards(&quest.rewards);
                    if !reward_msgs.is_empty() {
                        lines.push(format!("You received: {}", reward_msgs.join(", ")));
                    }
                    self.quest_log.complete(qid);
                    lines
                } else if first_visit {
                    to_strings(&def.intro)
                } else {
                    to_strings(&def.reminder)
                }
            }
        } else if first_visit {
            to_strings(&def.intro)
        } else {
            to_strings(&def.reminder)
        };

        EventState {
            title: def.name.to_string(),
            lines,
            return_pos,
            npc: Some(id),
        }
    }

    /// Removes the item a `DeliverItem` objective asked for, as a turn-in cost.
    fn consume_objective_item(&mut self, objective: &crate::game::quest::QuestObjective) {
        // Only DeliverItem objectives have a cost to consume on turn-in;
        // DefeatBoss is a fact about the world, not something to spend.
        let crate::game::quest::QuestObjective::DeliverItem { item_name, .. } = objective else {
            return;
        };
        if let Some(idx) = self
            .inventory
            .items
            .iter()
            .position(|(item, _)| item.name == *item_name)
        {
            self.inventory.use_at(idx);
        }
    }

    /// Applies every component of a quest's reward, returning a short
    /// description of each for the dialogue line that reports it.
    fn grant_rewards(&mut self, rewards: &[crate::game::quest::QuestReward]) -> Vec<String> {
        use crate::game::quest::QuestReward;
        rewards
            .iter()
            .map(|reward| match reward {
                QuestReward::Gold(amount) => {
                    self.party.gold += amount;
                    format!("{amount} gold")
                }
                QuestReward::Item(factory) => {
                    let item = factory();
                    let name = item.name.clone();
                    self.inventory.add(item, 1);
                    name
                }
                QuestReward::Weapon(factory) => {
                    let weapon = factory();
                    let msg = format!("{} ({})", weapon.name, weapon.rarity);
                    self.inventory.add_weapon(weapon);
                    msg
                }
                QuestReward::Armor(factory) => {
                    let armor = factory();
                    let msg = format!("{} ({})", armor.name, armor.rarity);
                    self.inventory.add_armor(armor);
                    msg
                }
                QuestReward::Ring(factory) => {
                    let ring = factory();
                    let msg = format!("{} ({})", ring.name, ring.rarity);
                    self.inventory.add_ring(ring);
                    msg
                }
            })
            .collect()
    }

    fn handle_quest_log_key(&mut self, key: KeyCode) {
        let GameState::QuestLog(ui_ref) = &self.state else {
            return;
        };
        let cursor = ui_ref.cursor;
        match key {
            KeyCode::Esc => {
                let GameState::QuestLog(ui) = &self.state else {
                    return;
                };
                let return_pos = ui.return_pos;
                let mut explore = ExploreState::for_chapter(self.current_chapter);
                explore.player_pos = return_pos;
                self.state = GameState::Explore(explore);
            }
            KeyCode::Up | KeyCode::Char('w') => {
                let len = self.quest_log.active.len().max(1);
                let GameState::QuestLog(ui) = &mut self.state else {
                    return;
                };
                ui.cursor = (cursor + len - 1) % len;
            }
            KeyCode::Down | KeyCode::Char('s') => {
                let len = self.quest_log.active.len().max(1);
                let GameState::QuestLog(ui) = &mut self.state else {
                    return;
                };
                ui.cursor = (cursor + 1) % len;
            }
            _ => {}
        }
    }

    fn handle_combat_key(&mut self, key: KeyCode) {
        let GameState::Combat(combat) = &mut self.state else {
            return;
        };

        match combat.phase {
            CombatPhase::SelectAction { actor } => {
                // Only players choose via keyboard; enemy turns resolve automatically (see tick()).
                let ActorRef::Player(pi) = actor else { return };
                let menu_len = 4; // Attack, Ability, Item, Flee
                match key {
                    KeyCode::Up | KeyCode::Char('w') => {
                        combat.menu_cursor = (combat.menu_cursor + menu_len - 1) % menu_len;
                    }
                    KeyCode::Down | KeyCode::Char('s') => {
                        combat.menu_cursor = (combat.menu_cursor + 1) % menu_len;
                    }
                    KeyCode::Enter => match combat.menu_cursor {
                        0 => {
                            let target_idx =
                                combat.alive_enemy_indices().first().copied().unwrap_or(0);
                            combat.phase = CombatPhase::SelectTarget {
                                actor,
                                action: CombatAction::Attack,
                                target_idx,
                            };
                        }
                        1 => {
                            if !self.party.members[pi].abilities.is_empty() {
                                combat.phase = CombatPhase::SelectAbility { actor, cursor: 0 };
                            }
                        }
                        2 => {
                            if !self.inventory.items.is_empty() {
                                combat.phase = CombatPhase::SelectItem { actor, cursor: 0 };
                            }
                        }
                        _ => {
                            combat.phase = CombatPhase::SelectTarget {
                                actor,
                                action: CombatAction::Flee,
                                target_idx: 0,
                            };
                            self.resolve_pending_target();
                        }
                    },
                    _ => {}
                }
            }
            CombatPhase::SelectAbility { actor, cursor } => {
                let ActorRef::Player(pi) = actor else { return };
                let ability_count = self.party.members[pi].abilities.len().max(1);
                match key {
                    KeyCode::Up | KeyCode::Char('w') => {
                        let new_cursor = (cursor + ability_count - 1) % ability_count;
                        combat.phase = CombatPhase::SelectAbility {
                            actor,
                            cursor: new_cursor,
                        };
                    }
                    KeyCode::Down | KeyCode::Char('s') => {
                        let new_cursor = (cursor + 1) % ability_count;
                        combat.phase = CombatPhase::SelectAbility {
                            actor,
                            cursor: new_cursor,
                        };
                    }
                    KeyCode::Enter => {
                        let is_heal = self.party.members[pi].ability_is_heal(cursor);
                        let target_idx = if is_heal {
                            pi
                        } else {
                            combat.alive_enemy_indices().first().copied().unwrap_or(0)
                        };
                        combat.phase = CombatPhase::SelectTarget {
                            actor,
                            action: CombatAction::Ability(cursor),
                            target_idx,
                        };
                    }
                    KeyCode::Esc => {
                        combat.phase = CombatPhase::SelectAction { actor };
                    }
                    _ => {}
                }
            }
            CombatPhase::SelectItem { actor, cursor } => {
                let ActorRef::Player(pi) = actor else { return };
                let item_count = self.inventory.items.len().max(1);
                match key {
                    KeyCode::Up | KeyCode::Char('w') => {
                        let new_cursor = (cursor + item_count - 1) % item_count;
                        combat.phase = CombatPhase::SelectItem {
                            actor,
                            cursor: new_cursor,
                        };
                    }
                    KeyCode::Down | KeyCode::Char('s') => {
                        let new_cursor = (cursor + 1) % item_count;
                        combat.phase = CombatPhase::SelectItem {
                            actor,
                            cursor: new_cursor,
                        };
                    }
                    KeyCode::Enter => {
                        if cursor < self.inventory.items.len() {
                            combat.phase = CombatPhase::SelectTarget {
                                actor,
                                action: CombatAction::Item(cursor),
                                target_idx: pi,
                            };
                        }
                    }
                    KeyCode::Esc => {
                        combat.phase = CombatPhase::SelectAction { actor };
                    }
                    _ => {}
                }
            }
            CombatPhase::SelectTarget {
                actor,
                action,
                target_idx,
            } => {
                let ActorRef::Player(pi) = actor else { return };
                let is_heal = match action {
                    CombatAction::Ability(idx) => self.party.members[pi].ability_is_heal(idx),
                    CombatAction::Item(_) => true, // potions/ethers are always ally-targeted
                    CombatAction::Attack | CombatAction::Flee => false,
                };
                match key {
                    KeyCode::Left | KeyCode::Char('a') | KeyCode::Up | KeyCode::Char('w') => {
                        let new_idx = cycle_target(&self.party, combat, is_heal, target_idx, -1);
                        combat.phase = CombatPhase::SelectTarget {
                            actor,
                            action,
                            target_idx: new_idx,
                        };
                    }
                    KeyCode::Right | KeyCode::Char('d') | KeyCode::Down | KeyCode::Char('s') => {
                        let new_idx = cycle_target(&self.party, combat, is_heal, target_idx, 1);
                        combat.phase = CombatPhase::SelectTarget {
                            actor,
                            action,
                            target_idx: new_idx,
                        };
                    }
                    KeyCode::Enter => self.resolve_pending_target(),
                    KeyCode::Esc => {
                        combat.phase = match action {
                            CombatAction::Ability(idx) => {
                                CombatPhase::SelectAbility { actor, cursor: idx }
                            }
                            CombatAction::Item(idx) => {
                                CombatPhase::SelectItem { actor, cursor: idx }
                            }
                            _ => CombatPhase::SelectAction { actor },
                        };
                    }
                    _ => {}
                }
            }
            _ => {
                // Victory / Defeat / Fled: any key returns to the appropriate next state.
                if matches!(key, KeyCode::Enter) {
                    self.conclude_combat();
                }
            }
        }
    }

    fn resolve_pending_target(&mut self) {
        let GameState::Combat(combat) = &mut self.state else {
            return;
        };
        let CombatPhase::SelectTarget {
            actor,
            action,
            target_idx,
        } = combat.phase
        else {
            return;
        };
        let ActorRef::Player(pi) = actor else { return };

        if let CombatAction::Item(idx) = action {
            // Items are drawn from Inventory (owned by World), applied, then we advance the turn.
            if let Some(kind) = self.inventory.use_at(idx) {
                let user_name = self.party.members[pi].name.clone();
                combat.apply_item_and_advance(
                    kind,
                    &user_name,
                    target_idx,
                    &mut self.party,
                    &mut self.rng,
                );
            } else {
                combat.phase = CombatPhase::SelectAction { actor };
            }
            return;
        }

        combat.resolve_current_turn(&mut self.party, &mut self.rng);
    }

    fn handle_inventory_key(&mut self, key: KeyCode) {
        // Pull out Copy state first so this immutable borrow of self.state ends
        // immediately, freeing self.inventory/self.party up for the branches below.
        let GameState::Inventory(inv_ref) = &self.state else {
            return;
        };
        let mode = inv_ref.mode;
        let tab = inv_ref.tab;
        let cursor = inv_ref.cursor;

        match mode {
            InventoryMode::Browsing => match key {
                KeyCode::Esc => {
                    let GameState::Inventory(inv) = &self.state else {
                        return;
                    };
                    let return_pos = inv.return_pos;
                    let mut explore = ExploreState::for_chapter(self.current_chapter);
                    explore.player_pos = return_pos;
                    self.state = GameState::Explore(explore);
                }
                KeyCode::Tab | KeyCode::Right | KeyCode::Char('d') => {
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.tab = inv.tab.next();
                    inv.cursor = 0;
                }
                KeyCode::Left | KeyCode::Char('a') => {
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.tab = inv.tab.prev();
                    inv.cursor = 0;
                }
                KeyCode::Char('p') => {
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.mode = InventoryMode::PartyGear {
                        member_cursor: 0,
                        slot_cursor: 0,
                    };
                }
                KeyCode::Up | KeyCode::Char('w') => {
                    let len = self.inventory_tab_len(tab);
                    if len > 0 {
                        let GameState::Inventory(inv) = &mut self.state else {
                            return;
                        };
                        inv.cursor = (cursor + len - 1) % len;
                    }
                }
                KeyCode::Down | KeyCode::Char('s') => {
                    let len = self.inventory_tab_len(tab);
                    if len > 0 {
                        let GameState::Inventory(inv) = &mut self.state else {
                            return;
                        };
                        inv.cursor = (cursor + 1) % len;
                    }
                }
                KeyCode::Enter => {
                    let len = self.inventory_tab_len(tab);
                    if len > 0 {
                        let GameState::Inventory(inv) = &mut self.state else {
                            return;
                        };
                        inv.mode = InventoryMode::SelectMember {
                            tab,
                            idx: cursor,
                            member_cursor: 0,
                        };
                    }
                }
                _ => {}
            },
            InventoryMode::SelectMember {
                tab,
                idx,
                member_cursor,
            } => match key {
                KeyCode::Esc => {
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.mode = InventoryMode::Browsing;
                }
                KeyCode::Up | KeyCode::Char('w') => {
                    let len = self.party.members.len().max(1);
                    let new_cursor = (member_cursor + len - 1) % len;
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.mode = InventoryMode::SelectMember {
                        tab,
                        idx,
                        member_cursor: new_cursor,
                    };
                }
                KeyCode::Down | KeyCode::Char('s') => {
                    let len = self.party.members.len().max(1);
                    let new_cursor = (member_cursor + 1) % len;
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.mode = InventoryMode::SelectMember {
                        tab,
                        idx,
                        member_cursor: new_cursor,
                    };
                }
                KeyCode::Enter => {
                    if tab == InventoryTab::Rings {
                        let GameState::Inventory(inv) = &mut self.state else {
                            return;
                        };
                        inv.mode = InventoryMode::SelectRingSlot {
                            idx,
                            member_idx: member_cursor,
                            slot_cursor: 0,
                        };
                    } else {
                        self.apply_inventory_selection(tab, idx, member_cursor);
                    }
                }
                _ => {}
            },
            InventoryMode::SelectRingSlot {
                idx,
                member_idx,
                slot_cursor,
            } => match key {
                KeyCode::Esc => {
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.mode = InventoryMode::Browsing;
                }
                KeyCode::Up | KeyCode::Char('w') | KeyCode::Down | KeyCode::Char('s') => {
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.mode = InventoryMode::SelectRingSlot {
                        idx,
                        member_idx,
                        slot_cursor: 1 - slot_cursor,
                    };
                }
                KeyCode::Enter => {
                    let slot = if slot_cursor == 0 {
                        RingSlot::First
                    } else {
                        RingSlot::Second
                    };
                    self.apply_inventory_ring_selection(idx, member_idx, slot);
                }
                _ => {}
            },
            InventoryMode::PartyGear {
                member_cursor,
                slot_cursor,
            } => match key {
                KeyCode::Esc => {
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.mode = InventoryMode::Browsing;
                }
                KeyCode::Up | KeyCode::Char('w') => {
                    let len = self.party.members.len().max(1);
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.mode = InventoryMode::PartyGear {
                        member_cursor: (member_cursor + len - 1) % len,
                        slot_cursor,
                    };
                }
                KeyCode::Down | KeyCode::Char('s') => {
                    let len = self.party.members.len().max(1);
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.mode = InventoryMode::PartyGear {
                        member_cursor: (member_cursor + 1) % len,
                        slot_cursor,
                    };
                }
                KeyCode::Left | KeyCode::Char('a') => {
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.mode = InventoryMode::PartyGear {
                        member_cursor,
                        slot_cursor: (slot_cursor + EQUIP_SLOTS.len() - 1) % EQUIP_SLOTS.len(),
                    };
                }
                KeyCode::Right | KeyCode::Char('d') | KeyCode::Tab => {
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.mode = InventoryMode::PartyGear {
                        member_cursor,
                        slot_cursor: (slot_cursor + 1) % EQUIP_SLOTS.len(),
                    };
                }
                KeyCode::Enter => {
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.mode = InventoryMode::PartyGearAction {
                        member_idx: member_cursor,
                        slot: EQUIP_SLOTS[slot_cursor],
                        action_cursor: 0,
                    };
                }
                _ => {}
            },
            InventoryMode::PartyGearAction {
                member_idx,
                slot,
                action_cursor,
            } => match key {
                KeyCode::Esc => {
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.mode = InventoryMode::PartyGear {
                        member_cursor: member_idx,
                        slot_cursor: equip_slot_index(slot),
                    };
                }
                KeyCode::Up | KeyCode::Char('w') | KeyCode::Down | KeyCode::Char('s') => {
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.mode = InventoryMode::PartyGearAction {
                        member_idx,
                        slot,
                        action_cursor: 1 - action_cursor,
                    };
                }
                KeyCode::Enter => {
                    if action_cursor == 0 {
                        let message = self.unequip_to_bag(member_idx, slot);
                        let GameState::Inventory(inv) = &mut self.state else {
                            return;
                        };
                        if let Some(msg) = message {
                            inv.message = Some(msg);
                        }
                        inv.mode = InventoryMode::PartyGear {
                            member_cursor: member_idx,
                            slot_cursor: equip_slot_index(slot),
                        };
                    } else {
                        let len = self.party.members.len().max(1);
                        let to_cursor = cycle_member_skip(len, member_idx, member_idx, 1);
                        let GameState::Inventory(inv) = &mut self.state else {
                            return;
                        };
                        inv.mode = InventoryMode::PartyGearTarget {
                            from_member: member_idx,
                            slot,
                            to_cursor,
                        };
                    }
                }
                _ => {}
            },
            InventoryMode::PartyGearTarget {
                from_member,
                slot,
                to_cursor,
            } => match key {
                KeyCode::Esc => {
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.mode = InventoryMode::PartyGear {
                        member_cursor: from_member,
                        slot_cursor: equip_slot_index(slot),
                    };
                }
                KeyCode::Up | KeyCode::Char('w') => {
                    let len = self.party.members.len().max(1);
                    let new_cursor = cycle_member_skip(len, from_member, to_cursor, -1);
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.mode = InventoryMode::PartyGearTarget {
                        from_member,
                        slot,
                        to_cursor: new_cursor,
                    };
                }
                KeyCode::Down | KeyCode::Char('s') => {
                    let len = self.party.members.len().max(1);
                    let new_cursor = cycle_member_skip(len, from_member, to_cursor, 1);
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.mode = InventoryMode::PartyGearTarget {
                        from_member,
                        slot,
                        to_cursor: new_cursor,
                    };
                }
                KeyCode::Enter => {
                    let message = self.move_gear_between_members(from_member, to_cursor, slot);
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    if let Some(msg) = message {
                        inv.message = Some(msg);
                    }
                    inv.mode = InventoryMode::PartyGear {
                        member_cursor: from_member,
                        slot_cursor: equip_slot_index(slot),
                    };
                }
                _ => {}
            },
        }
    }

    fn inventory_tab_len(&self, tab: InventoryTab) -> usize {
        match tab {
            InventoryTab::Items => self.inventory.items.len(),
            InventoryTab::Weapons => self.inventory.weapons.len(),
            InventoryTab::Armor => self.inventory.armors.len(),
            InventoryTab::Rings => self.inventory.rings.len(),
        }
    }

    /// Applies the pending item-use/weapon-equip/armor-equip to `member_idx`,
    /// then returns the inventory screen to browsing mode. Rings are handled
    /// separately by `apply_inventory_ring_selection` since they need a slot
    /// choice first.
    fn apply_inventory_selection(&mut self, tab: InventoryTab, idx: usize, member_idx: usize) {
        let message = match tab {
            InventoryTab::Items => {
                if let Some(kind) = self.inventory.use_at(idx) {
                    self.party.members.get_mut(member_idx).map(|member| {
                        let name = member.name.clone();
                        let effect_msg = crate::game::combat::use_item_kind(kind, member);
                        format!("{name}: {effect_msg}")
                    })
                } else {
                    None
                }
            }
            InventoryTab::Weapons => {
                if idx < self.inventory.weapons.len() {
                    let weapon = self.inventory.weapons.remove(idx);
                    let weapon_name = weapon.name.clone();
                    if let Some(member) = self.party.members.get_mut(member_idx) {
                        let member_name = member.name.clone();
                        if let Some(old_weapon) = member.equip_weapon(weapon) {
                            self.inventory.add_weapon(old_weapon);
                        }
                        Some(format!("Equipped {weapon_name} on {member_name}."))
                    } else {
                        // No such member (shouldn't happen) — don't lose the weapon.
                        self.inventory.add_weapon(weapon);
                        None
                    }
                } else {
                    None
                }
            }
            InventoryTab::Armor => {
                if idx < self.inventory.armors.len() {
                    let armor = self.inventory.armors.remove(idx);
                    let armor_name = armor.name.clone();
                    if let Some(member) = self.party.members.get_mut(member_idx) {
                        let member_name = member.name.clone();
                        if let Some(old_armor) = member.equip_armor(armor) {
                            self.inventory.add_armor(old_armor);
                        }
                        Some(format!("Equipped {armor_name} on {member_name}."))
                    } else {
                        self.inventory.add_armor(armor);
                        None
                    }
                } else {
                    None
                }
            }
            InventoryTab::Rings => None, // routed through apply_inventory_ring_selection instead
        };

        let items_len = self.inventory.items.len();
        let weapons_len = self.inventory.weapons.len();
        let armors_len = self.inventory.armors.len();

        if let GameState::Inventory(inv) = &mut self.state {
            if let Some(msg) = message {
                inv.message = Some(msg);
            }
            inv.cursor = match tab {
                InventoryTab::Items => inv.cursor.min(items_len.saturating_sub(1)),
                InventoryTab::Weapons => inv.cursor.min(weapons_len.saturating_sub(1)),
                InventoryTab::Armor => inv.cursor.min(armors_len.saturating_sub(1)),
                InventoryTab::Rings => inv.cursor,
            };
            inv.mode = InventoryMode::Browsing;
        }
    }

    /// Equips the ring at bag index `idx` into `member_idx`'s `slot`, then
    /// returns the inventory screen to browsing mode.
    fn apply_inventory_ring_selection(&mut self, idx: usize, member_idx: usize, slot: RingSlot) {
        let message = if idx < self.inventory.rings.len() {
            let ring = self.inventory.rings.remove(idx);
            let ring_name = ring.name.clone();
            if let Some(member) = self.party.members.get_mut(member_idx) {
                let member_name = member.name.clone();
                if let Some(old_ring) = member.equip_ring(slot, ring) {
                    self.inventory.add_ring(old_ring);
                }
                Some(format!("Equipped {ring_name} on {member_name}."))
            } else {
                self.inventory.add_ring(ring);
                None
            }
        } else {
            None
        };

        let rings_len = self.inventory.rings.len();
        if let GameState::Inventory(inv) = &mut self.state {
            if let Some(msg) = message {
                inv.message = Some(msg);
            }
            inv.cursor = inv.cursor.min(rings_len.saturating_sub(1));
            inv.mode = InventoryMode::Browsing;
        }
    }

    /// Removes whatever `member_idx` has equipped in `slot` and returns it to
    /// the shared bag. This is the direct fix for gear that previously could
    /// only become "spare" again by being bumped out via a replacement equip.
    fn unequip_to_bag(&mut self, member_idx: usize, slot: EquipSlot) -> Option<String> {
        let member = self.party.members.get_mut(member_idx)?;
        let member_name = member.name.clone();
        let piece = take_slot(member, slot)?;
        let name = piece.name().to_string();
        match piece {
            GearPiece::Weapon(w) => self.inventory.add_weapon(w),
            GearPiece::Armor(a) => self.inventory.add_armor(a),
            GearPiece::Ring(r) => self.inventory.add_ring(r),
        }
        Some(format!("Unequipped {name} from {member_name}."))
    }

    /// Directly swaps whatever is in `slot` between two party members —
    /// independent of the shared bag. If `to_member` already had something
    /// in that slot, `from_member` receives it back; if `from_member` had
    /// nothing, `to_member`'s slot is simply cleared.
    fn move_gear_between_members(
        &mut self,
        from_member: usize,
        to_member: usize,
        slot: EquipSlot,
    ) -> Option<String> {
        if from_member == to_member {
            return None;
        }
        let from_name = self.party.members.get(from_member)?.name.clone();
        let to_name = self.party.members.get(to_member)?.name.clone();

        let taken = take_slot(self.party.members.get_mut(from_member)?, slot);
        let taken_name = taken.as_ref().map(|p| p.name().to_string());
        let displaced = put_slot(self.party.members.get_mut(to_member)?, slot, taken);
        put_slot(self.party.members.get_mut(from_member)?, slot, displaced);

        match taken_name {
            Some(name) => Some(format!("Moved {name} from {from_name} to {to_name}.")),
            None => Some(format!("{from_name} had nothing there to give {to_name}.")),
        }
    }

    fn handle_shop_key(&mut self, key: KeyCode) {
        // Same read-then-branch shape as handle_inventory_key: copy out the
        // Copy fields first so this shared borrow of self.state ends before
        // we touch self.party/self.inventory in the branches below.
        let GameState::Shop(shop_ref) = &self.state else {
            return;
        };
        let mode = shop_ref.mode;
        let tab = shop_ref.tab;
        let cursor = shop_ref.cursor;

        match key {
            KeyCode::Esc => {
                let GameState::Shop(shop) = &self.state else {
                    return;
                };
                let return_pos = shop.return_pos;
                let mut explore = ExploreState::for_chapter(self.current_chapter);
                explore.player_pos = return_pos;
                self.state = GameState::Explore(explore);
            }
            KeyCode::Left | KeyCode::Char('a') | KeyCode::Right | KeyCode::Char('d') => {
                let GameState::Shop(shop) = &mut self.state else {
                    return;
                };
                shop.mode = shop.mode.toggled();
                shop.cursor = 0;
            }
            KeyCode::Tab => {
                let GameState::Shop(shop) = &mut self.state else {
                    return;
                };
                shop.tab = shop.tab.next();
                shop.cursor = 0;
            }
            KeyCode::Up | KeyCode::Char('w') => {
                let len = self.shop_list_len(mode, tab);
                if len > 0 {
                    let GameState::Shop(shop) = &mut self.state else {
                        return;
                    };
                    shop.cursor = (cursor + len - 1) % len;
                }
            }
            KeyCode::Down | KeyCode::Char('s') => {
                let len = self.shop_list_len(mode, tab);
                if len > 0 {
                    let GameState::Shop(shop) = &mut self.state else {
                        return;
                    };
                    shop.cursor = (cursor + 1) % len;
                }
            }
            KeyCode::Enter => match mode {
                ShopMode::Buy => self.apply_shop_buy(tab, cursor),
                ShopMode::Sell => self.apply_shop_sell(tab, cursor),
            },
            _ => {}
        }
    }

    fn shop_list_len(&self, mode: ShopMode, tab: ShopTab) -> usize {
        match mode {
            ShopMode::Buy => match tab {
                ShopTab::Items => shop_item_stock().len(),
                ShopTab::Weapons => shop_weapon_stock().len(),
                ShopTab::Armor => shop_armor_stock().len(),
                ShopTab::Rings => shop_ring_stock().len(),
            },
            ShopMode::Sell => match tab {
                ShopTab::Items => self.inventory.items.len(),
                ShopTab::Weapons => self.inventory.weapons.len(),
                ShopTab::Armor => self.inventory.armors.len(),
                ShopTab::Rings => self.inventory.rings.len(),
            },
        }
    }

    fn apply_shop_buy(&mut self, tab: ShopTab, idx: usize) {
        let message = match tab {
            ShopTab::Items => match shop_item_stock().get(idx) {
                Some(&(factory, price)) if self.party.gold >= price => {
                    self.party.gold -= price;
                    let item = factory();
                    let name = item.name.clone();
                    self.inventory.add(item, 1);
                    Some(format!("Bought a {name} for {price} gold."))
                }
                Some(_) => Some("Not enough gold for that.".to_string()),
                None => None,
            },
            ShopTab::Weapons => match shop_weapon_stock().get(idx) {
                Some(&(factory, price)) if self.party.gold >= price => {
                    self.party.gold -= price;
                    let weapon = factory();
                    let name = weapon.name.clone();
                    self.inventory.add_weapon(weapon);
                    Some(format!("Bought {name} for {price} gold."))
                }
                Some(_) => Some("Not enough gold for that.".to_string()),
                None => None,
            },
            ShopTab::Armor => match shop_armor_stock().get(idx) {
                Some(&(factory, price)) if self.party.gold >= price => {
                    self.party.gold -= price;
                    let armor = factory();
                    let name = armor.name.clone();
                    self.inventory.add_armor(armor);
                    Some(format!("Bought {name} for {price} gold."))
                }
                Some(_) => Some("Not enough gold for that.".to_string()),
                None => None,
            },
            ShopTab::Rings => match shop_ring_stock().get(idx) {
                Some(&(factory, price)) if self.party.gold >= price => {
                    self.party.gold -= price;
                    let ring = factory();
                    let name = ring.name.clone();
                    self.inventory.add_ring(ring);
                    Some(format!("Bought {name} for {price} gold."))
                }
                Some(_) => Some("Not enough gold for that.".to_string()),
                None => None,
            },
        };

        if let GameState::Shop(shop) = &mut self.state {
            if let Some(msg) = message {
                shop.message = Some(msg);
            }
        }
    }

    fn apply_shop_sell(&mut self, tab: ShopTab, idx: usize) {
        let message = match tab {
            ShopTab::Items => {
                if let Some((item, _qty)) = self.inventory.items.get(idx) {
                    let price = item.value / 2;
                    let name = item.name.clone();
                    self.inventory.use_at(idx);
                    self.party.gold += price;
                    Some(format!("Sold a {name} for {price} gold."))
                } else {
                    None
                }
            }
            ShopTab::Weapons => {
                if idx < self.inventory.weapons.len() {
                    let weapon = self.inventory.weapons.remove(idx);
                    let price = weapon.rarity.base_value() / 2;
                    let name = weapon.name.clone();
                    self.party.gold += price;
                    Some(format!("Sold {name} for {price} gold."))
                } else {
                    None
                }
            }
            ShopTab::Armor => {
                if idx < self.inventory.armors.len() {
                    let armor = self.inventory.armors.remove(idx);
                    let price = armor.rarity.base_value() / 2;
                    let name = armor.name.clone();
                    self.party.gold += price;
                    Some(format!("Sold {name} for {price} gold."))
                } else {
                    None
                }
            }
            ShopTab::Rings => {
                if idx < self.inventory.rings.len() {
                    let ring = self.inventory.rings.remove(idx);
                    let price = ring.rarity.base_value() / 2;
                    let name = ring.name.clone();
                    self.party.gold += price;
                    Some(format!("Sold {name} for {price} gold."))
                } else {
                    None
                }
            }
        };

        let items_len = self.inventory.items.len();
        let weapons_len = self.inventory.weapons.len();
        let armors_len = self.inventory.armors.len();
        let rings_len = self.inventory.rings.len();

        if let GameState::Shop(shop) = &mut self.state {
            if let Some(msg) = message {
                shop.message = Some(msg);
            }
            shop.cursor = match tab {
                ShopTab::Items => shop.cursor.min(items_len.saturating_sub(1)),
                ShopTab::Weapons => shop.cursor.min(weapons_len.saturating_sub(1)),
                ShopTab::Armor => shop.cursor.min(armors_len.saturating_sub(1)),
                ShopTab::Rings => shop.cursor.min(rings_len.saturating_sub(1)),
            };
        }
    }

    /// Called once per app loop tick; advances enemy turns automatically since they
    /// don't wait on keyboard input.
    pub fn tick(&mut self) {
        if let GameState::Combat(combat) = &mut self.state {
            if let CombatPhase::SelectAction {
                actor: ActorRef::Enemy(_),
            } = combat.phase
            {
                combat.resolve_current_turn(&mut self.party, &mut self.rng);
            }
        }
    }

    fn conclude_combat(&mut self) {
        let GameState::Combat(combat) = &self.state else {
            return;
        };
        let victory = matches!(combat.phase, CombatPhase::Victory);
        let fled = matches!(combat.phase, CombatPhase::Fled);
        let defeat = matches!(combat.phase, CombatPhase::Defeat);
        let loot = combat.loot.clone();
        let return_pos = combat.return_pos;
        let boss_was_here = combat.enemies.iter().any(|e| e.boss_kind.is_some());
        // The borrow of self.state (via `combat`) ends here, freeing self.party/self.inventory
        // up for mutation below.

        if victory {
            self.party.tick_effects();
            let mut messages = Vec::new();
            let mut chapter_advanced = false;
            if boss_was_here {
                let boss_name = chapter_def(self.current_chapter).boss_display_name;
                self.bosses_defeated.insert(self.current_chapter);
                messages.push(format!("{boss_name} falls, and the way forward opens."));
                match chapter_def(self.current_chapter).next {
                    Some(next) => {
                        self.current_chapter = next;
                        chapter_advanced = true;
                    }
                    None => {
                        self.state = GameState::GameOver { victory: true };
                        return;
                    }
                }
            }
            if let Some(loot) = loot {
                if loot.gold > 0 {
                    self.party.gold += loot.gold;
                    messages.push(format!("You found {} gold.", loot.gold));
                }
                if loot.overkill_bonus > 0 {
                    messages.push(format!(
                        "Overkill! That earned {} bonus gold.",
                        loot.overkill_bonus
                    ));
                }
                for item in loot.items {
                    messages.push(format!("You found a {}!", item.name));
                    self.inventory.add(item, 1);
                }
                for weapon in loot.weapons {
                    messages.push(format!(
                        "The enemy dropped a {} rarity weapon: {}!",
                        weapon.rarity, weapon.name
                    ));
                    self.inventory.add_weapon(weapon);
                }
            }
            if messages.is_empty() {
                messages.push("The enemies dropped nothing.".to_string());
            }
            let mut explore = ExploreState::for_chapter(self.current_chapter);
            // Beating a chapter's boss moves the party onto an entirely new
            // map, so `return_pos` (a position on the *old* map) no longer
            // means anything — only apply it for ordinary fights.
            if !chapter_advanced {
                if let Some(pos) = return_pos {
                    explore.player_pos = pos;
                }
            }
            explore.log = messages;
            self.state = GameState::Explore(explore);
        } else if fled {
            self.party.tick_effects();
            let mut explore = ExploreState::for_chapter(self.current_chapter);
            if let Some(pos) = return_pos {
                explore.player_pos = pos;
            }
            self.state = GameState::Explore(explore);
        } else if defeat {
            self.state = GameState::GameOver { victory: false };
        }
    }
}

/// A concrete piece of gear taken out of one equip slot, so it can be
/// carried over to another. Exists only so `take_slot`/`put_slot` can be
/// written once and dispatched generically over `EquipSlot`, instead of
/// duplicating the move/unequip logic once per gear type.
enum GearPiece {
    Weapon(Weapon),
    Armor(Armor),
    Ring(Ring),
}

impl GearPiece {
    fn name(&self) -> &str {
        match self {
            GearPiece::Weapon(w) => &w.name,
            GearPiece::Armor(a) => &a.name,
            GearPiece::Ring(r) => &r.name,
        }
    }
}

/// Removes and returns whatever `member` has equipped in `slot`, if anything.
fn take_slot(member: &mut Character, slot: EquipSlot) -> Option<GearPiece> {
    match slot {
        EquipSlot::Weapon => member.unequip_weapon().map(GearPiece::Weapon),
        EquipSlot::Armor => member.unequip_armor().map(GearPiece::Armor),
        EquipSlot::Ring(rs) => member.unequip_ring(rs).map(GearPiece::Ring),
    }
}

/// Installs `piece` into `member`'s `slot` (clearing it if `piece` is
/// `None`), returning whatever was previously there.
fn put_slot(member: &mut Character, slot: EquipSlot, piece: Option<GearPiece>) -> Option<GearPiece> {
    match piece {
        Some(GearPiece::Weapon(w)) => member.equip_weapon(w).map(GearPiece::Weapon),
        Some(GearPiece::Armor(a)) => member.equip_armor(a).map(GearPiece::Armor),
        Some(GearPiece::Ring(r)) => {
            let EquipSlot::Ring(rs) = slot else {
                unreachable!("a GearPiece::Ring always pairs with EquipSlot::Ring")
            };
            member.equip_ring(rs, r).map(GearPiece::Ring)
        }
        None => take_slot(member, slot),
    }
}

/// The index of `slot` within `EQUIP_SLOTS`, used to restore the party-gear
/// cursor to the slot an action was just taken on.
fn equip_slot_index(slot: EquipSlot) -> usize {
    EQUIP_SLOTS
        .iter()
        .position(|&s| s == slot)
        .unwrap_or(0)
}

/// Steps `current` by `dir` within `0..len`, wrapping, while skipping over
/// `skip` (used to cycle a "move to another member" cursor without ever
/// landing back on the member the gear is moving from).
fn cycle_member_skip(len: usize, skip: usize, current: usize, dir: i32) -> usize {
    if len <= 1 {
        return current;
    }
    let mut idx = current as i32;
    loop {
        idx = ((idx + dir) % len as i32 + len as i32) % len as i32;
        if idx as usize != skip {
            return idx as usize;
        }
    }
}

fn cycle_target(
    party: &Party,
    combat: &CombatState,
    is_heal: bool,
    current: usize,
    dir: i32,
) -> usize {
    let candidates: Vec<usize> = if is_heal {
        party
            .members
            .iter()
            .enumerate()
            .filter(|(_, c)| c.is_alive())
            .map(|(i, _)| i)
            .collect()
    } else {
        combat.alive_enemy_indices()
    };
    if candidates.is_empty() {
        return current;
    }
    let pos = candidates
        .iter()
        .position(|&i| i == current)
        .unwrap_or(0) as i32;
    let len = candidates.len() as i32;
    let new_pos = ((pos + dir) % len + len) % len;
    candidates[new_pos as usize]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::item::{copper_band, iron_sword, padded_vest, ring_of_vigor};

    #[test]
    fn i_opens_the_inventory_and_esc_returns_to_explore() {
        let mut world = World::new();
        assert!(matches!(world.state, GameState::Explore(_)));
        world.handle_key(KeyCode::Char('i'));
        assert!(matches!(world.state, GameState::Inventory(_)));
        world.handle_key(KeyCode::Esc);
        assert!(matches!(world.state, GameState::Explore(_)));
    }

    #[test]
    fn equipping_a_weapon_swaps_it_with_the_previous_one() {
        let mut world = World::new();
        world.inventory.add_weapon(iron_sword());

        world.handle_key(KeyCode::Char('i')); // open inventory (Items tab)
        world.handle_key(KeyCode::Tab); // switch to Weapons tab
        world.handle_key(KeyCode::Enter); // pick the Iron Sword -> choose member (Bram, cursor 0)

        let previous_weapon = world.party.members[0]
            .equipped_weapon
            .as_ref()
            .map(|w| w.name.clone());

        world.handle_key(KeyCode::Enter); // confirm equip on Bram

        assert_eq!(
            world.party.members[0]
                .equipped_weapon
                .as_ref()
                .map(|w| w.name.as_str()),
            Some("Iron Sword")
        );
        assert!(
            world
                .inventory
                .weapons
                .iter()
                .any(|w| Some(w.name.clone()) == previous_weapon),
            "the displaced weapon should be returned to the inventory"
        );
    }

    #[test]
    fn using_a_potion_from_the_inventory_heals_the_chosen_member() {
        let mut world = World::new();
        world.party.members[0].stats.hp = 1;

        world.handle_key(KeyCode::Char('i')); // open inventory (Items tab, Potion first)
        world.handle_key(KeyCode::Enter); // pick the potion -> choose member (Bram, cursor 0)
        world.handle_key(KeyCode::Enter); // confirm use on Bram

        assert!(
            world.party.members[0].stats.hp > 1,
            "using a potion out of combat should heal the target"
        );
    }

    #[test]
    fn combat_item_submenu_lets_you_pick_ether_over_potion() {
        use crate::game::character::slime;

        let mut world = World::new();
        world.party.members[0].stats.mp = 0;
        let combat = CombatState::new(&world.party, vec![slime("Slime")]);
        world.state = GameState::Combat(combat);
        let GameState::Combat(combat) = &mut world.state else {
            unreachable!()
        };
        combat.phase = CombatPhase::SelectAction {
            actor: ActorRef::Player(0),
        };
        combat.menu_cursor = 2; // Item

        world.handle_key(KeyCode::Enter); // open the item submenu (starts on Potion)
        world.handle_key(KeyCode::Down); // move to Ether
        world.handle_key(KeyCode::Enter); // pick Ether, target self by default
        world.handle_key(KeyCode::Enter); // confirm target

        assert_eq!(
            world.party.members[0].stats.mp, 10,
            "picking Ether from the submenu should restore MP, not HP"
        );
        let potions = world
            .inventory
            .items
            .iter()
            .find(|(i, _)| i.name == "Potion")
            .map(|(_, q)| *q)
            .unwrap_or(0);
        let ethers = world
            .inventory
            .items
            .iter()
            .find(|(i, _)| i.name == "Ether")
            .map(|(_, q)| *q)
            .unwrap_or(0);
        assert_eq!(potions, 3, "the untouched item slot should keep its count");
        assert_eq!(ethers, 1, "the picked item slot should be the one consumed");
    }

    #[test]
    fn e_opens_the_shop_while_in_town_and_esc_returns() {
        let mut world = World::new();
        // World::new() spawns the party inside the walled town square.
        assert!(matches!(world.state, GameState::Explore(_)));
        world.handle_key(KeyCode::Char('e'));
        assert!(matches!(world.state, GameState::Shop(_)));
        world.handle_key(KeyCode::Esc);
        assert!(matches!(world.state, GameState::Explore(_)));
    }

    #[test]
    fn buying_an_item_deducts_gold_and_adds_it_to_the_bag() {
        let mut world = World::new();
        let gold_before = world.party.gold;
        let potions_before = world
            .inventory
            .items
            .iter()
            .find(|(item, _)| item.name == "Potion")
            .map(|(_, qty)| *qty)
            .unwrap_or(0);

        world.handle_key(KeyCode::Char('e')); // Buy tab, Items tab, cursor on Potion (15 gold)
        world.handle_key(KeyCode::Enter);

        assert_eq!(world.party.gold, gold_before - 15);
        let potions_after = world
            .inventory
            .items
            .iter()
            .find(|(item, _)| item.name == "Potion")
            .map(|(_, qty)| *qty)
            .unwrap_or(0);
        assert_eq!(potions_after, potions_before + 1);
    }

    #[test]
    fn buying_without_enough_gold_changes_nothing() {
        let mut world = World::new();
        world.party.gold = 5; // Iron Sword costs 20

        world.handle_key(KeyCode::Char('e'));
        world.handle_key(KeyCode::Tab); // Weapons tab, cursor on Iron Sword
        world.handle_key(KeyCode::Enter);

        assert_eq!(world.party.gold, 5, "gold should be untouched on a failed buy");
        assert!(world.inventory.weapons.is_empty());
    }

    #[test]
    fn selling_a_spare_weapon_grants_gold_and_removes_it() {
        let mut world = World::new();
        world.inventory.add_weapon(iron_sword());
        let gold_before = world.party.gold;
        let expected_price = iron_sword().rarity.base_value() / 2;

        world.handle_key(KeyCode::Char('e')); // Buy/Items
        world.handle_key(KeyCode::Left); // -> Sell/Items
        world.handle_key(KeyCode::Tab); // -> Sell/Weapons, cursor on the Iron Sword
        world.handle_key(KeyCode::Enter); // sell it

        assert_eq!(world.party.gold, gold_before + expected_price);
        assert!(
            world.inventory.weapons.is_empty(),
            "the sold weapon should leave the bag"
        );
    }

    #[test]
    fn walking_into_the_boss_lair_starts_the_boss_fight() {
        let mut world = World::new();
        if let GameState::Explore(explore) = &mut world.state {
            explore.player_pos = Position { x: 26, y: 7 }; // one tile above the lair
        }
        world.handle_key(KeyCode::Down); // step onto the lair at (26, 8)

        let GameState::Combat(combat) = &world.state else {
            panic!("expected the boss fight to start");
        };
        assert!(combat.enemies.iter().any(|e| e.name == "The Barrow Knight"));
    }

    #[test]
    fn defeating_a_chapter_boss_advances_current_chapter_and_relocates_the_player() {
        let mut world = World::new();
        if let GameState::Explore(explore) = &mut world.state {
            explore.player_pos = Position { x: 26, y: 7 };
        }
        world.handle_key(KeyCode::Down); // enter the boss fight

        let GameState::Combat(combat) = &mut world.state else {
            panic!("expected the boss fight to start");
        };
        combat.phase = CombatPhase::Victory;
        world.conclude_combat();

        assert!(world.bosses_defeated.contains(&ChapterId::One));
        assert_eq!(world.current_chapter, ChapterId::Two);
        let GameState::Explore(explore) = &world.state else {
            panic!("expected to land back in Explore on the new chapter's map");
        };
        assert_eq!(explore.player_pos, chapter_def(ChapterId::Two).spawn);
    }

    #[test]
    fn defeating_the_final_chapter_boss_ends_the_game_in_victory() {
        let mut world = World::new();
        world.current_chapter = ChapterId::Three;
        world.state = GameState::Explore(ExploreState::for_chapter(ChapterId::Three));
        // Chapter three's boss lair sits at (26, 8); approach from directly above.
        if let GameState::Explore(explore) = &mut world.state {
            explore.player_pos = Position { x: 26, y: 7 };
        }
        world.handle_key(KeyCode::Down); // enter the boss fight

        let GameState::Combat(combat) = &mut world.state else {
            panic!("expected the boss fight to start");
        };
        combat.phase = CombatPhase::Victory;
        world.conclude_combat();

        assert!(world.bosses_defeated.contains(&ChapterId::Three));
        assert!(matches!(world.state, GameState::GameOver { victory: true }));
    }

    #[test]
    fn equipping_armor_from_the_bag_works_from_the_armor_tab() {
        let mut world = World::new();
        world.inventory.add_armor(padded_vest());

        world.handle_key(KeyCode::Char('i')); // Items tab
        world.handle_key(KeyCode::Tab); // Weapons
        world.handle_key(KeyCode::Tab); // Armor
        world.handle_key(KeyCode::Enter); // pick the Padded Vest -> choose member (Bram)
        world.handle_key(KeyCode::Enter); // confirm equip on Bram

        assert_eq!(
            world.party.members[0]
                .equipped_armor
                .as_ref()
                .map(|a| a.name.as_str()),
            Some("Padded Vest")
        );
        assert!(world.inventory.armors.is_empty());
    }

    #[test]
    fn equipping_a_ring_asks_which_slot_then_equips_it_there() {
        let mut world = World::new();
        world.inventory.add_ring(copper_band());

        world.handle_key(KeyCode::Char('i')); // Items
        world.handle_key(KeyCode::Tab); // Weapons
        world.handle_key(KeyCode::Tab); // Armor
        world.handle_key(KeyCode::Tab); // Rings
        world.handle_key(KeyCode::Enter); // pick the Copper Band -> choose member (Bram)
        world.handle_key(KeyCode::Enter); // confirm member -> now choose ring slot (First)
        world.handle_key(KeyCode::Enter); // confirm First slot

        assert_eq!(
            world.party.members[0].equipped_rings[0]
                .as_ref()
                .map(|r| r.name.as_str()),
            Some("Copper Band")
        );
        assert!(world.party.members[0].equipped_rings[1].is_none());
        assert!(world.inventory.rings.is_empty());
    }

    #[test]
    fn equipping_a_ring_into_the_second_slot() {
        let mut world = World::new();
        world.inventory.add_ring(ring_of_vigor());

        world.handle_key(KeyCode::Char('i'));
        world.handle_key(KeyCode::Tab);
        world.handle_key(KeyCode::Tab);
        world.handle_key(KeyCode::Tab); // Rings tab
        world.handle_key(KeyCode::Enter); // pick ring -> choose member (Bram)
        world.handle_key(KeyCode::Enter); // confirm member -> choose ring slot
        world.handle_key(KeyCode::Down); // move to Second slot
        world.handle_key(KeyCode::Enter); // confirm Second slot

        assert!(world.party.members[0].equipped_rings[0].is_none());
        assert_eq!(
            world.party.members[0].equipped_rings[1]
                .as_ref()
                .map(|r| r.name.as_str()),
            Some("Ring of Vigor")
        );
    }

    #[test]
    fn party_gear_unequip_empties_the_slot_and_grows_the_bag() {
        let mut world = World::new();
        assert!(world.inventory.weapons.is_empty());

        world.handle_key(KeyCode::Char('i')); // open inventory (Browsing)
        world.handle_key(KeyCode::Char('p')); // PartyGear, member 0 (Bram), slot 0 (Weapon)
        world.handle_key(KeyCode::Enter); // -> PartyGearAction, action_cursor 0 ("Unequip to bag")
        world.handle_key(KeyCode::Enter); // confirm unequip

        assert!(
            world.party.members[0].equipped_weapon.is_none(),
            "Bram's weapon slot should now be empty"
        );
        assert!(
            world
                .inventory
                .weapons
                .iter()
                .any(|w| w.name == "Worn Shortsword"),
            "the unequipped weapon should land back in the shared bag"
        );
    }

    #[test]
    fn party_gear_move_swaps_gear_between_two_members_without_touching_the_bag() {
        let mut world = World::new();

        world.handle_key(KeyCode::Char('i')); // open inventory (Browsing)
        world.handle_key(KeyCode::Char('p')); // PartyGear, member 0 (Bram), slot 0 (Weapon)
        world.handle_key(KeyCode::Enter); // -> PartyGearAction, action_cursor 0
        world.handle_key(KeyCode::Down); // -> action_cursor 1 ("Move to another member")
        world.handle_key(KeyCode::Enter); // -> PartyGearTarget, defaults to member 1 (Sella)
        world.handle_key(KeyCode::Enter); // confirm the swap

        assert_eq!(
            world.party.members[0]
                .equipped_weapon
                .as_ref()
                .map(|w| w.name.as_str()),
            Some("Apprentice's Wand"),
            "Bram should now hold what Sella was carrying"
        );
        assert_eq!(
            world.party.members[1]
                .equipped_weapon
                .as_ref()
                .map(|w| w.name.as_str()),
            Some("Worn Shortsword"),
            "Sella should now hold what Bram was carrying"
        );
        assert!(
            world.inventory.weapons.is_empty(),
            "a direct member-to-member move should never touch the shared bag"
        );
    }

    #[test]
    fn unequipping_then_selling_now_works() {
        // Regression test: before free unequip existed, the shop told players
        // to "unequip in the inventory screen first" even though no such
        // action was possible. This drives that exact end-to-end flow.
        let mut world = World::new();
        let gold_before = world.party.gold;
        let expected_price = iron_sword().rarity.base_value() / 2;
        // Swap in an Iron Sword first so we're not left fighting bare-handed,
        // matching how a player would actually reach this state.
        world.party.members[0].equip_weapon(iron_sword());

        world.handle_key(KeyCode::Char('i')); // open inventory
        world.handle_key(KeyCode::Char('p')); // PartyGear, member 0, slot Weapon
        world.handle_key(KeyCode::Enter); // -> PartyGearAction
        world.handle_key(KeyCode::Enter); // Unequip to bag

        assert!(world
            .inventory
            .weapons
            .iter()
            .any(|w| w.name == "Iron Sword"));

        world.handle_key(KeyCode::Esc); // back to PartyGear
        world.handle_key(KeyCode::Esc); // back to Explore

        world.handle_key(KeyCode::Char('e')); // open the shop (standing in town)
        world.handle_key(KeyCode::Left); // Buy -> Sell
        world.handle_key(KeyCode::Tab); // Items -> Weapons tab
                                         // Cursor starts on whichever spare weapon sorts first; find the Iron Sword
                                         // by pressing Down until it's selected, bounded by the bag's length.
        let idx = world
            .inventory
            .weapons
            .iter()
            .position(|w| w.name == "Iron Sword")
            .expect("iron sword should be in the bag");
        for _ in 0..idx {
            world.handle_key(KeyCode::Down);
        }
        world.handle_key(KeyCode::Enter); // sell it

        assert_eq!(world.party.gold, gold_before + expected_price);
        assert!(
            !world.inventory.weapons.iter().any(|w| w.name == "Iron Sword"),
            "the sold weapon should leave the bag"
        );
    }

    #[test]
    fn e_on_an_npc_opens_dialogue_with_the_npc_set() {
        let mut world = World::new();
        if let GameState::Explore(explore) = &mut world.state {
            explore.player_pos = Position { x: 12, y: 5 }; // the Old Herbalist's spot
        }
        world.handle_key(KeyCode::Char('e'));

        let GameState::Event(ev) = &world.state else {
            panic!("expected talking to the NPC to open a dialogue event");
        };
        assert_eq!(ev.npc, Some(crate::game::npc::NpcId::OldHerbalist));
        assert_eq!(ev.title, "Old Herbalist");
    }

    #[test]
    fn dismissing_npc_dialogue_marks_it_talked_to_and_switches_to_reminder_lines() {
        let mut world = World::new();
        if let GameState::Explore(explore) = &mut world.state {
            explore.player_pos = Position { x: 12, y: 5 };
        }
        world.handle_key(KeyCode::Char('e'));
        assert!(!world
            .npc_flags
            .contains(&crate::game::npc::NpcId::OldHerbalist));
        world.handle_key(KeyCode::Enter); // dismiss

        assert!(world
            .npc_flags
            .contains(&crate::game::npc::NpcId::OldHerbalist));
        assert!(
            world
                .quest_log
                .is_active(crate::game::quest::QuestId::HerbalistsRequest),
            "talking to the herbalist the first time should auto-accept her quest"
        );

        if let GameState::Explore(explore) = &mut world.state {
            explore.player_pos = Position { x: 12, y: 5 };
        }
        world.handle_key(KeyCode::Char('e')); // talk again (quest still unsatisfied: only 3/4 Potions)
        let GameState::Event(ev) = &world.state else {
            panic!("expected dialogue to open again");
        };
        let expected: Vec<String> = crate::game::npc::npc_def(crate::game::npc::NpcId::OldHerbalist)
            .reminder
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(ev.lines, expected);
    }

    #[test]
    fn delivering_the_requested_potions_completes_the_quest_and_grants_the_reward() {
        let mut world = World::new();
        world.inventory.add(crate::game::item::potion(), 1); // now have 4, satisfying the quest
        let gold_before = world.party.gold;

        if let GameState::Explore(explore) = &mut world.state {
            explore.player_pos = Position { x: 12, y: 5 };
        }
        world.handle_key(KeyCode::Char('e')); // first talk: accepts the quest and immediately satisfies it

        let GameState::Event(ev) = &world.state else {
            panic!("expected the turn-in dialogue to open");
        };
        assert!(ev.lines.iter().any(|l| l.contains("You received")));

        assert!(world
            .quest_log
            .completed
            .contains(&crate::game::quest::QuestId::HerbalistsRequest));
        assert!(world.party.gold > gold_before, "the gold reward should be granted");
        assert!(
            world
                .inventory
                .armors
                .iter()
                .any(|a| a.name == "Ranger's Cloak"),
            "the armor reward should be granted"
        );
    }

    #[test]
    fn reaching_chapter_two_and_talking_to_the_scout_completes_her_quest_immediately() {
        let mut world = World::new();
        world.current_chapter = ChapterId::Two;
        world.bosses_defeated.insert(ChapterId::One);
        world.state = GameState::Explore(ExploreState::for_chapter(ChapterId::Two));
        let gold_before = world.party.gold;

        if let GameState::Explore(explore) = &mut world.state {
            explore.player_pos = Position { x: 10, y: 5 }; // the Wounded Scout's spot
        }
        world.handle_key(KeyCode::Char('e'));

        let GameState::Event(ev) = &world.state else {
            panic!("expected talking to the scout to open dialogue");
        };
        assert!(ev.lines.iter().any(|l| l.contains("You received")));
        assert!(world
            .quest_log
            .completed
            .contains(&crate::game::quest::QuestId::ScoutsCommendation));
        assert!(world.party.gold > gold_before);
        assert!(world
            .inventory
            .weapons
            .iter()
            .any(|w| w.name == "Sunken Relic Blade"));
    }

    #[test]
    fn reaching_chapter_three_and_talking_to_the_pilgrim_completes_his_quest_immediately() {
        let mut world = World::new();
        world.current_chapter = ChapterId::Three;
        world.bosses_defeated.insert(ChapterId::One);
        world.bosses_defeated.insert(ChapterId::Two);
        world.state = GameState::Explore(ExploreState::for_chapter(ChapterId::Three));
        let gold_before = world.party.gold;

        if let GameState::Explore(explore) = &mut world.state {
            explore.player_pos = Position { x: 5, y: 5 }; // the Ashen Pilgrim's spot
        }
        world.handle_key(KeyCode::Char('e'));

        let GameState::Event(ev) = &world.state else {
            panic!("expected talking to the pilgrim to open dialogue");
        };
        assert!(ev.lines.iter().any(|l| l.contains("You received")));
        assert!(world
            .quest_log
            .completed
            .contains(&crate::game::quest::QuestId::PilgrimsBlessing));
        assert!(world.party.gold > gold_before);
        assert!(world
            .inventory
            .rings
            .iter()
            .any(|r| r.name == "Band of the Barrow"));
    }
}
