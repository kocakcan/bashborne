use rand::Rng;

use crate::game::chapter::BossKind;
use crate::game::character::{AbilityKind, Character};
use crate::game::item::{
    bone_blade, ether, goblin_shiv, knightsbane, mimics_fang, orcish_greataxe, potion,
    sovereigns_reckoning, wardens_fang, wraithbane_edge, Item, ItemFactory, ItemKind, Weapon,
    WeaponFactory, WeaponPassive,
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
    /// XP awarded to every party member (see `xp_value`).
    pub xp: u32,
    /// "Titanite Shards" earned this fight — see `Inventory::upgrade_materials`.
    pub upgrade_materials: u32,
}

/// How much XP defeating this enemy is worth, awarded to every party member.
/// Roughly proportional to how tough the enemy was to fight (HP pool plus
/// offense and defense); bosses are worth a flat multiple on top since they
/// represent a whole chapter's worth of a challenge in one kill.
fn xp_value(enemy: &Character) -> u32 {
    let base = (enemy.stats.max_hp / 4 + enemy.stats.attack + enemy.stats.defense).max(1) as u32;
    if enemy.boss_kind.is_some() {
        base * 3
    } else {
        base
    }
}

/// Per-species gold range, an optional (item, drop-chance) pair, an optional
/// (weapon, drop-chance) pair, and an optional (shard-range, drop-chance)
/// pair for the blacksmith's upgrade material. Keyed on the enemy's display
/// name, which currently doubles as its species tag. Only certain, tougher
/// species carry a weapon (or shards) worth looting off their corpse.
#[allow(clippy::type_complexity)]
fn loot_profile(
    species_name: &str,
) -> (
    std::ops::RangeInclusive<u32>,
    Option<(ItemFactory, f32)>,
    Option<(WeaponFactory, f32)>,
    Option<(std::ops::RangeInclusive<u32>, f32)>,
) {
    match species_name {
        "Slime" => (3..=8, Some((potion as ItemFactory, 0.25)), None, None),
        "Goblin" => (
            8..=16,
            Some((ether as ItemFactory, 0.2)),
            Some((goblin_shiv as WeaponFactory, 0.12)),
            Some((1..=2, 0.2)),
        ),
        "Bat" => (2..=5, Some((potion as ItemFactory, 0.15)), None, None),
        "Wolf" => (
            5..=10,
            Some((potion as ItemFactory, 0.2)),
            None,
            Some((1..=2, 0.15)),
        ),
        "Skeleton" => (
            6..=12,
            Some((ether as ItemFactory, 0.25)),
            Some((bone_blade as WeaponFactory, 0.15)),
            Some((1..=3, 0.25)),
        ),
        "Orc" => (
            10..=20,
            Some((potion as ItemFactory, 0.3)),
            Some((orcish_greataxe as WeaponFactory, 0.18)),
            Some((2..=4, 0.3)),
        ),
        "Wraith" => (
            12..=22,
            Some((ether as ItemFactory, 0.35)),
            Some((wraithbane_edge as WeaponFactory, 0.15)),
            Some((2..=4, 0.3)),
        ),
        // Mimics are meant to feel like a consolation prize for the ambush.
        "Mimic" => (
            25..=45,
            Some((potion as ItemFactory, 0.6)),
            Some((mimics_fang as WeaponFactory, 0.4)),
            Some((3..=6, 0.6)),
        ),
        _ => (5..=10, None, None, None),
    }
}

/// Per-boss gold range, an optional (item, drop-chance) pair, its
/// guaranteed signature weapon, and its guaranteed shard range. Unlike
/// `loot_profile`, the weapon and shards here are never a dice roll —
/// beating the fight itself is the gate.
#[allow(clippy::type_complexity)]
fn boss_loot_profile(
    kind: BossKind,
) -> (
    std::ops::RangeInclusive<u32>,
    Option<(ItemFactory, f32)>,
    WeaponFactory,
    std::ops::RangeInclusive<u32>,
) {
    match kind {
        BossKind::BarrowKnight => (
            80..=150,
            Some((potion as ItemFactory, 0.5)),
            knightsbane as WeaponFactory,
            5..=8,
        ),
        BossKind::WyrmscaleWarden => (
            150..=250,
            Some((ether as ItemFactory, 0.5)),
            wardens_fang as WeaponFactory,
            8..=12,
        ),
        BossKind::AshenSovereign => (
            250..=400,
            Some((potion as ItemFactory, 0.5)),
            sovereigns_reckoning as WeaponFactory,
            12..=18,
        ),
    }
}

fn roll_loot(enemies: &[Character], overkills: &[bool], rng: &mut impl Rng) -> Loot {
    let mut gold = 0u32;
    let mut items = Vec::new();
    let mut weapons = Vec::new();
    let mut overkill_bonus = 0u32;
    let mut xp = 0u32;
    let mut upgrade_materials = 0u32;
    for (i, e) in enemies.iter().enumerate() {
        let (gold_range, item_chance, weapon_chance, material_chance) = match e.boss_kind {
            Some(kind) => {
                let (gold_range, item_chance, weapon_factory, shard_range) =
                    boss_loot_profile(kind);
                (
                    gold_range,
                    item_chance,
                    Some((weapon_factory, 1.0)),
                    Some((shard_range, 1.0)),
                )
            }
            None => loot_profile(&e.name),
        };
        // Tougher, later-chapter specimens of the same species pay out more:
        // +15% per level past the first (see `Character::scale_to_level`).
        let rolled = rng.gen_range(gold_range);
        let base_gold = rolled + rolled * 15 * e.level.saturating_sub(1) / 100;
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
        if let Some((shard_range, chance)) = material_chance {
            if rng.gen::<f32>() < chance {
                upgrade_materials += rng.gen_range(shard_range);
            }
        }
        xp += xp_value(e);
    }
    Loot {
        gold,
        items,
        weapons,
        overkill_bonus,
        xp,
        upgrade_materials,
    }
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
                let attacker_passive = party.members[pi]
                    .equipped_weapon
                    .as_ref()
                    .and_then(|w| w.passive);
                let atk = party.members[pi].total_attack() + party.stat_delta(StatEffectTarget::Attack);
                let luck = party.members[pi].total_luck();
                if let Some(enemy) = self.enemies.get_mut(target_idx) {
                    let mut defense = enemy.stats.defense;
                    if let Some(WeaponPassive::ArmorPierce(pct)) = attacker_passive {
                        defense = ((defense as f32) * (1.0 - pct)).round() as i32;
                    }
                    let (mut dmg, crit) = roll_damage(atk, defense, luck, rng);
                    if let Some(WeaponPassive::BossSlayer(pct)) = attacker_passive {
                        if enemy.boss_kind.is_some() {
                            dmg += ((dmg as f32) * pct).round() as i32;
                        }
                    }
                    enemy.take_damage(dmg);
                    let ename = enemy.name.clone();
                    let max_hp = enemy.stats.max_hp;
                    let alive = enemy.is_alive();
                    self.push_log(format!(
                        "{attacker_name} attacks {ename} for {dmg} damage.{}",
                        if crit { " A critical hit!" } else { "" }
                    ));
                    if let Some(WeaponPassive::Lifesteal(pct)) = attacker_passive {
                        let healed = ((dmg as f32) * pct).round() as i32;
                        if healed > 0 {
                            party.members[pi].heal(healed);
                            self.push_log(format!(
                                "{attacker_name} drains {healed} HP from the blow."
                            ));
                        }
                    }
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
                        let power = ability.effective_power(&party.members[pi])
                            + party.stat_delta(StatEffectTarget::Attack);
                        let luck = party.members[pi].total_luck();
                        let (dmg, crit) = roll_damage(power, defense / 2, luck, rng);
                        if let Some(enemy) = self.enemies.get_mut(target_idx) {
                            enemy.take_damage(dmg);
                            let ename = enemy.name.clone();
                            let max_hp = enemy.stats.max_hp;
                            let alive = enemy.is_alive();
                            self.push_log(format!(
                                "{attacker_name} casts {} on {ename} for {dmg} damage.{}",
                                ability.name,
                                if crit { " A critical hit!" } else { "" }
                            ));
                            if !alive {
                                self.push_log(format!("{ename} is defeated!"));
                                self.check_overkill(target_idx, dmg, max_hp, &ename);
                            }
                        }
                    }
                    AbilityKind::Heal => {
                        let heal_amount = ability.effective_power(&party.members[pi]);
                        if let Some(target) = party.members.get_mut(target_idx) {
                            target.heal(heal_amount);
                            let tname = target.name.clone();
                            self.push_log(format!(
                                "{attacker_name} casts {} on {tname}, healing {heal_amount} HP.",
                                ability.name
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
        let luck = self.enemies[ei].total_luck();
        let (dmg, crit) = roll_damage(atk, def, luck, rng);
        let dmg = apply_damage_reduction(dmg, &party.members[target_idx]);
        party.members[target_idx].take_damage(dmg);
        let tname = party.members[target_idx].name.clone();
        self.push_log(format!(
            "{ename} attacks {tname} for {dmg} damage.{}",
            if crit { " A critical hit!" } else { "" }
        ));
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
                    let luck = self.enemies[ei].total_luck();
                    let (dmg, crit) = roll_damage(boosted_atk, def, luck, rng);
                    let dmg = apply_damage_reduction(dmg, &party.members[target_idx]);
                    party.members[target_idx].take_damage(dmg);
                    let tname = party.members[target_idx].name.clone();
                    self.push_log(format!(
                        "{ename} winds up a Rending Cleave on {tname} for {dmg} damage!{}",
                        if crit { " A critical hit!" } else { "" }
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
                    let luck = self.enemies[ei].total_luck();
                    for member in party.alive_members_mut() {
                        let def = member.total_defense() + def_delta;
                        let (dmg, crit) = roll_damage(atk, def, luck, rng);
                        let dmg = apply_damage_reduction(dmg, member);
                        member.take_damage(dmg);
                        let tname = member.name.clone();
                        let alive = member.is_alive();
                        self.push_log(format!(
                            "{tname} takes {dmg} damage.{}",
                            if crit { " A critical hit!" } else { "" }
                        ));
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
                    let luck = self.enemies[ei].total_luck();
                    let (dmg, crit) = roll_damage(boosted_atk, def, luck, rng);
                    let dmg = apply_damage_reduction(dmg, &party.members[target_idx]);
                    party.members[target_idx].take_damage(dmg);
                    let tname = party.members[target_idx].name.clone();
                    self.push_log(format!(
                        "{ename} unleashes a Cinder Nova on {tname} for {dmg} damage!{}",
                        if crit { " A critical hit!" } else { "" }
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

/// Critical-hit multiplier applied to damage when `roll_damage`'s crit roll
/// succeeds.
const CRIT_MULTIPLIER: f32 = 1.75;

/// Chance (0.0-1.0) of a critical hit, given the attacker's Luck stat.
/// 1.5% per point of luck, capped at 50%.
fn crit_chance(luck: i32) -> f32 {
    (luck as f32 * 1.5).min(50.0) / 100.0
}

/// Rolls damage from `power` vs `defense`, applying a chance of a critical
/// hit based on the attacker's `luck`. Returns the final damage and whether
/// it was a crit, so callers can log it. Shared by players, enemies, and
/// bosses alike — enemies and bosses can crit too, since `Character` always
/// carries a `luck` value regardless of who it belongs to.
fn roll_damage(power: i32, defense: i32, luck: i32, rng: &mut impl Rng) -> (i32, bool) {
    let base = (power - defense / 2).max(1);
    let variance = rng.gen_range(-2..=2);
    let mut dmg = (base + variance).max(1);
    let is_crit = rng.gen::<f32>() < crit_chance(luck);
    if is_crit {
        dmg = ((dmg as f32) * CRIT_MULTIPLIER).round() as i32;
    }
    (dmg, is_crit)
}

/// Shaves incoming damage down for `WeaponPassive::DamageReduction`,
/// regardless of who's attacking — a defensive weapon passive, applied right
/// before the damage lands so the logged number always matches what's taken.
fn apply_damage_reduction(dmg: i32, target: &Character) -> i32 {
    match target.equipped_weapon.as_ref().and_then(|w| w.passive) {
        Some(WeaponPassive::DamageReduction(pct)) => {
            (((dmg as f32) * (1.0 - pct)).round() as i32).max(1)
        }
        _ => dmg,
    }
}

pub fn use_item_kind(kind: ItemKind, target: &mut Character) -> String {
    match kind {
        ItemKind::Potion { heal_percent } => {
            let amount = (target.stats.max_hp as f32 * heal_percent).round() as i32;
            target.heal(amount);
            format!("{} recovers {} HP.", target.name, amount)
        }
        ItemKind::Ether { mp_percent } => {
            let amount = (target.stats.max_mp as f32 * mp_percent).round() as i32;
            target.stats.mp = (target.stats.mp + amount).min(target.stats.max_mp);
            format!("{} recovers {} MP.", target.name, amount)
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
    fn lifesteal_passive_heals_the_attacker() {
        use crate::game::item::knightsbane;

        let mut party = test_party();
        party.members[0].equip_weapon(knightsbane());
        party.members[0].take_damage(20); // leave room to observe healing
        let hp_before = party.members[0].stats.hp;

        let mut enemies = vec![slime("Slime")];
        enemies[0].stats.hp = 999;
        enemies[0].stats.max_hp = 999;
        let mut combat = CombatState::new(&party, enemies);
        let mut rng = StdRng::seed_from_u64(11);
        combat.phase = CombatPhase::SelectTarget {
            actor: ActorRef::Player(0),
            action: CombatAction::Attack,
            target_idx: 0,
        };
        combat.resolve_current_turn(&mut party, &mut rng);

        assert!(
            party.members[0].stats.hp > hp_before,
            "Knightsbane's lifesteal should heal the wielder on a hit"
        );
    }

    #[test]
    fn boss_slayer_passive_only_boosts_damage_against_bosses() {
        use crate::game::character::barrow_knight;
        use crate::game::item::{dragonslayers_oath, GearSource, Rarity, Weapon};

        // A plain weapon with the exact same attack_bonus as Dragonslayer's
        // Oath, so any damage gap comes only from the BossSlayer passive.
        let plain_equivalent = || Weapon {
            name: "Plain Greatsword".into(),
            rarity: Rarity::Legendary,
            attack_bonus: dragonslayers_oath().attack_bonus,
            defense_bonus: dragonslayers_oath().defense_bonus,
            description: String::new(),
            source: GearSource::World,
            upgrade_level: 0,
            passive: None,
        };

        let run_damage = |legendary: bool, boss: bool| {
            let mut party = test_party();
            party.members[0].equip_weapon(if legendary {
                dragonslayers_oath()
            } else {
                plain_equivalent()
            });
            let mut enemy = if boss {
                barrow_knight("Test Boss")
            } else {
                slime("Slime")
            };
            enemy.stats.hp = 9999;
            enemy.stats.max_hp = 9999;
            let mut combat = CombatState::new(&party, vec![enemy]);
            let mut rng = StdRng::seed_from_u64(11);
            combat.phase = CombatPhase::SelectTarget {
                actor: ActorRef::Player(0),
                action: CombatAction::Attack,
                target_idx: 0,
            };
            combat.resolve_current_turn(&mut party, &mut rng);
            9999 - combat.enemies[0].stats.hp
        };

        assert!(
            run_damage(true, true) > run_damage(false, true),
            "BossSlayer should deal bonus damage to a boss"
        );
        assert_eq!(
            run_damage(true, false),
            run_damage(false, false),
            "BossSlayer should grant no bonus against a non-boss enemy"
        );
    }

    #[test]
    fn armor_pierce_passive_increases_damage_against_defense() {
        use crate::game::item::{sovereigns_reckoning, GearSource, Rarity, Weapon};

        let plain_equivalent = || Weapon {
            name: "Plain Blade".into(),
            rarity: Rarity::Legendary,
            attack_bonus: sovereigns_reckoning().attack_bonus,
            defense_bonus: sovereigns_reckoning().defense_bonus,
            description: String::new(),
            source: GearSource::World,
            upgrade_level: 0,
            passive: None,
        };

        let run_damage = |pierce: bool| {
            let mut party = test_party();
            party.members[0].equip_weapon(if pierce {
                sovereigns_reckoning()
            } else {
                plain_equivalent()
            });
            let mut enemy = slime("Slime");
            enemy.stats.hp = 9999;
            enemy.stats.max_hp = 9999;
            enemy.stats.defense = 30;
            let mut combat = CombatState::new(&party, vec![enemy]);
            let mut rng = StdRng::seed_from_u64(11);
            combat.phase = CombatPhase::SelectTarget {
                actor: ActorRef::Player(0),
                action: CombatAction::Attack,
                target_idx: 0,
            };
            combat.resolve_current_turn(&mut party, &mut rng);
            9999 - combat.enemies[0].stats.hp
        };

        assert!(
            run_damage(true) > run_damage(false),
            "ArmorPierce should deal more damage against a defended target"
        );
    }

    #[test]
    fn damage_reduction_passive_shields_the_wielder() {
        use crate::game::character::orc;
        use crate::game::item::{wardens_fang, GearSource, Rarity, Weapon};

        let plain_equivalent = || Weapon {
            name: "Plain Fang".into(),
            rarity: Rarity::Legendary,
            attack_bonus: wardens_fang().attack_bonus,
            defense_bonus: wardens_fang().defense_bonus,
            description: String::new(),
            source: GearSource::World,
            upgrade_level: 0,
            passive: None,
        };

        let run_damage_taken = |shielded: bool| {
            let mut party = test_party();
            party.members[0].equip_weapon(if shielded {
                wardens_fang()
            } else {
                plain_equivalent()
            });
            party.members[0].stats.hp = 9999;
            party.members[0].stats.max_hp = 9999;
            let hp_before = party.members[0].stats.hp;
            let enemies = vec![orc("Orc")];
            let mut combat = CombatState::new(&party, enemies);
            let mut rng = StdRng::seed_from_u64(11);
            combat.turn_order = vec![ActorRef::Enemy(0), ActorRef::Player(0)];
            combat.turn_cursor = 0;
            combat.phase = CombatPhase::SelectAction { actor: ActorRef::Enemy(0) };
            combat.resolve_current_turn(&mut party, &mut rng);
            hp_before - party.members[0].stats.hp
        };

        assert!(
            run_damage_taken(true) < run_damage_taken(false),
            "Warden's Fang's damage reduction should lower damage taken"
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

    #[test]
    fn crit_chance_is_zero_at_zero_luck_and_monotonic_and_capped() {
        assert_eq!(crit_chance(0), 0.0);
        assert!(crit_chance(5) < crit_chance(10));
        assert!(crit_chance(1000) <= 0.5);
    }

    #[test]
    fn high_luck_crits_at_least_once_across_many_trials_low_luck_never_does() {
        let mut high_luck_crit = false;
        let mut low_luck_crit = false;
        for seed in 0..100u64 {
            let mut rng = StdRng::seed_from_u64(seed);
            let (_, crit) = roll_damage(20, 5, 40, &mut rng);
            if crit {
                high_luck_crit = true;
            }
            let mut rng = StdRng::seed_from_u64(seed);
            let (_, crit) = roll_damage(20, 5, 0, &mut rng);
            if crit {
                low_luck_crit = true;
            }
        }
        assert!(high_luck_crit, "a high-luck attacker should crit at least once");
        assert!(!low_luck_crit, "a zero-luck attacker should never crit");
    }

    #[test]
    fn a_level_scaled_enemy_pays_more_gold_and_xp_than_its_base_form() {
        let base = vec![orc("Orc")];
        let mut tough = orc("Orc");
        tough.scale_to_level(7);
        let scaled = vec![tough];

        // Same seed, so the underlying gold roll is identical — only the
        // level multiplier differs.
        let base_loot = roll_loot(&base, &[false], &mut StdRng::seed_from_u64(9));
        let scaled_loot = roll_loot(&scaled, &[false], &mut StdRng::seed_from_u64(9));
        assert!(scaled_loot.gold > base_loot.gold);
        assert!(scaled_loot.xp > base_loot.xp);
    }

    #[test]
    fn xp_value_is_tripled_for_bosses() {
        let boss = barrow_knight("The Barrow Knight");
        let mut as_regular = boss.clone();
        as_regular.boss_kind = None;
        assert_eq!(xp_value(&boss), xp_value(&as_regular) * 3);
    }

    #[test]
    fn victory_awards_xp_and_boss_kills_award_more() {
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
        let loot = combat.loot.expect("victory should always roll loot");
        assert!(loot.xp > 0, "defeating an enemy should award XP");
    }

    #[test]
    fn mimic_always_drops_upgrade_materials() {
        for seed in 0..20u64 {
            let mut party = test_party();
            let mut enemies = vec![mimic("Mimic")];
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
            assert!(
                (3..=6).contains(&loot.upgrade_materials) || loot.upgrade_materials == 0,
                "mimic shard drop should be in range or absent (60% chance), got {}",
                loot.upgrade_materials
            );
        }
    }

    #[test]
    fn slime_never_drops_upgrade_materials() {
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
            assert_eq!(loot.upgrade_materials, 0, "slimes should never drop shards");
        }
    }

    #[test]
    fn bosses_always_drop_upgrade_materials_in_their_guaranteed_range() {
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
        let loot = combat.loot.expect("victory should always roll loot");
        assert!(
            (5..=8).contains(&loot.upgrade_materials),
            "barrow knight should always drop 5-8 shards, got {}",
            loot.upgrade_materials
        );
    }

    #[test]
    fn effective_power_folds_in_half_the_casters_attack() {
        let hero = warrior("Bram");
        let ability = &hero.abilities[0]; // Power Strike, base_power 10
        let expected = ability.base_power + hero.total_attack() / 2;
        assert_eq!(ability.effective_power(&hero), expected);
    }

    #[test]
    fn item_healing_scales_with_max_hp() {
        let mut low = warrior("Bram");
        low.stats.hp = 1;
        let msg_low = use_item_kind(ItemKind::Potion { heal_percent: 0.35 }, &mut low);
        let low_amount = low.stats.hp - 1;
        assert!(msg_low.contains("recovers"));

        let mut high = warrior("Bram");
        high.stats.max_hp *= 3;
        high.stats.hp = 1;
        use_item_kind(ItemKind::Potion { heal_percent: 0.35 }, &mut high);
        let high_amount = high.stats.hp - 1;

        assert!(
            high_amount > low_amount,
            "a character with more max HP should heal for more from the same percent"
        );
    }
}
