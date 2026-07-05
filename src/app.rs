use crossterm::event::KeyCode;
use rand::rngs::ThreadRng;
use rand::Rng;

use crate::game::character::{barrow_knight, cleric, mage, warrior};
use crate::game::combat::{ActorRef, CombatAction, CombatPhase, CombatState};
use crate::game::inventory_ui::{InventoryMode, InventoryTab, InventoryUiState};
use crate::game::item::Inventory;
use crate::game::map::{Position, Tile};
use crate::game::party::Party;
use crate::game::shop::{shop_item_stock, shop_weapon_stock, ShopMode, ShopTab, ShopUiState};
use crate::game::state::{roll_field_event, EventState, ExploreState, FieldEvent, GameState};

pub struct World {
    pub party: Party,
    pub inventory: Inventory,
    pub state: GameState,
    pub rng: ThreadRng,
    pub should_quit: bool,
    /// Whether the Barrow Knight has been slain — once true, its lair tile
    /// no longer triggers a fight.
    pub boss_defeated: bool,
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
            state: GameState::Explore(ExploreState::new()),
            rng: rand::thread_rng(),
            should_quit: false,
            boss_defeated: false,
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
            GameState::Event(ev) => {
                if key == KeyCode::Enter {
                    let return_pos = ev.return_pos;
                    let mut explore = ExploreState::new();
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
        if key == KeyCode::Char('e') {
            let GameState::Explore(explore) = &self.state else {
                return;
            };
            if explore.map.tile_at(explore.player_pos) == Tile::Town {
                let return_pos = explore.player_pos;
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
                        });
                    }
                }
            }
        } else if tile == Tile::BossLair && !self.boss_defeated {
            let mut combat = CombatState::new(&self.party, vec![barrow_knight("The Barrow Knight")]);
            combat.return_pos = Some(next);
            self.state = GameState::Combat(combat);
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
                    let mut explore = ExploreState::new();
                    explore.player_pos = return_pos;
                    self.state = GameState::Explore(explore);
                }
                KeyCode::Tab
                | KeyCode::Left
                | KeyCode::Char('a')
                | KeyCode::Right
                | KeyCode::Char('d') => {
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.tab = inv.tab.toggled();
                    inv.cursor = 0;
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
                KeyCode::Enter => self.apply_inventory_selection(tab, idx, member_cursor),
                _ => {}
            },
        }
    }

    fn inventory_tab_len(&self, tab: InventoryTab) -> usize {
        match tab {
            InventoryTab::Items => self.inventory.items.len(),
            InventoryTab::Weapons => self.inventory.weapons.len(),
        }
    }

    /// Applies the pending item-use or weapon-equip to `member_idx`, then
    /// returns the inventory screen to browsing mode.
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
        };

        let items_len = self.inventory.items.len();
        let weapons_len = self.inventory.weapons.len();

        if let GameState::Inventory(inv) = &mut self.state {
            if let Some(msg) = message {
                inv.message = Some(msg);
            }
            inv.cursor = match tab {
                InventoryTab::Items => inv.cursor.min(items_len.saturating_sub(1)),
                InventoryTab::Weapons => inv.cursor.min(weapons_len.saturating_sub(1)),
            };
            inv.mode = InventoryMode::Browsing;
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
                let mut explore = ExploreState::new();
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
                shop.tab = shop.tab.toggled();
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
            },
            ShopMode::Sell => match tab {
                ShopTab::Items => self.inventory.items.len(),
                ShopTab::Weapons => self.inventory.weapons.len(),
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
        };

        let items_len = self.inventory.items.len();
        let weapons_len = self.inventory.weapons.len();

        if let GameState::Shop(shop) = &mut self.state {
            if let Some(msg) = message {
                shop.message = Some(msg);
            }
            shop.cursor = match tab {
                ShopTab::Items => shop.cursor.min(items_len.saturating_sub(1)),
                ShopTab::Weapons => shop.cursor.min(weapons_len.saturating_sub(1)),
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
        let boss_was_here = combat.enemies.iter().any(|e| e.name == "The Barrow Knight");
        // The borrow of self.state (via `combat`) ends here, freeing self.party/self.inventory
        // up for mutation below.

        if victory {
            self.party.tick_effects();
            let mut messages = Vec::new();
            if boss_was_here {
                self.boss_defeated = true;
                messages.push("The Barrow Knight falls, and the barrow goes silent.".to_string());
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
            let mut explore = ExploreState::new();
            if let Some(pos) = return_pos {
                explore.player_pos = pos;
            }
            explore.log = messages;
            self.state = GameState::Explore(explore);
        } else if fled {
            self.party.tick_effects();
            let mut explore = ExploreState::new();
            if let Some(pos) = return_pos {
                explore.player_pos = pos;
            }
            self.state = GameState::Explore(explore);
        } else if defeat {
            self.state = GameState::GameOver { victory: false };
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
    use crate::game::item::iron_sword;

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
    fn defeating_the_boss_sets_the_flag_and_the_lair_goes_quiet() {
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

        assert!(world.boss_defeated);
        assert!(matches!(world.state, GameState::Explore(_)));

        // Revisiting the lair afterward should no longer start a fight.
        if let GameState::Explore(explore) = &mut world.state {
            explore.player_pos = Position { x: 26, y: 7 };
        }
        world.handle_key(KeyCode::Down);
        assert!(
            matches!(world.state, GameState::Explore(_)),
            "the lair should be quiet after the boss is defeated"
        );
    }
}
