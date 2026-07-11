use macroquad::prelude::*;

use crate::game::character::AllocStat;
use crate::game::chapter::ChapterId;
use crate::game::inventory_ui::InventoryMode;
use crate::game::item::Inventory;
use crate::game::party::Party;
use crate::game::shop::{shop_armor_stock, shop_item_stock, shop_ring_stock, shop_weapon_stock, ShopMode, ShopTab, ShopUiState};
use crate::render::assets::{
    armor_icon_rect, item_kind_icon_rect, ring_icon_rect, weapon_icon_rect, Assets, CANVAS_HEIGHT, CANVAS_WIDTH,
};
use crate::render::common::{
    draw_gear_col_dividers, draw_gear_row_divider, draw_icon, push_gear_row, push_gear_table_header, push_text,
    rarity_color, scroll_window, stat_color, TextCmd, GEAR_DESC_COL_W,
};
use crate::render::inventory::{draw_party_gear, wrap_lines, wrap_lines_px, GearPreview};

const HEADER_Y: f32 = 12.0;
const HEADER_H: f32 = 18.0;
const FOOTER_H: f32 = 26.0;
const LEFT_W: f32 = CANVAS_WIDTH * 0.65;
const RIGHT_X: f32 = LEFT_W;
const RIGHT_W: f32 = CANVAS_WIDTH - LEFT_W;
const ICON_SIZE: f32 = 8.0;
const ICON_GAP: f32 = 10.0;

pub fn draw(
    assets: &Assets,
    shop: &ShopUiState,
    party: &Party,
    inventory: &Inventory,
    chapter: ChapterId,
    cmds: &mut Vec<TextCmd>,
) {
    let content_y0 = HEADER_Y + HEADER_H;
    let content_y1 = CANVAS_HEIGHT - FOOTER_H;

    draw_header(shop, cmds);
    draw_rectangle_lines(0.0, content_y0, LEFT_W, content_y1 - content_y0, 1.0, WHITE);

    match shop.mode {
        ShopMode::Buy => draw_buy_list(assets, shop, party, inventory, chapter, content_y0, cmds),
        ShopMode::Sell => draw_sell_list(assets, shop, inventory, content_y0, cmds),
    }

    let preview = buy_gear_preview(shop, chapter);
    draw_party_gear(
        assets,
        party,
        &InventoryMode::Browsing,
        preview.as_ref(),
        RIGHT_X,
        content_y0,
        RIGHT_W,
        content_y1,
        cmds,
    );
    draw_footer(shop.message.as_deref(), chapter, content_y1, CANVAS_HEIGHT, cmds);
}

fn draw_header(shop: &ShopUiState, cmds: &mut Vec<TextCmd>) {
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
    push_text(
        cmds,
        "left-right Buy/Sell, Tab cycles tabs, x sells all Common, Esc leave",
        4.0,
        HEADER_Y + HEADER_H - 2.0,
        7.0,
        GRAY,
    );
}

/// Gear preview (see `GearPreview`) for whatever's under the cursor in the
/// Buy list — only meaningful in Buy mode on the Weapons/Armor/Rings tabs,
/// same idea as `inventory::browsing_gear_preview` but keyed off shop stock
/// instead of the bag.
fn buy_gear_preview(shop: &ShopUiState, chapter: ChapterId) -> Option<GearPreview> {
    if shop.mode != ShopMode::Buy {
        return None;
    }
    match shop.tab {
        ShopTab::Weapons => shop_weapon_stock(chapter).get(shop.cursor).map(|(factory, _)| {
            let w = factory();
            GearPreview::Weapon { attack_bonus: w.attack_bonus, defense_bonus: w.defense_bonus }
        }),
        ShopTab::Armor => shop_armor_stock(chapter)
            .get(shop.cursor)
            .map(|(factory, _)| GearPreview::Armor { defense_bonus: factory().defense_bonus }),
        ShopTab::Rings => shop_ring_stock(chapter).get(shop.cursor).map(|(factory, _)| {
            let r = factory();
            GearPreview::Ring { attack_bonus: r.attack_bonus, defense_bonus: r.defense_bonus, slot: None }
        }),
        ShopTab::Items => None,
    }
}

fn draw_buy_list(
    assets: &Assets,
    shop: &ShopUiState,
    party: &Party,
    inventory: &Inventory,
    chapter: ChapterId,
    y0: f32,
    cmds: &mut Vec<TextCmd>,
) {
    let pad = 4.0;
    let text_pad = pad + ICON_GAP;
    let visible = 6usize;
    match shop.tab {
        ShopTab::Items => {
            let stock = shop_item_stock(chapter);
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
                let ty = y0 + 10.0 + row as f32 * 28.0;
                if selected {
                    draw_rectangle(0.0, ty - 8.0, LEFT_W, 26.0, Color::new(1.0, 1.0, 1.0, 0.12));
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
                for (li, line) in wrap_lines(&sample.description, 52, 2).iter().enumerate() {
                    push_text(cmds, line, text_pad + 4.0, ty + 10.0 + li as f32 * 7.0, 7.0, detail_color);
                }
            }
        }
        ShopTab::Weapons => {
            let stock = shop_weapon_stock(chapter);
            let range = scroll_window(stock.len(), shop.cursor, visible);
            draw_gear_table(cmds, text_pad, y0, true);
            for (row, i) in range.enumerate() {
                let (factory, price) = stock[i];
                let sample = factory();
                let affordable = party.gold >= price;
                let selected = i == shop.cursor;
                let ty = y0 + 22.0 + row as f32 * 28.0;
                if row > 0 {
                    draw_gear_row_divider(0.0, LEFT_W, ty - 9.0);
                }
                if selected {
                    draw_rectangle(0.0, ty - 8.0, LEFT_W, 26.0, Color::new(1.0, 1.0, 1.0, 0.12));
                }
                let name_color = if !affordable { DARKGRAY } else { rarity_color(sample.rarity) };
                let marker = if selected { "> " } else { "  " };
                draw_icon(&assets.characters, weapon_icon_rect(), pad, ty - 7.0, ICON_SIZE);
                push_gear_row(
                    cmds,
                    text_pad,
                    ty,
                    8.0,
                    format!("{marker}{} [{}]", sample.name, sample.rarity),
                    name_color,
                    Some((sample.attack_bonus, stat_cell_color(affordable, sample.attack_bonus, stat_color(AllocStat::Attack)))),
                    Some((sample.defense_bonus, stat_cell_color(affordable, sample.defense_bonus, stat_color(AllocStat::Defense)))),
                    Some((format!("{price}g"), name_color)),
                );
                let detail_color = if selected { LIGHTGRAY } else { DARKGRAY };
                for (li, line) in wrap_lines_px(&assets.font, &sample.description, GEAR_DESC_COL_W - 4.0, 7.0, 2).iter().enumerate() {
                    push_text(cmds, line, text_pad + 4.0, ty + 10.0 + li as f32 * 7.0, 7.0, detail_color);
                }
            }
        }
        ShopTab::Armor => {
            let stock = shop_armor_stock(chapter);
            let range = scroll_window(stock.len(), shop.cursor, visible);
            draw_gear_table(cmds, text_pad, y0, true);
            for (row, i) in range.enumerate() {
                let (factory, price) = stock[i];
                let sample = factory();
                let affordable = party.gold >= price;
                let selected = i == shop.cursor;
                let ty = y0 + 22.0 + row as f32 * 28.0;
                if row > 0 {
                    draw_gear_row_divider(0.0, LEFT_W, ty - 9.0);
                }
                if selected {
                    draw_rectangle(0.0, ty - 8.0, LEFT_W, 26.0, Color::new(1.0, 1.0, 1.0, 0.12));
                }
                let name_color = if !affordable { DARKGRAY } else { rarity_color(sample.rarity) };
                let marker = if selected { "> " } else { "  " };
                draw_icon(&assets.characters, armor_icon_rect(), pad, ty - 7.0, ICON_SIZE);
                // Armor never grants attack (no `attack_bonus` field on
                // `Armor`), so the ATK cell is always a dimmed "ATK+0" —
                // still drawn, not skipped, so the column stays populated
                // like a real table cell across every tab.
                push_gear_row(
                    cmds,
                    text_pad,
                    ty,
                    8.0,
                    format!("{marker}{} [{}]", sample.name, sample.rarity),
                    name_color,
                    Some((0, DARKGRAY)),
                    Some((sample.defense_bonus, stat_cell_color(affordable, sample.defense_bonus, stat_color(AllocStat::Defense)))),
                    Some((format!("{price}g"), name_color)),
                );
                let detail_color = if selected { LIGHTGRAY } else { DARKGRAY };
                for (li, line) in wrap_lines_px(&assets.font, &sample.description, GEAR_DESC_COL_W - 4.0, 7.0, 2).iter().enumerate() {
                    push_text(cmds, line, text_pad + 4.0, ty + 10.0 + li as f32 * 7.0, 7.0, detail_color);
                }
            }
        }
        ShopTab::Rings => {
            let stock = shop_ring_stock(chapter);
            let range = scroll_window(stock.len(), shop.cursor, visible);
            draw_gear_table(cmds, text_pad, y0, true);
            for (row, i) in range.enumerate() {
                let (factory, price) = stock[i];
                let sample = factory();
                let affordable = party.gold >= price;
                let selected = i == shop.cursor;
                let ty = y0 + 22.0 + row as f32 * 28.0;
                if row > 0 {
                    draw_gear_row_divider(0.0, LEFT_W, ty - 9.0);
                }
                if selected {
                    draw_rectangle(0.0, ty - 8.0, LEFT_W, 26.0, Color::new(1.0, 1.0, 1.0, 0.12));
                }
                let name_color = if !affordable { DARKGRAY } else { rarity_color(sample.rarity) };
                let marker = if selected { "> " } else { "  " };
                draw_icon(&assets.tiles, ring_icon_rect(), pad, ty - 7.0, ICON_SIZE);
                push_gear_row(
                    cmds,
                    text_pad,
                    ty,
                    8.0,
                    format!("{marker}{} [{}]", sample.name, sample.rarity),
                    name_color,
                    Some((sample.attack_bonus, stat_cell_color(affordable, sample.attack_bonus, stat_color(AllocStat::Attack)))),
                    Some((sample.defense_bonus, stat_cell_color(affordable, sample.defense_bonus, stat_color(AllocStat::Defense)))),
                    Some((format!("{price}g"), name_color)),
                );
                let detail_color = if selected { LIGHTGRAY } else { DARKGRAY };
                for (li, line) in wrap_lines_px(&assets.font, &sample.description, GEAR_DESC_COL_W - 4.0, 7.0, 2).iter().enumerate() {
                    push_text(cmds, line, text_pad + 4.0, ty + 10.0 + li as f32 * 7.0, 7.0, detail_color);
                }
            }
        }
    }
}

/// Draws the header row + header/body divider + continuous column
/// gridlines for a gear tab (Weapons/Armor/Rings) — text-flushed cells
/// always render on top of these canvas-space lines (`flush_text` runs in
/// its own later pass over the whole blitted canvas), so a full-height
/// divider can safely run behind a wrapped description that overflows its
/// row without visually cutting through the letters.
fn draw_gear_table(cmds: &mut Vec<TextCmd>, text_pad: f32, y0: f32, show_price: bool) {
    push_gear_table_header(cmds, text_pad, y0 + 8.0, show_price);
    draw_gear_row_divider(0.0, LEFT_W, y0 + 12.0);
    draw_gear_col_dividers(text_pad, y0 + 12.0, CANVAS_HEIGHT - FOOTER_H, show_price);
}

/// ATK/DEF cell color for a buy-list row — dimmed gray when the value is
/// zero (so the cell still reads as "present" rather than a hole in the
/// table) or when the party can't afford the item, matching the row's own
/// name/price dimming.
fn stat_cell_color(affordable: bool, value: i32, base: Color) -> Color {
    if !affordable || value == 0 {
        DARKGRAY
    } else {
        base
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
                let ty = y0 + 10.0 + row as f32 * 28.0;
                if selected {
                    draw_rectangle(0.0, ty - 8.0, LEFT_W, 26.0, Color::new(1.0, 1.0, 1.0, 0.12));
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
                for (li, line) in wrap_lines(&item.description, 52, 2).iter().enumerate() {
                    push_text(cmds, line, text_pad + 4.0, ty + 10.0 + li as f32 * 7.0, 7.0, detail_color);
                }
            }
        }
        ShopTab::Weapons => {
            if inventory.weapons.is_empty() {
                push_text(cmds, "No spare weapons to sell.", pad, y0 + 12.0, 8.0, GRAY);
                return;
            }
            let range = scroll_window(inventory.weapons.len(), shop.cursor, visible);
            draw_gear_table(cmds, text_pad, y0, true);
            for (row, i) in range.enumerate() {
                let w = &inventory.weapons[i];
                let selected = i == shop.cursor;
                let ty = y0 + 22.0 + row as f32 * 28.0;
                if row > 0 {
                    draw_gear_row_divider(0.0, LEFT_W, ty - 9.0);
                }
                if selected {
                    draw_rectangle(0.0, ty - 8.0, LEFT_W, 26.0, Color::new(1.0, 1.0, 1.0, 0.12));
                }
                let marker = if selected { "> " } else { "  " };
                draw_icon(&assets.characters, weapon_icon_rect(), pad, ty - 7.0, ICON_SIZE);
                let name_color = rarity_color(w.rarity);
                push_gear_row(
                    cmds,
                    text_pad,
                    ty,
                    8.0,
                    format!("{marker}{} [{}]", w.display_name(), w.rarity),
                    name_color,
                    Some((w.attack_bonus, stat_cell_color(true, w.attack_bonus, stat_color(AllocStat::Attack)))),
                    Some((w.defense_bonus, stat_cell_color(true, w.defense_bonus, stat_color(AllocStat::Defense)))),
                    Some((format!("{}g", w.rarity.base_value() / 2), name_color)),
                );
                let detail_color = if selected { LIGHTGRAY } else { DARKGRAY };
                for (li, line) in wrap_lines_px(&assets.font, &w.description, GEAR_DESC_COL_W - 4.0, 7.0, 2).iter().enumerate() {
                    push_text(cmds, line, text_pad + 4.0, ty + 10.0 + li as f32 * 7.0, 7.0, detail_color);
                }
            }
        }
        ShopTab::Armor => {
            if inventory.armors.is_empty() {
                push_text(cmds, "No spare armor to sell.", pad, y0 + 12.0, 8.0, GRAY);
                return;
            }
            let range = scroll_window(inventory.armors.len(), shop.cursor, visible);
            draw_gear_table(cmds, text_pad, y0, true);
            for (row, i) in range.enumerate() {
                let a = &inventory.armors[i];
                let selected = i == shop.cursor;
                let ty = y0 + 22.0 + row as f32 * 28.0;
                if row > 0 {
                    draw_gear_row_divider(0.0, LEFT_W, ty - 9.0);
                }
                if selected {
                    draw_rectangle(0.0, ty - 8.0, LEFT_W, 26.0, Color::new(1.0, 1.0, 1.0, 0.12));
                }
                let marker = if selected { "> " } else { "  " };
                draw_icon(&assets.characters, armor_icon_rect(), pad, ty - 7.0, ICON_SIZE);
                let name_color = rarity_color(a.rarity);
                push_gear_row(
                    cmds,
                    text_pad,
                    ty,
                    8.0,
                    format!("{marker}{} [{}]", a.name, a.rarity),
                    name_color,
                    Some((0, DARKGRAY)),
                    Some((a.defense_bonus, stat_cell_color(true, a.defense_bonus, stat_color(AllocStat::Defense)))),
                    Some((format!("{}g", a.rarity.base_value() / 2), name_color)),
                );
                let detail_color = if selected { LIGHTGRAY } else { DARKGRAY };
                for (li, line) in wrap_lines_px(&assets.font, &a.description, GEAR_DESC_COL_W - 4.0, 7.0, 2).iter().enumerate() {
                    push_text(cmds, line, text_pad + 4.0, ty + 10.0 + li as f32 * 7.0, 7.0, detail_color);
                }
            }
        }
        ShopTab::Rings => {
            if inventory.rings.is_empty() {
                push_text(cmds, "No spare rings to sell.", pad, y0 + 12.0, 8.0, GRAY);
                return;
            }
            let range = scroll_window(inventory.rings.len(), shop.cursor, visible);
            draw_gear_table(cmds, text_pad, y0, true);
            for (row, i) in range.enumerate() {
                let r = &inventory.rings[i];
                let selected = i == shop.cursor;
                let ty = y0 + 22.0 + row as f32 * 28.0;
                if row > 0 {
                    draw_gear_row_divider(0.0, LEFT_W, ty - 9.0);
                }
                if selected {
                    draw_rectangle(0.0, ty - 8.0, LEFT_W, 26.0, Color::new(1.0, 1.0, 1.0, 0.12));
                }
                let marker = if selected { "> " } else { "  " };
                draw_icon(&assets.tiles, ring_icon_rect(), pad, ty - 7.0, ICON_SIZE);
                let name_color = rarity_color(r.rarity);
                push_gear_row(
                    cmds,
                    text_pad,
                    ty,
                    8.0,
                    format!("{marker}{} [{}]", r.name, r.rarity),
                    name_color,
                    Some((r.attack_bonus, stat_cell_color(true, r.attack_bonus, stat_color(AllocStat::Attack)))),
                    Some((r.defense_bonus, stat_cell_color(true, r.defense_bonus, stat_color(AllocStat::Defense)))),
                    Some((format!("{}g", r.rarity.base_value() / 2), name_color)),
                );
                let detail_color = if selected { LIGHTGRAY } else { DARKGRAY };
                for (li, line) in wrap_lines_px(&assets.font, &r.description, GEAR_DESC_COL_W - 4.0, 7.0, 2).iter().enumerate() {
                    push_text(cmds, line, text_pad + 4.0, ty + 10.0 + li as f32 * 7.0, 7.0, detail_color);
                }
            }
        }
    }
}

fn draw_footer(message: Option<&str>, chapter: ChapterId, y0: f32, y1: f32, cmds: &mut Vec<TextCmd>) {
    draw_rectangle_lines(0.0, y0, CANVAS_WIDTH, y1 - y0, 1.0, WHITE);
    let hint = if chapter == ChapterId::One {
        "Epic and Legendary gear can't be bought - you'll have to earn it."
    } else {
        "Legendary gear can't be bought - you'll have to earn it."
    };
    let text = message.unwrap_or(hint);
    push_text(cmds, text, 4.0, y0 + 12.0, 7.0, WHITE);
}
