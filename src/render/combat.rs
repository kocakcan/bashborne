use macroquad::prelude::*;

use crate::game::character::{AbilityKind, AllocStat};
use crate::game::combat::{targets_party, ActorRef, CombatAction, CombatPhase, CombatState};
use crate::game::item::Inventory;
use crate::game::party::Party;
use crate::render::assets::{
    armor_icon_rect, boss_monster_rect, item_kind_icon_rect, monster_rect, ring_icon_rect, weapon_icon_rect, Assets,
    CANVAS_HEIGHT, CANVAS_WIDTH,
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

fn species_color(name: &str, elite: bool) -> Color {
    let hash: u32 = name.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    let mut c = SPECIES_PALETTE[(hash as usize) % SPECIES_PALETTE.len()];
    if elite {
        c.r = (c.r + 0.2).min(1.0);
        c.g = (c.g * 0.8).max(0.0);
    }
    c
}

const EFFECTS_STRIP_H: f32 = 12.0;

pub fn draw(assets: &Assets, font: &Font, combat: &CombatState, party: &Party, inventory: &Inventory, cmds: &mut Vec<TextCmd>) {
    draw_enemies(assets, font, combat, party, 12.0, CANVAS_WIDTH, 100.0, cmds);
    draw_party(combat, party, 12.0 + 100.0, CANVAS_WIDTH * 0.5, 70.0, cmds);
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

fn draw_enemies(
    assets: &Assets,
    font: &Font,
    combat: &CombatState,
    party: &Party,
    y: f32,
    w: f32,
    h: f32,
    cmds: &mut Vec<TextCmd>,
) {
    draw_rectangle_lines(0.0, y, w, h, 1.0, WHITE);
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
            draw_rectangle(cx - box_w / 2.0, y + 10.0, box_w, box_h, Color::new(color.r, color.g, color.b, 0.18));
            if is_targeted {
                draw_rectangle_lines(cx - box_w / 2.0, y + 10.0, box_w, box_h, 2.0, WHITE);
            }
            let sprite_rect = match enemy.boss_kind {
                Some(kind) => boss_monster_rect(kind),
                None => monster_rect(&enemy.name),
            };
            let sprite_size = box_w.min(box_h) - 4.0;
            draw_icon_tinted(
                &assets.monsters,
                sprite_rect,
                cx - sprite_size / 2.0,
                y + 10.0 + (box_h - sprite_size) / 2.0,
                sprite_size,
                color,
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

fn draw_party(combat: &CombatState, party: &Party, y: f32, w: f32, h: f32, cmds: &mut Vec<TextCmd>) {
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
        push_text(cmds, format!("{marker}{:<8}", m.name), 4.0, row_y + 8.0, 9.0, color);
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
        CombatPhase::Resolving
        | CombatPhase::SelectAction {
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

fn draw_log(log: &[String], y: f32, w: f32, h: f32, cmds: &mut Vec<TextCmd>) {
    draw_rectangle_lines(0.0, y, w, h, 1.0, WHITE);
    let visible = 5;
    let start = log.len().saturating_sub(visible);
    for (i, line) in log[start..].iter().enumerate() {
        push_text(cmds, line.clone(), 4.0, y + 12.0 + i as f32 * 11.0, 8.0, WHITE);
    }
}
