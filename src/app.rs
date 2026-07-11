use rand::rngs::ThreadRng;
use rand::Rng;

use crate::input::Key;

use crate::game::blacksmith::{
    upgrade_cost, upgrade_increments, weapon_for_mut, weapon_refs, BlacksmithUiState, SHARD_PRICE,
};
use crate::game::chapter::{chapter_def, ChapterId};
use crate::game::character::{
    cleric, mage, rogue, warrior, Character, RingSlot, ALLOC_STATS,
};
use crate::game::combat::{
    ActorRef, CombatAction, CombatPhase, CombatState, FLOATING_NUMBER_TTL, RESOLVING_HOLD_SECONDS,
};
use crate::game::inventory_ui::{
    EquipSlot, InventoryMode, InventoryTab, InventoryUiState, EQUIP_SLOTS,
};
use crate::game::item::{Armor, Inventory, ItemKind, Rarity, Ring, Weapon};
use crate::game::levelup::LevelUpUiState;
use crate::game::map::{Direction, Position, Tile};
use crate::game::npc::{npc_def, NpcId};
use crate::game::party::Party;
use crate::game::shop::{
    sell_price, shop_armor_stock, shop_item_stock, shop_ring_stock, shop_weapon_stock, ShopMode,
    ShopTab, ShopUiState,
};
use crate::game::state::{
    roll_field_event, EventState, ExploreState, FieldEvent, GameState, MainMenuState,
    DIFFICULTY_OPTIONS, MAIN_MENU_ROWS, STEP_ANIM_SECONDS,
};

/// `World.anim_timer` wraps at this many seconds so it doesn't grow without
/// bound over a very long session — large enough that no in-flight sine-wave
/// animation (see `render::combat`) ever notices the wrap.
const ANIM_TIMER_WRAP: f32 = 10_000.0;

/// If the turn that was just resolved produced an animation event, parks
/// `combat.phase` at `Resolving` and stashes the real outcome in
/// `pending_phase` until `World::tick` restores it — see
/// `CombatState::pending_phase`'s doc comment for why this lives here
/// rather than inside `game::combat`.
fn begin_resolving_hold(combat: &mut CombatState) {
    if combat.last_action_anim.is_some() {
        let real_phase = std::mem::replace(&mut combat.phase, CombatPhase::Resolving);
        combat.pending_phase = Some(real_phase);
        combat.resolving_timer = RESOLVING_HOLD_SECONDS;
    }
}

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
    /// Dark-Souls-style New Game+ cycle: 0 on a first playthrough, bumped by
    /// `start_new_game_plus` after beating the final boss, capped at 7.
    /// Multiplies every enemy's stats via `Character::apply_ng_plus`.
    pub ng_plus: u32,
    /// Starting difficulty offset picked on the main menu (0 = Normal,
    /// see `DIFFICULTY_OPTIONS`). Composed with `ng_plus` by
    /// `challenge_cycle()` for all combat scaling; never shown as part of
    /// the NG+ story-cycle counter.
    pub difficulty: u32,
    /// Raw `Character.name`s of every species/boss the party has fought at
    /// least once — gates what the bestiary screen reveals. Recorded at
    /// combat start (seeing the fight counts, winning isn't required) and
    /// deliberately NOT cleared by `start_new_game_plus`: the codex is a
    /// chronicle of the whole journey, not one cycle.
    pub bestiary_seen: std::collections::HashSet<String>,
    /// Accumulates real elapsed seconds, wrapping at `ANIM_TIMER_WRAP` — a
    /// plain monotonic clock `render::combat` samples to drive the idle-bob
    /// sine wave on combat sprites.
    pub anim_timer: f32,
    /// Whether the `?` keybind reference popup is currently shown, overlaid
    /// on top of whatever screen is active.
    pub show_help: bool,
    /// True while `q` is waiting for the player to confirm quitting without
    /// an autosave. Lives on `World` (not `ExploreState`) so quitting works
    /// the same everywhere `current_player_pos` returns `Some` — Explore and
    /// every sub-screen that carries a `return_pos` — not just Explore.
    pub confirm_quit: bool,
    /// Which of the 3 save slots this session is playing in, if any — set
    /// when a slot is picked (or started fresh) on the main menu, and read
    /// by the Shift+S quicksave handler so it always writes back to the same
    /// slot rather than a single fixed path. `None` only before a slot has
    /// ever been chosen (e.g. a `World::new()` not routed through the menu).
    pub active_slot: Option<u8>,
}

impl World {
    pub fn new() -> Self {
        let party = Party::new(vec![
            warrior("Bram"),
            mage("Sella"),
            cleric("Idris"),
            rogue("Wren"),
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
            ng_plus: 0,
            difficulty: 0,
            bestiary_seen: std::collections::HashSet::new(),
            anim_timer: 0.0,
            show_help: false,
            confirm_quit: false,
            active_slot: None,
        }
    }

    /// The startup state: a fresh world parked on the title screen, with all
    /// 3 save slots loaded from disk for the slot picker.
    pub fn at_main_menu() -> Self {
        let mut world = Self::new();
        world.state = GameState::MainMenu(MainMenuState::new(crate::game::save::read_all_slots()));
        world
    }

    /// Snapshots everything persistent into a `SaveData`. Only meaningful
    /// while exploring — `player_pos` is where a loaded game will resume.
    pub fn to_save(&self, player_pos: Position) -> crate::game::save::SaveData {
        crate::game::save::SaveData {
            version: crate::game::save::SAVE_VERSION,
            party: self.party.clone(),
            inventory: self.inventory.clone(),
            current_chapter: self.current_chapter,
            bosses_defeated: self.bosses_defeated.clone(),
            npc_flags: self.npc_flags.clone(),
            quest_log: self.quest_log.clone(),
            player_pos,
            ng_plus: self.ng_plus,
            bestiary_seen: self.bestiary_seen.clone(),
            difficulty: self.difficulty,
        }
    }

    /// The gameplay challenge tier: the NG+ story cycle plus the starting
    /// difficulty offset, saturating at the same NG+7 cap. Every combat
    /// consumer (encounter scaling, loot, flee odds, elites, boss NG+
    /// multipliers) reads this; the HUD and menu keep displaying the raw
    /// `ng_plus` so the story-cycle counter never lies. Accepted
    /// consequence: an Accursed run unlocks the NG+1/NG+2 blessing and
    /// curse pools immediately and saturates the cap two cycles early.
    pub fn challenge_cycle(&self) -> u32 {
        (self.ng_plus + self.difficulty).min(7)
    }

    /// Rebuilds a `World` from a snapshot loaded out of `slot`, dropping the
    /// player back onto the saved chapter's map at the saved position and
    /// arming `active_slot` so a later quicksave lands back in the same slot.
    pub fn from_save(data: crate::game::save::SaveData, slot: u8) -> Self {
        let mut explore = ExploreState::for_chapter(data.current_chapter);
        explore.player_pos = data.player_pos;
        explore.push_log("Loaded saved game.");
        // Abilities are serialized with the party, so a save written before
        // a class gained an ability would never show it — resync every
        // member's kit from the class table on load.
        let mut party = data.party;
        for member in &mut party.members {
            if member.class != crate::game::character::Class::Monster {
                member.abilities = crate::game::character::class_abilities(member.class);
            }
        }
        Self {
            party,
            inventory: data.inventory,
            state: GameState::Explore(explore),
            rng: rand::thread_rng(),
            should_quit: false,
            current_chapter: data.current_chapter,
            bosses_defeated: data.bosses_defeated,
            npc_flags: data.npc_flags,
            quest_log: data.quest_log,
            ng_plus: data.ng_plus,
            difficulty: data.difficulty,
            bestiary_seen: data.bestiary_seen,
            anim_timer: 0.0,
            show_help: false,
            confirm_quit: false,
            active_slot: Some(slot),
        }
    }

    /// Starts a brand-new game and marks `slot` as the active save slot from
    /// now on, so a later Shift+S quicksave lands in the slot the player
    /// picked rather than a default.
    fn start_new_game_in_slot(&mut self, slot: u8, difficulty: u32) {
        *self = Self::new();
        self.active_slot = Some(slot);
        self.difficulty = difficulty;
    }

    /// Where the player is standing right now, if there's a meaningful
    /// answer — `Some` while exploring or in any sub-screen reachable from
    /// Explore (all five carry their own `return_pos`), `None` on the main
    /// menu, in combat, mid-event, or on the game-over screen. Used to scope
    /// `q`/`S` (quit/save) to exactly the states where "go back to the map"
    /// makes sense.
    pub fn current_player_pos(&self) -> Option<Position> {
        match &self.state {
            GameState::Explore(explore) => Some(explore.player_pos),
            GameState::Inventory(ui) => Some(ui.return_pos),
            GameState::Shop(ui) => Some(ui.return_pos),
            GameState::QuestLog(ui) => Some(ui.return_pos),
            GameState::Bestiary(ui) => Some(ui.return_pos),
            GameState::LevelUp(ui) => Some(ui.return_pos),
            GameState::Blacksmith(ui) => Some(ui.return_pos),
            GameState::MainMenu(_)
            | GameState::Combat(_)
            | GameState::Event(_)
            | GameState::GameOver { .. } => None,
        }
    }

    /// Surfaces a one-off status message (currently only the save result) on
    /// whichever screen is active, using that screen's own `message`/log
    /// field. `QuestLog` has no such field (it's a read-only browser with no
    /// existing message slot) so the save there is silent but still happens.
    fn push_status_message(&mut self, message: String) {
        match &mut self.state {
            GameState::Explore(explore) => explore.push_log(message),
            GameState::Inventory(ui) => ui.message = Some(message),
            GameState::Shop(ui) => ui.message = Some(message),
            GameState::LevelUp(ui) => ui.message = Some(message),
            GameState::Blacksmith(ui) => ui.message = Some(message),
            GameState::QuestLog(_)
            | GameState::Bestiary(_)
            | GameState::MainMenu(_)
            | GameState::Combat(_)
            | GameState::Event(_)
            | GameState::GameOver { .. } => {}
        }
    }

    /// Starts the next New Game+ cycle after beating the final boss: bumps
    /// `ng_plus` (capped at 7, matching Dark Souls' own NG+7 cap), resets
    /// chapter/boss/NPC/quest progress so the story replays from chapter
    /// one, and drops the player back at its spawn. Party level, gear, gold,
    /// and inventory are deliberately preserved — that's the whole point of
    /// New Game+.
    /// Marks every species/boss in `enemies` as encountered, unlocking its
    /// bestiary entry. Keyed on the raw `Character.name` (never
    /// `display_name`), matching `loot_profile`/`bestiary_entries`.
    fn record_bestiary(&mut self, enemies: &[Character]) {
        for e in enemies {
            self.bestiary_seen.insert(e.name.clone());
        }
    }

    pub fn start_new_game_plus(&mut self) {
        self.ng_plus = (self.ng_plus + 1).min(7);
        self.current_chapter = ChapterId::One;
        self.bosses_defeated.clear();
        self.npc_flags.clear();
        self.quest_log = crate::game::quest::QuestLog::new();
        let mut explore = ExploreState::for_chapter(ChapterId::One);
        explore.push_log(format!(
            "New Game+{} begins. The world grows harsher...",
            self.ng_plus
        ));
        self.state = GameState::Explore(explore);
    }

    pub fn handle_key(&mut self, key: Key) {
        if self.show_help {
            if matches!(key, Key::Char('?') | Key::Esc) {
                self.show_help = false;
            }
            return;
        }
        // The quit confirmation is modal: nothing else reacts until the
        // player commits or backs out, mirroring `confirm_overwrite` on the
        // main menu.
        if self.confirm_quit {
            match key {
                Key::Enter | Key::Char('y') => self.should_quit = true,
                Key::Esc | Key::Char('n') => self.confirm_quit = false,
                _ => {}
            }
            return;
        }
        if key == Key::Char('?')
            && !matches!(
                self.state,
                GameState::MainMenu(_) | GameState::GameOver { .. }
            )
        {
            self.show_help = true;
            return;
        }
        // Quit and save are handled once, here, rather than per-screen, so
        // they work identically from Explore and every sub-screen that
        // carries a `return_pos` (Inventory/Shop/QuestLog/LevelUp/
        // Blacksmith) — not just Explore. `current_player_pos` returns
        // `None` on MainMenu/Combat/Event/GameOver, where neither makes
        // sense (MainMenu has its own unconfirmed `q`, handled below).
        if key == Key::Char('q') && self.current_player_pos().is_some() {
            self.confirm_quit = true;
            return;
        }
        if key == Key::Char('S') {
            if let Some(pos) = self.current_player_pos() {
                let message = match self.active_slot {
                    Some(slot) => {
                        let data = self.to_save(pos);
                        match crate::game::save::write(&data, slot) {
                            Ok(()) => "Game saved.".to_string(),
                            Err(e) => format!("Couldn't save the game: {e}"),
                        }
                    }
                    None => "No active save slot.".to_string(),
                };
                self.push_status_message(message);
                return;
            }
        }
        match &mut self.state {
            GameState::MainMenu(_) => self.handle_main_menu_key(key),
            GameState::Explore(_) => self.handle_explore_key(key),
            GameState::Combat(_) => self.handle_combat_key(key),
            GameState::Inventory(_) => self.handle_inventory_key(key),
            GameState::Shop(_) => self.handle_shop_key(key),
            GameState::QuestLog(_) => self.handle_quest_log_key(key),
            GameState::Bestiary(_) => self.handle_bestiary_key(key),
            GameState::LevelUp(_) => self.handle_levelup_key(key),
            GameState::Blacksmith(_) => self.handle_blacksmith_key(key),
            GameState::Event(ev) => {
                if key == Key::Enter {
                    let return_pos = ev.return_pos;
                    if let Some(id) = ev.npc {
                        self.npc_flags.insert(id);
                    }
                    let mut explore = ExploreState::for_chapter(self.current_chapter);
                    explore.player_pos = return_pos;
                    self.state = GameState::Explore(explore);
                }
            }
            GameState::GameOver { victory } => {
                let victory = *victory;
                if key == Key::Enter {
                    self.should_quit = true;
                } else if victory && matches!(key, Key::Char('n') | Key::Char('N')) {
                    self.start_new_game_plus();
                }
            }
        }
    }

    fn handle_main_menu_key(&mut self, key: Key) {
        let GameState::MainMenu(menu) = &mut self.state else {
            return;
        };
        // The difficulty picker is modal: it's the last stop before any new
        // game actually starts (every new-game entry point below arms it).
        if let Some(slot) = menu.pending_new_game {
            match key {
                Key::Up | Key::Char('w') | Key::Left | Key::Char('a') => {
                    menu.difficulty_cursor = (menu.difficulty_cursor + DIFFICULTY_OPTIONS.len()
                        - 1)
                        % DIFFICULTY_OPTIONS.len();
                }
                Key::Down | Key::Char('s') | Key::Right | Key::Char('d') => {
                    menu.difficulty_cursor = (menu.difficulty_cursor + 1) % DIFFICULTY_OPTIONS.len();
                }
                Key::Enter => {
                    let difficulty = DIFFICULTY_OPTIONS[menu.difficulty_cursor].1;
                    self.start_new_game_in_slot(slot, difficulty);
                }
                Key::Esc => {
                    menu.pending_new_game = None;
                    menu.difficulty_cursor = 0;
                }
                _ => {}
            }
            return;
        }
        // The overwrite confirmation is modal: nothing else reacts until the
        // player commits or backs out.
        if let Some(slot) = menu.confirm_overwrite {
            match key {
                Key::Enter | Key::Char('y') => {
                    menu.confirm_overwrite = None;
                    menu.pending_new_game = Some(slot);
                }
                Key::Esc | Key::Char('n') => menu.confirm_overwrite = None,
                _ => {}
            }
            return;
        }
        match key {
            Key::Up | Key::Char('w') => {
                menu.cursor = (menu.cursor + MAIN_MENU_ROWS - 1) % MAIN_MENU_ROWS;
            }
            Key::Down | Key::Char('s') => {
                menu.cursor = (menu.cursor + 1) % MAIN_MENU_ROWS;
            }
            Key::Char('q') | Key::Esc => self.should_quit = true,
            // Forces "New Game" on the highlighted slot regardless of
            // whether Enter would have loaded it — arms the overwrite
            // confirm if the slot is occupied, same as picking an occupied
            // slot's row used to always do.
            Key::Char('n') => {
                let cursor = menu.cursor;
                if cursor < 3 {
                    let slot = (cursor + 1) as u8;
                    if menu.slots[cursor].is_some() {
                        menu.confirm_overwrite = Some(slot);
                    } else {
                        menu.pending_new_game = Some(slot);
                    }
                }
            }
            Key::Enter => {
                let cursor = menu.cursor;
                if cursor == 3 {
                    self.should_quit = true;
                } else {
                    let slot = (cursor + 1) as u8;
                    match menu.slots[cursor].take() {
                        Some(data) => *self = Self::from_save(data, slot),
                        None => menu.pending_new_game = Some(slot),
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_explore_key(&mut self, key: Key) {
        if !matches!(self.state, GameState::Explore(_)) {
            return;
        }
        if key == Key::PageUp {
            let GameState::Explore(explore) = &mut self.state else {
                return;
            };
            explore.log_scroll = (explore.log_scroll + 10).min(explore.log.len());
            return;
        }
        if key == Key::PageDown {
            let GameState::Explore(explore) = &mut self.state else {
                return;
            };
            explore.log_scroll = explore.log_scroll.saturating_sub(10);
            return;
        }
        if key == Key::Char('i') {
            let GameState::Explore(explore) = &self.state else {
                return;
            };
            let return_pos = explore.player_pos;
            self.state = GameState::Inventory(InventoryUiState::new(return_pos));
            return;
        }
        if key == Key::Char('l') {
            let GameState::Explore(explore) = &self.state else {
                return;
            };
            let return_pos = explore.player_pos;
            self.state =
                GameState::QuestLog(crate::game::quest_ui::QuestLogUiState::new(return_pos));
            return;
        }
        if key == Key::Char('u') {
            let GameState::Explore(explore) = &self.state else {
                return;
            };
            let return_pos = explore.player_pos;
            self.state = GameState::LevelUp(LevelUpUiState::new(return_pos));
            return;
        }
        if key == Key::Char('b') {
            let GameState::Explore(explore) = &self.state else {
                return;
            };
            let return_pos = explore.player_pos;
            self.state =
                GameState::Bestiary(crate::game::bestiary_ui::BestiaryUiState::new(return_pos));
            return;
        }
        if key == Key::Char('e') {
            let GameState::Explore(explore) = &self.state else {
                return;
            };
            let return_pos = explore.player_pos;
            let npc_id = explore.map.npc_at(return_pos);
            let is_town = explore.map.tile_at(return_pos) == Tile::Town;
            // The borrow of self.state (via `explore`) ends here, freeing
            // self up for the mutable interact_with_npc call below.
            if npc_id == Some(NpcId::Blacksmith) {
                self.state = GameState::Blacksmith(BlacksmithUiState::new(return_pos));
            } else if let Some(id) = npc_id {
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
            Key::Up | Key::Char('w') => Some((0, -1, Direction::Up)),
            Key::Down | Key::Char('s') => Some((0, 1, Direction::Down)),
            Key::Left | Key::Char('a') => Some((-1, 0, Direction::Left)),
            Key::Right | Key::Char('d') => Some((1, 0, Direction::Right)),
            _ => None,
        };
        let Some((dx, dy, dir)) = delta else { return };
        explore.facing = dir;
        let next = Position {
            x: explore.player_pos.x + dx,
            y: explore.player_pos.y + dy,
        };
        if !explore.map.is_walkable(next) {
            return;
        }
        explore.player_pos = next;
        explore.step_elapsed = 0.0;
        let tile = explore.map.tile_at(next);
        if tile == Tile::TallGrass {
            explore.steps_in_grass += 1;
            // ~1 in 4 steps through grass triggers *some* field event (not always a fight).
            if self.rng.gen_ratio(1, 4) {
                let enemy_level = chapter_def(self.current_chapter).enemy_level;
                let challenge = self.challenge_cycle();
                match roll_field_event(&mut self.rng, enemy_level, challenge) {
                    FieldEvent::Combat(enemies) => {
                        self.record_bestiary(&enemies);
                        let mut combat = CombatState::new(&self.party, enemies);
                        combat.return_pos = Some(next);
                        combat.ng_plus = self.challenge_cycle();
                        self.state = GameState::Combat(combat);
                    }
                    FieldEvent::Blessing(effect) => {
                        let lines = vec![
                            "A warm light washes over your party...".to_string(),
                            format!(
                                "{} takes hold! (+{} {} for the next {} encounters)",
                                effect.name,
                                effect.delta,
                                effect.target,
                                effect.encounters_remaining
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
                                effect.name,
                                effect.delta,
                                effect.target,
                                effect.encounters_remaining
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
                    FieldEvent::Treasure {
                        gold,
                        item,
                        weapon,
                        armor,
                        ring,
                        materials,
                    } => {
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
                        if let Some(armor) = armor {
                            lines.push(format!(
                                "Folded inside: a {} rarity {}!",
                                armor.rarity, armor.name
                            ));
                            self.inventory.add_armor(armor);
                        }
                        if let Some(ring) = ring {
                            lines.push(format!(
                                "Tucked in a corner: a {} rarity {}!",
                                ring.rarity, ring.name
                            ));
                            self.inventory.add_ring(ring);
                        }
                        if materials > 0 {
                            lines.push(format!(
                                "Buried alongside it: {materials} Titanite Shard(s)!"
                            ));
                            self.inventory.upgrade_materials += materials;
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
            let mut boss = (def.boss)(def.boss_display_name);
            boss.scale_boss_to_party(self.party.average_level(), def.boss_baseline_level());
            boss.apply_ng_plus(self.challenge_cycle());
            self.record_bestiary(std::slice::from_ref(&boss));
            let mut combat = CombatState::new(&self.party, vec![boss]);
            combat.return_pos = Some(next);
            combat.ng_plus = self.challenge_cycle();
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

    fn handle_quest_log_key(&mut self, key: Key) {
        let GameState::QuestLog(ui_ref) = &self.state else {
            return;
        };
        let cursor = ui_ref.cursor;
        match key {
            Key::Esc => {
                let GameState::QuestLog(ui) = &self.state else {
                    return;
                };
                let return_pos = ui.return_pos;
                let mut explore = ExploreState::for_chapter(self.current_chapter);
                explore.player_pos = return_pos;
                self.state = GameState::Explore(explore);
            }
            Key::Up | Key::Char('w') => {
                let len = self.quest_log.active.len().max(1);
                let GameState::QuestLog(ui) = &mut self.state else {
                    return;
                };
                ui.cursor = (cursor + len - 1) % len;
            }
            Key::Down | Key::Char('s') => {
                let len = self.quest_log.active.len().max(1);
                let GameState::QuestLog(ui) = &mut self.state else {
                    return;
                };
                ui.cursor = (cursor + 1) % len;
            }
            _ => {}
        }
    }

    fn handle_bestiary_key(&mut self, key: Key) {
        let GameState::Bestiary(ui_ref) = &self.state else {
            return;
        };
        let cursor = ui_ref.cursor;
        let len = crate::game::bestiary_ui::bestiary_entries().len().max(1);
        match key {
            Key::Esc => {
                let GameState::Bestiary(ui) = &self.state else {
                    return;
                };
                let return_pos = ui.return_pos;
                let mut explore = ExploreState::for_chapter(self.current_chapter);
                explore.player_pos = return_pos;
                self.state = GameState::Explore(explore);
            }
            Key::Up | Key::Char('w') => {
                let GameState::Bestiary(ui) = &mut self.state else {
                    return;
                };
                ui.cursor = (cursor + len - 1) % len;
            }
            Key::Down | Key::Char('s') => {
                let GameState::Bestiary(ui) = &mut self.state else {
                    return;
                };
                ui.cursor = (cursor + 1) % len;
            }
            _ => {}
        }
    }

    fn handle_combat_key(&mut self, key: Key) {
        let GameState::Combat(combat) = &mut self.state else {
            return;
        };

        if key == Key::PageUp {
            combat.log_scroll = (combat.log_scroll + 10).min(combat.log.len());
            return;
        }
        if key == Key::PageDown {
            combat.log_scroll = combat.log_scroll.saturating_sub(10);
            return;
        }

        match combat.phase {
            CombatPhase::SelectAction { actor } => {
                // Only players choose via keyboard; enemy turns resolve automatically (see tick()).
                let ActorRef::Player(pi) = actor else { return };
                let menu_len = 4; // Attack, Ability, Item, Flee
                match key {
                    Key::Up | Key::Char('w') => {
                        combat.menu_cursor = (combat.menu_cursor + menu_len - 1) % menu_len;
                    }
                    Key::Down | Key::Char('s') => {
                        combat.menu_cursor = (combat.menu_cursor + 1) % menu_len;
                    }
                    Key::Enter => match combat.menu_cursor {
                        0 => {
                            let alive = combat.alive_enemy_indices();
                            let target_idx = alive.first().copied().unwrap_or(0);
                            combat.phase = CombatPhase::SelectTarget {
                                actor,
                                action: CombatAction::Attack,
                                target_idx,
                            };
                            // Only one enemy to hit — skip the redundant
                            // confirm and resolve the attack immediately.
                            if alive.len() == 1 {
                                self.resolve_pending_target();
                            }
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
                    Key::Up | Key::Char('w') => {
                        let new_cursor = (cursor + ability_count - 1) % ability_count;
                        combat.phase = CombatPhase::SelectAbility {
                            actor,
                            cursor: new_cursor,
                        };
                    }
                    Key::Down | Key::Char('s') => {
                        let new_cursor = (cursor + 1) % ability_count;
                        combat.phase = CombatPhase::SelectAbility {
                            actor,
                            cursor: new_cursor,
                        };
                    }
                    Key::Enter => {
                        let Some(ability) = self.party.members[pi].abilities.get(cursor).cloned()
                        else {
                            return;
                        };
                        if self.party.members[pi].stats.mp < ability.mp_cost {
                            let name = self.party.members[pi].name.clone();
                            combat.push_log(format!(
                                "{name} doesn't have enough MP for {}.",
                                ability.name
                            ));
                            combat.phase = CombatPhase::SelectAction { actor };
                            return;
                        }
                        let is_heal = self.party.members[pi].ability_is_heal(cursor);
                        let targets_all = ability.targets_all_enemies;
                        let alive = combat.alive_enemy_indices();
                        let target_idx = if is_heal {
                            pi
                        } else {
                            alive.first().copied().unwrap_or(0)
                        };
                        combat.phase = CombatPhase::SelectTarget {
                            actor,
                            action: CombatAction::Ability(cursor),
                            target_idx,
                        };
                        // Party-wide abilities and lone-enemy fights don't
                        // need a second confirm on top of picking the ability.
                        if targets_all || (!is_heal && alive.len() == 1) {
                            self.resolve_pending_target();
                        }
                    }
                    Key::Esc => {
                        combat.phase = CombatPhase::SelectAction { actor };
                    }
                    _ => {}
                }
            }
            CombatPhase::SelectItem { actor, cursor } => {
                let ActorRef::Player(pi) = actor else { return };
                let item_count = self.inventory.items.len().max(1);
                match key {
                    Key::Up | Key::Char('w') => {
                        let new_cursor = (cursor + item_count - 1) % item_count;
                        combat.phase = CombatPhase::SelectItem {
                            actor,
                            cursor: new_cursor,
                        };
                    }
                    Key::Down | Key::Char('s') => {
                        let new_cursor = (cursor + 1) % item_count;
                        combat.phase = CombatPhase::SelectItem {
                            actor,
                            cursor: new_cursor,
                        };
                    }
                    Key::Enter => {
                        if let Some((item, _)) = self.inventory.items.get(cursor) {
                            // Revives can only target the fallen; refuse to
                            // enter target selection if no one is down.
                            let target_idx = if matches!(item.kind, ItemKind::Revive { .. }) {
                                let Some(fallen) =
                                    self.party.members.iter().position(|m| !m.is_alive())
                                else {
                                    combat
                                        .push_log("No one has fallen — the ember would be wasted.");
                                    return;
                                };
                                fallen
                            } else {
                                pi
                            };
                            combat.phase = CombatPhase::SelectTarget {
                                actor,
                                action: CombatAction::Item(cursor),
                                target_idx,
                            };
                        }
                    }
                    Key::Esc => {
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
                    CombatAction::Item(_) => true, // consumables are always ally-targeted
                    CombatAction::Attack | CombatAction::Flee => false,
                };
                // Revives invert the ally filter: only the fallen qualify.
                let is_revive = match action {
                    CombatAction::Item(idx) => matches!(
                        self.inventory.items.get(idx).map(|(i, _)| i.kind),
                        Some(ItemKind::Revive { .. })
                    ),
                    _ => false,
                };
                match key {
                    Key::Left | Key::Char('a') | Key::Up | Key::Char('w') => {
                        let new_idx =
                            cycle_target(&self.party, combat, is_heal, is_revive, target_idx, -1);
                        combat.phase = CombatPhase::SelectTarget {
                            actor,
                            action,
                            target_idx: new_idx,
                        };
                    }
                    Key::Right | Key::Char('d') | Key::Down | Key::Char('s') => {
                        let new_idx =
                            cycle_target(&self.party, combat, is_heal, is_revive, target_idx, 1);
                        combat.phase = CombatPhase::SelectTarget {
                            actor,
                            action,
                            target_idx: new_idx,
                        };
                    }
                    Key::Enter => self.resolve_pending_target(),
                    Key::Esc => {
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
                if matches!(key, Key::Enter) {
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
        begin_resolving_hold(combat);
    }

    fn handle_inventory_key(&mut self, key: Key) {
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
                Key::Esc => {
                    let GameState::Inventory(inv) = &self.state else {
                        return;
                    };
                    let return_pos = inv.return_pos;
                    let mut explore = ExploreState::for_chapter(self.current_chapter);
                    explore.player_pos = return_pos;
                    self.state = GameState::Explore(explore);
                }
                Key::Tab | Key::Right | Key::Char('d') => {
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.tab = inv.tab.next();
                    inv.cursor = 0;
                }
                Key::Left | Key::Char('a') => {
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.tab = inv.tab.prev();
                    inv.cursor = 0;
                }
                Key::Char('p') => {
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.mode = InventoryMode::PartyGear {
                        member_cursor: 0,
                        slot_cursor: 0,
                    };
                }
                Key::Up | Key::Char('w') => {
                    let len = self.inventory_tab_len(tab);
                    if len > 0 {
                        let GameState::Inventory(inv) = &mut self.state else {
                            return;
                        };
                        inv.cursor = (cursor + len - 1) % len;
                    }
                }
                Key::Down | Key::Char('s') => {
                    let len = self.inventory_tab_len(tab);
                    if len > 0 {
                        let GameState::Inventory(inv) = &mut self.state else {
                            return;
                        };
                        inv.cursor = (cursor + 1) % len;
                    }
                }
                Key::Enter => {
                    if tab == InventoryTab::Materials {
                        let GameState::Inventory(inv) = &mut self.state else {
                            return;
                        };
                        inv.message =
                            Some("Titanite Shards are spent at the blacksmith.".to_string());
                        return;
                    }
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
                Key::Esc => {
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.mode = InventoryMode::Browsing;
                }
                Key::Up | Key::Char('w') => {
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
                Key::Down | Key::Char('s') => {
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
                Key::Enter => {
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
                Key::Esc => {
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.mode = InventoryMode::Browsing;
                }
                Key::Up | Key::Char('w') | Key::Down | Key::Char('s') => {
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.mode = InventoryMode::SelectRingSlot {
                        idx,
                        member_idx,
                        slot_cursor: 1 - slot_cursor,
                    };
                }
                Key::Enter => {
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
                Key::Esc => {
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.mode = InventoryMode::Browsing;
                }
                Key::Up | Key::Char('w') => {
                    let len = self.party.members.len().max(1);
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.mode = InventoryMode::PartyGear {
                        member_cursor: (member_cursor + len - 1) % len,
                        slot_cursor,
                    };
                }
                Key::Down | Key::Char('s') => {
                    let len = self.party.members.len().max(1);
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.mode = InventoryMode::PartyGear {
                        member_cursor: (member_cursor + 1) % len,
                        slot_cursor,
                    };
                }
                Key::Left | Key::Char('a') => {
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.mode = InventoryMode::PartyGear {
                        member_cursor,
                        slot_cursor: (slot_cursor + EQUIP_SLOTS.len() - 1) % EQUIP_SLOTS.len(),
                    };
                }
                Key::Right | Key::Char('d') | Key::Tab => {
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.mode = InventoryMode::PartyGear {
                        member_cursor,
                        slot_cursor: (slot_cursor + 1) % EQUIP_SLOTS.len(),
                    };
                }
                Key::Enter => {
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
                Key::Esc => {
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.mode = InventoryMode::PartyGear {
                        member_cursor: member_idx,
                        slot_cursor: equip_slot_index(slot),
                    };
                }
                Key::Up | Key::Char('w') | Key::Down | Key::Char('s') => {
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.mode = InventoryMode::PartyGearAction {
                        member_idx,
                        slot,
                        action_cursor: 1 - action_cursor,
                    };
                }
                Key::Enter => {
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
                Key::Esc => {
                    let GameState::Inventory(inv) = &mut self.state else {
                        return;
                    };
                    inv.mode = InventoryMode::PartyGear {
                        member_cursor: from_member,
                        slot_cursor: equip_slot_index(slot),
                    };
                }
                Key::Up | Key::Char('w') => {
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
                Key::Down | Key::Char('s') => {
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
                Key::Enter => {
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
            InventoryTab::Materials => usize::from(self.inventory.upgrade_materials > 0),
        }
    }

    /// Applies the pending item-use/weapon-equip/armor-equip to `member_idx`,
    /// then returns the inventory screen to browsing mode. Rings are handled
    /// separately by `apply_inventory_ring_selection` since they need a slot
    /// choice first.
    fn apply_inventory_selection(&mut self, tab: InventoryTab, idx: usize, member_idx: usize) {
        let message = match tab {
            InventoryTab::Items => {
                match self.inventory.items.get(idx).map(|(item, _)| item.kind) {
                    // Party-wide, ignores the chosen member entirely.
                    Some(ItemKind::CureCurse) => {
                        self.inventory.use_at(idx);
                        Some(if self.party.cure_curses() > 0 {
                            "The curses clinging to the party crumble away.".to_string()
                        } else {
                            "No curses cling to the party — the stone is spent anyway.".to_string()
                        })
                    }
                    // Refuse (without consuming) rather than waste a revive
                    // on someone still standing.
                    Some(ItemKind::Revive { .. })
                        if self
                            .party
                            .members
                            .get(member_idx)
                            .is_some_and(|m| m.is_alive()) =>
                    {
                        self.party
                            .members
                            .get(member_idx)
                            .map(|m| format!("{} is still standing — save the ember.", m.name))
                    }
                    Some(_) => {
                        let kind = self.inventory.use_at(idx);
                        match (kind, self.party.members.get_mut(member_idx)) {
                            (Some(kind), Some(member)) => {
                                let name = member.name.clone();
                                let effect_msg = crate::game::combat::use_item_kind(kind, member);
                                Some(format!("{name}: {effect_msg}"))
                            }
                            _ => None,
                        }
                    }
                    None => None,
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
            InventoryTab::Materials => None, // never reaches here; handled in the Enter handler above
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
                InventoryTab::Rings | InventoryTab::Materials => inv.cursor,
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

    fn handle_shop_key(&mut self, key: Key) {
        // Same read-then-branch shape as handle_inventory_key: copy out the
        // Copy fields first so this shared borrow of self.state ends before
        // we touch self.party/self.inventory in the branches below.
        let GameState::Shop(shop_ref) = &self.state else {
            return;
        };
        let mode = shop_ref.mode;
        let tab = shop_ref.tab;
        let cursor = shop_ref.cursor;
        let pending_sell = shop_ref.pending_sell;

        match key {
            Key::Esc => {
                let GameState::Shop(shop) = &self.state else {
                    return;
                };
                let return_pos = shop.return_pos;
                let mut explore = ExploreState::for_chapter(self.current_chapter);
                explore.player_pos = return_pos;
                self.state = GameState::Explore(explore);
            }
            Key::Left | Key::Char('a') | Key::Right | Key::Char('d') => {
                let GameState::Shop(shop) = &mut self.state else {
                    return;
                };
                shop.mode = shop.mode.toggled();
                shop.cursor = 0;
                shop.pending_sell = None;
            }
            Key::Tab => {
                let GameState::Shop(shop) = &mut self.state else {
                    return;
                };
                shop.tab = shop.tab.next();
                shop.cursor = 0;
                shop.pending_sell = None;
            }
            Key::Up | Key::Char('w') => {
                let len = self.shop_list_len(mode, tab);
                if len > 0 {
                    let GameState::Shop(shop) = &mut self.state else {
                        return;
                    };
                    shop.cursor = (cursor + len - 1) % len;
                    shop.pending_sell = None;
                }
            }
            Key::Down | Key::Char('s') => {
                let len = self.shop_list_len(mode, tab);
                if len > 0 {
                    let GameState::Shop(shop) = &mut self.state else {
                        return;
                    };
                    shop.cursor = (cursor + 1) % len;
                    shop.pending_sell = None;
                }
            }
            Key::Enter => match mode {
                ShopMode::Buy => self.apply_shop_buy(tab, cursor),
                ShopMode::Sell => {
                    if pending_sell == Some((tab, cursor)) || !self.needs_sell_confirm(tab, cursor)
                    {
                        self.apply_shop_sell(tab, cursor);
                    } else {
                        let message = self.sell_confirm_message(tab, cursor);
                        if let GameState::Shop(shop) = &mut self.state {
                            shop.pending_sell = Some((tab, cursor));
                            shop.message = Some(message);
                        }
                    }
                }
            },
            Key::Char('x') if mode == ShopMode::Sell => self.apply_shop_sell_all_common(),
            _ => {}
        }
    }

    /// Whether selling `tab`/`idx` in the current bag needs a second Enter
    /// to confirm — gated on Rare+ gear only, since Common/Uncommon pieces
    /// and consumables (which have no rarity at all) are cheap and plentiful
    /// enough that a confirm step would just be friction.
    fn needs_sell_confirm(&self, tab: ShopTab, idx: usize) -> bool {
        match tab {
            ShopTab::Items => false,
            ShopTab::Weapons => self
                .inventory
                .weapons
                .get(idx)
                .is_some_and(|w| w.rarity >= Rarity::Rare),
            ShopTab::Armor => self
                .inventory
                .armors
                .get(idx)
                .is_some_and(|a| a.rarity >= Rarity::Rare),
            ShopTab::Rings => self
                .inventory
                .rings
                .get(idx)
                .is_some_and(|r| r.rarity >= Rarity::Rare),
        }
    }

    fn sell_confirm_message(&self, tab: ShopTab, idx: usize) -> String {
        match tab {
            ShopTab::Items => String::new(),
            ShopTab::Weapons => self
                .inventory
                .weapons
                .get(idx)
                .map(|w| {
                    format!(
                        "Sell {} [{}] for {}g? Press Enter again to confirm.",
                        w.display_name(),
                        w.rarity,
                        sell_price(w.rarity)
                    )
                })
                .unwrap_or_default(),
            ShopTab::Armor => self
                .inventory
                .armors
                .get(idx)
                .map(|a| {
                    format!(
                        "Sell {} [{}] for {}g? Press Enter again to confirm.",
                        a.name,
                        a.rarity,
                        sell_price(a.rarity)
                    )
                })
                .unwrap_or_default(),
            ShopTab::Rings => self
                .inventory
                .rings
                .get(idx)
                .map(|r| {
                    format!(
                        "Sell {} [{}] for {}g? Press Enter again to confirm.",
                        r.name,
                        r.rarity,
                        sell_price(r.rarity)
                    )
                })
                .unwrap_or_default(),
        }
    }

    /// Sells every Common-rarity spare weapon/armor/ring in one go — the
    /// smallest useful slice of bulk gear management: Common is the tier
    /// players accumulate fastest and care about least individually, and it
    /// never needs `needs_sell_confirm`'s confirmation step.
    fn apply_shop_sell_all_common(&mut self) {
        let mut gold_gained = 0u32;
        let mut count = 0u32;
        self.inventory.weapons.retain(|w| {
            if w.rarity == Rarity::Common {
                gold_gained += sell_price(w.rarity);
                count += 1;
                false
            } else {
                true
            }
        });
        self.inventory.armors.retain(|a| {
            if a.rarity == Rarity::Common {
                gold_gained += sell_price(a.rarity);
                count += 1;
                false
            } else {
                true
            }
        });
        self.inventory.rings.retain(|r| {
            if r.rarity == Rarity::Common {
                gold_gained += sell_price(r.rarity);
                count += 1;
                false
            } else {
                true
            }
        });
        self.party.gold += gold_gained;
        let items_len = self.inventory.items.len();
        let weapons_len = self.inventory.weapons.len();
        let armors_len = self.inventory.armors.len();
        let rings_len = self.inventory.rings.len();
        if let GameState::Shop(shop) = &mut self.state {
            shop.message = Some(if count == 0 {
                "No spare Common gear to sell.".to_string()
            } else {
                format!("Sold {count} Common item(s) for {gold_gained} gold.")
            });
            shop.pending_sell = None;
            shop.cursor = match shop.tab {
                ShopTab::Items => shop.cursor.min(items_len.saturating_sub(1)),
                ShopTab::Weapons => shop.cursor.min(weapons_len.saturating_sub(1)),
                ShopTab::Armor => shop.cursor.min(armors_len.saturating_sub(1)),
                ShopTab::Rings => shop.cursor.min(rings_len.saturating_sub(1)),
            };
        }
    }

    fn shop_list_len(&self, mode: ShopMode, tab: ShopTab) -> usize {
        match mode {
            ShopMode::Buy => match tab {
                ShopTab::Items => shop_item_stock(self.current_chapter).len(),
                ShopTab::Weapons => shop_weapon_stock(self.current_chapter).len(),
                ShopTab::Armor => shop_armor_stock(self.current_chapter).len(),
                ShopTab::Rings => shop_ring_stock(self.current_chapter).len(),
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
            ShopTab::Items => match shop_item_stock(self.current_chapter).get(idx) {
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
            ShopTab::Weapons => match shop_weapon_stock(self.current_chapter).get(idx) {
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
            ShopTab::Armor => match shop_armor_stock(self.current_chapter).get(idx) {
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
            ShopTab::Rings => match shop_ring_stock(self.current_chapter).get(idx) {
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
                    let price = crate::game::shop::sell_price(weapon.rarity);
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
                    let price = crate::game::shop::sell_price(armor.rarity);
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
                    let price = crate::game::shop::sell_price(ring.rarity);
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
            shop.pending_sell = None;
            shop.cursor = match tab {
                ShopTab::Items => shop.cursor.min(items_len.saturating_sub(1)),
                ShopTab::Weapons => shop.cursor.min(weapons_len.saturating_sub(1)),
                ShopTab::Armor => shop.cursor.min(armors_len.saturating_sub(1)),
                ShopTab::Rings => shop.cursor.min(rings_len.saturating_sub(1)),
            };
        }
    }

    fn handle_levelup_key(&mut self, key: Key) {
        let GameState::LevelUp(ui) = &self.state else {
            return;
        };
        let member_cursor = ui.member_cursor;
        let stat_cursor = ui.stat_cursor;

        match key {
            Key::Esc => {
                let GameState::LevelUp(ui) = &self.state else {
                    return;
                };
                let return_pos = ui.return_pos;
                let mut explore = ExploreState::for_chapter(self.current_chapter);
                explore.player_pos = return_pos;
                self.state = GameState::Explore(explore);
            }
            Key::Up | Key::Char('w') => {
                let len = self.party.members.len();
                if len > 0 {
                    let GameState::LevelUp(ui) = &mut self.state else {
                        return;
                    };
                    ui.member_cursor = (member_cursor + len - 1) % len;
                }
            }
            Key::Down | Key::Char('s') => {
                let len = self.party.members.len();
                if len > 0 {
                    let GameState::LevelUp(ui) = &mut self.state else {
                        return;
                    };
                    ui.member_cursor = (member_cursor + 1) % len;
                }
            }
            Key::Left | Key::Char('a') => {
                let len = ALLOC_STATS.len();
                let GameState::LevelUp(ui) = &mut self.state else {
                    return;
                };
                ui.stat_cursor = (stat_cursor + len - 1) % len;
            }
            Key::Right | Key::Char('d') => {
                let len = ALLOC_STATS.len();
                let GameState::LevelUp(ui) = &mut self.state else {
                    return;
                };
                ui.stat_cursor = (stat_cursor + 1) % len;
            }
            Key::Enter => self.apply_levelup_allocation(member_cursor, stat_cursor),
            Key::Char('f') => self.apply_levelup_fill(member_cursor, stat_cursor),
            Key::Backspace => self.apply_levelup_undo(),
            _ => {}
        }
    }

    fn apply_levelup_allocation(&mut self, member_idx: usize, stat_idx: usize) {
        let Some(stat) = ALLOC_STATS.get(stat_idx).copied() else {
            return;
        };
        let result = self
            .party
            .members
            .get_mut(member_idx)
            .map(|member| (member.allocate_point_tracked(stat), member.name.clone(), member.unspent_points));
        if let GameState::LevelUp(ui) = &mut self.state {
            if let Some((gain, name, left)) = result {
                ui.message = Some(match gain {
                    Some(gain) => {
                        ui.history.push((member_idx, stat, gain));
                        format!("{name} spends a point on {stat}. ({left} left)")
                    }
                    None => "No points left to spend.".to_string(),
                });
            }
        }
    }

    /// Spends every banked point on `stat` in one go. Since `allocate_point`
    /// can no longer be refused for any reason but zero points banked, this
    /// loop is guaranteed to end with `unspent_points == 0`. Pushes one
    /// history entry per point spent (not one for the whole fill), so
    /// Backspace always rewinds exactly one point regardless of how it was
    /// spent.
    fn apply_levelup_fill(&mut self, member_idx: usize, stat_idx: usize) {
        let Some(stat) = ALLOC_STATS.get(stat_idx).copied() else {
            return;
        };
        let result = self.party.members.get_mut(member_idx).map(|member| {
            let mut gains = Vec::new();
            while let Some(gain) = member.allocate_point_tracked(stat) {
                gains.push(gain);
            }
            (gains, member.name.clone())
        });
        if let GameState::LevelUp(ui) = &mut self.state {
            if let Some((gains, name)) = result {
                ui.message = Some(if gains.is_empty() {
                    "No points left to spend.".to_string()
                } else {
                    let spent = gains.len();
                    for gain in gains {
                        ui.history.push((member_idx, stat, gain));
                    }
                    format!("{name} spends {spent} point(s) on {stat}.")
                });
            }
        }
    }

    /// Undoes the most recent point spent during this visit to the level-up
    /// screen, if any. Scoped to `LevelUpUiState::history`, which is empty
    /// again the next time the screen is opened.
    fn apply_levelup_undo(&mut self) {
        let Some((member_idx, stat, gain)) = (if let GameState::LevelUp(ui) = &mut self.state {
            ui.history.pop()
        } else {
            None
        }) else {
            return;
        };
        let name = self.party.members.get_mut(member_idx).map(|member| {
            member.deallocate_point(stat, gain);
            member.name.clone()
        });
        if let GameState::LevelUp(ui) = &mut self.state {
            ui.message = Some(match name {
                Some(name) => format!("Undid {name}'s point on {stat}."),
                None => "Nothing to undo.".to_string(),
            });
        }
    }

    fn handle_blacksmith_key(&mut self, key: Key) {
        let GameState::Blacksmith(bs) = &self.state else {
            return;
        };
        let cursor = bs.cursor;

        match key {
            Key::Esc => {
                let GameState::Blacksmith(bs) = &self.state else {
                    return;
                };
                let return_pos = bs.return_pos;
                let mut explore = ExploreState::for_chapter(self.current_chapter);
                explore.player_pos = return_pos;
                self.state = GameState::Explore(explore);
            }
            Key::Up | Key::Char('w') => {
                let len = weapon_refs(&self.inventory, &self.party).len();
                if len > 0 {
                    let GameState::Blacksmith(bs) = &mut self.state else {
                        return;
                    };
                    bs.cursor = (cursor + len - 1) % len;
                }
            }
            Key::Down | Key::Char('s') => {
                let len = weapon_refs(&self.inventory, &self.party).len();
                if len > 0 {
                    let GameState::Blacksmith(bs) = &mut self.state else {
                        return;
                    };
                    bs.cursor = (cursor + 1) % len;
                }
            }
            Key::Enter => self.apply_blacksmith_upgrade(cursor),
            Key::Char('b') | Key::Char('B') => self.buy_titanite_shard(),
            _ => {}
        }
    }

    /// Andre only stocks Titanite Shards once the party has reached chapter
    /// three — before that, upgrade materials come solely from drops/finds.
    fn buy_titanite_shard(&mut self) {
        let message = if self.current_chapter != ChapterId::Three {
            "Andre has no Titanite Shards to sell yet.".to_string()
        } else if self.party.gold < SHARD_PRICE {
            format!("Not enough gold ({SHARD_PRICE}g needed for a Titanite Shard).")
        } else {
            self.party.gold -= SHARD_PRICE;
            self.inventory.upgrade_materials += 1;
            format!("Bought a Titanite Shard for {SHARD_PRICE}g.")
        };
        if let GameState::Blacksmith(bs) = &mut self.state {
            bs.message = Some(message);
        }
    }

    fn apply_blacksmith_upgrade(&mut self, idx: usize) {
        let refs = weapon_refs(&self.inventory, &self.party);
        let Some(&r) = refs.get(idx) else { return };
        let (rarity, tier) = match weapon_for_mut(r, &mut self.inventory, &mut self.party) {
            Some(weapon) => (weapon.rarity, weapon.upgrade_level),
            None => return,
        };
        let message = match upgrade_cost(rarity, tier) {
            None => "Already at max upgrade level.".to_string(),
            Some((gold, shards)) => {
                if self.party.gold < gold || self.inventory.upgrade_materials < shards {
                    "Not enough gold or Titanite Shards for that upgrade.".to_string()
                } else {
                    self.party.gold -= gold;
                    self.inventory.upgrade_materials -= shards;
                    let (atk_inc, def_inc) = upgrade_increments(rarity);
                    let weapon = weapon_for_mut(r, &mut self.inventory, &mut self.party)
                        .expect("weapon_refs stays in sync");
                    weapon.apply_upgrade(atk_inc, def_inc);
                    format!(
                        "{} upgraded to +{}!",
                        weapon.display_name(),
                        weapon.upgrade_level
                    )
                }
            }
        };
        // A weapon that just hit MAX_UPGRADE_LEVEL drops out of the next
        // weapon_refs() call, which can leave the cursor pointing past the
        // now-shorter list — clamp it back into range.
        let max_idx = weapon_refs(&self.inventory, &self.party)
            .len()
            .saturating_sub(1);
        if let GameState::Blacksmith(bs) = &mut self.state {
            bs.cursor = bs.cursor.min(max_idx);
            bs.message = Some(message);
        }
    }

    /// Called once per rendered frame; advances enemy turns automatically
    /// since they don't wait on keyboard input, steps the sprite-animation
    /// clock by the frame's real elapsed time, and counts down a resolving
    /// turn's animation hold (see `CombatState::resolving_timer`).
    pub fn tick(&mut self, dt: f32) {
        self.anim_timer += dt;
        if self.anim_timer > ANIM_TIMER_WRAP {
            self.anim_timer -= ANIM_TIMER_WRAP;
        }
        if let GameState::Explore(explore) = &mut self.state {
            if explore.step_elapsed < STEP_ANIM_SECONDS {
                explore.step_elapsed = (explore.step_elapsed + dt).min(STEP_ANIM_SECONDS);
            }
        }
        if let GameState::Combat(combat) = &mut self.state {
            for n in &mut combat.floating_numbers {
                n.age += dt;
            }
            combat.floating_numbers.retain(|n| n.age < FLOATING_NUMBER_TTL);
            if combat.resolving_timer > 0.0 {
                combat.resolving_timer -= dt;
                if combat.resolving_timer <= 0.0 {
                    combat.resolving_timer = 0.0;
                    if let Some(phase) = combat.pending_phase.take() {
                        combat.phase = phase;
                    }
                }
            } else if let CombatPhase::SelectAction {
                actor: ActorRef::Enemy(_),
            } = combat.phase
            {
                combat.resolve_current_turn(&mut self.party, &mut self.rng);
                begin_resolving_hold(combat);
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
            let mut game_won = false;
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
                        game_won = true;
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
                for armor in loot.armors {
                    messages.push(format!(
                        "The enemy dropped a {} rarity armor: {}!",
                        armor.rarity, armor.name
                    ));
                    self.inventory.add_armor(armor);
                }
                for ring in loot.rings {
                    messages.push(format!(
                        "The enemy dropped a {} rarity ring: {}!",
                        ring.rarity, ring.name
                    ));
                    self.inventory.add_ring(ring);
                }
                if loot.upgrade_materials > 0 {
                    self.inventory.upgrade_materials += loot.upgrade_materials;
                    messages.push(format!(
                        "The enemy dropped {} Titanite Shard(s).",
                        loot.upgrade_materials
                    ));
                }
                if loot.xp > 0 {
                    for member in self.party.members.iter_mut() {
                        let levels = member.gain_xp(loot.xp);
                        if levels > 0 {
                            messages.push(format!(
                                "{} reaches level {}! ({} stat point{} to spend — press 'u')",
                                member.name,
                                member.level,
                                member.unspent_points,
                                if member.unspent_points == 1 { "" } else { "s" }
                            ));
                        }
                    }
                }
            }
            if messages.is_empty() {
                messages.push("The enemies dropped nothing.".to_string());
            }
            if game_won {
                self.state = GameState::GameOver { victory: true };
                return;
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
fn put_slot(
    member: &mut Character,
    slot: EquipSlot,
    piece: Option<GearPiece>,
) -> Option<GearPiece> {
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
    EQUIP_SLOTS.iter().position(|&s| s == slot).unwrap_or(0)
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
    is_revive: bool,
    current: usize,
    dir: i32,
) -> usize {
    let candidates: Vec<usize> = if is_revive {
        // A member revived mid-fight isn't in the turn order (it was built
        // at combat start), so they won't act until the next encounter —
        // but they're back on their feet and healable.
        party
            .members
            .iter()
            .enumerate()
            .filter(|(_, c)| !c.is_alive())
            .map(|(i, _)| i)
            .collect()
    } else if is_heal {
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
    let pos = candidates.iter().position(|&i| i == current).unwrap_or(0) as i32;
    let len = candidates.len() as i32;
    let new_pos = ((pos + dir) % len + len) % len;
    candidates[new_pos as usize]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::item::{copper_band, iron_sword, padded_vest, ring_of_vigor};

    /// Puts a fresh world on the title screen without touching the
    /// filesystem — `at_main_menu()` would read the real save file.
    fn world_at_menu(slots: [Option<crate::game::save::SaveData>; 3]) -> World {
        let mut world = World::new();
        world.state = GameState::MainMenu(MainMenuState::new(slots));
        world
    }

    fn empty_slots() -> [Option<crate::game::save::SaveData>; 3] {
        [None, None, None]
    }

    /// A minimal but valid occupied-slot fixture, built through the same
    /// `to_save` production code path other save/load tests use rather than
    /// hand-rolling a `SaveData` literal.
    fn occupied_slot() -> Option<crate::game::save::SaveData> {
        let mut world = World::new();
        world.party.gold = 555;
        Some(world.to_save(Position { x: 1, y: 1 }))
    }

    fn menu(world: &World) -> &MainMenuState {
        let GameState::MainMenu(menu) = &world.state else {
            panic!("expected the world to be on the main menu");
        };
        menu
    }

    #[test]
    fn menu_starts_with_cursor_on_the_first_slot() {
        let world = world_at_menu(empty_slots());
        assert_eq!(menu(&world).cursor, 0);
        assert!(menu(&world).slots.iter().all(|s| s.is_none()));
    }

    #[test]
    fn menu_navigation_wraps_across_four_rows_and_enter_on_quit_quits() {
        let mut world = world_at_menu(empty_slots());
        world.handle_key(Key::Up); // wraps from Slot 1 to Quit
        assert_eq!(menu(&world).cursor, 3);
        world.handle_key(Key::Down); // wraps back to Slot 1
        assert_eq!(menu(&world).cursor, 0);
        world.handle_key(Key::Down);
        world.handle_key(Key::Down);
        world.handle_key(Key::Down); // Quit
        assert_eq!(menu(&world).cursor, 3);
        world.handle_key(Key::Enter);
        assert!(world.should_quit);
    }

    #[test]
    fn enter_on_an_empty_slot_starts_a_fresh_game_there() {
        let mut world = world_at_menu(empty_slots());
        world.handle_key(Key::Down); // Slot 1 -> Slot 2
        world.handle_key(Key::Enter);
        assert_eq!(
            menu(&world).pending_new_game,
            Some(2),
            "picking an empty slot should ask for a difficulty first"
        );
        world.handle_key(Key::Enter); // confirm Normal
        assert!(matches!(world.state, GameState::Explore(_)));
        assert_eq!(world.active_slot, Some(2));
        assert_eq!(world.difficulty, 0);
    }

    #[test]
    fn enter_on_an_occupied_slot_loads_it() {
        let mut slots = empty_slots();
        slots[0] = occupied_slot();
        let mut world = world_at_menu(slots);
        world.handle_key(Key::Enter); // cursor starts on Slot 1
        assert!(matches!(world.state, GameState::Explore(_)));
        assert_eq!(world.active_slot, Some(1));
        assert_eq!(world.party.gold, 555, "should load that slot's save data");
    }

    #[test]
    fn n_on_an_occupied_slot_asks_before_overwriting() {
        let mut slots = empty_slots();
        slots[0] = occupied_slot();
        let mut world = world_at_menu(slots);
        world.handle_key(Key::Char('n'));
        assert_eq!(
            menu(&world).confirm_overwrite,
            Some(1),
            "should ask, not start"
        );

        world.handle_key(Key::Esc); // turn back
        assert_eq!(menu(&world).confirm_overwrite, None);
        assert!(
            matches!(world.state, GameState::MainMenu(_)),
            "backing out should stay on the menu"
        );
        assert_eq!(
            menu(&world).slots[0].as_ref().map(|d| d.party.gold),
            Some(555),
            "backing out must not consume the slot's data"
        );

        world.handle_key(Key::Char('n')); // force New Game again
        world.handle_key(Key::Enter); // confirm this time -> difficulty picker
        assert_eq!(menu(&world).pending_new_game, Some(1));
        world.handle_key(Key::Enter); // pick Normal
        assert!(matches!(world.state, GameState::Explore(_)));
        assert_eq!(world.active_slot, Some(1));
        assert_ne!(
            world.party.gold, 555,
            "a fresh game shouldn't keep the old save's gold"
        );
    }

    #[test]
    fn n_on_an_empty_slot_starts_fresh_without_confirming() {
        let mut world = world_at_menu(empty_slots());
        world.handle_key(Key::Char('n'));
        assert_eq!(
            menu(&world).pending_new_game,
            Some(1),
            "no overwrite confirm needed, straight to the difficulty picker"
        );
        world.handle_key(Key::Enter); // pick Normal
        assert!(matches!(world.state, GameState::Explore(_)));
        assert_eq!(world.active_slot, Some(1));
    }

    #[test]
    fn picking_accursed_starts_at_difficulty_two_without_touching_ng_plus() {
        let mut world = world_at_menu(empty_slots());
        world.handle_key(Key::Enter); // empty Slot 1 -> difficulty picker
        world.handle_key(Key::Down); // Normal -> Accursed
        world.handle_key(Key::Enter);
        assert!(matches!(world.state, GameState::Explore(_)));
        assert_eq!(world.difficulty, 2);
        assert_eq!(world.ng_plus, 0, "difficulty must not masquerade as a story cycle");
        assert_eq!(world.challenge_cycle(), 2);
    }

    #[test]
    fn esc_backs_out_of_the_difficulty_picker_without_starting() {
        let mut world = world_at_menu(empty_slots());
        world.handle_key(Key::Enter);
        assert_eq!(menu(&world).pending_new_game, Some(1));
        world.handle_key(Key::Down); // move to Accursed, then bail
        world.handle_key(Key::Esc);
        assert!(matches!(world.state, GameState::MainMenu(_)));
        assert_eq!(menu(&world).pending_new_game, None);
        assert_eq!(
            menu(&world).difficulty_cursor,
            0,
            "the picker should reset for the next visit"
        );
    }

    #[test]
    fn challenge_cycle_composes_ng_plus_and_difficulty_capped_at_seven() {
        let mut world = World::new();
        world.difficulty = 2;
        assert_eq!(world.challenge_cycle(), 2);
        world.ng_plus = 3;
        assert_eq!(world.challenge_cycle(), 5);
        world.ng_plus = 7;
        assert_eq!(world.challenge_cycle(), 7, "the NG+7 cap holds");
    }

    #[test]
    fn new_game_plus_preserves_the_chosen_difficulty() {
        let mut world = World::new();
        world.difficulty = 2;
        world.start_new_game_plus();
        assert_eq!(world.ng_plus, 1);
        assert_eq!(world.difficulty, 2, "the burden carries across cycles");
    }

    #[test]
    fn a_save_round_trip_preserves_difficulty() {
        let mut world = World::new();
        world.difficulty = 2;
        let json = serde_json::to_string(&world.to_save(Position { x: 1, y: 1 }))
            .expect("save should serialize");
        let restored =
            World::from_save(serde_json::from_str(&json).expect("save should parse"), 1);
        assert_eq!(restored.difficulty, 2);
    }

    #[test]
    fn q_on_the_menu_quits() {
        let mut world = world_at_menu(empty_slots());
        world.handle_key(Key::Char('q'));
        assert!(world.should_quit);
    }

    #[test]
    fn i_opens_the_inventory_and_esc_returns_to_explore() {
        let mut world = World::new();
        assert!(matches!(world.state, GameState::Explore(_)));
        world.handle_key(Key::Char('i'));
        assert!(matches!(world.state, GameState::Inventory(_)));
        world.handle_key(Key::Esc);
        assert!(matches!(world.state, GameState::Explore(_)));
    }

    #[test]
    fn u_opens_levelup_and_backspace_undoes_the_last_allocation() {
        let mut world = World::new();
        world.party.members[0].unspent_points = 3;
        let base_hp = world.party.members[0].stats.max_hp;

        world.handle_key(Key::Char('u'));
        assert!(matches!(world.state, GameState::LevelUp(_)));

        // stat_cursor starts on the first ALLOC_STATS entry, Max HP.
        world.handle_key(Key::Enter);
        let after_spend = world.party.members[0].stats.max_hp;
        assert!(after_spend > base_hp, "spending a point should grow max HP");
        assert_eq!(world.party.members[0].unspent_points, 2);

        world.handle_key(Key::Backspace);
        assert_eq!(
            world.party.members[0].stats.max_hp, base_hp,
            "undo should restore the exact stat value"
        );
        assert_eq!(
            world.party.members[0].unspent_points, 3,
            "undo should refund the spent point"
        );
    }

    #[test]
    fn backspace_undoes_one_point_at_a_time_after_a_fill() {
        let mut world = World::new();
        world.party.members[0].unspent_points = 3;
        let base_hp = world.party.members[0].stats.max_hp;

        world.handle_key(Key::Char('u'));
        world.handle_key(Key::Char('f')); // spend all 3 banked points on Max HP
        assert_eq!(world.party.members[0].unspent_points, 0);
        let filled_hp = world.party.members[0].stats.max_hp;
        assert!(filled_hp > base_hp);

        world.handle_key(Key::Backspace);
        assert_eq!(world.party.members[0].unspent_points, 1);
        assert!(
            world.party.members[0].stats.max_hp < filled_hp,
            "one undo should only rewind one point"
        );

        world.handle_key(Key::Backspace);
        world.handle_key(Key::Backspace);
        assert_eq!(world.party.members[0].unspent_points, 3);
        assert_eq!(
            world.party.members[0].stats.max_hp, base_hp,
            "undoing every point returns to the pre-fill baseline"
        );
    }

    #[test]
    fn backspace_with_no_history_does_nothing() {
        let mut world = World::new();
        world.handle_key(Key::Char('u'));
        world.handle_key(Key::Backspace);
        assert!(
            matches!(world.state, GameState::LevelUp(_)),
            "an empty undo history should just be a no-op, not crash or exit the screen"
        );
    }

    #[test]
    fn equipping_a_weapon_swaps_it_with_the_previous_one() {
        let mut world = World::new();
        world.inventory.add_weapon(iron_sword());

        world.handle_key(Key::Char('i')); // open inventory (Items tab)
        world.handle_key(Key::Tab); // switch to Weapons tab
        world.handle_key(Key::Enter); // pick the Iron Sword -> choose member (Bram, cursor 0)

        let previous_weapon = world.party.members[0]
            .equipped_weapon
            .as_ref()
            .map(|w| w.name.clone());

        world.handle_key(Key::Enter); // confirm equip on Bram

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

        world.handle_key(Key::Char('i')); // open inventory (Items tab, Potion first)
        world.handle_key(Key::Enter); // pick the potion -> choose member (Bram, cursor 0)
        world.handle_key(Key::Enter); // confirm use on Bram

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

        world.handle_key(Key::Enter); // open the item submenu (starts on Potion)
        world.handle_key(Key::Down); // move to Ether
        world.handle_key(Key::Enter); // pick Ether, target self by default
        world.handle_key(Key::Enter); // confirm target

        assert_eq!(
            world.party.members[0].stats.mp, 4,
            "picking Ether from the submenu should restore MP, not HP (35% of 10 max MP, rounded)"
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
        world.handle_key(Key::Char('e'));
        assert!(matches!(world.state, GameState::Shop(_)));
        world.handle_key(Key::Esc);
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

        world.handle_key(Key::Char('e')); // Buy tab, Items tab, cursor on Potion (15 gold)
        world.handle_key(Key::Enter);

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

        world.handle_key(Key::Char('e'));
        world.handle_key(Key::Tab); // Weapons tab, cursor on Iron Sword
        world.handle_key(Key::Enter);

        assert_eq!(
            world.party.gold, 5,
            "gold should be untouched on a failed buy"
        );
        assert!(world.inventory.weapons.is_empty());
    }

    #[test]
    fn selling_a_spare_weapon_grants_gold_and_removes_it() {
        let mut world = World::new();
        world.inventory.add_weapon(iron_sword());
        let gold_before = world.party.gold;
        let expected_price = iron_sword().rarity.base_value() / 2;

        world.handle_key(Key::Char('e')); // Buy/Items
        world.handle_key(Key::Left); // -> Sell/Items
        world.handle_key(Key::Tab); // -> Sell/Weapons, cursor on the Iron Sword
        world.handle_key(Key::Enter); // sell it

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
        world.handle_key(Key::Down); // step onto the lair at (26, 8)

        let GameState::Combat(combat) = &world.state else {
            panic!("expected the boss fight to start");
        };
        assert!(combat.enemies.iter().any(|e| e.name == "The Barrow Knight"));
    }

    #[test]
    fn starting_a_fight_records_the_enemy_in_the_bestiary() {
        let mut world = World::new();
        assert!(world.bestiary_seen.is_empty());
        if let GameState::Explore(explore) = &mut world.state {
            explore.player_pos = Position { x: 26, y: 7 };
        }
        world.handle_key(Key::Down); // step onto the boss lair

        assert!(matches!(world.state, GameState::Combat(_)));
        assert!(
            world.bestiary_seen.contains("The Barrow Knight"),
            "seeing the fight (not winning it) should unlock the codex entry"
        );
    }

    #[test]
    fn b_opens_the_bestiary_and_esc_returns_to_the_same_tile() {
        let mut world = World::new();
        let pos = Position { x: 3, y: 2 };
        if let GameState::Explore(explore) = &mut world.state {
            explore.player_pos = pos;
        }
        world.handle_key(Key::Char('b'));
        let GameState::Bestiary(ui) = &world.state else {
            panic!("expected 'b' to open the bestiary");
        };
        assert_eq!(ui.return_pos, pos);

        world.handle_key(Key::Down);
        world.handle_key(Key::Esc);
        let GameState::Explore(explore) = &world.state else {
            panic!("expected Esc to return to Explore");
        };
        assert_eq!(explore.player_pos, pos, "the player should stand where they left off");
    }

    #[test]
    fn new_game_plus_preserves_the_bestiary() {
        let mut world = World::new();
        world.bestiary_seen.insert("Slime".to_string());
        world.start_new_game_plus();
        assert!(
            world.bestiary_seen.contains("Slime"),
            "the codex chronicles the whole journey, not one cycle"
        );
    }

    #[test]
    fn defeating_a_chapter_boss_advances_current_chapter_and_relocates_the_player() {
        let mut world = World::new();
        if let GameState::Explore(explore) = &mut world.state {
            explore.player_pos = Position { x: 26, y: 7 };
        }
        world.handle_key(Key::Down); // enter the boss fight

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
        world.handle_key(Key::Down); // enter the boss fight

        let GameState::Combat(combat) = &mut world.state else {
            panic!("expected the boss fight to start");
        };
        combat.phase = CombatPhase::Victory;
        world.conclude_combat();

        assert!(world.bosses_defeated.contains(&ChapterId::Three));
        assert!(matches!(world.state, GameState::GameOver { victory: true }));
    }

    #[test]
    fn defeating_the_final_boss_grants_its_loot_before_ending_the_game() {
        let mut world = World::new();
        world.current_chapter = ChapterId::Three;
        world.state = GameState::Explore(ExploreState::for_chapter(ChapterId::Three));
        if let GameState::Explore(explore) = &mut world.state {
            explore.player_pos = Position { x: 26, y: 7 };
        }
        world.handle_key(Key::Down); // enter the boss fight

        let GameState::Combat(combat) = &mut world.state else {
            panic!("expected the boss fight to start");
        };
        combat.phase = CombatPhase::Victory;
        combat.loot = Some(crate::game::combat::Loot {
            gold: 500,
            items: Vec::new(),
            weapons: vec![crate::game::item::sovereigns_reckoning()],
            armors: Vec::new(),
            rings: vec![crate::game::item::sovereigns_signet()],
            overkill_bonus: 0,
            xp: 0,
            upgrade_materials: 0,
        });
        let starting_gold = world.party.gold;
        world.conclude_combat();

        assert!(matches!(world.state, GameState::GameOver { victory: true }));
        assert_eq!(world.party.gold, starting_gold + 500);
        assert!(world
            .inventory
            .weapons
            .iter()
            .any(|w| w.name == crate::game::item::sovereigns_reckoning().name));
        assert!(world
            .inventory
            .rings
            .iter()
            .any(|r| r.name == crate::game::item::sovereigns_signet().name));
    }

    #[test]
    fn pressing_n_on_a_victorious_game_over_starts_new_game_plus() {
        let mut world = World::new();
        world.party.gold = 500;
        world.bosses_defeated.insert(ChapterId::One);
        world.bosses_defeated.insert(ChapterId::Two);
        world.bosses_defeated.insert(ChapterId::Three);
        world.npc_flags.insert(NpcId::OldHerbalist);
        world.current_chapter = ChapterId::Three;
        world.state = GameState::GameOver { victory: true };

        world.handle_key(Key::Char('n'));

        assert_eq!(world.ng_plus, 1);
        assert_eq!(world.current_chapter, ChapterId::One);
        assert!(world.bosses_defeated.is_empty());
        assert!(world.npc_flags.is_empty());
        assert_eq!(
            world.party.gold, 500,
            "party progress should carry over into NG+"
        );
        assert!(matches!(world.state, GameState::Explore(_)));
        assert!(!world.should_quit);
    }

    #[test]
    fn pressing_n_on_a_defeat_game_over_does_nothing() {
        let mut world = World::new();
        world.state = GameState::GameOver { victory: false };

        world.handle_key(Key::Char('n'));

        assert_eq!(world.ng_plus, 0);
        assert!(matches!(
            world.state,
            GameState::GameOver { victory: false }
        ));
    }

    #[test]
    fn ng_plus_caps_at_seven_across_repeated_victories() {
        let mut world = World::new();
        for _ in 0..10 {
            world.state = GameState::GameOver { victory: true };
            world.handle_key(Key::Char('n'));
        }
        assert_eq!(world.ng_plus, 7);
    }

    #[test]
    fn andre_sells_titanite_shards_only_in_chapter_three() {
        let mut world = World::new();
        world.current_chapter = ChapterId::One;
        world.party.gold = 1000;
        if let GameState::Explore(explore) = &mut world.state {
            explore.player_pos = chapter_def(ChapterId::One)
                .npcs
                .iter()
                .find(|(_, npc)| *npc == NpcId::Blacksmith)
                .unwrap()
                .0;
        }
        world.handle_key(Key::Char('e'));
        assert!(matches!(world.state, GameState::Blacksmith(_)));
        let shards_before = world.inventory.upgrade_materials;

        world.handle_key(Key::Char('b'));

        assert_eq!(
            world.inventory.upgrade_materials, shards_before,
            "Andre shouldn't sell shards before chapter three"
        );
        assert_eq!(world.party.gold, 1000);
    }

    #[test]
    fn buying_a_titanite_shard_in_chapter_three_spends_gold_and_grants_a_shard() {
        let mut world = World::new();
        world.current_chapter = ChapterId::Three;
        world.party.gold = 1000;
        world.state =
            GameState::Blacksmith(crate::game::blacksmith::BlacksmithUiState::new(Position {
                x: 0,
                y: 0,
            }));
        let shards_before = world.inventory.upgrade_materials;

        world.handle_key(Key::Char('b'));

        assert_eq!(world.party.gold, 1000 - SHARD_PRICE);
        assert_eq!(world.inventory.upgrade_materials, shards_before + 1);
    }

    #[test]
    fn buying_a_titanite_shard_without_enough_gold_changes_nothing() {
        let mut world = World::new();
        world.current_chapter = ChapterId::Three;
        world.party.gold = SHARD_PRICE - 1;
        world.state =
            GameState::Blacksmith(crate::game::blacksmith::BlacksmithUiState::new(Position {
                x: 0,
                y: 0,
            }));
        let shards_before = world.inventory.upgrade_materials;

        world.handle_key(Key::Char('b'));

        assert_eq!(world.party.gold, SHARD_PRICE - 1);
        assert_eq!(world.inventory.upgrade_materials, shards_before);
    }

    #[test]
    fn cursor_clamps_after_the_last_listed_weapon_reaches_max_upgrade_level() {
        use crate::game::character::warrior;
        use crate::game::item::{iron_sword, MAX_UPGRADE_LEVEL};

        let mut world = World::new();
        world.party = Party::new(vec![warrior("Bram")]); // single member, single equipped weapon
        world.inventory.add_weapon(iron_sword()); // Bag(0), left untouched
        world.party.members[0]
            .equipped_weapon
            .as_mut()
            .unwrap()
            .upgrade_level = MAX_UPGRADE_LEVEL - 1;
        world.party.gold = 10_000;
        world.inventory.upgrade_materials = 100;
        world.state =
            GameState::Blacksmith(crate::game::blacksmith::BlacksmithUiState::new(Position {
                x: 0,
                y: 0,
            }));
        // refs = [Bag(0), Equipped(0)]; select the equipped weapon (last row).
        if let GameState::Blacksmith(bs) = &mut world.state {
            bs.cursor = 1;
        }

        world.handle_key(Key::Enter);

        assert_eq!(
            world.party.members[0]
                .equipped_weapon
                .as_ref()
                .unwrap()
                .upgrade_level,
            MAX_UPGRADE_LEVEL
        );
        // Equipped(0) just dropped out of the list, shrinking it to just
        // Bag(0) — the stale cursor=1 must be clamped back into range.
        let GameState::Blacksmith(bs) = &world.state else {
            panic!("still in blacksmith state");
        };
        assert_eq!(bs.cursor, 0);
    }

    #[test]
    fn equipping_armor_from_the_bag_works_from_the_armor_tab() {
        let mut world = World::new();
        world.inventory.add_armor(padded_vest());

        world.handle_key(Key::Char('i')); // Items tab
        world.handle_key(Key::Tab); // Weapons
        world.handle_key(Key::Tab); // Armor
        world.handle_key(Key::Enter); // pick the Padded Vest -> choose member (Bram)
        world.handle_key(Key::Enter); // confirm equip on Bram

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

        world.handle_key(Key::Char('i')); // Items
        world.handle_key(Key::Tab); // Weapons
        world.handle_key(Key::Tab); // Armor
        world.handle_key(Key::Tab); // Rings
        world.handle_key(Key::Enter); // pick the Copper Band -> choose member (Bram)
        world.handle_key(Key::Enter); // confirm member -> now choose ring slot (First)
        world.handle_key(Key::Enter); // confirm First slot

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

        world.handle_key(Key::Char('i'));
        world.handle_key(Key::Tab);
        world.handle_key(Key::Tab);
        world.handle_key(Key::Tab); // Rings tab
        world.handle_key(Key::Enter); // pick ring -> choose member (Bram)
        world.handle_key(Key::Enter); // confirm member -> choose ring slot
        world.handle_key(Key::Down); // move to Second slot
        world.handle_key(Key::Enter); // confirm Second slot

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

        world.handle_key(Key::Char('i')); // open inventory (Browsing)
        world.handle_key(Key::Char('p')); // PartyGear, member 0 (Bram), slot 0 (Weapon)
        world.handle_key(Key::Enter); // -> PartyGearAction, action_cursor 0 ("Unequip to bag")
        world.handle_key(Key::Enter); // confirm unequip

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

        world.handle_key(Key::Char('i')); // open inventory (Browsing)
        world.handle_key(Key::Char('p')); // PartyGear, member 0 (Bram), slot 0 (Weapon)
        world.handle_key(Key::Enter); // -> PartyGearAction, action_cursor 0
        world.handle_key(Key::Down); // -> action_cursor 1 ("Move to another member")
        world.handle_key(Key::Enter); // -> PartyGearTarget, defaults to member 1 (Sella)
        world.handle_key(Key::Enter); // confirm the swap

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

        world.handle_key(Key::Char('i')); // open inventory
        world.handle_key(Key::Char('p')); // PartyGear, member 0, slot Weapon
        world.handle_key(Key::Enter); // -> PartyGearAction
        world.handle_key(Key::Enter); // Unequip to bag

        assert!(world
            .inventory
            .weapons
            .iter()
            .any(|w| w.name == "Iron Sword"));

        world.handle_key(Key::Esc); // back to PartyGear
        world.handle_key(Key::Esc); // back to Explore

        world.handle_key(Key::Char('e')); // open the shop (standing in town)
        world.handle_key(Key::Left); // Buy -> Sell
        world.handle_key(Key::Tab); // Items -> Weapons tab
                                        // Cursor starts on whichever spare weapon sorts first; find the Iron Sword
                                        // by pressing Down until it's selected, bounded by the bag's length.
        let idx = world
            .inventory
            .weapons
            .iter()
            .position(|w| w.name == "Iron Sword")
            .expect("iron sword should be in the bag");
        for _ in 0..idx {
            world.handle_key(Key::Down);
        }
        world.handle_key(Key::Enter); // sell it

        assert_eq!(world.party.gold, gold_before + expected_price);
        assert!(
            !world
                .inventory
                .weapons
                .iter()
                .any(|w| w.name == "Iron Sword"),
            "the sold weapon should leave the bag"
        );
    }

    #[test]
    fn the_starting_party_includes_the_rogue() {
        use crate::game::character::Class;
        let world = World::new();
        assert_eq!(world.party.members.len(), 4);
        assert!(
            world.party.members.iter().any(|m| m.class == Class::Rogue),
            "the rogue should be part of the starting party"
        );
    }

    #[test]
    fn saving_and_restoring_a_world_round_trips_its_progress() {
        let mut world = World::new();
        world.party.gold = 777;
        world.current_chapter = ChapterId::Two;
        world.bosses_defeated.insert(ChapterId::One);
        world.npc_flags.insert(NpcId::OldHerbalist);
        world
            .quest_log
            .accept(crate::game::quest::QuestId::ScoutsCommendation);
        world.party.members[0].gain_xp(60); // level 2, growth applied
        world.inventory.add_weapon(iron_sword());
        let pos = Position { x: 5, y: 6 };

        // Round-trip through the actual JSON representation, not just the
        // in-memory structs, since that's what the save file stores.
        let json = serde_json::to_string(&world.to_save(pos)).expect("save should serialize");
        let restored =
            World::from_save(serde_json::from_str(&json).expect("save should parse"), 1);

        assert_eq!(restored.party.gold, 777);
        assert_eq!(restored.current_chapter, ChapterId::Two);
        assert!(restored.bosses_defeated.contains(&ChapterId::One));
        assert!(restored.npc_flags.contains(&NpcId::OldHerbalist));
        assert!(restored
            .quest_log
            .is_active(crate::game::quest::QuestId::ScoutsCommendation));
        assert_eq!(restored.party.members[0].level, 2);
        assert_eq!(
            restored.party.members[0].stats.max_hp,
            world.party.members[0].stats.max_hp
        );
        assert!(restored
            .inventory
            .weapons
            .iter()
            .any(|w| w.name == "Iron Sword"));
        let GameState::Explore(explore) = &restored.state else {
            panic!("a loaded game should resume in Explore");
        };
        assert_eq!(
            explore.player_pos, pos,
            "the player should resume where they saved"
        );
        assert_eq!(
            explore.map.tile_at(Position { x: 26, y: 1 }),
            Tile::BossLair,
            "the loaded map should be chapter two's"
        );
    }

    #[test]
    fn loading_an_old_save_backfills_newly_added_class_abilities() {
        let world = World::new();
        let mut data = world.to_save(Position { x: 1, y: 1 });
        // Simulate a save written before every class gained its 3rd ability.
        for member in &mut data.party.members {
            member.abilities.truncate(2);
        }
        let json = serde_json::to_string(&data).expect("save should serialize");
        let restored = World::from_save(serde_json::from_str(&json).expect("save should parse"), 1);
        for member in &restored.party.members {
            assert_eq!(
                member.abilities.len(),
                3,
                "{} should have been resynced to the full class kit",
                member.name
            );
        }
    }

    #[test]
    fn e_on_an_npc_opens_dialogue_with_the_npc_set() {
        let mut world = World::new();
        if let GameState::Explore(explore) = &mut world.state {
            explore.player_pos = Position { x: 12, y: 5 }; // the Old Herbalist's spot
        }
        world.handle_key(Key::Char('e'));

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
        world.handle_key(Key::Char('e'));
        assert!(!world
            .npc_flags
            .contains(&crate::game::npc::NpcId::OldHerbalist));
        world.handle_key(Key::Enter); // dismiss

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
        world.handle_key(Key::Char('e')); // talk again (quest still unsatisfied: only 3/4 Potions)
        let GameState::Event(ev) = &world.state else {
            panic!("expected dialogue to open again");
        };
        let expected: Vec<String> =
            crate::game::npc::npc_def(crate::game::npc::NpcId::OldHerbalist)
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
        world.handle_key(Key::Char('e')); // first talk: accepts the quest and immediately satisfies it

        let GameState::Event(ev) = &world.state else {
            panic!("expected the turn-in dialogue to open");
        };
        assert!(ev.lines.iter().any(|l| l.contains("You received")));

        assert!(world
            .quest_log
            .completed
            .contains(&crate::game::quest::QuestId::HerbalistsRequest));
        assert!(
            world.party.gold > gold_before,
            "the gold reward should be granted"
        );
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
        world.handle_key(Key::Char('e'));

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
        world.handle_key(Key::Char('e'));

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

    #[test]
    fn q_in_explore_asks_before_quitting() {
        let mut world = World::new();
        world.handle_key(Key::Char('q'));
        assert!(!world.should_quit, "should ask first, not quit outright");
        assert!(world.confirm_quit);

        world.handle_key(Key::Esc); // back out
        assert!(!world.confirm_quit);
        assert!(!world.should_quit);

        world.handle_key(Key::Char('q'));
        world.handle_key(Key::Enter); // confirm
        assert!(world.should_quit);
    }

    #[test]
    fn explore_log_is_capped_and_scroll_resets_on_new_lines() {
        let mut world = World::new();
        let GameState::Explore(explore) = &mut world.state else {
            unreachable!()
        };
        for i in 0..250 {
            explore.push_log(format!("line {i}"));
        }
        assert_eq!(explore.log.len(), 200, "log should be capped at 200 lines");
        assert_eq!(explore.log.first().unwrap(), "line 50");

        explore.log_scroll = 5;
        explore.push_log("newest");
        assert_eq!(
            explore.log_scroll, 0,
            "a new line should snap back to the bottom"
        );
    }

    #[test]
    fn page_up_and_down_scroll_the_explore_log_and_clamp() {
        let mut world = World::new();
        let GameState::Explore(explore) = &mut world.state else {
            unreachable!()
        };
        for i in 0..5 {
            explore.push_log(format!("line {i}"));
        }
        let log_len = explore.log.len();

        world.handle_key(Key::PageUp);
        let GameState::Explore(explore) = &world.state else {
            unreachable!()
        };
        assert_eq!(explore.log_scroll, 10.min(log_len));

        // Scrolling further up clamps at the total number of lines, it never
        // goes negative or past the start of history.
        world.handle_key(Key::PageUp);
        world.handle_key(Key::PageUp);
        let GameState::Explore(explore) = &world.state else {
            unreachable!()
        };
        assert_eq!(explore.log_scroll, log_len);

        world.handle_key(Key::PageDown);
        let GameState::Explore(explore) = &world.state else {
            unreachable!()
        };
        assert_eq!(explore.log_scroll, log_len.saturating_sub(10));
    }

    #[test]
    fn attacking_a_lone_enemy_resolves_without_a_second_target_confirm() {
        use crate::game::character::slime;

        let mut world = World::new();
        let combat = CombatState::new(&world.party, vec![slime("Slime")]);
        world.state = GameState::Combat(combat);
        let GameState::Combat(combat) = &mut world.state else {
            unreachable!()
        };
        combat.phase = CombatPhase::SelectAction {
            actor: ActorRef::Player(0),
        };
        combat.menu_cursor = 0; // Attack

        world.handle_key(Key::Enter);

        let GameState::Combat(combat) = &world.state else {
            panic!("expected to still be in combat");
        };
        assert!(
            !matches!(combat.phase, CombatPhase::SelectTarget { .. }),
            "a lone enemy should resolve immediately instead of waiting on a second Enter"
        );
    }

    #[test]
    fn page_up_and_down_scroll_the_combat_log_and_clamp() {
        use crate::game::character::slime;

        let mut world = World::new();
        let mut combat = CombatState::new(&world.party, vec![slime("Slime")]);
        for i in 0..5 {
            combat.push_log(format!("line {i}"));
        }
        let log_len = combat.log.len();
        world.state = GameState::Combat(combat);

        world.handle_key(Key::PageUp);
        let GameState::Combat(combat) = &world.state else {
            unreachable!()
        };
        assert_eq!(combat.log_scroll, log_len);

        world.handle_key(Key::PageDown);
        let GameState::Combat(combat) = &world.state else {
            unreachable!()
        };
        assert_eq!(combat.log_scroll, log_len.saturating_sub(10));
    }

    #[test]
    fn help_overlay_toggles_and_blocks_other_input_while_open() {
        let mut world = World::new();
        assert!(!world.show_help);

        world.handle_key(Key::Char('?'));
        assert!(world.show_help);

        // While the overlay is open, other keys (like movement) shouldn't
        // reach the underlying screen.
        let pos_before = match &world.state {
            GameState::Explore(explore) => explore.player_pos,
            _ => panic!("expected to be exploring"),
        };
        world.handle_key(Key::Down);
        let pos_after = match &world.state {
            GameState::Explore(explore) => explore.player_pos,
            _ => panic!("expected to be exploring"),
        };
        assert_eq!(
            pos_before, pos_after,
            "input should be swallowed by the help overlay"
        );

        world.handle_key(Key::Char('?'));
        assert!(!world.show_help);
    }
}
