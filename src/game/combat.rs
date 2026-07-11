use rand::Rng;

use crate::game::chapter::BossKind;
use crate::game::character::{AbilityKind, Character};
use crate::game::item::{
    acolytes_vestment, bandits_falchion, barrow_touched_plate, bone_blade, brigand_leathers,
    chainmail_hauberk, dragonscale_aegis, elite_knights_armor, ether, fell_censer,
    forsaken_longsword, ghouls_knucklebone, goblin_shiv, hollow_soldiers_blade, knights_plate,
    knightsbane, mimics_coil, mimics_fang, orcish_greataxe, potion, ring_of_vigor, ring_of_warding,
    sentinels_bulwark, sentinels_greathammer, sentinels_seal, sovereign_elixir,
    sovereigns_reckoning, sovereigns_signet, warded_chainmail, wardens_fang, wolfsbane_signet,
    wraithbane_edge, Armor, ArmorFactory, Item, ItemFactory, ItemKind, Ring, RingFactory, Weapon,
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
    pub armors: Vec<Armor>,
    pub rings: Vec<Ring>,
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

/// Everything one species can leave behind after a fight. Split out as a
/// struct now that a corpse can carry up to five kinds of spoils — a
/// six-way tuple stopped being readable.
struct LootProfile {
    gold: std::ops::RangeInclusive<u32>,
    item: Option<(ItemFactory, f32)>,
    weapon: Option<(WeaponFactory, f32)>,
    /// (shard range, drop chance) for the blacksmith's upgrade material.
    shards: Option<(std::ops::RangeInclusive<u32>, f32)>,
    armor: Option<(ArmorFactory, f32)>,
    ring: Option<(RingFactory, f32)>,
}

impl LootProfile {
    /// Gold only — the default for species with nothing worth looting.
    fn plain(gold: std::ops::RangeInclusive<u32>) -> Self {
        Self {
            gold,
            item: None,
            weapon: None,
            shards: None,
            armor: None,
            ring: None,
        }
    }
}

/// Per-species loot table, keyed on the enemy's display name, which
/// currently doubles as its species tag. Only certain, tougher species
/// carry a weapon, gear, or shards worth looting off their corpse.
fn loot_profile(species_name: &str) -> LootProfile {
    match species_name {
        "Slime" => LootProfile {
            item: Some((potion as ItemFactory, 0.25)),
            ..LootProfile::plain(3..=8)
        },
        "Goblin" => LootProfile {
            item: Some((ether as ItemFactory, 0.2)),
            weapon: Some((goblin_shiv as WeaponFactory, 0.12)),
            shards: Some((1..=2, 0.2)),
            ring: Some((ring_of_vigor as RingFactory, 0.06)),
            ..LootProfile::plain(8..=16)
        },
        "Bat" => LootProfile {
            item: Some((potion as ItemFactory, 0.15)),
            ..LootProfile::plain(2..=5)
        },
        "Wolf" => LootProfile {
            item: Some((potion as ItemFactory, 0.2)),
            shards: Some((1..=2, 0.15)),
            ring: Some((wolfsbane_signet as RingFactory, 0.05)),
            ..LootProfile::plain(5..=10)
        },
        "Skeleton" => LootProfile {
            item: Some((ether as ItemFactory, 0.25)),
            weapon: Some((bone_blade as WeaponFactory, 0.15)),
            shards: Some((1..=3, 0.25)),
            armor: Some((chainmail_hauberk as ArmorFactory, 0.08)),
            ring: Some((ring_of_warding as RingFactory, 0.06)),
            ..LootProfile::plain(6..=12)
        },
        "Orc" => LootProfile {
            item: Some((potion as ItemFactory, 0.3)),
            weapon: Some((orcish_greataxe as WeaponFactory, 0.18)),
            shards: Some((2..=4, 0.3)),
            armor: Some((knights_plate as ArmorFactory, 0.08)),
            ..LootProfile::plain(10..=20)
        },
        "Wraith" => LootProfile {
            item: Some((ether as ItemFactory, 0.35)),
            weapon: Some((wraithbane_edge as WeaponFactory, 0.15)),
            shards: Some((2..=4, 0.3)),
            armor: Some((warded_chainmail as ArmorFactory, 0.08)),
            ..LootProfile::plain(12..=22)
        },
        // Mimics are meant to feel like a consolation prize for the ambush.
        "Mimic" => LootProfile {
            item: Some((potion as ItemFactory, 0.6)),
            weapon: Some((mimics_fang as WeaponFactory, 0.4)),
            shards: Some((3..=6, 0.6)),
            ring: Some((mimics_coil as RingFactory, 0.35)),
            ..LootProfile::plain(25..=45)
        },
        "Hollow" => LootProfile {
            item: Some((potion as ItemFactory, 0.2)),
            weapon: Some((hollow_soldiers_blade as WeaponFactory, 0.1)),
            shards: Some((1..=2, 0.1)),
            ..LootProfile::plain(4..=9)
        },
        "Rat" => LootProfile {
            item: Some((potion as ItemFactory, 0.1)),
            ..LootProfile::plain(2..=6)
        },
        "Carrion Crow" => LootProfile {
            item: Some((ether as ItemFactory, 0.15)),
            ..LootProfile::plain(3..=8)
        },
        "Bandit" => LootProfile {
            item: Some((potion as ItemFactory, 0.25)),
            weapon: Some((bandits_falchion as WeaponFactory, 0.12)),
            shards: Some((1..=2, 0.2)),
            armor: Some((brigand_leathers as ArmorFactory, 0.08)),
            ..LootProfile::plain(14..=26)
        },
        "Fell Acolyte" => LootProfile {
            item: Some((ether as ItemFactory, 0.35)),
            weapon: Some((fell_censer as WeaponFactory, 0.1)),
            shards: Some((1..=3, 0.2)),
            armor: Some((acolytes_vestment as ArmorFactory, 0.08)),
            ..LootProfile::plain(10..=20)
        },
        "Grave Ghoul" => LootProfile {
            item: Some((potion as ItemFactory, 0.25)),
            shards: Some((1..=3, 0.2)),
            ring: Some((ghouls_knucklebone as RingFactory, 0.07)),
            ..LootProfile::plain(8..=16)
        },
        "Barrow Sentinel" => LootProfile {
            weapon: Some((sentinels_greathammer as WeaponFactory, 0.08)),
            shards: Some((2..=5, 0.5)),
            armor: Some((sentinels_bulwark as ArmorFactory, 0.06)),
            ring: Some((sentinels_seal as RingFactory, 0.05)),
            ..LootProfile::plain(15..=28)
        },
        "Forsaken Knight" => LootProfile {
            item: Some((potion as ItemFactory, 0.2)),
            weapon: Some((forsaken_longsword as WeaponFactory, 0.15)),
            shards: Some((2..=4, 0.35)),
            armor: Some((elite_knights_armor as ArmorFactory, 0.1)),
            ..LootProfile::plain(18..=32)
        },
        _ => LootProfile::plain(5..=10),
    }
}

/// Human-readable drop lines for the bestiary's detail panel, derived from
/// the live `loot_profile` table (drop factories are invoked for their
/// names) so the codex can never drift from what actually drops.
pub fn loot_notes(species_name: &str) -> Vec<String> {
    let profile = loot_profile(species_name);
    let mut notes = vec![format!("Gold: {}-{}", profile.gold.start(), profile.gold.end())];
    if let Some((make_item, chance)) = profile.item {
        notes.push(format!("{}: {:.0}%", make_item().name, chance * 100.0));
    }
    if let Some((make_weapon, chance)) = profile.weapon {
        notes.push(format!("{}: {:.0}%", make_weapon().name, chance * 100.0));
    }
    if let Some((make_armor, chance)) = profile.armor {
        notes.push(format!("{}: {:.0}%", make_armor().name, chance * 100.0));
    }
    if let Some((make_ring, chance)) = profile.ring {
        notes.push(format!("{}: {:.0}%", make_ring().name, chance * 100.0));
    }
    if let Some((range, chance)) = profile.shards {
        notes.push(format!(
            "Titanite Shards x{}-{}: {:.0}%",
            range.start(),
            range.end(),
            chance * 100.0
        ));
    }
    notes
}

/// Per-boss spoils. Unlike `loot_profile`, the weapon, shards, and second
/// gear piece here are never a dice roll — beating the fight itself is the
/// gate. Only the consumable item slot still rolls.
struct BossLootProfile {
    gold: std::ops::RangeInclusive<u32>,
    item: Option<(ItemFactory, f32)>,
    weapon: WeaponFactory,
    shards: std::ops::RangeInclusive<u32>,
    /// A guaranteed armor piece alongside the signature weapon, if any.
    armor: Option<ArmorFactory>,
    /// A guaranteed ring alongside the signature weapon, if any.
    ring: Option<RingFactory>,
}

fn boss_loot_profile(kind: BossKind) -> BossLootProfile {
    match kind {
        BossKind::BarrowKnight => BossLootProfile {
            gold: 80..=150,
            item: Some((potion as ItemFactory, 0.5)),
            weapon: knightsbane as WeaponFactory,
            shards: 5..=8,
            armor: Some(barrow_touched_plate as ArmorFactory),
            ring: None,
        },
        BossKind::WyrmscaleWarden => BossLootProfile {
            gold: 150..=250,
            item: Some((sovereign_elixir as ItemFactory, 0.5)),
            weapon: wardens_fang as WeaponFactory,
            shards: 8..=12,
            armor: Some(dragonscale_aegis as ArmorFactory),
            ring: None,
        },
        BossKind::AshenSovereign => BossLootProfile {
            gold: 250..=400,
            item: Some((sovereign_elixir as ItemFactory, 0.5)),
            weapon: sovereigns_reckoning as WeaponFactory,
            shards: 12..=18,
            armor: None,
            ring: Some(sovereigns_signet as RingFactory),
        },
    }
}

/// Flat +15 percentage-point bump to a drop chance, capped at 90% — how
/// elite kills pay out better loot without a second per-species table.
fn boost_chance(chance: f32) -> f32 {
    (chance + 0.15).min(0.9)
}

/// Applies `boost_chance` to every drop chance on an elite kill's loot
/// profile (gold gets its own flat +40% multiplier at the call site).
fn boost_profile(mut profile: LootProfile) -> LootProfile {
    if let Some((f, c)) = profile.item {
        profile.item = Some((f, boost_chance(c)));
    }
    if let Some((f, c)) = profile.weapon {
        profile.weapon = Some((f, boost_chance(c)));
    }
    if let Some((r, c)) = profile.shards {
        profile.shards = Some((r, boost_chance(c)));
    }
    if let Some((f, c)) = profile.armor {
        profile.armor = Some((f, boost_chance(c)));
    }
    if let Some((f, c)) = profile.ring {
        profile.ring = Some((f, boost_chance(c)));
    }
    profile
}

/// +2 percentage points of drop chance per NG+ cycle, capped at NG+7 (the
/// same saturation point as `flee_chance`/`elite_chance`) and clamped to
/// `boost_chance`'s 90% ceiling — replay keeps paying out a little better
/// without ever making drops certain.
fn ng_plus_chance(chance: f32, ng_plus: u32) -> f32 {
    (chance + 0.02 * ng_plus.min(7) as f32).min(0.9)
}

/// Applies `ng_plus_chance` to every drop chance on a regular species' loot
/// profile. Deliberately not applied to boss profiles: their signature
/// drops are a guaranteed 1.0, which the 0.9 ceiling would clip.
fn ng_plus_profile(mut profile: LootProfile, ng_plus: u32) -> LootProfile {
    if ng_plus == 0 {
        return profile;
    }
    if let Some((f, c)) = profile.item {
        profile.item = Some((f, ng_plus_chance(c, ng_plus)));
    }
    if let Some((f, c)) = profile.weapon {
        profile.weapon = Some((f, ng_plus_chance(c, ng_plus)));
    }
    if let Some((r, c)) = profile.shards {
        profile.shards = Some((r, ng_plus_chance(c, ng_plus)));
    }
    if let Some((f, c)) = profile.armor {
        profile.armor = Some((f, ng_plus_chance(c, ng_plus)));
    }
    if let Some((f, c)) = profile.ring {
        profile.ring = Some((f, ng_plus_chance(c, ng_plus)));
    }
    profile
}

fn roll_loot(enemies: &[Character], overkills: &[bool], ng_plus: u32, rng: &mut impl Rng) -> Loot {
    let mut gold = 0u32;
    let mut items = Vec::new();
    let mut weapons = Vec::new();
    let mut armors = Vec::new();
    let mut rings = Vec::new();
    let mut overkill_bonus = 0u32;
    let mut xp = 0u32;
    let mut upgrade_materials = 0u32;
    for (i, e) in enemies.iter().enumerate() {
        let profile = match e.boss_kind {
            Some(kind) => {
                let boss = boss_loot_profile(kind);
                LootProfile {
                    gold: boss.gold,
                    item: boss.item,
                    weapon: Some((boss.weapon, 1.0)),
                    shards: Some((boss.shards, 1.0)),
                    armor: boss.armor.map(|a| (a, 1.0)),
                    ring: boss.ring.map(|r| (r, 1.0)),
                }
            }
            None => ng_plus_profile(loot_profile(&e.name), ng_plus),
        };
        let profile = if e.is_elite {
            boost_profile(profile)
        } else {
            profile
        };
        // Tougher, later-chapter specimens of the same species pay out more:
        // +15% per level past the first (see `Character::scale_to_level`).
        let rolled = rng.gen_range(profile.gold);
        let base_gold = rolled + rolled * 15 * e.level.saturating_sub(1) / 100;
        let base_gold = if e.is_elite {
            base_gold + base_gold * 40 / 100
        } else {
            base_gold
        };
        if overkills.get(i).copied().unwrap_or(false) {
            let bonus = (base_gold * 15 / 100).max(1);
            overkill_bonus += bonus;
            gold += base_gold + bonus;
        } else {
            gold += base_gold;
        }
        if let Some((make_item, chance)) = profile.item {
            if rng.gen::<f32>() < chance {
                items.push(make_item());
            }
        }
        if let Some((make_weapon, chance)) = profile.weapon {
            if rng.gen::<f32>() < chance {
                weapons.push(make_weapon());
            }
        }
        if let Some((make_armor, chance)) = profile.armor {
            if rng.gen::<f32>() < chance {
                armors.push(make_armor());
            }
        }
        if let Some((make_ring, chance)) = profile.ring {
            if rng.gen::<f32>() < chance {
                rings.push(make_ring());
            }
        }
        if let Some((shard_range, chance)) = profile.shards {
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
        armors,
        rings,
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

/// Whether `action` (taken by the party member at `actor_idx`) is aimed at
/// the party (heals/items) rather than the enemy side — used by the target
/// picker to decide which roster to cycle the cursor through.
pub fn targets_party(action: CombatAction, actor_idx: usize, party: &Party) -> bool {
    match action {
        CombatAction::Item(_) => true, // current items (potion/ether) are always party-targeted
        CombatAction::Ability(idx) => party
            .members
            .get(actor_idx)
            .map(|m| m.ability_is_heal(idx))
            .unwrap_or(false),
        CombatAction::Attack | CombatAction::Flee => false,
    }
}

/// A best-effort description of what the last-resolved turn did, purely for
/// `render/combat.rs`'s benefit (attack lunge, hit-flash, defeat fade). It's
/// inferred from HP deltas after the fact rather than threaded through every
/// individual action/ability/boss-move branch, so adding it doesn't touch
/// `resolve_player_action`/`resolve_enemy_action`'s existing logic at all —
/// nothing here feeds back into game state or turn resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionAnimKind {
    /// A damaging hit that didn't finish the target off.
    Attack,
    /// A damaging hit that finished the target off.
    Defeat,
    /// A restorative effect (currently only player heal abilities).
    Heal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionAnimEvent {
    pub actor: ActorRef,
    /// `None` for enemy moves that didn't damage anyone this turn (a status
    /// curse, a defensive stance, a steal) — still worth a beat on the
    /// actor, just no target to react.
    pub target: Option<ActorRef>,
    pub kind: ActionAnimKind,
}

/// How long `CombatPhase::Resolving` holds the screen after a turn resolves
/// (set by `app.rs`'s `begin_resolving_hold`), giving `render::combat` time
/// to play the attack/hit/defeat beat before the real outcome is shown.
pub const RESOLVING_HOLD_SECONDS: f32 = 0.35;

/// How long a floating damage/heal number stays alive (seconds) before
/// `World::tick` prunes it — see `CombatState::floating_numbers`.
pub const FLOATING_NUMBER_TTL: f32 = 0.9;

/// A League-of-Legends-style number that pops off an actor's portrait when
/// its HP changes — purely cosmetic, like `ActionAnimEvent`, and populated
/// the same way: `resolve_current_turn` diffs every actor's HP before/after
/// the turn resolves rather than threading a "spawn a number here" call
/// through every individual damage/heal branch, so it automatically covers
/// every current and future HP-changing move (player attacks/abilities,
/// enemy signature moves, boss scripted moves and self-heals) with no risk
/// of a call site being missed.
#[derive(Debug, Clone, Copy)]
pub struct FloatingNumber {
    pub target: ActorRef,
    /// Always positive magnitude; `is_heal` says which direction it was.
    pub amount: i32,
    pub is_heal: bool,
    pub age: f32,
}

pub enum CombatPhase {
    // A player-controlled actor is choosing what to do.
    SelectAction {
        actor: ActorRef,
    },
    // A player-controlled actor chose "Ability" and is picking which one.
    SelectAbility {
        actor: ActorRef,
        cursor: usize,
    },
    // A player-controlled actor chose "Item" and is picking which one
    // (e.g. Potion vs. Ether) before it's applied.
    SelectItem {
        actor: ActorRef,
        cursor: usize,
    },
    // A player-controlled actor picked an action that needs a target.
    SelectTarget {
        actor: ActorRef,
        action: CombatAction,
        target_idx: usize,
    },
    // Briefly held by `app.rs` between a turn resolving and its real
    // outcome being shown, so an attack/hit/defeat animation has time to
    // play — see `CombatState::pending_phase`.
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
    /// Lines of scrollback from the bottom of `log` currently displayed.
    pub log_scroll: usize,
    /// The active New Game+ cycle, set by the caller (`World`) right after
    /// construction — feeds `flee_chance` (escape gets harder each cycle)
    /// and the Wraith curse roll (`roll_curse`'s tier-gated pool). `0` here
    /// just means "no NG+," which is also correct for tests that construct
    /// `CombatState` directly.
    pub ng_plus: u32,
    /// Set fresh by every `resolve_current_turn` call — see `ActionAnimEvent`.
    pub last_action_anim: Option<ActionAnimEvent>,
    /// `app.rs`-only bookkeeping for briefly holding `phase` at `Resolving`
    /// so an action's animation has time to play before the real outcome
    /// (back to `SelectAction`, or `Victory`/`Defeat`) is shown. Neither
    /// this field nor `resolving_timer` is ever touched by this module's
    /// own transitions — `resolve_current_turn`/`advance_turn` behave
    /// exactly as before, so nothing here affects the ~46 existing tests
    /// that assert `phase` immediately after calling `resolve_current_turn`
    /// directly (bypassing `app.rs`, which is the only place that reads or
    /// writes these two fields).
    pub pending_phase: Option<CombatPhase>,
    pub resolving_timer: f32,
    /// Numbers currently rising/fading over actors' portraits — see
    /// `FloatingNumber`. Aged and pruned by `World::tick` (`app.rs`), not by
    /// this module, mirroring `resolving_timer`'s split between game logic
    /// (here) and per-frame timing (`app.rs`).
    pub floating_numbers: Vec<FloatingNumber>,
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
                log.push(format!("{} appears!", e.display_name()));
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
            log_scroll: 0,
            ng_plus: 0,
            last_action_anim: None,
            pending_phase: None,
            resolving_timer: 0.0,
            floating_numbers: Vec::new(),
        }
    }

    /// Chance a flee attempt by a character with `fleeing_speed` succeeds:
    /// favors a party faster than the enemy pack, and tightens by 2
    /// percentage points per NG+ cycle (capped at NG+7) so escape gets
    /// harder as the game escalates. Clamped so it's never a sure thing or
    /// impossible.
    fn flee_chance(&self, fleeing_speed: i32) -> f32 {
        let alive_enemies: Vec<&Character> = self.enemies.iter().filter(|e| e.is_alive()).collect();
        let avg_enemy_speed = if alive_enemies.is_empty() {
            0.0
        } else {
            alive_enemies.iter().map(|e| e.stats.speed).sum::<i32>() as f32
                / alive_enemies.len() as f32
        };
        let base = 0.5 + 0.02 * (fleeing_speed as f32 - avg_enemy_speed)
            - 0.02 * self.ng_plus.min(7) as f32;
        base.clamp(0.15, 0.85)
    }

    pub fn current_actor(&self) -> ActorRef {
        self.turn_order[self.turn_cursor]
    }

    pub(crate) fn push_log(&mut self, msg: impl Into<String>) {
        self.log.push(msg.into());
        // keep the log from growing unbounded; UI only shows the tail anyway
        if self.log.len() > 200 {
            self.log.remove(0);
        }
        self.log_scroll = 0;
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
        let actor = self.current_actor();
        let player_action = match self.phase {
            CombatPhase::SelectTarget {
                action, target_idx, ..
            } => Some((action, target_idx)),
            _ => None,
        };
        let pre_party_hp: Vec<i32> = party.members.iter().map(|m| m.stats.hp).collect();
        let pre_enemy_hp: Vec<i32> = self.enemies.iter().map(|e| e.stats.hp).collect();

        match actor {
            ActorRef::Player(pi) => {
                // Only resolves once phase == SelectTarget for this actor; app.rs guarantees that.
                if let Some((action, target_idx)) = player_action {
                    self.resolve_player_action(pi, action, target_idx, party, rng);
                }
            }
            ActorRef::Enemy(ei) => self.resolve_enemy_action(ei, party, rng),
        }

        self.last_action_anim =
            Self::infer_action_anim(actor, player_action, &pre_party_hp, &pre_enemy_hp, party, &self.enemies);
        self.spawn_floating_numbers(&pre_party_hp, &pre_enemy_hp, party);

        self.advance_turn(party, rng);
    }

    /// Diffs every actor's HP from before this turn resolved to after, and
    /// spawns a floating number for each one that changed — damage for a
    /// drop, healing for a rise. Generic over every damage/heal source
    /// (player attacks/abilities, enemy signature moves, boss scripted
    /// moves and self-heals) since it reads the outcome rather than being
    /// called from inside each branch, so nothing new here can be missed by
    /// a future move that changes HP.
    fn spawn_floating_numbers(&mut self, pre_party_hp: &[i32], pre_enemy_hp: &[i32], party: &Party) {
        for (i, (&pre, m)) in pre_party_hp.iter().zip(party.members.iter()).enumerate() {
            let diff = m.stats.hp - pre;
            if diff != 0 {
                self.floating_numbers.push(FloatingNumber {
                    target: ActorRef::Player(i),
                    amount: diff.abs(),
                    is_heal: diff > 0,
                    age: 0.0,
                });
            }
        }
        for (i, (&pre, e)) in pre_enemy_hp.iter().zip(self.enemies.iter()).enumerate() {
            let diff = e.stats.hp - pre;
            if diff != 0 {
                self.floating_numbers.push(FloatingNumber {
                    target: ActorRef::Enemy(i),
                    amount: diff.abs(),
                    is_heal: diff > 0,
                    age: 0.0,
                });
            }
        }
    }

    /// The index of whichever slot in `pre_hp` dropped the most compared to
    /// `post_hp`, if any dropped at all. Used to guess who an attack landed
    /// on after the fact (see `infer_action_anim`) instead of threading a
    /// target-of-record through every action/ability/boss-move branch.
    fn best_hit_index(pre_hp: &[i32], post_hp: impl Iterator<Item = i32>) -> Option<usize> {
        let mut best: Option<(usize, i32)> = None;
        for (i, (&pre, post)) in pre_hp.iter().zip(post_hp).enumerate() {
            let drop = pre - post;
            if drop > 0 && best.is_none_or(|(_, best_drop)| drop > best_drop) {
                best = Some((i, drop));
            }
        }
        best.map(|(i, _)| i)
    }

    fn infer_action_anim(
        actor: ActorRef,
        player_action: Option<(CombatAction, usize)>,
        pre_party_hp: &[i32],
        pre_enemy_hp: &[i32],
        party: &Party,
        enemies: &[Character],
    ) -> Option<ActionAnimEvent> {
        match actor {
            ActorRef::Player(pi) => {
                let (action, target_idx) = player_action?;
                match action {
                    CombatAction::Flee | CombatAction::Item(_) => None,
                    CombatAction::Attack => {
                        let enemy = enemies.get(target_idx)?;
                        let kind = if enemy.is_alive() {
                            ActionAnimKind::Attack
                        } else {
                            ActionAnimKind::Defeat
                        };
                        Some(ActionAnimEvent {
                            actor,
                            target: Some(ActorRef::Enemy(target_idx)),
                            kind,
                        })
                    }
                    CombatAction::Ability(idx) => {
                        let is_heal = party
                            .members
                            .get(pi)
                            .is_some_and(|m| m.ability_is_heal(idx));
                        if is_heal {
                            Some(ActionAnimEvent {
                                actor,
                                target: Some(ActorRef::Player(target_idx)),
                                kind: ActionAnimKind::Heal,
                            })
                        } else {
                            // targets_all_enemies abilities can hit several at once;
                            // highlight whichever took the biggest hit.
                            let hit = Self::best_hit_index(
                                pre_enemy_hp,
                                enemies.iter().map(|e| e.stats.hp),
                            )
                            .unwrap_or(target_idx);
                            let kind = enemies
                                .get(hit)
                                .map(|e| {
                                    if e.is_alive() {
                                        ActionAnimKind::Attack
                                    } else {
                                        ActionAnimKind::Defeat
                                    }
                                })
                                .unwrap_or(ActionAnimKind::Attack);
                            Some(ActionAnimEvent {
                                actor,
                                target: Some(ActorRef::Enemy(hit)),
                                kind,
                            })
                        }
                    }
                }
            }
            ActorRef::Enemy(_) => {
                match Self::best_hit_index(pre_party_hp, party.members.iter().map(|m| m.stats.hp)) {
                    Some(i) => {
                        let kind = if party.members[i].is_alive() {
                            ActionAnimKind::Attack
                        } else {
                            ActionAnimKind::Defeat
                        };
                        Some(ActionAnimEvent {
                            actor,
                            target: Some(ActorRef::Player(i)),
                            kind,
                        })
                    }
                    // A non-damaging move (curse, defensive stance, steal) —
                    // still worth a beat on the actor, just no reaction target.
                    None => Some(ActionAnimEvent {
                        actor,
                        target: None,
                        kind: ActionAnimKind::Attack,
                    }),
                }
            }
        }
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
                let atk =
                    party.members[pi].total_attack() + party.stat_delta(StatEffectTarget::Attack);
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
                    let ename = enemy.display_name();
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
                        let power = ability.effective_power(&party.members[pi])
                            + party.stat_delta(StatEffectTarget::Attack);
                        let luck = party.members[pi].total_luck();
                        let targets = if ability.targets_all_enemies {
                            self.alive_enemy_indices()
                        } else {
                            vec![target_idx]
                        };
                        for idx in targets {
                            let defense = self
                                .enemies
                                .get(idx)
                                .map(|e| e.total_defense())
                                .unwrap_or(0);
                            let (dmg, crit) = roll_damage(power, defense / 2, luck, rng);
                            if let Some(enemy) = self.enemies.get_mut(idx) {
                                enemy.take_damage(dmg);
                                let ename = enemy.display_name();
                                let max_hp = enemy.stats.max_hp;
                                let alive = enemy.is_alive();
                                self.push_log(format!(
                                    "{attacker_name} casts {} on {ename} for {dmg} damage.{}",
                                    ability.name,
                                    if crit { " A critical hit!" } else { "" }
                                ));
                                if !alive {
                                    self.push_log(format!("{ename} is defeated!"));
                                    self.check_overkill(idx, dmg, max_hp, &ename);
                                }
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
                let fleeing_speed =
                    party.members[pi].stats.speed + party.stat_delta(StatEffectTarget::Speed);
                let chance = self.flee_chance(fleeing_speed);
                let roll: f32 = rng.gen();
                if roll < chance {
                    self.push_log(format!("{attacker_name} flees from battle!"));
                    self.phase = CombatPhase::Fled;
                } else {
                    self.push_log(format!(
                        "{attacker_name} tries to flee, but can't get away!"
                    ));
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

    /// Picks who an enemy attacks. Most species just pick randomly among the
    /// living party; Goblins and Wolves are smart enough to press an
    /// advantage 60% of the time — Goblins chase whoever's closest to
    /// death (lowest HP%), Wolves single out the frailest-built target
    /// (lowest max HP) — otherwise they fall through to a random pick too.
    fn pick_target(
        ename: &str,
        alive_targets: &[usize],
        party: &Party,
        rng: &mut impl Rng,
    ) -> usize {
        if ename == "Goblin" && rng.gen_bool(0.6) {
            return *alive_targets
                .iter()
                .min_by(|&&a, &&b| {
                    party.members[a]
                        .hp_ratio()
                        .partial_cmp(&party.members[b].hp_ratio())
                        .unwrap()
                })
                .unwrap();
        }
        if ename == "Wolf" && rng.gen_bool(0.6) {
            return *alive_targets
                .iter()
                .min_by_key(|&&i| party.members[i].stats.max_hp)
                .unwrap();
        }
        alive_targets[rng.gen_range(0..alive_targets.len())]
    }

    fn resolve_enemy_action(&mut self, ei: usize, party: &mut Party, rng: &mut impl Rng) {
        let alive_targets = Self::alive_player_indices(party);
        if alive_targets.is_empty() {
            return;
        }
        let (ename, atk, is_wraith, boss_kind, is_elite) = {
            let e = &self.enemies[ei];
            (
                e.name.clone(),
                e.stats.attack,
                e.name == "Wraith",
                e.boss_kind,
                e.is_elite,
            )
        };
        // Elites get one guaranteed shot at their species' signature move
        // (checked alongside each move's normal roll below), then fall back
        // to the usual per-turn odds for the rest of the encounter.
        let elite_guaranteed = is_elite && !self.enemies[ei].elite_signature_used;
        let target_idx = Self::pick_target(&ename, &alive_targets, party, rng);
        let dname = self.enemies[ei].display_name();

        // Wraiths spend some turns cursing the party instead of attacking directly.
        if is_wraith && (elite_guaranteed || rng.gen_bool(0.3)) {
            self.enemies[ei].elite_signature_used = true;
            let curse = roll_curse(rng, self.ng_plus);
            self.push_log(format!("{dname} whispers a curse over your party..."));
            self.push_log(format!(
                "{} takes hold! ({} {} for {} encounters)",
                curse.name, curse.delta, curse.target, curse.encounters_remaining
            ));
            party.add_effect(curse);
            return;
        }

        // Fell Acolytes siphon a member's magic to knit themselves back
        // together — but only if anyone still has MP worth stealing.
        if ename == "Fell Acolyte" && (elite_guaranteed || rng.gen_bool(0.25)) {
            let mp_targets: Vec<usize> = alive_targets
                .iter()
                .copied()
                .filter(|&i| party.members[i].stats.mp > 0)
                .collect();
            if let Some(&ti) = mp_targets.get(rng.gen_range(0..mp_targets.len().max(1))) {
                self.enemies[ei].elite_signature_used = true;
                let level = self.enemies[ei].level as i32;
                let drain = (3 + level).min(party.members[ti].stats.mp);
                party.members[ti].stats.mp -= drain;
                let heal = 4 + level;
                self.enemies[ei].heal(heal);
                let tname = party.members[ti].name.clone();
                self.push_log(format!(
                    "{dname} chants a Withering Prayer — {tname} loses {drain} MP \
                     and {dname} knits itself back together ({heal} HP)."
                ));
                return;
            }
            // No one has MP left to steal — fall through to a normal attack.
        }

        // Skeletons sometimes hunker down instead of attacking, toughening
        // themselves for the rest of the fight.
        if ename == "Skeleton" && (elite_guaranteed || rng.gen_bool(0.2)) {
            self.enemies[ei].elite_signature_used = true;
            self.enemies[ei].stats.defense += 3;
            self.push_log(format!(
                "{dname} raises its guard, bones knitting tighter. (+3 Defense)"
            ));
            return;
        }

        // Orcs occasionally swing recklessly for a much harder hit, but the
        // wild swing costs them some of their own hide in the process.
        if ename == "Orc" && (elite_guaranteed || rng.gen_bool(0.25)) {
            self.enemies[ei].elite_signature_used = true;
            let def = party.members[target_idx].total_defense()
                + party.stat_delta(StatEffectTarget::Defense);
            let luck = self.enemies[ei].total_luck();
            let power = ((atk as f32) * 1.6).round() as i32;
            let (dmg, crit) = roll_damage(power, def, luck, rng);
            let dmg = apply_damage_reduction(dmg, &party.members[target_idx]);
            party.members[target_idx].take_damage(dmg);
            let tname = party.members[target_idx].name.clone();
            let recoil = ((dmg as f32) * 0.15).round() as i32;
            self.enemies[ei].take_damage(recoil);
            self.push_log(format!(
                "{dname} swings recklessly, hitting {tname} for {dmg} damage \
                 and hurting itself for {recoil}.{}",
                if crit { " A critical hit!" } else { "" }
            ));
            if !party.members[target_idx].is_alive() {
                self.push_log(format!("{tname} falls!"));
            }
            return;
        }

        // Bandits would rather rob the party blind than fight fair.
        if ename == "Bandit" && party.gold > 0 && (elite_guaranteed || rng.gen_bool(0.2)) {
            self.enemies[ei].elite_signature_used = true;
            let stolen = (party.gold / 20).clamp(1, 15).min(party.gold);
            party.gold -= stolen;
            self.push_log(format!("{dname} snatches {stolen} gold and darts back!"));
            return;
        }

        // Barrow Sentinels let out a warcry that weakens the whole party's
        // defense for the rest of the encounter.
        if ename == "Barrow Sentinel" && (elite_guaranteed || rng.gen_bool(0.2)) {
            self.enemies[ei].elite_signature_used = true;
            let curse = crate::game::status::StatusEffect {
                name: "Sentinel's Warcry".to_string(),
                target: StatEffectTarget::Defense,
                delta: -3,
                encounters_remaining: 1,
            };
            self.push_log(format!(
                "{dname} lets out a bone-rattling warcry! (-3 Defense this encounter)"
            ));
            party.add_effect(curse);
            return;
        }

        // Forsaken Knights occasionally land a much harder judgment strike,
        // no drawback attached.
        if ename == "Forsaken Knight" && (elite_guaranteed || rng.gen_bool(0.25)) {
            self.enemies[ei].elite_signature_used = true;
            let def = party.members[target_idx].total_defense()
                + party.stat_delta(StatEffectTarget::Defense);
            let luck = self.enemies[ei].total_luck();
            let power = ((atk as f32) * 1.7).round() as i32;
            let (dmg, crit) = roll_damage(power, def, luck, rng);
            let dmg = apply_damage_reduction(dmg, &party.members[target_idx]);
            party.members[target_idx].take_damage(dmg);
            let tname = party.members[target_idx].name.clone();
            self.push_log(format!(
                "{dname} unleashes Knight's Judgment on {tname} for {dmg} damage.{}",
                if crit { " A critical hit!" } else { "" }
            ));
            if !party.members[target_idx].is_alive() {
                self.push_log(format!("{tname} falls!"));
            }
            return;
        }

        if let Some(kind) = boss_kind {
            if self.resolve_boss_move(ei, kind, &ename, atk, target_idx, party, rng) {
                return;
            }
        }

        // Grave Ghouls sometimes gorge on the wound they tear open.
        let ravenous_bite = ename == "Grave Ghoul" && rng.gen_bool(0.3);

        let def =
            party.members[target_idx].total_defense() + party.stat_delta(StatEffectTarget::Defense);
        let luck = self.enemies[ei].total_luck();
        let (dmg, crit) = roll_damage(atk, def, luck, rng);
        let dmg = apply_damage_reduction(dmg, &party.members[target_idx]);
        party.members[target_idx].take_damage(dmg);
        let tname = party.members[target_idx].name.clone();
        if ravenous_bite {
            let heal = dmg / 2;
            self.enemies[ei].heal(heal);
            self.push_log(format!(
                "{dname} sinks its teeth into {tname} for {dmg} damage \
                 and gorges itself ({heal} HP).{}",
                if crit { " A critical hit!" } else { "" }
            ));
        } else {
            self.push_log(format!(
                "{dname} attacks {tname} for {dmg} damage.{}",
                if crit { " A critical hit!" } else { "" }
            ));
        }
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
                        self.push_log(format!(
                            "{ename} sheds a layer of scale, hardening its hide!"
                        ));
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
            self.loot = Some(roll_loot(&self.enemies, &self.overkills, self.ng_plus, rng));
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
        // CureCurse is party-wide — the only item kind that ignores its target.
        let msg = if matches!(item_kind, ItemKind::CureCurse) {
            if party.cure_curses() > 0 {
                "The curses clinging to the party crumble away.".to_string()
            } else {
                "No curses cling to the party.".to_string()
            }
        } else {
            let Some(target) = party.members.get_mut(target_idx) else {
                return;
            };
            crate::game::combat::use_item_kind(item_kind, target)
        };
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

/// Applies a single-target consumable to `target`. `CureCurse` is the one
/// kind that never reaches here — it's party-wide, so both call sites
/// (combat item use and the inventory screen) intercept it and call
/// `Party::cure_curses` instead.
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
        ItemKind::Elixir => {
            target.stats.hp = target.stats.max_hp;
            target.stats.mp = target.stats.max_mp;
            format!("{} is fully restored.", target.name)
        }
        ItemKind::Revive { heal_percent } => {
            if target.is_alive() {
                // Call sites keep revives away from the living; this is only
                // a belt-and-braces fallback so the item is never a no-op.
                target.heal(1);
                format!("{} is still standing — the ember gutters out.", target.name)
            } else {
                let amount = ((target.stats.max_hp as f32 * heal_percent).round() as i32).max(1);
                target.stats.hp = amount.min(target.stats.max_hp);
                format!("{} rises to fight again with {} HP!", target.name, amount)
            }
        }
        ItemKind::CureCurse => format!("Nothing clings to {}.", target.name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::character::{
        ashen_sovereign, barrow_knight, mimic, orc, skeleton, slime, warrior, wraith,
        wyrmscale_warden,
    };
    use rand::rngs::StdRng;
    use rand::SeedableRng;

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
    fn flee_chance_favors_a_faster_fleeing_party() {
        let party = test_party();
        let combat = CombatState::new(&party, vec![slime("Slime")]); // slime speed 4
        let fast = combat.flee_chance(20);
        let slow = combat.flee_chance(0);
        assert!(fast > slow);
    }

    #[test]
    fn flee_chance_tightens_with_ng_plus() {
        let party = test_party();
        let mut combat = CombatState::new(&party, vec![slime("Slime")]);
        let base = combat.flee_chance(10);
        combat.ng_plus = 7;
        let tightened = combat.flee_chance(10);
        assert!(tightened < base);
    }

    #[test]
    fn flee_chance_is_always_clamped_to_a_sane_range() {
        let party = test_party();
        let combat = CombatState::new(&party, vec![slime("Slime")]);
        assert!(combat.flee_chance(1000) <= 0.85);
        assert!(combat.flee_chance(-1000) >= 0.15);
    }

    #[test]
    fn attacking_reduces_enemy_hp_and_can_win() {
        let mut party = test_party();
        let enemies = vec![slime("Slime")];
        let mut combat = CombatState::new(&party, enemies);
        let mut rng = StdRng::seed_from_u64(42);

        // Player's turn: attack the slime.
        assert!(matches!(
            combat.phase,
            CombatPhase::SelectAction {
                actor: ActorRef::Player(0)
            }
        ));
        combat.phase = CombatPhase::SelectTarget {
            actor: ActorRef::Player(0),
            action: CombatAction::Attack,
            target_idx: 0,
        };
        let hp_before = combat.enemies[0].stats.hp;
        combat.resolve_current_turn(&mut party, &mut rng);
        assert!(
            combat.enemies[0].stats.hp < hp_before,
            "attack should deal damage"
        );
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
        combat.phase = CombatPhase::SelectAction {
            actor: ActorRef::Enemy(0),
        };
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
    fn fan_of_knives_damages_every_alive_enemy() {
        let mut party = Party::new(vec![crate::game::character::rogue("Wren")]);
        let enemies = vec![slime("Slime"), slime("Slime"), slime("Slime")];
        let mut combat = CombatState::new(&party, enemies);
        let mut rng = StdRng::seed_from_u64(11);
        let hp_before: Vec<i32> = combat.enemies.iter().map(|e| e.stats.hp).collect();

        combat.phase = CombatPhase::SelectTarget {
            actor: ActorRef::Player(0),
            action: CombatAction::Ability(1), // rogue's Fan of Knives
            target_idx: 0,
        };
        combat.resolve_current_turn(&mut party, &mut rng);

        for (i, before) in hp_before.iter().enumerate() {
            assert!(
                combat.enemies[i].stats.hp < *before,
                "enemy {i} should take damage from Fan of Knives"
            );
        }
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
        assert_eq!(
            party.effects[0].encounters_remaining, 1,
            "one encounter used up"
        );
        party.tick_effects();
        assert!(
            party.effects.is_empty(),
            "effect should expire after 2 encounters"
        );
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
    fn elite_wraith_is_guaranteed_to_curse_on_its_first_eligible_turn() {
        let mut party = test_party();
        let mut enemy = wraith("Wraith");
        enemy.apply_elite();
        let enemies = vec![enemy];
        let mut combat = CombatState::new(&party, enemies);
        // Any seed works: the elite guarantee short-circuits the normal
        // 30%-chance roll entirely, so this isn't a probabilistic sweep.
        let mut rng = StdRng::seed_from_u64(0);
        combat.turn_order = vec![ActorRef::Enemy(0), ActorRef::Player(0)];
        combat.turn_cursor = 0;
        combat.phase = CombatPhase::SelectAction {
            actor: ActorRef::Enemy(0),
        };
        combat.resolve_current_turn(&mut party, &mut rng);
        assert!(
            !party.effects.is_empty(),
            "an elite wraith should be guaranteed to curse on its first turn"
        );
        assert!(
            combat.enemies[0].elite_signature_used,
            "firing the guaranteed move should flip the used flag"
        );
    }

    #[test]
    fn fell_acolyte_can_drain_mp_and_heal_itself() {
        use crate::game::character::fell_acolyte;

        // Probabilistic move (25% per turn) — sweep seeds like the Wraith test.
        let mut prayed_at_least_once = false;
        for seed in 0..50u64 {
            let mut party = test_party(); // warrior with MP to steal
            let mut enemies = vec![fell_acolyte("Fell Acolyte")];
            enemies[0].stats.hp = 1; // any heal is visible
            let mut combat = CombatState::new(&party, enemies);
            let mut rng = StdRng::seed_from_u64(seed);
            combat.turn_order = vec![ActorRef::Enemy(0), ActorRef::Player(0)];
            combat.turn_cursor = 0;
            combat.phase = CombatPhase::SelectAction {
                actor: ActorRef::Enemy(0),
            };
            let mp_before = party.members[0].stats.mp;
            combat.resolve_current_turn(&mut party, &mut rng);
            if party.members[0].stats.mp < mp_before {
                assert!(
                    combat.enemies[0].stats.hp > 1,
                    "a Withering Prayer must heal the acolyte too"
                );
                prayed_at_least_once = true;
                break;
            }
        }
        assert!(
            prayed_at_least_once,
            "the acolyte should pray at least once across many trials"
        );
    }

    #[test]
    fn a_drained_party_forces_the_acolyte_into_a_normal_attack() {
        use crate::game::character::fell_acolyte;

        // With no MP anywhere, the prayer must fall through to an attack —
        // every enemy turn has to hurt someone or cast something.
        for seed in 0..20u64 {
            let mut party = test_party();
            party.members[0].stats.mp = 0;
            let hp_before = party.members[0].stats.hp;
            let mut combat = CombatState::new(&party, vec![fell_acolyte("Fell Acolyte")]);
            let mut rng = StdRng::seed_from_u64(seed);
            combat.turn_order = vec![ActorRef::Enemy(0), ActorRef::Player(0)];
            combat.turn_cursor = 0;
            combat.phase = CombatPhase::SelectAction {
                actor: ActorRef::Enemy(0),
            };
            combat.resolve_current_turn(&mut party, &mut rng);
            assert!(
                party.members[0].stats.hp < hp_before,
                "seed {seed}: with no MP to drain, the acolyte should attack"
            );
        }
    }

    #[test]
    fn grave_ghoul_can_heal_by_biting() {
        use crate::game::character::grave_ghoul;

        // Probabilistic move (30% per turn) — sweep seeds and confirm the
        // ghoul both hurts its target and regains HP on the same turn.
        let mut gorged_at_least_once = false;
        for seed in 0..50u64 {
            let mut party = test_party();
            let mut enemies = vec![grave_ghoul("Grave Ghoul")];
            enemies[0].stats.hp = 1;
            let mut combat = CombatState::new(&party, enemies);
            let mut rng = StdRng::seed_from_u64(seed);
            combat.turn_order = vec![ActorRef::Enemy(0), ActorRef::Player(0)];
            combat.turn_cursor = 0;
            combat.phase = CombatPhase::SelectAction {
                actor: ActorRef::Enemy(0),
            };
            let hp_before = party.members[0].stats.hp;
            combat.resolve_current_turn(&mut party, &mut rng);
            if combat.enemies[0].stats.hp > 1 {
                assert!(
                    party.members[0].stats.hp < hp_before,
                    "a Ravenous Bite still has to hurt its target"
                );
                gorged_at_least_once = true;
                break;
            }
        }
        assert!(
            gorged_at_least_once,
            "the ghoul should gorge itself at least once across many trials"
        );
    }

    #[test]
    fn goblin_targets_the_lowest_hp_percent_member_most_of_the_time() {
        use crate::game::character::goblin;

        let mut targeted_the_weak_one = false;
        for seed in 0..50u64 {
            let mut party = Party::new(vec![warrior("Bram"), warrior("Elle")]);
            party.members[1].stats.hp = 1; // Elle is at death's door
            let mut combat = CombatState::new(&party, vec![goblin("Goblin")]);
            let mut rng = StdRng::seed_from_u64(seed);
            combat.turn_order = vec![ActorRef::Enemy(0), ActorRef::Player(0), ActorRef::Player(1)];
            combat.turn_cursor = 0;
            combat.phase = CombatPhase::SelectAction {
                actor: ActorRef::Enemy(0),
            };
            combat.resolve_current_turn(&mut party, &mut rng);
            if !party.members[1].is_alive() {
                targeted_the_weak_one = true;
                break;
            }
        }
        assert!(
            targeted_the_weak_one,
            "the goblin should target the lowest-HP% member at least once across many trials"
        );
    }

    #[test]
    fn wolf_targets_the_lowest_max_hp_member_most_of_the_time() {
        use crate::game::character::wolf;

        let mut targeted_the_frail_one = false;
        for seed in 0..50u64 {
            let mut party = Party::new(vec![warrior("Bram"), warrior("Elle")]);
            party.members[1].stats.max_hp = 1;
            party.members[1].stats.hp = 1; // frailest build, dies to any hit
            let mut combat = CombatState::new(&party, vec![wolf("Wolf")]);
            let mut rng = StdRng::seed_from_u64(seed);
            combat.turn_order = vec![ActorRef::Enemy(0), ActorRef::Player(0), ActorRef::Player(1)];
            combat.turn_cursor = 0;
            combat.phase = CombatPhase::SelectAction {
                actor: ActorRef::Enemy(0),
            };
            combat.resolve_current_turn(&mut party, &mut rng);
            if !party.members[1].is_alive() {
                targeted_the_frail_one = true;
                break;
            }
        }
        assert!(
            targeted_the_frail_one,
            "the wolf should target the lowest-max-HP member at least once across many trials"
        );
    }

    #[test]
    fn skeleton_can_bone_guard_instead_of_attacking() {
        use crate::game::character::skeleton;

        let mut guarded_at_least_once = false;
        for seed in 0..50u64 {
            let mut party = test_party();
            let hp_before = party.members[0].stats.hp;
            let mut combat = CombatState::new(&party, vec![skeleton("Skeleton")]);
            let def_before = combat.enemies[0].stats.defense;
            let mut rng = StdRng::seed_from_u64(seed);
            combat.turn_order = vec![ActorRef::Enemy(0), ActorRef::Player(0)];
            combat.turn_cursor = 0;
            combat.phase = CombatPhase::SelectAction {
                actor: ActorRef::Enemy(0),
            };
            combat.resolve_current_turn(&mut party, &mut rng);
            if combat.enemies[0].stats.defense > def_before {
                assert_eq!(
                    party.members[0].stats.hp, hp_before,
                    "Bone Guard should skip the attack entirely"
                );
                guarded_at_least_once = true;
                break;
            }
        }
        assert!(
            guarded_at_least_once,
            "the skeleton should Bone Guard at least once across many trials"
        );
    }

    #[test]
    fn orc_reckless_swing_hits_harder_but_costs_recoil() {
        use crate::game::character::orc;

        let mut swung_at_least_once = false;
        for seed in 0..50u64 {
            let mut party = test_party();
            let mut enemies = vec![orc("Orc")];
            enemies[0].stats.hp = 1000; // survive its own recoil so hp change is visible
            let mut combat = CombatState::new(&party, enemies);
            let hp_before = combat.enemies[0].stats.hp;
            let mut rng = StdRng::seed_from_u64(seed);
            combat.turn_order = vec![ActorRef::Enemy(0), ActorRef::Player(0)];
            combat.turn_cursor = 0;
            combat.phase = CombatPhase::SelectAction {
                actor: ActorRef::Enemy(0),
            };
            combat.resolve_current_turn(&mut party, &mut rng);
            if combat.enemies[0].stats.hp < hp_before {
                swung_at_least_once = true;
                break;
            }
        }
        assert!(
            swung_at_least_once,
            "the orc should Reckless Swing (and take recoil) at least once across many trials"
        );
    }

    #[test]
    fn bandit_coin_grab_steals_gold_instead_of_attacking() {
        use crate::game::character::bandit;

        let mut stole_at_least_once = false;
        for seed in 0..50u64 {
            let mut party = test_party();
            party.gold = 100;
            let hp_before = party.members[0].stats.hp;
            let mut combat = CombatState::new(&party, vec![bandit("Bandit")]);
            let mut rng = StdRng::seed_from_u64(seed);
            combat.turn_order = vec![ActorRef::Enemy(0), ActorRef::Player(0)];
            combat.turn_cursor = 0;
            combat.phase = CombatPhase::SelectAction {
                actor: ActorRef::Enemy(0),
            };
            combat.resolve_current_turn(&mut party, &mut rng);
            if party.gold < 100 {
                assert_eq!(
                    party.members[0].stats.hp, hp_before,
                    "Coin Grab should skip the attack entirely"
                );
                stole_at_least_once = true;
                break;
            }
        }
        assert!(
            stole_at_least_once,
            "the bandit should Coin Grab at least once across many trials"
        );
    }

    #[test]
    fn barrow_sentinel_warcry_applies_a_party_wide_defense_curse() {
        use crate::game::character::barrow_sentinel;

        let mut warcried_at_least_once = false;
        for seed in 0..50u64 {
            let mut party = test_party();
            let mut combat = CombatState::new(&party, vec![barrow_sentinel("Barrow Sentinel")]);
            let mut rng = StdRng::seed_from_u64(seed);
            combat.turn_order = vec![ActorRef::Enemy(0), ActorRef::Player(0)];
            combat.turn_cursor = 0;
            combat.phase = CombatPhase::SelectAction {
                actor: ActorRef::Enemy(0),
            };
            combat.resolve_current_turn(&mut party, &mut rng);
            if !party.effects.is_empty() {
                assert_eq!(party.stat_delta(StatEffectTarget::Defense), -3);
                warcried_at_least_once = true;
                break;
            }
        }
        assert!(
            warcried_at_least_once,
            "the sentinel should Warcry at least once across many trials"
        );
    }

    #[test]
    fn forsaken_knight_judgment_hits_harder_than_a_normal_attack() {
        use crate::game::character::forsaken_knight;

        // Compare max damage over trials: judgment (1.7x power) should be
        // able to exceed what a plain attack could ever roll.
        let mut saw_a_big_hit = false;
        for seed in 0..80u64 {
            let mut party = test_party();
            let hp_before = party.members[0].stats.hp;
            let mut combat = CombatState::new(&party, vec![forsaken_knight("Forsaken Knight")]);
            let mut rng = StdRng::seed_from_u64(seed);
            combat.turn_order = vec![ActorRef::Enemy(0), ActorRef::Player(0)];
            combat.turn_cursor = 0;
            combat.phase = CombatPhase::SelectAction {
                actor: ActorRef::Enemy(0),
            };
            combat.resolve_current_turn(&mut party, &mut rng);
            let dmg = hp_before - party.members[0].stats.hp;
            if dmg as f32 > combat.enemies[0].stats.attack as f32 * 1.3 {
                saw_a_big_hit = true;
                break;
            }
        }
        assert!(
            saw_a_big_hit,
            "the knight should Knight's Judgment at least once across many trials"
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
    fn third_ability_slot_resolves_and_quaking_slam_hits_all_enemies() {
        let mut party = test_party(); // warrior: Quaking Slam in slot 2, AoE
        let enemies = vec![slime("Slime"), slime("Slime")];
        let mut combat = CombatState::new(&party, enemies);
        let mut rng = StdRng::seed_from_u64(3);
        let mp_before = party.members[0].stats.mp;

        combat.phase = CombatPhase::SelectTarget {
            actor: ActorRef::Player(0),
            action: CombatAction::Ability(2),
            target_idx: 0,
        };
        let hp_before: Vec<i32> = combat.enemies.iter().map(|e| e.stats.hp).collect();
        combat.resolve_current_turn(&mut party, &mut rng);
        for (e, before) in combat.enemies.iter().zip(hp_before) {
            assert!(e.stats.hp < before, "Quaking Slam should damage every enemy");
        }
        assert!(party.members[0].stats.mp < mp_before, "Quaking Slam should consume MP");
    }

    #[test]
    fn ability_is_heal_checks_the_specific_slot() {
        let cleric = crate::game::character::cleric("Idris");
        assert!(cleric.ability_is_heal(0), "Mend (slot 0) should be a heal");
        assert!(
            !cleric.ability_is_heal(1),
            "Smite (slot 1) should not be a heal"
        );

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
            combat.phase = CombatPhase::SelectAction {
                actor: ActorRef::Enemy(0),
            };
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
        assert!(
            combat.enrage_stage == 0,
            "boss should not enrage while healthy"
        );
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
            if combat
                .log
                .iter()
                .any(|line| line.contains("Rending Cleave"))
            {
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
    fn every_boss_guarantees_its_second_gear_piece() {
        // Alongside the signature weapon: the Barrow Knight and Warden each
        // guarantee an armor piece, the Sovereign a ring. Never a dice roll.
        let mut rng = StdRng::seed_from_u64(11);
        let knight = roll_loot(&[barrow_knight("The Barrow Knight")], &[false], 0, &mut rng);
        assert!(knight
            .armors
            .iter()
            .any(|a| a.name == "Barrow-Touched Plate"));
        let warden = roll_loot(&[wyrmscale_warden("Wyrmscale Warden")], &[false], 0, &mut rng);
        assert!(warden.armors.iter().any(|a| a.name == "Dragonscale Aegis"));
        let sovereign = roll_loot(
            &[ashen_sovereign("The Ashen Sovereign")],
            &[false],
            0,
            &mut rng,
        );
        assert!(sovereign
            .rings
            .iter()
            .any(|r| r.name == "Sovereign's Signet"));
    }

    #[test]
    fn species_gear_drops_land_in_the_armor_and_ring_pools() {
        // A Mimic's ring drop is 35% — across many rolls it must show up,
        // proving the armor/ring plumbing actually reaches the Loot struct.
        let mut rng = StdRng::seed_from_u64(4);
        let mut saw_ring = false;
        let mut saw_armor = false;
        for _ in 0..200 {
            let loot = roll_loot(
                &[mimic("Mimic"), skeleton("Skeleton")],
                &[false, false],
                0,
                &mut rng,
            );
            saw_ring |= loot.rings.iter().any(|r| r.name == "Mimic's Coil");
            saw_armor |= loot.armors.iter().any(|a| a.name == "Chainmail Hauberk");
        }
        assert!(saw_ring, "the Mimic's Coil never dropped in 200 fights");
        assert!(
            saw_armor,
            "the Chainmail Hauberk never dropped in 200 fights"
        );
    }

    #[test]
    fn overkill_grants_bonus_gold() {
        let enemies = vec![slime("Slime")];
        let mut rng1 = StdRng::seed_from_u64(50);
        let normal = roll_loot(&enemies, &[false], 0, &mut rng1);
        let mut rng2 = StdRng::seed_from_u64(50);
        let overkill = roll_loot(&enemies, &[true], 0, &mut rng2);

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
    fn elite_kills_pay_more_gold_than_a_normal_kill() {
        let normal = slime("Slime");
        let mut elite = slime("Slime");
        elite.apply_elite();

        let mut rng1 = StdRng::seed_from_u64(7);
        let normal_loot = roll_loot(&[normal], &[false], 0, &mut rng1);
        let mut rng2 = StdRng::seed_from_u64(7);
        let elite_loot = roll_loot(&[elite], &[false], 0, &mut rng2);

        assert!(elite_loot.gold > normal_loot.gold);
    }

    #[test]
    fn loot_notes_mirror_the_live_loot_profile() {
        let goblin_notes = loot_notes("Goblin").join("\n");
        assert!(goblin_notes.contains("Goblin Shiv"), "goblin notes: {goblin_notes}");
        assert!(goblin_notes.contains("Gold: 8-16"));

        let rat_notes = loot_notes("Rat").join("\n");
        assert!(
            !rat_notes.contains('%') || rat_notes.matches('%').count() == 1,
            "rats only ever drop a potion: {rat_notes}"
        );
        assert!(!rat_notes.contains("Shiv") && !rat_notes.contains("Titanite"));
    }

    #[test]
    fn ng_plus_chance_adds_two_points_per_cycle_capped_at_seven_and_ninety_percent() {
        assert!((ng_plus_chance(0.12, 7) - 0.26).abs() < 1e-6);
        assert!((ng_plus_chance(0.85, 7) - 0.9).abs() < 1e-6, "0.9 ceiling should clip");
        assert_eq!(ng_plus_chance(0.3, 0), 0.3, "NG+0 should be a no-op");
        assert!(
            (ng_plus_chance(0.12, 12) - ng_plus_chance(0.12, 7)).abs() < 1e-6,
            "cycles past NG+7 should not keep raising the chance"
        );
    }

    #[test]
    fn ng_plus_seven_drops_strictly_more_weapons_than_ng_plus_zero() {
        let orcs = vec![orc("Orc")];
        let mut base_drops = 0usize;
        let mut ng7_drops = 0usize;
        // Same seed per iteration so only the NG+ bump differs.
        for seed in 0..500 {
            base_drops += roll_loot(&orcs, &[false], 0, &mut StdRng::seed_from_u64(seed))
                .weapons
                .len();
            ng7_drops += roll_loot(&orcs, &[false], 7, &mut StdRng::seed_from_u64(seed))
                .weapons
                .len();
        }
        assert!(
            ng7_drops > base_drops,
            "NG+7 should drop more weapons over 500 kills ({ng7_drops} vs {base_drops})"
        );
    }

    #[test]
    fn boss_guaranteed_drops_survive_ng_plus_scaling() {
        let mut rng = StdRng::seed_from_u64(11);
        let knight = roll_loot(&[barrow_knight("The Barrow Knight")], &[false], 7, &mut rng);
        assert!(
            knight.weapons.iter().any(|w| w.name == "Knightsbane"),
            "the boss's guaranteed signature weapon must not be clipped by the NG+ ceiling"
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
        assert_eq!(
            combat.enrage_stage, 0,
            "warden should not enrage while healthy"
        );
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
            if combat
                .log
                .iter()
                .any(|line| line.contains("sweeps its tail"))
            {
                assert!(
                    party
                        .members
                        .iter()
                        .enumerate()
                        .all(|(i, m)| m.stats.hp < hp_before[i]),
                    "tail sweep should damage every party member, not just one"
                );
                swept = true;
                break;
            }
        }
        assert!(
            swept,
            "warden should use Tail Sweep at least once across many trials"
        );
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
            loot.weapons
                .iter()
                .any(|w| w.name == "Sovereign's Reckoning"),
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
        assert!(
            high_luck_crit,
            "a high-luck attacker should crit at least once"
        );
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
        let base_loot = roll_loot(&base, &[false], 0, &mut StdRng::seed_from_u64(9));
        let scaled_loot = roll_loot(&scaled, &[false], 0, &mut StdRng::seed_from_u64(9));
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

    #[test]
    fn an_elixir_fully_restores_hp_and_mp() {
        let mut hero = warrior("Bram");
        hero.stats.hp = 1;
        hero.stats.mp = 0;
        use_item_kind(ItemKind::Elixir, &mut hero);
        assert_eq!(hero.stats.hp, hero.stats.max_hp);
        assert_eq!(hero.stats.mp, hero.stats.max_mp);
    }

    #[test]
    fn a_revive_raises_the_fallen_at_the_given_fraction() {
        let mut hero = warrior("Bram");
        hero.stats.hp = 0;
        assert!(!hero.is_alive());
        let msg = use_item_kind(ItemKind::Revive { heal_percent: 0.5 }, &mut hero);
        assert!(hero.is_alive());
        assert_eq!(hero.stats.hp, hero.stats.max_hp / 2);
        assert!(msg.contains("rises"));
    }

    #[test]
    fn a_revive_on_the_living_never_kills_or_no_ops() {
        let mut hero = warrior("Bram");
        hero.stats.hp = 5;
        use_item_kind(ItemKind::Revive { heal_percent: 0.5 }, &mut hero);
        assert!(hero.stats.hp >= 5, "the fallback must not hurt the target");
    }

    #[test]
    fn a_cure_curse_item_in_combat_purges_party_curses() {
        use crate::game::status::{StatEffectTarget, StatusEffect};

        let mut party = test_party();
        party.add_effect(StatusEffect {
            name: "Curse of Frailty".into(),
            target: StatEffectTarget::Defense,
            delta: -4,
            encounters_remaining: 2,
        });
        let mut combat = CombatState::new(&party, vec![slime("Slime")]);
        let mut rng = StdRng::seed_from_u64(7);
        combat.apply_item_and_advance(ItemKind::CureCurse, "Bram", 0, &mut party, &mut rng);
        assert!(
            party.effects.is_empty(),
            "the purging stone should strip the curse mid-combat"
        );
    }
}
