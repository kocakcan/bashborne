use macroquad::prelude::*;

use crate::game::blacksmith::{upgrade_cost, weapon_for, weapon_ref_label, weapon_refs, BlacksmithUiState, SHARD_PRICE};
use crate::game::chapter::ChapterId;
use crate::game::inventory_ui::InventoryMode;
use crate::game::item::Inventory;
use crate::game::party::Party;
use crate::render::assets::{CANVAS_HEIGHT, CANVAS_WIDTH};
use crate::render::common::{push_text, rarity_color, scroll_window, TextCmd};
use crate::render::inventory::draw_party_gear;

const HEADER_Y: f32 = 12.0;
const HEADER_H: f32 = 18.0;
const FOOTER_H: f32 = 30.0;
const LEFT_W: f32 = CANVAS_WIDTH * 0.65;
const RIGHT_X: f32 = LEFT_W;
const RIGHT_W: f32 = CANVAS_WIDTH - LEFT_W;

pub fn draw(bs: &BlacksmithUiState, party: &Party, inventory: &Inventory, current_chapter: ChapterId, cmds: &mut Vec<TextCmd>) {
    let content_y0 = HEADER_Y + HEADER_H;
    let content_y1 = CANVAS_HEIGHT - FOOTER_H;

    draw_header(party, inventory, cmds);
    draw_weapon_list(bs, party, inventory, content_y0, content_y1, cmds);
    draw_party_gear(party, &InventoryMode::Browsing, RIGHT_X, content_y0, RIGHT_W, content_y1, cmds);
    draw_footer(bs.message.as_deref(), current_chapter == ChapterId::Three, content_y1, cmds);
}

fn draw_header(party: &Party, inventory: &Inventory, cmds: &mut Vec<TextCmd>) {
    draw_rectangle_lines(0.0, HEADER_Y, CANVAS_WIDTH, HEADER_H, 1.0, WHITE);
    push_text(cmds, "Andre of Astora - Weapon Upgrades", 4.0, HEADER_Y + 9.0, 9.0, WHITE);
    push_text(
        cmds,
        format!("Gold: {}   Titanite Shards: {}", party.gold, inventory.upgrade_materials),
        4.0,
        HEADER_Y + HEADER_H - 2.0,
        7.0,
        GOLD,
    );
}

fn draw_weapon_list(bs: &BlacksmithUiState, party: &Party, inventory: &Inventory, y0: f32, y1: f32, cmds: &mut Vec<TextCmd>) {
    draw_rectangle_lines(0.0, y0, LEFT_W, y1 - y0, 1.0, WHITE);
    let pad = 4.0;
    let refs = weapon_refs(inventory, party);
    if refs.is_empty() {
        push_text(cmds, "No weapons to upgrade.", pad, y0 + 12.0, 8.0, GRAY);
        return;
    }

    let visible = 8usize;
    let range = scroll_window(refs.len(), bs.cursor, visible);
    for (row, i) in range.enumerate() {
        let r = refs[i];
        let weapon = weapon_for(r, inventory, party).expect("weapon_refs stays in sync");
        let selected = i == bs.cursor;
        let ty = y0 + 10.0 + row as f32 * 24.0;
        if selected {
            draw_rectangle(0.0, ty - 8.0, LEFT_W, 22.0, Color::new(1.0, 1.0, 1.0, 0.12));
        }
        let marker = if selected { "> " } else { "  " };
        push_text(
            cmds,
            format!(
                "{marker}{} [{}] ATK+{} DEF+{} {}",
                weapon.display_name(),
                weapon.rarity,
                weapon.attack_bonus,
                weapon.defense_bonus,
                weapon_ref_label(r, party),
            ),
            pad,
            ty,
            8.0,
            rarity_color(weapon.rarity),
        );
        let cost_text = match upgrade_cost(weapon.rarity, weapon.upgrade_level) {
            Some((gold, shards)) => {
                let affordable = party.gold >= gold && inventory.upgrade_materials >= shards;
                let color = if affordable { GREEN } else { RED };
                (format!("Upgrade: {gold}g + {shards} shards"), color)
            }
            None => ("MAX".to_string(), YELLOW),
        };
        push_text(cmds, cost_text.0, pad + 8.0, ty + 10.0, 7.0, cost_text.1);
        if let Some(passive) = &weapon.passive {
            push_text(cmds, passive.description(), pad + 8.0, ty + 19.0, 7.0, YELLOW);
        }
    }
}

fn draw_footer(message: Option<&str>, shards_available: bool, y0: f32, cmds: &mut Vec<TextCmd>) {
    draw_rectangle_lines(0.0, y0, CANVAS_WIDTH, CANVAS_HEIGHT - y0, 1.0, WHITE);
    let default_msg;
    let text = match message {
        Some(m) => m,
        None => {
            default_msg = if shards_available {
                format!("Upgrading raises ATK/DEF using gold and shards. Press B to buy a shard for {SHARD_PRICE}g.")
            } else {
                "Upgrading raises ATK (and DEF where applicable) using gold and Titanite Shards.".to_string()
            };
            &default_msg
        }
    };
    push_text(cmds, text, 4.0, y0 + 12.0, 7.0, WHITE);
}
