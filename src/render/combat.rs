use macroquad::prelude::*;

use crate::game::character::AbilityKind;
use crate::game::combat::{targets_party, ActorRef, CombatAction, CombatPhase, CombatState};
use crate::game::item::Inventory;
use crate::game::party::Party;
use crate::render::assets::{CANVAS_HEIGHT, CANVAS_WIDTH};
use crate::render::common::{hp_color, push_text, rarity_color, TextCmd};

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

pub fn draw(font: &Font, combat: &CombatState, party: &Party, inventory: &Inventory, cmds: &mut Vec<TextCmd>) {
    draw_enemies(font, combat, party, 12.0, CANVAS_WIDTH, 100.0, cmds);
    draw_party(combat, party, 12.0 + 100.0, CANVAS_WIDTH * 0.5, 70.0, cmds);
    draw_menu_or_result(
        combat,
        party,
        inventory,
        CANVAS_WIDTH * 0.5,
        12.0 + 100.0,
        CANVAS_WIDTH * 0.5,
        70.0,
        cmds,
    );
    draw_log(
        &combat.log,
        12.0 + 100.0 + 70.0,
        CANVAS_WIDTH,
        CANVAS_HEIGHT - (12.0 + 100.0 + 70.0),
        cmds,
    );
}

fn draw_enemies(
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
            draw_rectangle(cx - box_w / 2.0, y + 10.0, box_w, box_h, color);
            if is_targeted {
                draw_rectangle_lines(cx - box_w / 2.0, y + 10.0, box_w, box_h, 2.0, WHITE);
            }
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
        push_text(
            cmds,
            format!("{}/{} MP{}/{}", m.stats.hp, m.stats.max_hp, m.stats.mp, m.stats.max_mp),
            bar_x,
            row_y + 13.0,
            7.0,
            LIGHTGRAY,
        );
    }
}

fn draw_menu_or_result(
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
                push_text(cmds, desc, pad, y + 24.0 + i as f32 * 22.0, 8.0, LIGHTGRAY);
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
                push_text(
                    cmds,
                    format!("{prefix}{} x{qty}", item.name),
                    pad,
                    y + 14.0 + i as f32 * 22.0,
                    9.0,
                    color,
                );
                push_text(
                    cmds,
                    item.description.clone(),
                    pad,
                    y + 24.0 + i as f32 * 22.0,
                    7.0,
                    LIGHTGRAY,
                );
            }
        }
        CombatPhase::SelectTarget { action, .. } => {
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
            push_text(cmds, hint, pad, y + 14.0, 9.0, WHITE);
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
            let mut ty = y + 28.0;
            if let Some(loot) = &combat.loot {
                if loot.gold > 0 {
                    push_text(cmds, format!("Found {} gold.", loot.gold), pad, ty, 8.0, WHITE);
                    ty += 10.0;
                }
                if loot.overkill_bonus > 0 {
                    push_text(
                        cmds,
                        format!("Overkill bonus: +{} gold!", loot.overkill_bonus),
                        pad,
                        ty,
                        8.0,
                        YELLOW,
                    );
                    ty += 10.0;
                }
                for item in &loot.items {
                    push_text(cmds, format!("Found: {}", item.name), pad, ty, 8.0, WHITE);
                    ty += 10.0;
                }
                for weapon in &loot.weapons {
                    push_text(
                        cmds,
                        format!("Weapon: {} [{}]", weapon.name, weapon.rarity),
                        pad,
                        ty,
                        8.0,
                        rarity_color(weapon.rarity),
                    );
                    ty += 10.0;
                }
                for armor in &loot.armors {
                    push_text(
                        cmds,
                        format!("Armor: {} [{}]", armor.name, armor.rarity),
                        pad,
                        ty,
                        8.0,
                        rarity_color(armor.rarity),
                    );
                    ty += 10.0;
                }
                for ring in &loot.rings {
                    push_text(
                        cmds,
                        format!("Ring: {} [{}]", ring.name, ring.rarity),
                        pad,
                        ty,
                        8.0,
                        rarity_color(ring.rarity),
                    );
                    ty += 10.0;
                }
            }
            push_text(cmds, "Press Enter to continue.", pad, y + h - 8.0, 8.0, LIGHTGRAY);
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
