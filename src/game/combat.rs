use rand::Rng;

use crate::game::chapter::BossKind;
use crate::game::character::{AbilityKind, Character};
use crate::game::item::{
    bone_blade, ether, goblin_shiv, knightsbane, mimics_fang, orcish_greataxe, potion,
    sovereigns_reckoning, wardens_fang, wraithbane_edge, Item, ItemFactory, ItemKind, Weapon,
    WeaponFactory,
};
use crate::game::map::Position;
use crate::game::party::Party;
use crate::game::status::{roll_curse, StatEffectTarget};

#[derive(Debug, Clone)]
pub struct Loot {
    pub gold: u32,
    pub items: Vec<Item>,
    pub weapons: Vec<Weapon>,
    /// Extra gold earned from overkill kills this fight (already folded into
    /// `gold`); kept separate so the UI can call it out.
    pub overkill_bonus: u32,
}

/// Per-species gold range, an optional (item, drop-chance) pair, and an
/// optional (weapon, drop-chance) pair. Keyed on the enemy's display name,
/// which currently doubles as its species tag. Only certain, tougher species
/// carry a weapon worth looting off their corpse.
fn loot_profile(
    species_name: &str,
) -> (
    std::ops::RangeInclusive<u32>,
    Option<(ItemFactory, f32)>,
    Option<(WeaponFactory, f32)>,
) {
    match species_name {
        "Slime" => (3..=8, Some((potion as ItemFactory, 0.25)), None),
        "Goblin" => (
            8..=16,
            Some((ether as ItemFactory, 0.2)),
            Some((goblin_shiv as WeaponFactory, 0.12)),
        ),
        "Bat" => (2..=5, Some((potion as ItemFactory, 0.15)), None),
        "Wolf" => (5..=10, Some((potion as ItemFactory, 0.2)), None),
        "Skeleton" => (
            6..=12,
            Some((ether as ItemFactory, 0.25)),
            Some((bone_blade as WeaponFactory, 0.15)),
        ),
        "Orc" => (
            10..=20,
            Some((potion as ItemFactory, 0.3)),
            Some((orcish_greataxe as WeaponFactory, 0.18)),
        ),
        "Wraith" => (
            12..=22,
            Some((ether as ItemFactory, 0.35)),
            Some((wraithbane_edge as WeaponFactory, 0.15)),
        ),
        // Mimics are meant to feel like a consolation prize for the ambush.
        "Mimic" => (
            25..=45,
            Some((potion as ItemFactory, 0.6)),
            Some((mimics_fang as WeaponFactory, 0.4)),
        ),
        _ => (5..=10, None, None),
    }
}

/// Per-boss gold range, an optional (item, drop-chance) pair, and its
/// guaranteed signature weapon. Unlike `loot_profile`, the weapon here is
/// never a dice roll — beating the fight itself is the gate.
fn boss_loot_profile(
    kind: BossKind,
) -> (
    std::ops::RangeInclusive<u32>,
    Option<(ItemFactory, f32)>,
    WeaponFactory,
) {
    match kind {
        BossKind::BarrowKnight => (
            80..=150,
            Some((potion as ItemFactory, 0.5)),
            knightsbane as WeaponFactory,
        ),
        BossKind::WyrmscaleWarden => (
            150..=250,
            Some((ether as ItemFactory, 0.5)),
            wardens_fang as WeaponFactory,
        ),
        BossKind::AshenSovereign => (
            250..=400,
            Some((potion as ItemFactory, 0.5)),
            sovereigns_reckoning as WeaponFactory,
        ),
    }
}

fn roll_loot(enemies: &[Character], overkills: &[bool], rng: &mut impl Rng) -> Loot {
    let mut gold = 0u32;
    let mut items = Vec::new();
    let mut weapons = Vec::new();
    let mut overkill_bonus = 0u32;
    for (i, e) in enemies.iter().enumerate() {
        let (gold_range, item_chance, weapon_chance) = match e.boss_kind {
            Some(kind) => {
                let (gold_range, item_chance, weapon_factory) = boss_loot_profile(kind);
                (gold_range, item_chance, Some((weapon_factory, 1.0)))
            }
            None => loot_profile(&e.name),
        };
        let base_gold = rng.gen_range(gold_range);
        if overkills.get(i).copied().unwrap_or(false) {
            let bonus = base_gold / 2;
            overkill_bonus += bonus;
            gold += base_gold + bonus;
        } else {
            gold += base_gold;
        }
        if let Some((make_item, chance)) = item_chance {
            if rng.gen::<f32>() < chance {
                items.push(make_item());
            }
        }
        if let Some((make_weapon, chance)) = weapon_chance {
            if rng.gen::<f32>() < chance {
                weapons.push(make_weapon());
            }
        }
    }
    Loot { gold, items, weapons, overkill_bonus }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorRef {
    Player(usize),
    Enemy(usize),
}

#[derive(Debug, Clone, Copy)]
pub enum CombatAction {
    Attack,
    Ability(usize),
    Item(usize),
    Flee,
}

pub enum CombatPhase {
    // A player-controlled actor is choosing what to do.
    SelectAction { actor: ActorRef },
    // A player-controlled actor chose "Ability" and is picking which one.
    SelectAbility { actor: ActorRef, cursor: usize },
    // A player-controlled actor chose "Item" and is picking which one
    // (e.g. Potion vs. Ether) before it's applied.
    SelectItem { actor: ActorRef, cursor: usize },
    // A player-controlled actor picked an action that needs a target.
    SelectTarget {
        actor: ActorRef,
        action: CombatAction,
        target_idx: usize,
    },
    // Reserved: not currently used, but useful once actions get animation delays.
    #[allow(dead_code)]
    Resolving,
    Victory,
    Defeat,
    Fled,
}

pub struct CombatState {
    pub enemies: Vec<Character>,
    pub turn_order: Vec<ActorRef>,
    pub turn_cursor: usize,
    pub phase: CombatPhase,
    pub log: Vec<String>,
    pub menu_cursor: usize,
    pub loot: Option<Loot>,
    /// Parallel to `enemies`: whether that enemy died to a hit dealing at
    /// least 1.5x its max HP in one blow, which earns bonus gold on victory.
    pub overkills: Vec<bool>,
    /// How many of the current boss's rally thresholds have already fired
    /// this fight (0 = none yet). Most bosses only ever reach 1; the final
    /// boss's two-stage Ashen Rebirth can reach 2. Irrelevant for every
    /// non-boss enemy; see `resolve_boss_move`.
    pub enrage_stage: u8,
    /// Where to place the player back on the map once this fight ends.
    /// Set explicitly by the caller (`World`) right after construction —
    /// `None` here just means "use the default spawn," which only matters
    /// for tests that construct `CombatState` directly.
    pub return_pos: Option<Position>,
}

impl CombatState {
    pub fn new(party: &Party, enemies: Vec<Character>) -> Self {
        let mut order: Vec<ActorRef> = Vec::new();
        for (i, m) in party.members.iter().enumerate() {
            if m.is_alive() {
                order.push(ActorRef::Player(i));
            }
        }
        for (i, e) in enemies.iter().enumerate() {
            if e.is_alive() {
                order.push(ActorRef::Enemy(i));
            }
        }
        // Sort fastest-first. Speed lookup needs both party+enemies, done via closure below.
        let speed_of = |r: &ActorRef| -> i32 {
            match r {
                ActorRef::Player(i) => {
                    party.members[*i].stats.speed + party.stat_delta(StatEffectTarget::Speed)
                }
                ActorRef::Enemy(i) => enemies[*i].stats.speed,
            }
        };
        order.sort_by(|a, b| speed_of(b).cmp(&speed_of(a)));

        let first = order[0];
        let mut log = Vec::new();
        if enemies.iter().any(|e| e.name == "Mimic") {
            log.push("It was a trap! The treasure chest was a Mimic!".to_string());
        } else {
            log.push("A wild encounter begins!".to_string());
            for e in &enemies {
                log.push(format!("{} appears!", e.name));
            }
        }

        let overkills = vec![false; enemies.len()];

        Self {
            enemies,
            turn_order: order,
            turn_cursor: 0,
            phase: CombatPhase::SelectAction { actor: first },
            log,
            menu_cursor: 0,
            loot: None,
            overkills,
            enrage_stage: 0,
            return_pos: None,
        }
    }

    pub fn current_actor(&self) -> ActorRef {
        self.turn_order[self.turn_cursor]
    }

    fn push_log(&mut self, msg: impl Into<String>) {
        self.log.push(msg.into());
        // keep the log from growing unbounded; UI only shows the tail anyway
        if self.log.len() > 200 {
            self.log.remove(0);
        }
    }

    pub fn alive_enemy_indices(&self) -> Vec<usize> {
        self.enemies
            .iter()
            .enumerate()
            .filter(|(_, e)| e.is_alive())
            .map(|(i, _)| i)
            .collect()
    }

    fn alive_player_indices(party: &Party) -> Vec<usize> {
        party
            .members
            .iter()
            .enumerate()
            .filter(|(_, c)| c.is_alive())
            .map(|(i, _)| i)
            .collect()
    }

    /// Resolves whatever action is currently pending (player-selected, or enemy AI),
    /// applies damage/healing, appends log lines, and advances the turn cursor.
    /// Called once per "tick" from the app loop after a player confirms a target,
    /// or immediately for enemy turns.
    pub fn resolve_current_turn(&mut self, party: &mut Party, rng: &mut impl Rng) {
        match self.current_actor() {
            ActorRef::Player(pi) => {
                // Only resolves once phase == SelectTarget for this actor; app.rs guarantees that.
                if let CombatPhase::SelectTarget {
                    action, target_idx, ..
                } = self.phase
                {
                    self.resolve_player_action(pi, action, target_idx, party, rng);
                }
            }
            ActorRef::Enemy(ei) => self.resolve_enemy_action(ei, party, rng),
        }
        self.advance_turn(party, rng);
    }

    fn resolve_player_action(
        &mut self,
        pi: usize,
        action: CombatAction,
        target_idx: usize,
        party: &mut Party,
        rng: &mut impl Rng,
    ) {
        let attacker_name = party.members[pi].name.clone();
        match action {
            CombatAction::Attack => {
                let atk = party.members[pi].total_attack() + party.stat_delta(StatEffectTarget::Attack);
                if let Some(enemy) = self.enemies.get_mut(target_idx) {
                    let dmg = roll_damage(atk, enemy.stats.defense, rng);
                    enemy.take_damage(dmg);
                    let ename = enemy.name.clone();
                    let max_hp = enemy.stats.max_hp;
                    let alive = enemy.is_alive();
                    self.push_log(format!("{attacker_name} attacks {ename} for {dmg} damage."));
                    if !alive {
                        self.push_log(format!("{ename} is defeated!"));
                        self.check_overkill(target_idx, dmg, max_hp, &ename);
                    }
                }
            }
            CombatAction::Ability(ability_idx) => {
                let Some(ability) = party.members[pi].abilities.get(ability_idx).cloned() else {
                    return;
                };
                if !party.members[pi].spend_mp(ability.mp_cost) {
                    self.push_log(format!("{attacker_name} doesn't have enough MP!"));
                    return;
                }
                match ability.kind {
                    AbilityKind::PhysicalDamage | AbilityKind::MagicDamage => {
                        let defense = self
                            .enemies
                            .get(target_idx)
                            .map(|e| e.total_defense())
                            .unwrap_or(0);
                        let power = ability.power + party.stat_delta(StatEffectTarget::Attack);
                        let dmg = roll_damage(power, defense / 2, rng);
                        if let Some(enemy) = self.enemies.get_mut(target_idx) {
                            enemy.take_damage(dmg);
                            let ename = enemy.name.clone();
                            let max_hp = enemy.stats.max_hp;
                            let alive = enemy.is_alive();
                            self.push_log(format!(
                                "{attacker_name} casts {} on {ename} for {dmg} damage.",
                                ability.name
                            ));
                            if !alive {
                                self.push_log(format!("{ename} is defeated!"));
                                self.check_overkill(target_idx, dmg, max_hp, &ename);
                            }
                        }
                    }
                    AbilityKind::Heal => {
                        if let Some(target) = party.members.get_mut(target_idx) {
                            target.heal(ability.power);
                            let tname = target.name.clone();
                            self.push_log(format!(
                                "{attacker_name} casts {} on {tname}, healing {} HP.",
                                ability.name, ability.power
                            ));
                        }
                    }
                }
            }
            CombatAction::Item(_) => {
                // Items are resolved via `apply_item_and_advance` (Inventory lives outside
                // CombatState), so this action variant never reaches here in practice.
            }
            CombatAction::Flee => {
                let roll: f32 = rng.gen();
                if roll < 0.6 {
                    self.push_log(format!("{attacker_name} flees from battle!"));
                    self.phase = CombatPhase::Fled;
                } else {
                    self.push_log(format!("{attacker_name} tries to flee, but can't get away!"));
                }
            }
        }
    }

    /// Flags `target_idx` as overkilled (for bonus gold on victory) if the
    /// killing blow dealt at least 1.5x the enemy's max HP in one hit.
    fn check_overkill(&mut self, target_idx: usize, dmg: i32, max_hp: i32, ename: &str) {
        if dmg as f32 >= max_hp as f32 * 1.5 {
            self.overkills[target_idx] = true;
            self.push_log(format!(
                "Overkill! {ename} never stood a chance — expect a bigger bounty."
            ));
        }
    }

    fn resolve_enemy_action(&mut self, ei: usize, party: &mut Party, rng: &mut impl Rng) {
        let alive_targets = Self::alive_player_indices(party);
        if alive_targets.is_empty() {
            return;
        }
        let target_idx = alive_targets[rng.gen_range(0..alive_targets.len())];
        let (ename, atk, is_wraith, boss_kind) = {
            let e = &self.enemies[ei];
            (e.name.clone(), e.stats.attack, e.name == "Wraith", e.boss_kind)
        };

        // Wraiths spend some turns cursing the party instead of attacking directly.
        if is_wraith && rng.gen_bool(0.3) {
            let curse = roll_curse(rng);
            self.push_log(format!("{ename} whispers a curse over your party..."));
            self.push_log(format!(
                "{} takes hold! ({} {} for {} encounters)",
                curse.name, curse.delta, curse.target, curse.encounters_remaining
            ));
            party.add_effect(curse);
            return;
        }

        if let Some(kind) = boss_kind {
            if self.resolve_boss_move(ei, kind, &ename, atk, target_idx, party, rng) {
                return;
            }
        }

        let def = party.members[target_idx].total_defense() + party.stat_delta(StatEffectTarget::Defense);
        let dmg = roll_damage(atk, def, rng);
        party.members[target_idx].take_damage(dmg);
        let tname = party.members[target_idx].name.clone();
        self.push_log(format!("{ename} attacks {tname} for {dmg} damage."));
        if !party.members[target_idx].is_alive() {
            self.push_log(format!("{tname} falls!"));
        }
    }

    /// Runs `kind`'s scripted moves, if one fires this turn. Returns `true`
    /// if it did (the caller should skip the default attack roll), `false`
    /// if it should fall through to a normal attack. One arm per `BossKind`
    /// — the direct generalization of what was previously a pair of
    /// `is_boss`-gated `if` blocks hardcoded to the Barrow Knight alone.
    fn resolve_boss_move(
        &mut self,
        ei: usize,
        kind: BossKind,
        ename: &str,
        atk: i32,
        target_idx: usize,
        party: &mut Party,
        rng: &mut impl Rng,
    ) -> bool {
        match kind {
            BossKind::BarrowKnight => {
                // Rallies once, the instant it's beaten down to a sliver of
                // health, rather than just dying quietly. This check runs
                // before its normal attack roll and doesn't consume any RNG
                // itself, so it can't shift outcomes for any other enemy.
                if self.enrage_stage == 0 {
                    let boss = &self.enemies[ei];
                    let hp_ratio = boss.stats.hp as f32 / boss.stats.max_hp as f32;
                    if hp_ratio <= 0.3 {
                        self.enrage_stage = 1;
                        let heal_amount = self.enemies[ei].stats.max_hp / 5; // 20% of max HP
                        self.enemies[ei].heal(heal_amount);
                        self.enemies[ei].stats.attack += 6;
                        self.push_log(format!("{ename} roars and calls on a second wind!"));
                        self.push_log(format!(
                            "{ename} recovers {heal_amount} HP and fights harder!"
                        ));
                        return true;
                    }
                }

                // Occasionally winds up a much heavier blow.
                if rng.gen_bool(0.35) {
                    let boosted_atk = (atk as f32 * 1.8) as i32;
                    let def = party.members[target_idx].total_defense()
                        + party.stat_delta(StatEffectTarget::Defense);
                    let dmg = roll_damage(boosted_atk, def, rng);
                    party.members[target_idx].take_damage(dmg);
                    let tname = party.members[target_idx].name.clone();
                    self.push_log(format!(
                        "{ename} winds up a Rending Cleave on {tname} for {dmg} damage!"
                    ));
                    if !party.members[target_idx].is_alive() {
                        self.push_log(format!("{tname} falls!"));
                    }
                    return true;
                }

                false
            }
            BossKind::WyrmscaleWarden => {
                // Rallies once, permanently hardening its hide, once beaten
                // down under 40% HP.
                if self.enrage_stage == 0 {
                    let boss = &self.enemies[ei];
                    let hp_ratio = boss.stats.hp as f32 / boss.stats.max_hp as f32;
                    if hp_ratio <= 0.4 {
                        self.enrage_stage = 1;
                        self.enemies[ei].stats.defense += 8;
                        self.push_log(format!("{ename} sheds a layer of scale, hardening its hide!"));
                        self.push_log(format!("{ename}'s defense rises!"));
                        return true;
                    }
                }

                // Occasionally sweeps its tail across the whole party at once —
                // the first party-wide enemy move in the game.
                if rng.gen_bool(0.3) {
                    self.push_log(format!("{ename} sweeps its tail across the whole party!"));
                    let def_delta = party.stat_delta(StatEffectTarget::Defense);
                    for member in party.alive_members_mut() {
                        let def = member.total_defense() + def_delta;
                        let dmg = roll_damage(atk, def, rng);
                        member.take_damage(dmg);
                        let tname = member.name.clone();
                        let alive = member.is_alive();
                        self.push_log(format!("{tname} takes {dmg} damage."));
                        if !alive {
                            self.push_log(format!("{tname} falls!"));
                        }
                    }
                    return true;
                }

                false
            }
            BossKind::AshenSovereign => {
                // Two-stage rebirth: rallies once below 50% HP, and again
                // below 20% — each stage heals a bit less but still hits harder.
                if self.enrage_stage < 2 {
                    let boss = &self.enemies[ei];
                    let hp_ratio = boss.stats.hp as f32 / boss.stats.max_hp as f32;
                    let threshold = if self.enrage_stage == 0 { 0.5 } else { 0.2 };
                    if hp_ratio <= threshold {
                        self.enrage_stage += 1;
                        let heal_pct = if self.enrage_stage == 1 { 0.15 } else { 0.10 };
                        let heal_amount = (self.enemies[ei].stats.max_hp as f32 * heal_pct) as i32;
                        self.enemies[ei].heal(heal_amount);
                        self.enemies[ei].stats.attack += 5;
                        self.push_log(format!("{ename} crumbles to ash and reforms!"));
                        self.push_log(format!(
                            "{ename} recovers {heal_amount} HP and fights harder!"
                        ));
                        return true;
                    }
                }

                // Occasionally unleashes a devastating single-target nova.
                if rng.gen_bool(0.4) {
                    let boosted_atk = (atk as f32 * 2.0) as i32;
                    let def = party.members[target_idx].total_defense()
                        + party.stat_delta(StatEffectTarget::Defense);
                    let dmg = roll_damage(boosted_atk, def, rng);
                    party.members[target_idx].take_damage(dmg);
                    let tname = party.members[target_idx].name.clone();
                    self.push_log(format!(
                        "{ename} unleashes a Cinder Nova on {tname} for {dmg} damage!"
                    ));
                    if !party.members[target_idx].is_alive() {
                        self.push_log(format!("{tname} falls!"));
                    }
                    return true;
                }

                false
            }
        }
    }

    fn advance_turn(&mut self, party: &mut Party, rng: &mut impl Rng) {
        if party.is_wiped() {
            self.phase = CombatPhase::Defeat;
            return;
        }
        if self.alive_enemy_indices().is_empty() {
            self.loot = Some(roll_loot(&self.enemies, &self.overkills, rng));
            self.phase = CombatPhase::Victory;
            return;
        }
        if matches!(self.phase, CombatPhase::Fled) {
            return;
        }

        // Drop dead actors from the turn order lazily by skipping them.
        loop {
            self.turn_cursor = (self.turn_cursor + 1) % self.turn_order.len();
            let is_live = match self.current_actor() {
                ActorRef::Player(i) => party.members[i].is_alive(),
                ActorRef::Enemy(i) => self.enemies[i].is_alive(),
            };
            if is_live {
                break;
            }
        }
        self.menu_cursor = 0;
        self.phase = CombatPhase::SelectAction {
            actor: self.current_actor(),
        };
    }

    /// Applies an already-consumed item's effect to a target and advances the turn.
    /// Called from app.rs, which owns the Inventory and pops the item before invoking this.
    pub fn apply_item_and_advance(
        &mut self,
        item_kind: ItemKind,
        user_name: &str,
        target_idx: usize,
        party: &mut Party,
        rng: &mut impl Rng,
    ) {
        let Some(target) = party.members.get_mut(target_idx) else {
            return;
        };
        let msg = crate::game::combat::use_item_kind(item_kind, target);
        self.push_log(format!("{user_name} uses an item. {msg}"));
        self.advance_turn(party, rng);
    }
}

fn roll_damage(power: i32, defense: i32, rng: &mut impl Rng) -> i32 {
    let base = (power - defense / 2).max(1);
    let variance = rng.gen_range(-2..=2);
    (base + variance).max(1)
}

pub fn use_item_kind(kind: ItemKind, target: &mut Character) -> String {
    match kind {
        ItemKind::Potion { heal } => {
            target.heal(heal);
            format!("{} recovers {} HP.", target.name, heal)
        }
        ItemKind::Ether { mp } => {
            target.stats.mp = (target.stats.mp + mp).min(target.stats.max_mp);
            format!("{} recovers {} MP.", target.name, mp)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::character::{
        ashen_sovereign, barrow_knight, mimic, orc, slime, warrior, wraith, wyrmscale_warden,
    };
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn test_party() -> Party {
        Party::new(vec![warrior("Bram")])
    }

    #[test]
    fn turn_order_is_speed_sorted() {
        let party = test_party(); // warrior speed 6
        let enemies = vec![slime("Slime")]; // speed 4
        let combat = CombatState::new(&party, enemies);
        // warrior (speed 6) should go before the slime (speed 4)
        assert_eq!(combat.turn_order[0], ActorRef::Player(0));
        assert_eq!(combat.turn_order[1], ActorRef::Enemy(0));
    }

    #[test]
    fn attacking_reduces_enemy_hp_and_can_win() {
        let mut party = test_party();
        let enemies = vec![slime("Slime")];
        let mut combat = CombatState::new(&party, enemies);
        let mut rng = StdRng::seed_from_u64(42);

        // Player's turn: attack the slime.
        assert!(matches!(combat.phase, CombatPhase::SelectAction { actor: ActorRef::Player(0) }));
        combat.phase = CombatPhase::SelectTarget {
            actor: ActorRef::Player(0),
            action: CombatAction::Attack,
            target_idx: 0,
        };
        let hp_before = combat.enemies[0].stats.hp;
        combat.resolve_current_turn(&mut party, &mut rng);
        assert!(combat.enemies[0].stats.hp < hp_before, "attack should deal damage");
    }

    #[test]
    fn defeated_enemies_trigger_victory() {
        let mut party = test_party();
        let mut enemies = vec![slime("Slime")];
        enemies[0].stats.hp = 1; // one hit from a warrior always kills this
        let mut combat = CombatState::new(&party, enemies);
        let mut rng = StdRng::seed_from_u64(7);

        combat.phase = CombatPhase::SelectTarget {
            actor: ActorRef::Player(0),
            action: CombatAction::Attack,
            target_idx: 0,
        };
        combat.resolve_current_turn(&mut party, &mut rng);
        assert!(matches!(combat.phase, CombatPhase::Victory));
    }

    #[test]
    fn wiped_party_triggers_defeat() {
        let mut party = test_party();
        party.members[0].stats.hp = 1;
        let enemies = vec![slime("Slime")];
        let mut combat = CombatState::new(&party, enemies);
        let mut rng = StdRng::seed_from_u64(1);

        // Force the enemy to act first regardless of speed roll, to deterministically test defeat.
        combat.turn_order = vec![ActorRef::Enemy(0), ActorRef::Player(0)];
        combat.turn_cursor = 0;
        combat.phase = CombatPhase::SelectAction { actor: ActorRef::Enemy(0) };
        combat.resolve_current_turn(&mut party, &mut rng);
        assert!(matches!(combat.phase, CombatPhase::Defeat));
    }

    #[test]
    fn healing_ability_restores_hp() {
        let mut party = Party::new(vec![crate::game::character::cleric("Idris")]);
        party.members[0].stats.hp = 5; // damaged
        let enemies = vec![slime("Slime")];
        let mut combat = CombatState::new(&party, enemies);
        let mut rng = StdRng::seed_from_u64(3);

        combat.phase = CombatPhase::SelectTarget {
            actor: ActorRef::Player(0),
            action: CombatAction::Ability(0), // cleric's Mend
            target_idx: 0,
        };
        combat.resolve_current_turn(&mut party, &mut rng);
        assert!(party.members[0].stats.hp > 5, "Mend should heal the cleric");
    }

    #[test]
    fn victory_populates_loot() {
        let mut party = test_party();
        let mut enemies = vec![slime("Slime")];
        enemies[0].stats.hp = 1;
        let mut combat = CombatState::new(&party, enemies);
        let mut rng = StdRng::seed_from_u64(99);

        combat.phase = CombatPhase::SelectTarget {
            actor: ActorRef::Player(0),
            action: CombatAction::Attack,
            target_idx: 0,
        };
        combat.resolve_current_turn(&mut party, &mut rng);
        assert!(matches!(combat.phase, CombatPhase::Victory));
        let loot = combat.loot.expect("victory should always roll loot");
        assert!(
            (3..=8).contains(&loot.gold),
            "slime gold should be in its 3-8 range, got {}",
            loot.gold
        );
    }

    #[test]
    fn attack_blessing_increases_damage_dealt() {
        use crate::game::status::{StatEffectTarget, StatusEffect};

        // Same seed, same fight, only difference is an active attack blessing —
        // damage dealt should be strictly higher with the buff active.
        let run_damage = |bless: bool| {
            let mut party = test_party();
            if bless {
                party.add_effect(StatusEffect {
                    name: "Warrior's Blessing".into(),
                    target: StatEffectTarget::Attack,
                    delta: 5,
                    encounters_remaining: 2,
                });
            }
            let mut enemies = vec![slime("Slime")];
            enemies[0].stats.hp = 999; // don't let it die mid-comparison
            let mut combat = CombatState::new(&party, enemies);
            let mut rng = StdRng::seed_from_u64(11);
            combat.phase = CombatPhase::SelectTarget {
                actor: ActorRef::Player(0),
                action: CombatAction::Attack,
                target_idx: 0,
            };
            combat.resolve_current_turn(&mut party, &mut rng);
            999 - combat.enemies[0].stats.hp
        };

        assert!(
            run_damage(true) > run_damage(false),
            "an active attack blessing should increase damage dealt"
        );
    }

    #[test]
    fn defense_curse_increases_damage_taken() {
        use crate::game::status::{StatEffectTarget, StatusEffect};

        let run_damage_taken = |cursed: bool| {
            let mut party = test_party();
            party.members[0].stats.hp = 999;
            if cursed {
                party.add_effect(StatusEffect {
                    name: "Curse of Frailty".into(),
                    target: StatEffectTarget::Defense,
                    delta: -4,
                    encounters_remaining: 2,
                });
            }
            let enemies = vec![slime("Slime")];
            let mut combat = CombatState::new(&party, enemies);
            let mut rng = StdRng::seed_from_u64(5);
            combat.turn_order = vec![ActorRef::Enemy(0), ActorRef::Player(0)];
            combat.turn_cursor = 0;
            combat.phase = CombatPhase::SelectAction {
                actor: ActorRef::Enemy(0),
            };
            combat.resolve_current_turn(&mut party, &mut rng);
            999 - party.members[0].stats.hp
        };

        assert!(
            run_damage_taken(true) > run_damage_taken(false),
            "an active defense curse should increase damage taken"
        );
    }

    #[test]
    fn effects_expire_after_their_encounter_count() {
        use crate::game::status::{StatEffectTarget, StatusEffect};

        let mut party = test_party();
        party.add_effect(StatusEffect {
            name: "Blessing of Swiftness".into(),
            target: StatEffectTarget::Speed,
            delta: 4,
            encounters_remaining: 2,
        });
        assert_eq!(party.effects.len(), 1);
        party.tick_effects();
        assert_eq!(party.effects[0].encounters_remaining, 1, "one encounter used up");
        party.tick_effects();
        assert!(party.effects.is_empty(), "effect should expire after 2 encounters");
    }

    #[test]
    fn wraith_can_curse_the_party() {
        // Probabilistic move (30% chance per turn) — sweep seeds and confirm it
        // fires at least once, rather than hardcoding one seed's RNG output.
        let mut cursed_at_least_once = false;
        for seed in 0..50u64 {
            let mut party = test_party();
            let enemies = vec![wraith("Wraith")];
            let mut combat = CombatState::new(&party, enemies);
            let mut rng = StdRng::seed_from_u64(seed);
            combat.turn_order = vec![ActorRef::Enemy(0), ActorRef::Player(0)];
            combat.turn_cursor = 0;
            combat.phase = CombatPhase::SelectAction {
                actor: ActorRef::Enemy(0),
            };
            combat.resolve_current_turn(&mut party, &mut rng);
            if !party.effects.is_empty() {
                cursed_at_least_once = true;
                break;
            }
        }
        assert!(
            cursed_at_least_once,
            "wraith should curse the party at least once across many trials"
        );
    }

    #[test]
    fn mimic_grants_high_value_loot() {
        let mut party = test_party();
        let mut enemies = vec![mimic("Mimic")];
        enemies[0].stats.hp = 1;
        let mut combat = CombatState::new(&party, enemies);
        let mut rng = StdRng::seed_from_u64(21);

        combat.phase = CombatPhase::SelectTarget {
            actor: ActorRef::Player(0),
            action: CombatAction::Attack,
            target_idx: 0,
        };
        combat.resolve_current_turn(&mut party, &mut rng);
        assert!(matches!(combat.phase, CombatPhase::Victory));
        let loot = combat.loot.expect("victory should always roll loot");
        assert!(
            (25..=45).contains(&loot.gold),
            "mimic gold should be in its 25-45 range, got {}",
            loot.gold
        );
    }

    #[test]
    fn second_ability_slot_resolves_correctly() {
        let mut party = test_party(); // warrior with Power Strike (0) and Crushing Blow (1)
        let enemies = vec![slime("Slime")];
        let mut combat = CombatState::new(&party, enemies);
        let mut rng = StdRng::seed_from_u64(2);
        let mp_before = party.members[0].stats.mp;

        combat.phase = CombatPhase::SelectTarget {
            actor: ActorRef::Player(0),
            action: CombatAction::Ability(1), // Crushing Blow
            target_idx: 0,
        };
        let hp_before = combat.enemies[0].stats.hp;
        combat.resolve_current_turn(&mut party, &mut rng);
        assert!(
            combat.enemies[0].stats.hp < hp_before,
            "Crushing Blow should deal damage"
        );
        assert!(
            party.members[0].stats.mp < mp_before,
            "Crushing Blow should consume MP"
        );
    }

    #[test]
    fn ability_is_heal_checks_the_specific_slot() {
        let cleric = crate::game::character::cleric("Idris");
        assert!(cleric.ability_is_heal(0), "Mend (slot 0) should be a heal");
        assert!(!cleric.ability_is_heal(1), "Smite (slot 1) should not be a heal");

        let bram = warrior("Bram");
        assert!(!bram.ability_is_heal(0));
        assert!(!bram.ability_is_heal(1));
    }

    #[test]
    fn stacking_the_same_blessing_merges_instead_of_duplicating() {
        use crate::game::status::{StatEffectTarget, StatusEffect};

        let mut party = test_party();
        party.add_effect(StatusEffect {
            name: "Warrior's Blessing".into(),
            target: StatEffectTarget::Attack,
            delta: 5,
            encounters_remaining: 1,
        });
        party.add_effect(StatusEffect {
            name: "Warrior's Blessing".into(),
            target: StatEffectTarget::Attack,
            delta: 5,
            encounters_remaining: 2,
        });

        assert_eq!(
            party.effects.len(),
            1,
            "same-named effects should merge into a single entry"
        );
        assert_eq!(party.effects[0].delta, 10, "magnitudes should stack");
        assert_eq!(
            party.effects[0].encounters_remaining, 2,
            "duration should refresh to the longer of the two"
        );
    }

    #[test]
    fn a_stronger_weapon_increases_damage_dealt() {
        use crate::game::item::dragonslayers_oath;

        // Same seed, same fight; only difference is the warrior's weapon.
        // The gap (+2 starting sword vs. +22 legendary greatsword) is large
        // enough that damage variance (-2..=2) can never mask the result.
        let run_damage = |legendary: bool| {
            let mut party = test_party();
            if legendary {
                party.members[0].equip_weapon(dragonslayers_oath());
            }
            let mut enemies = vec![slime("Slime")];
            enemies[0].stats.hp = 999;
            let mut combat = CombatState::new(&party, enemies);
            let mut rng = StdRng::seed_from_u64(11);
            combat.phase = CombatPhase::SelectTarget {
                actor: ActorRef::Player(0),
                action: CombatAction::Attack,
                target_idx: 0,
            };
            combat.resolve_current_turn(&mut party, &mut rng);
            999 - combat.enemies[0].stats.hp
        };

        assert!(
            run_damage(true) > run_damage(false),
            "a stronger equipped weapon should increase damage dealt"
        );
    }

    #[test]
    fn orc_can_drop_a_weapon() {
        // Probabilistic drop (18% chance) — sweep seeds and confirm it fires
        // at least once, rather than hardcoding one seed's RNG output.
        let mut dropped_at_least_once = false;
        for seed in 0..100u64 {
            let mut party = test_party();
            let mut enemies = vec![orc("Orc")];
            enemies[0].stats.hp = 1;
            let mut combat = CombatState::new(&party, enemies);
            let mut rng = StdRng::seed_from_u64(seed);
            combat.phase = CombatPhase::SelectTarget {
                actor: ActorRef::Player(0),
                action: CombatAction::Attack,
                target_idx: 0,
            };
            combat.resolve_current_turn(&mut party, &mut rng);
            if let Some(loot) = &combat.loot {
                if !loot.weapons.is_empty() {
                    dropped_at_least_once = true;
                    break;
                }
            }
        }
        assert!(
            dropped_at_least_once,
            "orc should drop a weapon at least once across many trials"
        );
    }

    #[test]
    fn slime_never_drops_a_weapon() {
        // Slimes have no weapon_chance entry in the loot table at all.
        for seed in 0..30u64 {
            let mut party = test_party();
            let mut enemies = vec![slime("Slime")];
            enemies[0].stats.hp = 1;
            let mut combat = CombatState::new(&party, enemies);
            let mut rng = StdRng::seed_from_u64(seed);
            combat.phase = CombatPhase::SelectTarget {
                actor: ActorRef::Player(0),
                action: CombatAction::Attack,
                target_idx: 0,
            };
            combat.resolve_current_turn(&mut party, &mut rng);
            let loot = combat.loot.expect("victory should always roll loot");
            assert!(loot.weapons.is_empty(), "slimes should never drop a weapon");
        }
    }

    #[test]
    fn boss_triggers_second_wind_below_30_percent_hp() {
        let mut party = test_party();
        let mut enemies = vec![barrow_knight("The Barrow Knight")];
        let max_hp = enemies[0].stats.max_hp;
        enemies[0].stats.hp = (max_hp as f32 * 0.25) as i32; // below the 30% threshold
        let hp_before = enemies[0].stats.hp;
        let atk_before = enemies[0].stats.attack;
        let mut combat = CombatState::new(&party, enemies);
        let mut rng = StdRng::seed_from_u64(5);
        combat.turn_order = vec![ActorRef::Enemy(0), ActorRef::Player(0)];
        combat.turn_cursor = 0;
        combat.phase = CombatPhase::SelectAction {
            actor: ActorRef::Enemy(0),
        };
        combat.resolve_current_turn(&mut party, &mut rng);

        assert!(combat.enrage_stage > 0, "boss should enrage below 30% hp");
        assert!(
            combat.enemies[0].stats.hp > hp_before,
            "second wind should heal the boss"
        );
        assert!(
            combat.enemies[0].stats.attack > atk_before,
            "second wind should buff the boss's attack"
        );
    }

    #[test]
    fn boss_does_not_enrage_above_the_threshold() {
        let mut party = test_party();
        let enemies = vec![barrow_knight("The Barrow Knight")]; // full HP
        let mut combat = CombatState::new(&party, enemies);
        let mut rng = StdRng::seed_from_u64(5);
        combat.turn_order = vec![ActorRef::Enemy(0), ActorRef::Player(0)];
        combat.turn_cursor = 0;
        combat.phase = CombatPhase::SelectAction {
            actor: ActorRef::Enemy(0),
        };
        combat.resolve_current_turn(&mut party, &mut rng);
        assert!(combat.enrage_stage == 0, "boss should not enrage while healthy");
    }

    #[test]
    fn boss_can_use_rending_cleave() {
        // Probabilistic move (35% chance per turn) — sweep seeds like the
        // other special-move tests rather than hardcoding one seed's output.
        let mut used_cleave = false;
        for seed in 0..50u64 {
            let mut party = test_party();
            let enemies = vec![barrow_knight("The Barrow Knight")]; // full HP, won't enrage
            let mut combat = CombatState::new(&party, enemies);
            let mut rng = StdRng::seed_from_u64(seed);
            combat.turn_order = vec![ActorRef::Enemy(0), ActorRef::Player(0)];
            combat.turn_cursor = 0;
            combat.phase = CombatPhase::SelectAction {
                actor: ActorRef::Enemy(0),
            };
            combat.resolve_current_turn(&mut party, &mut rng);
            if combat.log.iter().any(|line| line.contains("Rending Cleave")) {
                used_cleave = true;
                break;
            }
        }
        assert!(
            used_cleave,
            "boss should use Rending Cleave at least once across many trials"
        );
    }

    #[test]
    fn defeating_the_boss_guarantees_its_signature_weapon() {
        let mut party = test_party();
        let mut enemies = vec![barrow_knight("The Barrow Knight")];
        enemies[0].stats.hp = 1;
        let mut combat = CombatState::new(&party, enemies);
        let mut rng = StdRng::seed_from_u64(3);
        combat.phase = CombatPhase::SelectTarget {
            actor: ActorRef::Player(0),
            action: CombatAction::Attack,
            target_idx: 0,
        };
        combat.resolve_current_turn(&mut party, &mut rng);
        assert!(matches!(combat.phase, CombatPhase::Victory));
        let loot = combat.loot.expect("victory should always roll loot");
        assert!(
            loot.weapons.iter().any(|w| w.name == "Knightsbane"),
            "defeating the boss should always drop Knightsbane"
        );
    }

    #[test]
    fn overkill_grants_bonus_gold() {
        let enemies = vec![slime("Slime")];
        let mut rng1 = StdRng::seed_from_u64(50);
        let normal = roll_loot(&enemies, &[false], &mut rng1);
        let mut rng2 = StdRng::seed_from_u64(50);
        let overkill = roll_loot(&enemies, &[true], &mut rng2);

        assert!(
            overkill.gold > normal.gold,
            "an overkilled kill should pay out more gold than a normal one for the same roll"
        );
        assert!(
            overkill.overkill_bonus > 0,
            "overkill_bonus should reflect the extra gold earned"
        );
    }

    #[test]
    fn one_shotting_an_enemy_with_massive_damage_triggers_overkill() {
        let mut party = test_party();
        let enemies = vec![slime("Slime")]; // max_hp 18, so overkill needs >= 27 damage in one hit
        let mut combat = CombatState::new(&party, enemies);
        let mut rng = StdRng::seed_from_u64(1);
        // Force damage far past the 1.5x-max-HP threshold regardless of variance.
        party.members[0].stats.attack = 200;

        combat.phase = CombatPhase::SelectTarget {
            actor: ActorRef::Player(0),
            action: CombatAction::Attack,
            target_idx: 0,
        };
        combat.resolve_current_turn(&mut party, &mut rng);

        assert!(matches!(combat.phase, CombatPhase::Victory));
        assert!(
            combat.overkills[0],
            "massive damage should flag the kill as an overkill"
        );
        assert!(
            combat.log.iter().any(|line| line.contains("Overkill")),
            "the log should call out the overkill"
        );
        let loot = combat.loot.expect("victory should always roll loot");
        assert!(
            loot.overkill_bonus > 0,
            "an overkilled fight should report a bonus"
        );
    }

    #[test]
    fn warden_rallies_below_40_percent_hp() {
        let mut party = test_party();
        let mut enemies = vec![wyrmscale_warden("Wyrmscale Warden")];
        let max_hp = enemies[0].stats.max_hp;
        enemies[0].stats.hp = (max_hp as f32 * 0.35) as i32; // below the 40% threshold
        let def_before = enemies[0].stats.defense;
        let mut combat = CombatState::new(&party, enemies);
        let mut rng = StdRng::seed_from_u64(5);
        combat.turn_order = vec![ActorRef::Enemy(0), ActorRef::Player(0)];
        combat.turn_cursor = 0;
        combat.phase = CombatPhase::SelectAction {
            actor: ActorRef::Enemy(0),
        };
        combat.resolve_current_turn(&mut party, &mut rng);

        assert!(combat.enrage_stage > 0, "warden should enrage below 40% hp");
        assert!(
            combat.enemies[0].stats.defense > def_before,
            "molting rage should permanently raise defense"
        );
    }

    #[test]
    fn warden_does_not_enrage_above_the_threshold() {
        let mut party = test_party();
        let enemies = vec![wyrmscale_warden("Wyrmscale Warden")]; // full HP
        let mut combat = CombatState::new(&party, enemies);
        let mut rng = StdRng::seed_from_u64(5);
        combat.turn_order = vec![ActorRef::Enemy(0), ActorRef::Player(0)];
        combat.turn_cursor = 0;
        combat.phase = CombatPhase::SelectAction {
            actor: ActorRef::Enemy(0),
        };
        combat.resolve_current_turn(&mut party, &mut rng);
        assert_eq!(combat.enrage_stage, 0, "warden should not enrage while healthy");
    }

    #[test]
    fn warden_tail_sweep_hits_every_alive_party_member() {
        // Probabilistic move (30% chance per turn) — sweep seeds like the
        // other special-move tests.
        let mut swept = false;
        for seed in 0..50u64 {
            let mut party = Party::new(vec![warrior("Bram"), warrior("Second")]);
            let enemies = vec![wyrmscale_warden("Wyrmscale Warden")]; // full HP, won't enrage
            let mut combat = CombatState::new(&party, enemies);
            let mut rng = StdRng::seed_from_u64(seed);
            combat.turn_order = vec![ActorRef::Enemy(0), ActorRef::Player(0), ActorRef::Player(1)];
            combat.turn_cursor = 0;
            combat.phase = CombatPhase::SelectAction {
                actor: ActorRef::Enemy(0),
            };
            let hp_before: Vec<i32> = party.members.iter().map(|m| m.stats.hp).collect();
            combat.resolve_current_turn(&mut party, &mut rng);
            if combat.log.iter().any(|line| line.contains("sweeps its tail")) {
                assert!(
                    party.members.iter().enumerate().all(|(i, m)| m.stats.hp < hp_before[i]),
                    "tail sweep should damage every party member, not just one"
                );
                swept = true;
                break;
            }
        }
        assert!(swept, "warden should use Tail Sweep at least once across many trials");
    }

    #[test]
    fn defeating_the_warden_guarantees_its_signature_weapon() {
        let mut party = test_party();
        let mut enemies = vec![wyrmscale_warden("Wyrmscale Warden")];
        enemies[0].stats.hp = 1;
        let mut combat = CombatState::new(&party, enemies);
        let mut rng = StdRng::seed_from_u64(3);
        // The warden (speed 8) is faster than the test party (speed 6), so
        // the natural turn order would put it first — force the player's
        // turn explicitly instead.
        combat.turn_order = vec![ActorRef::Player(0), ActorRef::Enemy(0)];
        combat.turn_cursor = 0;
        combat.phase = CombatPhase::SelectTarget {
            actor: ActorRef::Player(0),
            action: CombatAction::Attack,
            target_idx: 0,
        };
        combat.resolve_current_turn(&mut party, &mut rng);
        assert!(matches!(combat.phase, CombatPhase::Victory));
        let loot = combat.loot.expect("victory should always roll loot");
        assert!(
            loot.weapons.iter().any(|w| w.name == "Warden's Fang"),
            "defeating the warden should always drop Warden's Fang"
        );
    }

    #[test]
    fn ashen_sovereign_can_reach_both_rebirth_stages() {
        let mut party = test_party();
        let mut enemies = vec![ashen_sovereign("The Ashen Sovereign")];
        let max_hp = enemies[0].stats.max_hp;
        enemies[0].stats.hp = (max_hp as f32 * 0.15) as i32; // below both thresholds at once
        let mut combat = CombatState::new(&party, enemies);
        let mut rng = StdRng::seed_from_u64(5);
        combat.turn_order = vec![ActorRef::Enemy(0), ActorRef::Player(0)];
        combat.turn_cursor = 0;

        // First enemy turn should trigger stage 1 (the 50% threshold, since
        // hp_ratio <= 0.5 is checked before <= 0.2 within a single stage).
        combat.phase = CombatPhase::SelectAction {
            actor: ActorRef::Enemy(0),
        };
        combat.resolve_current_turn(&mut party, &mut rng);
        assert_eq!(combat.enrage_stage, 1);

        // Re-lower HP below the second threshold and let it act again.
        combat.enemies[0].stats.hp = (max_hp as f32 * 0.15) as i32;
        combat.turn_cursor = 0;
        combat.phase = CombatPhase::SelectAction {
            actor: ActorRef::Enemy(0),
        };
        combat.resolve_current_turn(&mut party, &mut rng);
        assert_eq!(
            combat.enrage_stage, 2,
            "a second rebirth should fire once hp drops below the 20% threshold again"
        );
    }

    #[test]
    fn ashen_sovereign_can_use_cinder_nova() {
        let mut used_nova = false;
        for seed in 0..50u64 {
            let mut party = test_party();
            let enemies = vec![ashen_sovereign("The Ashen Sovereign")]; // full HP, won't enrage
            let mut combat = CombatState::new(&party, enemies);
            let mut rng = StdRng::seed_from_u64(seed);
            combat.turn_order = vec![ActorRef::Enemy(0), ActorRef::Player(0)];
            combat.turn_cursor = 0;
            combat.phase = CombatPhase::SelectAction {
                actor: ActorRef::Enemy(0),
            };
            combat.resolve_current_turn(&mut party, &mut rng);
            if combat.log.iter().any(|line| line.contains("Cinder Nova")) {
                used_nova = true;
                break;
            }
        }
        assert!(
            used_nova,
            "the sovereign should use Cinder Nova at least once across many trials"
        );
    }

    #[test]
    fn defeating_the_ashen_sovereign_guarantees_its_signature_weapon() {
        let mut party = test_party();
        let mut enemies = vec![ashen_sovereign("The Ashen Sovereign")];
        enemies[0].stats.hp = 1;
        let mut combat = CombatState::new(&party, enemies);
        let mut rng = StdRng::seed_from_u64(3);
        // The sovereign (speed 10) is faster than the test party (speed 6),
        // so force the player's turn explicitly rather than relying on the
        // natural speed-sorted order.
        combat.turn_order = vec![ActorRef::Player(0), ActorRef::Enemy(0)];
        combat.turn_cursor = 0;
        combat.phase = CombatPhase::SelectTarget {
            actor: ActorRef::Player(0),
            action: CombatAction::Attack,
            target_idx: 0,
        };
        combat.resolve_current_turn(&mut party, &mut rng);
        assert!(matches!(combat.phase, CombatPhase::Victory));
        let loot = combat.loot.expect("victory should always roll loot");
        assert!(
            loot.weapons.iter().any(|w| w.name == "Sovereign's Reckoning"),
            "defeating the sovereign should always drop Sovereign's Reckoning"
        );
    }
}
