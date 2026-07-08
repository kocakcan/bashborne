use macroquad::prelude::*;

use crate::game::character::AllocStat;
use crate::game::inventory_ui::InventoryMode;
use crate::game::item::Inventory;
use crate::game::party::Party;
use crate::game::shop::{shop_armor_stock, shop_item_stock, shop_ring_stock, shop_weapon_stock, ShopMode, ShopTab, ShopUiState};
use crate::render::assets::{
    armor_icon_rect, item_kind_icon_rect, ring_icon_rect, weapon_icon_rect, Assets, CANVAS_HEIGHT, CANVAS_WIDTH,
};
use crate::render::common::{draw_icon, push_text, rarity_color, scroll_window, stat_color, TextCmd};
use crate::render::inventory::draw_party_gear;

const HEADER_Y: f32 = 12.0;
const HEADER_H: f32 = 18.0;
const FOOTER_H: f32 = 26.0;
const LEFT_W: f32 = CANVAS_WIDTH * 0.65;
const RIGHT_X: f32 = LEFT_W;
const RIGHT_W: f32 = CANVAS_WIDTH - LEFT_W;
const ICON_SIZE: f32 = 8.0;
const ICON_GAP: f32 = 10.0;

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max_chars.saturating_sub(3)).collect();
        out.push_str("...");
        out
    }
}

pub fn draw(assets: &Assets, shop: &ShopUiState, party: &Party, inventory: &Inventory, cmds: &mut Vec<TextCmd>) {
    let content_y0 = HEADER_Y + HEADER_H;
    let content_y1 = CANVAS_HEIGHT - FOOTER_H;

    draw_header(shop, party, cmds);
    draw_rectangle_lines(0.0, content_y0, LEFT_W, content_y1 - content_y0, 1.0, WHITE);

    match shop.mode {
        ShopMode::Buy => draw_buy_list(assets, shop, party, inventory, content_y0, cmds),
        ShopMode::Sell => draw_sell_list(assets, shop, inventory, content_y0, cmds),
    }

    draw_party_gear(assets, party, &InventoryMode::Browsing, RIGHT_X, content_y0, RIGHT_W, content_y1, cmds);
    draw_footer(shop.message.as_deref(), content_y1, CANVAS_HEIGHT, cmds);
}

fn draw_header(shop: &ShopUiState, party: &Party, cmds: &mut Vec<TextCmd>) {
    let buy_color = if shop.mode == ShopMode::Buy { YELLOW } else { GRAY };
    let sell_color = if shop.mode == ShopMode::Sell { YELLOW } else { GRAY };
    push_text(cmds, "Buy", 4.0, HEADER_Y + 9.0, 9.0, buy_color);
    push_text(cmds, "Sell", 32.0, HEADER_Y + 9.0, 9.0, sell_color);

    let tabs = [
        ("Items", ShopTab::Items),
        ("Weapons", ShopTab::Weapons),
        ("Armor", ShopTab::Armor),
        ("Rings", ShopTab::Rings),
    ];
    let mut x = 70.0;
    for (label, tab) in tabs {
        let color = if shop.tab == tab { YELLOW } else { DARKGRAY };
        push_text(cmds, label, x, HEADER_Y + 9.0, 8.0, color);
        x += label.len() as f32 * 5.5 + 8.0;
    }
    push_text(cmds, format!("Gold: {}", party.gold), CANVAS_WIDTH - 70.0, HEADER_Y + 9.0, 9.0, GOLD);
    push_text(
        cmds,
        "left-right Buy/Sell, Tab cycles tabs, Esc leave",
        4.0,
        HEADER_Y + HEADER_H - 2.0,
        7.0,
        GRAY,
    );
}

fn draw_buy_list(assets: &Assets, shop: &ShopUiState, party: &Party, inventory: &Inventory, y0: f32, cmds: &mut Vec<TextCmd>) {
    let pad = 4.0;
    let text_pad = pad + ICON_GAP;
    let visible = 6usize;
    match shop.tab {
        ShopTab::Items => {
            let stock = shop_item_stock();
            let range = scroll_window(stock.len(), shop.cursor, visible);
            for (row, i) in range.enumerate() {
                let (factory, price) = stock[i];
                let sample = factory();
                let affordable = party.gold >= price;
                let owned = inventory
                    .items
                    .iter()
                    .find(|(item, _)| item.name == sample.name)
                    .map(|(_, qty)| *qty)
                    .unwrap_or(0);
                let selected = i == shop.cursor;
                let ty = y0 + 10.0 + row as f32 * 22.0;
                if selected {
                    draw_rectangle(0.0, ty - 8.0, LEFT_W, 20.0, Color::new(1.0, 1.0, 1.0, 0.12));
                }
                let color = if selected { YELLOW } else if !affordable { DARKGRAY } else { WHITE };
                let marker = if selected { "> " } else { "  " };
                draw_icon(&assets.tiles, item_kind_icon_rect(&sample.kind), pad, ty - 7.0, ICON_SIZE);
                push_text(
                    cmds,
                    format!("{marker}{} {price}g (have x{owned})", sample.name),
                    text_pad,
                    ty,
                    8.0,
                    color,
                );
                let detail_color = if selected { LIGHTGRAY } else { DARKGRAY };
                push_text(cmds, truncate(&sample.description, 52), text_pad + 4.0, ty + 10.0, 7.0, detail_color);
            }
        }
        ShopTab::Weapons => {
            let stock = shop_weapon_stock();
            let range = scroll_window(stock.len(), shop.cursor, visible);
            for (row, i) in range.enumerate() {
                let (factory, price) = stock[i];
                let sample = factory();
                let affordable = party.gold >= price;
                let selected = i == shop.cursor;
                let ty = y0 + 10.0 + row as f32 * 12.0;
                if selected {
                    draw_rectangle(0.0, ty - 8.0, LEFT_W, 11.0, Color::new(1.0, 1.0, 1.0, 0.12));
                }
                let color = if !affordable { DARKGRAY } else { rarity_color(sample.rarity) };
                let marker = if selected { "> " } else { "  " };
                draw_icon(&assets.characters, weapon_icon_rect(), pad, ty - 7.0, ICON_SIZE);
                let name_part = format!("{marker}{} [{}]", sample.name, sample.rarity);
                push_text(cmds, name_part.clone(), text_pad, ty, 8.0, color);
                let atk_x = text_pad + name_part.len() as f32 * 5.5;
                let atk_part = format!(" ATK+{}", sample.attack_bonus);
                let atk_color = if affordable { stat_color(AllocStat::Attack) } else { DARKGRAY };
                push_text(cmds, atk_part.clone(), atk_x, ty, 8.0, atk_color);
                push_text(cmds, format!(" {price}g"), atk_x + atk_part.len() as f32 * 5.5, ty, 8.0, color);
            }
        }
        ShopTab::Armor => {
            let stock = shop_armor_stock();
            let range = scroll_window(stock.len(), shop.cursor, visible);
            for (row, i) in range.enumerate() {
                let (factory, price) = stock[i];
                let sample = factory();
                let affordable = party.gold >= price;
                let selected = i == shop.cursor;
                let ty = y0 + 10.0 + row as f32 * 12.0;
                if selected {
                    draw_rectangle(0.0, ty - 8.0, LEFT_W, 11.0, Color::new(1.0, 1.0, 1.0, 0.12));
                }
                let color = if !affordable { DARKGRAY } else { rarity_color(sample.rarity) };
                let marker = if selected { "> " } else { "  " };
                draw_icon(&assets.characters, armor_icon_rect(), pad, ty - 7.0, ICON_SIZE);
                let name_part = format!("{marker}{} [{}]", sample.name, sample.rarity);
                push_text(cmds, name_part.clone(), text_pad, ty, 8.0, color);
                let def_x = text_pad + name_part.len() as f32 * 5.5;
                let def_part = format!(" DEF+{}", sample.defense_bonus);
                let def_color = if affordable { stat_color(AllocStat::Defense) } else { DARKGRAY };
                push_text(cmds, def_part.clone(), def_x, ty, 8.0, def_color);
                push_text(cmds, format!(" {price}g"), def_x + def_part.len() as f32 * 5.5, ty, 8.0, color);
            }
        }
        ShopTab::Rings => {
            let stock = shop_ring_stock();
            let range = scroll_window(stock.len(), shop.cursor, visible);
            for (row, i) in range.enumerate() {
                let (factory, price) = stock[i];
                let sample = factory();
                let affordable = party.gold >= price;
                let selected = i == shop.cursor;
                let ty = y0 + 10.0 + row as f32 * 12.0;
                if selected {
                    draw_rectangle(0.0, ty - 8.0, LEFT_W, 11.0, Color::new(1.0, 1.0, 1.0, 0.12));
                }
                let mut bonus = String::new();
                if sample.attack_bonus > 0 {
                    bonus.push_str(&format!("ATK+{} ", sample.attack_bonus));
                }
                if sample.defense_bonus > 0 {
                    bonus.push_str(&format!("DEF+{} ", sample.defense_bonus));
                }
                let color = if !affordable { DARKGRAY } else { rarity_color(sample.rarity) };
                let marker = if selected { "> " } else { "  " };
                draw_icon(&assets.tiles, ring_icon_rect(), pad, ty - 7.0, ICON_SIZE);
                push_text(
                    cmds,
                    format!("{marker}{} [{}] {bonus}{price}g", sample.name, sample.rarity),
                    text_pad,
                    ty,
                    8.0,
                    color,
                );
            }
        }
    }
}

fn draw_sell_list(assets: &Assets, shop: &ShopUiState, inventory: &Inventory, y0: f32, cmds: &mut Vec<TextCmd>) {
    let pad = 4.0;
    let text_pad = pad + ICON_GAP;
    let visible = 6usize;
    match shop.tab {
        ShopTab::Items => {
            if inventory.items.is_empty() {
                push_text(cmds, "Nothing to sell.", pad, y0 + 12.0, 8.0, GRAY);
                return;
            }
            let range = scroll_window(inventory.items.len(), shop.cursor, visible);
            for (row, i) in range.enumerate() {
                let (item, qty) = &inventory.items[i];
                let selected = i == shop.cursor;
                let ty = y0 + 10.0 + row as f32 * 22.0;
                if selected {
                    draw_rectangle(0.0, ty - 8.0, LEFT_W, 20.0, Color::new(1.0, 1.0, 1.0, 0.12));
                }
                let color = if selected { YELLOW } else { WHITE };
                let marker = if selected { "> " } else { "  " };
                draw_icon(&assets.tiles, item_kind_icon_rect(&item.kind), pad, ty - 7.0, ICON_SIZE);
                push_text(
                    cmds,
                    format!("{marker}{} x{qty} sells for {}g", item.name, item.value / 2),
                    text_pad,
                    ty,
                    8.0,
                    color,
                );
                let detail_color = if selected { LIGHTGRAY } else { DARKGRAY };
                push_text(cmds, truncate(&item.description, 52), text_pad + 4.0, ty + 10.0, 7.0, detail_color);
            }
        }
        ShopTab::Weapons => {
            if inventory.weapons.is_empty() {
                push_text(cmds, "No spare weapons to sell.", pad, y0 + 12.0, 8.0, GRAY);
                return;
            }
            let range = scroll_window(inventory.weapons.len(), shop.cursor, visible);
            for (row, i) in range.enumerate() {
                let w = &inventory.weapons[i];
                let selected = i == shop.cursor;
                let ty = y0 + 10.0 + row as f32 * 12.0;
                if selected {
                    draw_rectangle(0.0, ty - 8.0, LEFT_W, 11.0, Color::new(1.0, 1.0, 1.0, 0.12));
                }
                let marker = if selected { "> " } else { "  " };
                draw_icon(&assets.characters, weapon_icon_rect(), pad, ty - 7.0, ICON_SIZE);
                push_text(
                    cmds,
                    format!("{marker}{} [{}] sells for {}g", w.display_name(), w.rarity, w.rarity.base_value() / 2),
                    text_pad,
                    ty,
                    8.0,
                    rarity_color(w.rarity),
                );
            }
        }
        ShopTab::Armor => {
            if inventory.armors.is_empty() {
                push_text(cmds, "No spare armor to sell.", pad, y0 + 12.0, 8.0, GRAY);
                return;
            }
            let range = scroll_window(inventory.armors.len(), shop.cursor, visible);
            for (row, i) in range.enumerate() {
                let a = &inventory.armors[i];
                let selected = i == shop.cursor;
                let ty = y0 + 10.0 + row as f32 * 12.0;
                if selected {
                    draw_rectangle(0.0, ty - 8.0, LEFT_W, 11.0, Color::new(1.0, 1.0, 1.0, 0.12));
                }
                let marker = if selected { "> " } else { "  " };
                draw_icon(&assets.characters, armor_icon_rect(), pad, ty - 7.0, ICON_SIZE);
                push_text(
                    cmds,
                    format!("{marker}{} [{}] sells for {}g", a.name, a.rarity, a.rarity.base_value() / 2),
                    text_pad,
                    ty,
                    8.0,
                    rarity_color(a.rarity),
                );
            }
        }
        ShopTab::Rings => {
            if inventory.rings.is_empty() {
                push_text(cmds, "No spare rings to sell.", pad, y0 + 12.0, 8.0, GRAY);
                return;
            }
            let range = scroll_window(inventory.rings.len(), shop.cursor, visible);
            for (row, i) in range.enumerate() {
                let r = &inventory.rings[i];
                let selected = i == shop.cursor;
                let ty = y0 + 10.0 + row as f32 * 12.0;
                if selected {
                    draw_rectangle(0.0, ty - 8.0, LEFT_W, 11.0, Color::new(1.0, 1.0, 1.0, 0.12));
                }
                let marker = if selected { "> " } else { "  " };
                draw_icon(&assets.tiles, ring_icon_rect(), pad, ty - 7.0, ICON_SIZE);
                push_text(
                    cmds,
                    format!("{marker}{} [{}] sells for {}g", r.name, r.rarity, r.rarity.base_value() / 2),
                    text_pad,
                    ty,
                    8.0,
                    rarity_color(r.rarity),
                );
            }
        }
    }
}

fn draw_footer(message: Option<&str>, y0: f32, y1: f32, cmds: &mut Vec<TextCmd>) {
    draw_rectangle_lines(0.0, y0, CANVAS_WIDTH, y1 - y0, 1.0, WHITE);
    let text = message.unwrap_or("Epic and Legendary gear can't be bought - you'll have to earn it.");
    push_text(cmds, text, 4.0, y0 + 12.0, 7.0, WHITE);
}
