use macroquad::prelude::*;

use crate::game::character::{AbilityKind, AllocStat, Class};
use crate::game::combat::{
    targets_party, ActionAnimKind, ActorRef, CombatAction, CombatPhase, CombatState,
    RESOLVING_HOLD_SECONDS,
};
use crate::game::item::Inventory;
use crate::game::party::Party;
use crate::render::assets::{
    armor_icon_rect, boss_monster_rect, item_kind_icon_rect, monster_rect, player_rect, ring_icon_rect,
    weapon_icon_rect, Assets, CANVAS_HEIGHT, CANVAS_WIDTH,
};
use crate::render::common::{
    draw_icon, draw_icon_tinted, hp_color, icon_y_for_text, push_text, rarity_color, stat_color, TextCmd,
};

const ACTION_LABELS: [&str; 4] = ["Attack", "Ability", "Item", "Flee"];

/// A small, deterministic palette so each monster species gets a stable
/// (but not hand-curated) placeholder color until real sprite art lands.
const SPECIES_PALETTE: [Color; 8] = [
    Color::new(0.72, 0.30, 0.30, 1.0),
    Color::new(0.30, 0.55, 0.72, 1.0),
    Color::new(0.40, 0.68, 0.35, 1.0),
    Color::new(0.68, 0.55, 0.25, 1.0),
    Color::new(0.55, 0.35, 0.68, 1.0),
    Color::new(0.35, 0.68, 0.62, 1.0),
    Color::new(0.68, 0.40, 0.55, 1.0),
    Color::new(0.50, 0.50, 0.50, 1.0),
];

pub(super) fn species_color(name: &str, elite: bool) -> Color {
    let hash: u32 = name.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    let mut c = SPECIES_PALETTE[(hash as usize) % SPECIES_PALETTE.len()];
    if elite {
        c.r = (c.r + 0.2).min(1.0);
        c.g = (c.g * 0.8).max(0.0);
    }
    c
}

/// Tints the single shared `player_rect()` icon per class, until per-class
/// sprite cells are picked from `assets/roguelike_characters.png` by hand
/// (see the doc comment on `draw_party`).
fn class_color(class: Class) -> Color {
    match class {
        Class::Warrior => Color::new(0.75, 0.35, 0.30, 1.0),
        Class::Mage => Color::new(0.35, 0.45, 0.80, 1.0),
        Class::Cleric => Color::new(0.85, 0.80, 0.40, 1.0),
        Class::Rogue => Color::new(0.40, 0.70, 0.45, 1.0),
        Class::Monster => WHITE,
    }
}

/// Idle-bob sine wave shared by `draw_enemies`/`draw_party`: a small
/// vertical drift so a static icon doesn't read as a frozen screenshot,
/// staggered per-actor so a whole row doesn't bob in lockstep.
fn idle_bob(t: f32, actor_index: usize) -> f32 {
    const BOB_SPEED: f32 = 3.0;
    const BOB_PHASE_STAGGER: f32 = 0.7;
    const BOB_AMPLITUDE: f32 = 1.5;
    (t * BOB_SPEED + actor_index as f32 * BOB_PHASE_STAGGER).sin() * BOB_AMPLITUDE
}

/// One-shot lunge/flash/fade beat for whichever actor/target
/// `CombatState::last_action_anim` names, active only while `phase ==
/// Resolving` (see `app.rs::begin_resolving_hold`) — `progress` runs 0..1
/// over the hold's duration. Returns `(position_offset, scale, tint_alpha)`
/// deltas to apply on top of an actor's normal draw.
#[derive(Clone, Copy)]
struct ActorBeat {
    /// Added to the icon's draw position — a lunge toward its target and
    /// back for an attacker, zero for everyone else.
    offset: Vec2,
    /// Multiplies the icon's draw size — shrinks toward 0 on a defeated
    /// target as `progress` approaches 1.
    scale: f32,
    /// Multiplies the icon's alpha — fades a defeated target out.
    alpha: f32,
    /// Flashes the icon toward white (a hit) — 0 is untouched, 1 is fully
    /// white, purely additive on top of the icon's own tint.
    flash: f32,
}

impl Default for ActorBeat {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            scale: 1.0,
            alpha: 1.0,
            flash: 0.0,
        }
    }
}

fn actor_beat(combat: &CombatState, who: ActorRef, lunge_toward: Vec2) -> ActorBeat {
    let mut beat = ActorBeat::default();
    if !matches!(combat.phase, CombatPhase::Resolving) {
        return beat;
    }
    let Some(ev) = combat.last_action_anim else {
        return beat;
    };
    let progress = (1.0 - combat.resolving_timer / RESOLVING_HOLD_SECONDS).clamp(0.0, 1.0);
    // Out-and-back easing, peaking at the midpoint of the hold.
    let lunge_ease = (progress * std::f32::consts::PI).sin();

    if ev.actor == who {
        beat.offset = lunge_toward * (lunge_ease * 4.0);
    }
    if ev.target == Some(who) {
        beat.flash = (1.0 - progress) * 0.7;
        if ev.kind == ActionAnimKind::Defeat {
            beat.scale = 1.0 - progress * 0.6;
            beat.alpha = 1.0 - progress;
        }
    }
    beat
}

/// Blends `base` toward white by `flash` (a hit reaction) and scales its
/// alpha by `alpha_mult` (a defeat fade-out), leaving `base` untouched when
/// both are at their resting values (0 and 1 respectively).
fn flash_tint(base: Color, flash: f32, alpha_mult: f32) -> Color {
    Color::new(
        base.r + (1.0 - base.r) * flash,
        base.g + (1.0 - base.g) * flash,
        base.b + (1.0 - base.b) * flash,
        base.a * alpha_mult,
    )
}

const EFFECTS_STRIP_H: f32 = 12.0;

pub fn draw(
    assets: &Assets,
    font: &Font,
    combat: &CombatState,
    party: &Party,
    inventory: &Inventory,
    t: f32,
    cmds: &mut Vec<TextCmd>,
) {
    draw_enemies(assets, font, combat, party, 12.0, CANVAS_WIDTH, 100.0, t, cmds);
    draw_party(assets, combat, party, 12.0 + 100.0, CANVAS_WIDTH * 0.5, 70.0, t, cmds);
    draw_menu_or_result(
        assets,
        combat,
        party,
        inventory,
        CANVAS_WIDTH * 0.5,
        12.0 + 100.0,
        CANVAS_WIDTH * 0.5,
        70.0,
        cmds,
    );
    draw_effects_strip(font, party, 12.0 + 100.0 + 70.0, CANVAS_WIDTH, EFFECTS_STRIP_H, cmds);
    draw_log(
        &combat.log,
        combat.log_scroll,
        12.0 + 100.0 + 70.0 + EFFECTS_STRIP_H,
        CANVAS_WIDTH,
        CANVAS_HEIGHT - (12.0 + 100.0 + 70.0 + EFFECTS_STRIP_H),
        cmds,
    );
}

/// Shows the party's active blessings/curses (`Party::effects`) so the
/// player can see what's boosting/hurting them mid-fight — previously this
/// data was fully tracked but never surfaced anywhere in combat.
fn draw_effects_strip(font: &Font, party: &Party, y: f32, w: f32, h: f32, cmds: &mut Vec<TextCmd>) {
    draw_rectangle_lines(0.0, y, w, h, 1.0, WHITE);
    if party.effects.is_empty() {
        push_text(cmds, "No active effects.", 4.0, y + h - 3.0, 7.0, GRAY);
        return;
    }
    let mut tx = 4.0;
    let ty = y + h - 3.0;
    for (i, effect) in party.effects.iter().enumerate() {
        let tag = format!(
            "{} {:+} {} ({}){}",
            effect.name,
            effect.delta,
            effect.target,
            effect.encounters_remaining,
            if i + 1 < party.effects.len() { "," } else { "" }
        );
        let color = if effect.delta >= 0 { GREEN } else { RED };
        let d = measure_text(&tag, Some(font), 7, 1.0);
        if tx + d.width > w - 4.0 {
            push_text(cmds, format!("+{} more", party.effects.len() - i), tx, ty, 7.0, GRAY);
            break;
        }
        push_text(cmds, tag.clone(), tx, ty, 7.0, color);
        tx += d.width + 4.0;
    }
}

/// Names of the next few actors due to act after whoever's acting now,
/// walking `turn_order` from `turn_cursor + 1` and skipping dead actors —
/// the same dead-actor-skipping walk `CombatState::advance_turn` does
/// internally, just for display rather than to actually advance anything.
fn upcoming_actor_names(combat: &CombatState, party: &Party, count: usize) -> Vec<String> {
    let n = combat.turn_order.len();
    let mut names = Vec::new();
    for step in 1..=n {
        if names.len() >= count {
            break;
        }
        let actor = combat.turn_order[(combat.turn_cursor + step) % n];
        let alive = match actor {
            ActorRef::Player(i) => party.members.get(i).is_some_and(|m| m.is_alive()),
            ActorRef::Enemy(i) => combat.enemies.get(i).is_some_and(|e| e.is_alive()),
        };
        if !alive {
            continue;
        }
        names.push(match actor {
            ActorRef::Player(i) => party.members[i].name.clone(),
            ActorRef::Enemy(i) => combat.enemies[i].display_name(),
        });
    }
    names
}

fn draw_enemies(
    assets: &Assets,
    font: &Font,
    combat: &CombatState,
    party: &Party,
    y: f32,
    w: f32,
    h: f32,
    t: f32,
    cmds: &mut Vec<TextCmd>,
) {
    draw_rectangle_lines(0.0, y, w, h, 1.0, WHITE);
    let upcoming = upcoming_actor_names(combat, party, 3);
    if !upcoming.is_empty() {
        let text = format!("Next: {}", upcoming.join(" > "));
        let d = measure_text(&text, Some(font), 7, 1.0);
        push_text(cmds, text, w - d.width - 4.0, y + 9.0, 7.0, LIGHTGRAY);
    }
    if combat.enemies.is_empty() {
        return;
    }
    let target_idx = match combat.phase {
        CombatPhase::SelectTarget {
            actor: ActorRef::Player(pi),
            action,
            target_idx,
        } if !targets_party(action, pi, party) => Some(target_idx),
        _ => None,
    };

    let col_w = w / combat.enemies.len() as f32;
    for (i, enemy) in combat.enemies.iter().enumerate() {
        let cx = i as f32 * col_w + col_w / 2.0;
        let is_targeted = target_idx == Some(i) && enemy.is_alive();

        if enemy.is_alive() {
            let color = species_color(&enemy.name, enemy.is_elite);
            let box_w = (col_w - 20.0).min(48.0);
            let box_h = 40.0;
            if is_targeted {
                draw_rectangle_lines(cx - box_w / 2.0, y + 10.0, box_w, box_h, 2.0, WHITE);
            }
            let sprite_rect = match enemy.boss_kind {
                Some(kind) => boss_monster_rect(kind),
                None => monster_rect(&enemy.name),
            };
            let sprite_size = box_w.min(box_h) - 4.0;
            // Enemies lunge downward toward the party panel when attacking;
            // the idle bob keeps a static-looking encounter from reading as
            // a frozen screenshot.
            let beat = actor_beat(combat, ActorRef::Enemy(i), vec2(0.0, 6.0));
            let draw_size = sprite_size * beat.scale;
            draw_icon_tinted(
                &assets.monsters,
                sprite_rect,
                cx - draw_size / 2.0 + beat.offset.x,
                y + 10.0 + (box_h - sprite_size) / 2.0 + beat.offset.y + idle_bob(t, i),
                draw_size,
                flash_tint(color, beat.flash, beat.alpha),
            );
            let name = enemy.display_name();
            let nd = measure_text(&name, Some(font), 9, 1.0);
            push_text(cmds, name, cx - nd.width / 2.0, y + 60.0, 9.0, WHITE);
            let bar_w = box_w;
            let ratio = enemy.hp_ratio().clamp(0.0, 1.0) as f32;
            draw_rectangle(cx - bar_w / 2.0, y + 66.0, bar_w, 5.0, DARKGRAY);
            draw_rectangle(cx - bar_w / 2.0, y + 66.0, bar_w * ratio, 5.0, hp_color(enemy.hp_ratio()));
            let hp_text = format!("{}/{}", enemy.stats.hp, enemy.stats.max_hp);
            let hd = measure_text(&hp_text, Some(font), 8, 1.0);
            push_text(cmds, hp_text, cx - hd.width / 2.0, y + 82.0, 8.0, LIGHTGRAY);
        } else {
            let name = format!("{} (defeated)", enemy.display_name());
            let nd = measure_text(&name, Some(font), 8, 1.0);
            push_text(cmds, name, cx - nd.width / 2.0, y + 50.0, 8.0, GRAY);
        }
    }
}

/// Icon size and left margin for `draw_party`'s per-member sprite — chosen
/// small enough to leave the name column (which starts right after it)
/// enough room before `bar_x` at 70.0.
const PARTY_ICON_SIZE: f32 = 11.0;
const PARTY_ICON_X: f32 = 4.0;
const PARTY_NAME_X: f32 = PARTY_ICON_X + PARTY_ICON_SIZE + 2.0;

/// Draws the party's HP/MP panel during combat, including — for the first
/// time — a sprite per member. There's no per-class art in
/// `assets/roguelike_characters.png` wired up yet, so this reuses the
/// single overworld `player_rect()` icon tinted per class via
/// `class_color`, the same placeholder approach `species_color` uses for
/// monsters; picking distinct per-class cells from the sheet by hand is a
/// good follow-up once someone can eyeball the actual art.
fn draw_party(assets: &Assets, combat: &CombatState, party: &Party, y: f32, w: f32, h: f32, t: f32, cmds: &mut Vec<TextCmd>) {
    draw_rectangle_lines(0.0, y, w, h, 1.0, WHITE);
    let acting_player = match combat.phase {
        CombatPhase::SelectAction {
            actor: ActorRef::Player(i),
        }
        | CombatPhase::SelectTarget {
            actor: ActorRef::Player(i),
            ..
        } => Some(i),
        _ => None,
    };
    let party_target = match combat.phase {
        CombatPhase::SelectTarget {
            actor: ActorRef::Player(pi),
            action,
            target_idx,
        } if targets_party(action, pi, party) => Some(target_idx),
        _ => None,
    };

    for (i, m) in party.members.iter().enumerate() {
        let row_y = y + 6.0 + i as f32 * 15.0;
        let marker = if party_target == Some(i) {
            ">"
        } else if acting_player == Some(i) {
            "*"
        } else {
            " "
        };
        let color = if !m.is_alive() { GRAY } else { WHITE };
        // Fallen members stay drawn (dimmed) rather than vanish, so a dead
        // ally's row still visually anchors that HP bar.
        let icon_color = if m.is_alive() { class_color(m.class) } else { GRAY };
        // Players lunge upward toward the enemy panel above when attacking.
        let beat = actor_beat(combat, ActorRef::Player(i), vec2(0.0, -6.0));
        let draw_size = PARTY_ICON_SIZE * beat.scale;
        draw_icon_tinted(
            &assets.characters,
            player_rect(),
            PARTY_ICON_X + beat.offset.x,
            icon_y_for_text(row_y + 8.0, 9.0, PARTY_ICON_SIZE) + beat.offset.y + idle_bob(t, i),
            draw_size,
            flash_tint(icon_color, beat.flash, beat.alpha),
        );
        push_text(cmds, format!("{marker}{:<8}", m.name), PARTY_NAME_X, row_y + 8.0, 9.0, color);
        let bar_x = 70.0;
        let bar_w = w - bar_x - 4.0;
        let ratio = m.hp_ratio().clamp(0.0, 1.0) as f32;
        draw_rectangle(bar_x, row_y + 2.0, bar_w, 4.0, DARKGRAY);
        draw_rectangle(
            bar_x,
            row_y + 2.0,
            bar_w * ratio,
            4.0,
            if m.is_alive() { hp_color(m.hp_ratio()) } else { GRAY },
        );
        let (hp_color_, mp_color_) = if m.is_alive() {
            (stat_color(AllocStat::MaxHp), stat_color(AllocStat::MaxMp))
        } else {
            (GRAY, GRAY)
        };
        push_text(cmds, format!("{}/{}", m.stats.hp, m.stats.max_hp), bar_x, row_y + 13.0, 7.0, hp_color_);
        push_text(
            cmds,
            format!("MP{}/{}", m.stats.mp, m.stats.max_mp),
            bar_x + bar_w * 0.55,
            row_y + 13.0,
            7.0,
            mp_color_,
        );
    }
}

fn draw_menu_or_result(
    assets: &Assets,
    combat: &CombatState,
    party: &Party,
    inventory: &Inventory,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    cmds: &mut Vec<TextCmd>,
) {
    draw_rectangle_lines(x, y, w, h, 1.0, WHITE);
    let pad = x + 6.0;

    match combat.phase {
        CombatPhase::SelectAction {
            actor: ActorRef::Player(_),
        } => {
            for (i, label) in ACTION_LABELS.iter().enumerate() {
                let selected = i == combat.menu_cursor;
                let color = if selected { YELLOW } else { WHITE };
                let prefix = if selected { "> " } else { "  " };
                push_text(
                    cmds,
                    format!("{prefix}{label}"),
                    pad,
                    y + 14.0 + i as f32 * 13.0,
                    10.0,
                    color,
                );
            }
        }
        CombatPhase::SelectAbility {
            actor: ActorRef::Player(pi),
            cursor,
        } => {
            for (i, ability) in party.members[pi].abilities.iter().enumerate() {
                let selected = i == cursor;
                let affordable = party.members[pi].stats.mp >= ability.mp_cost;
                let color = if selected {
                    YELLOW
                } else if !affordable {
                    GRAY
                } else {
                    WHITE
                };
                let prefix = if selected { "> " } else { "  " };
                push_text(
                    cmds,
                    format!("{prefix}{} (MP {})", ability.name, ability.mp_cost),
                    pad,
                    y + 14.0 + i as f32 * 22.0,
                    9.0,
                    color,
                );
                let (desc, desc_color) = if !affordable {
                    ("   Not enough MP".to_string(), RED)
                } else {
                    let power = ability.effective_power(&party.members[pi]);
                    let desc = match ability.kind {
                        AbilityKind::PhysicalDamage | AbilityKind::MagicDamage => {
                            if ability.targets_all_enemies {
                                format!("   ~{power} dmg, all enemies")
                            } else {
                                format!("   ~{power} dmg, one enemy")
                            }
                        }
                        AbilityKind::Heal => format!("   heals {power} HP"),
                    };
                    (desc, LIGHTGRAY)
                };
                push_text(cmds, desc, pad, y + 24.0 + i as f32 * 22.0, 8.0, desc_color);
            }
        }
        CombatPhase::SelectItem {
            actor: ActorRef::Player(_),
            cursor,
        } => {
            if inventory.items.is_empty() {
                push_text(cmds, "No items.", pad, y + 14.0, 9.0, GRAY);
            }
            for (i, (item, qty)) in inventory.items.iter().enumerate() {
                let selected = i == cursor;
                let color = if selected { YELLOW } else { WHITE };
                let prefix = if selected { "> " } else { "  " };
                let row_y = y + 14.0 + i as f32 * 22.0;
                draw_icon(
                    &assets.tiles,
                    item_kind_icon_rect(&item.kind),
                    pad,
                    icon_y_for_text(row_y, 9.0, 8.0),
                    8.0,
                );
                push_text(cmds, format!("{prefix}{} x{qty}", item.name), pad + 10.0, row_y, 9.0, color);
                push_text(
                    cmds,
                    item.kind.purpose_description(),
                    pad + 10.0,
                    y + 24.0 + i as f32 * 22.0,
                    7.0,
                    LIGHTGRAY,
                );
            }
        }
        CombatPhase::SelectTarget { action, .. } => {
            let item_icon = match action {
                CombatAction::Item(idx) => inventory.items.get(idx).map(|(i, _)| item_kind_icon_rect(&i.kind)),
                _ => None,
            };
            let hint = match action {
                CombatAction::Item(idx) => {
                    let item_name = inventory
                        .items
                        .get(idx)
                        .map(|(i, qty)| format!("{} x{qty}", i.name))
                        .unwrap_or_else(|| "no items".to_string());
                    format!("Using: {item_name}")
                }
                _ => "Choose a target".to_string(),
            };
            let hint_x = if let Some(rect) = item_icon {
                draw_icon(&assets.tiles, rect, pad, icon_y_for_text(y + 14.0, 9.0, 8.0), 8.0);
                pad + 10.0
            } else {
                pad
            };
            push_text(cmds, hint, hint_x, y + 14.0, 9.0, WHITE);
            push_text(
                cmds,
                "left/right target, Enter confirm, Esc back",
                pad,
                y + 28.0,
                8.0,
                LIGHTGRAY,
            );
        }
        CombatPhase::Victory => {
            push_text(cmds, "Victory!", pad, y + 14.0, 12.0, GREEN);
            let footer_y = y + h - 8.0;
            let list_bottom = footer_y - 10.0;
            let mut ty = y + 28.0;
            if let Some(loot) = &combat.loot {
                let mut entries: Vec<(Option<(&Texture2D, Rect)>, String, Color, f32)> = Vec::new();
                if loot.gold > 0 {
                    entries.push((None, format!("Found {} gold.", loot.gold), WHITE, 10.0));
                }
                if loot.overkill_bonus > 0 {
                    entries.push((
                        None,
                        format!("Overkill bonus: +{} gold!", loot.overkill_bonus),
                        YELLOW,
                        10.0,
                    ));
                }
                for item in &loot.items {
                    entries.push((
                        Some((&assets.tiles, item_kind_icon_rect(&item.kind))),
                        format!("Found: {}", item.name),
                        WHITE,
                        11.0,
                    ));
                }
                for weapon in &loot.weapons {
                    entries.push((
                        Some((&assets.characters, weapon_icon_rect())),
                        format!("Weapon: {} [{}]", weapon.name, weapon.rarity),
                        rarity_color(weapon.rarity),
                        11.0,
                    ));
                }
                for armor in &loot.armors {
                    entries.push((
                        Some((&assets.characters, armor_icon_rect())),
                        format!("Armor: {} [{}]", armor.name, armor.rarity),
                        rarity_color(armor.rarity),
                        11.0,
                    ));
                }
                for ring in &loot.rings {
                    entries.push((
                        Some((&assets.tiles, ring_icon_rect())),
                        format!("Ring: {} [{}]", ring.name, ring.rarity),
                        rarity_color(ring.rarity),
                        11.0,
                    ));
                }

                let total = entries.len();
                for (i, (icon, text, color, step)) in entries.into_iter().enumerate() {
                    if ty + step > list_bottom {
                        push_text(cmds, format!("+{} more", total - i), pad, ty, 8.0, GRAY);
                        break;
                    }
                    if let Some((tex, rect)) = icon {
                        draw_icon(tex, rect, pad, icon_y_for_text(ty, 8.0, 7.0), 7.0);
                        push_text(cmds, text, pad + 9.0, ty, 8.0, color);
                    } else {
                        push_text(cmds, text, pad, ty, 8.0, color);
                    }
                    ty += step;
                }
            }
            push_text(cmds, "Press Enter to continue.", pad, footer_y, 8.0, LIGHTGRAY);
        }
        CombatPhase::Defeat => {
            push_text(cmds, "Your party has fallen.", pad, y + 14.0, 10.0, RED);
            push_text(cmds, "Press Enter.", pad, y + 28.0, 9.0, LIGHTGRAY);
        }
        CombatPhase::Fled => {
            push_text(cmds, "You got away safely.", pad, y + 14.0, 10.0, SKYBLUE);
            push_text(cmds, "Press Enter.", pad, y + 28.0, 9.0, LIGHTGRAY);
        }
        CombatPhase::Resolving => {
            push_text(cmds, "...", pad, y + 14.0, 9.0, GRAY);
        }
        CombatPhase::SelectAction {
            actor: ActorRef::Enemy(_),
        }
        | CombatPhase::SelectAbility {
            actor: ActorRef::Enemy(_),
            ..
        }
        | CombatPhase::SelectItem {
            actor: ActorRef::Enemy(_),
            ..
        } => {
            push_text(cmds, "The enemy is acting...", pad, y + 14.0, 9.0, GRAY);
        }
    }
}

fn draw_log(log: &[String], log_scroll: usize, y: f32, w: f32, h: f32, cmds: &mut Vec<TextCmd>) {
    draw_rectangle_lines(0.0, y, w, h, 1.0, WHITE);
    let visible = 5;
    let end = log.len().saturating_sub(log_scroll);
    let start = end.saturating_sub(visible);
    for (i, line) in log[start..end].iter().enumerate() {
        push_text(cmds, line.clone(), 4.0, y + 12.0 + i as f32 * 11.0, 8.0, WHITE);
    }
}
