use macroquad::prelude::*;

use crate::game::character::{AllocStat, Character, RingSlot};
use crate::game::inventory_ui::{
    move_preview, EquipSlot, InventoryMode, InventoryTab, InventoryUiState, EQUIP_SLOTS,
};
use crate::game::item::{Inventory, Rarity, Ring};
use crate::game::party::Party;
use crate::render::assets::{
    armor_icon_rect, item_kind_icon_rect, material_icon_rect, ring_icon_rect, weapon_icon_rect, Assets,
    CANVAS_HEIGHT, CANVAS_WIDTH,
};
use crate::render::common::{
    draw_gear_col_dividers, draw_gear_row_divider, draw_icon, hp_color, icon_y_for_text, push_gear_row,
    push_gear_table_header, push_text, rarity_color, scroll_window, stat_color, TextCmd, GEAR_DESC_COL_W,
};

/// Icons are drawn at this size, with row text shifted this many px to the
/// right of where it used to start, to leave room for them.
const ICON_SIZE: f32 = 8.0;
const ICON_GAP: f32 = 10.0;
/// Vertical space reserved per list row — wide enough for a name line plus a
/// 2-line word-wrapped description underneath without crowding the next row.
const ROW_STRIDE: f32 = 34.0;
/// Weapons additionally show a passive-effect line below the description
/// (up to 2 wrapped lines, same as the description itself, since Legendary
/// passive text at the Item column's width regularly wraps), so they need
/// more vertical room than the other tabs' rows.
const WEAPON_ROW_STRIDE: f32 = 49.0;

// Leaves room for the persistent status bar chrome drawn at y 0-12 (see
// `hud::draw_status_bar`) — the same convention `explore::MAP_TOP` and
// `combat::draw`'s enemy-panel `y` use.
const TAB_Y: f32 = 12.0;
const TAB_H: f32 = 18.0;
const FOOTER_H: f32 = 26.0;
const LEFT_W: f32 = CANVAS_WIDTH * 0.55;
const RIGHT_X: f32 = LEFT_W;
const RIGHT_W: f32 = CANVAS_WIDTH - LEFT_W;

/// Word-wraps `s` to at most `max_lines` lines of ~`max_chars` characters
/// each — the left panel is only ~260 logical px wide, far short of the
/// terminal width the original ratatui text assumed, so long flavor lines
/// need to wrap rather than get cut off mid-sentence. Only ellipsizes the
/// last line if the text still doesn't fit in `max_lines`.
pub(super) fn wrap_lines(s: &str, max_chars: usize, max_lines: usize) -> Vec<String> {
    let words: Vec<&str> = s.split_whitespace().collect();
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut idx = 0;
    while idx < words.len() {
        let word = words[idx];
        let extra = if current.is_empty() { 0 } else { 1 };
        if !current.is_empty() && current.chars().count() + extra + word.chars().count() > max_chars {
            lines.push(std::mem::take(&mut current));
            if lines.len() == max_lines {
                break;
            }
            continue;
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
        idx += 1;
    }
    let all_consumed = idx == words.len();
    if lines.len() < max_lines && !current.is_empty() {
        lines.push(current);
    } else if !all_consumed {
        if let Some(last) = lines.last_mut() {
            let mut truncated: String = last.chars().take(max_chars.saturating_sub(3)).collect();
            truncated.push_str("...");
            *last = truncated;
        }
    }
    lines
}

/// Same job as `wrap_lines`, but measures actual glyph width with the real
/// font instead of counting characters — needed for descriptions sitting
/// inside the gear table's "Item" column, which is a fixed pixel width
/// (`GEAR_DESC_COL_W`) regardless of the font's average character width, so
/// a char-count wrap either overflows into the ATK gridline or wraps too
/// early depending on how wide the current line's letters happen to be.
pub(super) fn wrap_lines_px(font: &Font, s: &str, max_width: f32, size: f32, max_lines: usize) -> Vec<String> {
    let words: Vec<&str> = s.split_whitespace().collect();
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut idx = 0;
    while idx < words.len() {
        let word = words[idx];
        let candidate = if current.is_empty() { word.to_string() } else { format!("{current} {word}") };
        let width = measure_text(&candidate, Some(font), size as u16, 1.0).width;
        if !current.is_empty() && width > max_width {
            lines.push(std::mem::take(&mut current));
            if lines.len() == max_lines {
                break;
            }
            continue;
        }
        current = candidate;
        idx += 1;
    }
    let all_consumed = idx == words.len();
    if lines.len() < max_lines && !current.is_empty() {
        lines.push(current);
    } else if !all_consumed {
        if let Some(last) = lines.last_mut() {
            let mut truncated = last.clone();
            while !truncated.is_empty()
                && measure_text(&format!("{truncated}..."), Some(font), size as u16, 1.0).width > max_width
            {
                truncated.pop();
            }
            truncated.push_str("...");
            *last = truncated;
        }
    }
    lines
}

pub fn draw(assets: &Assets, inv_ui: &InventoryUiState, party: &Party, inventory: &Inventory, cmds: &mut Vec<TextCmd>) {
    let content_y0 = TAB_Y + TAB_H;
    let content_y1 = CANVAS_HEIGHT - FOOTER_H;

    draw_tabs(inv_ui.tab, cmds);
    draw_rectangle_lines(0.0, content_y0, CANVAS_WIDTH, content_y1 - content_y0, 1.0, WHITE);

    // Every mode that asks the player to pick a party member for a piece of
    // gear shows the same right-side ATK/DEF delta preview — hovering a bag
    // item, giving one to a member, or moving an already-equipped piece to
    // another member all funnel into the one `GearPreview` shown by
    // `draw_party_gear`, rather than each mode drawing its own ad-hoc
    // before/after text under the candidate list.
    let preview = match &inv_ui.mode {
        InventoryMode::Browsing => gear_preview_for(inv_ui.tab, inv_ui.cursor, inventory),
        InventoryMode::SelectMember { tab, idx, .. } => gear_preview_for(*tab, *idx, inventory),
        InventoryMode::PartyGearTarget { from_member, slot, .. } => move_target_preview(party, *from_member, *slot),
        _ => None,
    };

    match &inv_ui.mode {
        InventoryMode::Browsing => draw_list(assets, inv_ui, inventory, content_y0, cmds),
        InventoryMode::SelectMember { tab, idx, member_cursor } => {
            draw_member_picker(party, inventory, *tab, *idx, *member_cursor, content_y0, cmds)
        }
        InventoryMode::SelectRingSlot { idx, member_idx, slot_cursor } => {
            draw_ring_slot_picker(&assets.font, party, inventory, *idx, *member_idx, *slot_cursor, content_y0, cmds)
        }
        InventoryMode::PartyGear { .. } => draw_party_gear_hint(content_y0, cmds),
        InventoryMode::PartyGearAction { action_cursor, .. } => {
            draw_party_gear_action_menu(*action_cursor, content_y0, cmds)
        }
        InventoryMode::PartyGearTarget { from_member, slot, to_cursor } => {
            draw_party_gear_target_picker(party, *from_member, *slot, *to_cursor, content_y0, cmds)
        }
        InventoryMode::Roster { active_cursor } => draw_roster_picker(party, *active_cursor, content_y0, cmds),
        InventoryMode::RosterTarget { active_idx, bench_cursor } => {
            draw_roster_target_picker(party, *active_idx, *bench_cursor, content_y0, cmds)
        }
    }

    draw_party_gear(
        assets,
        party,
        &inv_ui.mode,
        preview.as_ref(),
        RIGHT_X,
        content_y0,
        RIGHT_W,
        content_y1,
        cmds,
    );
    draw_footer(inv_ui.message.as_deref(), content_y1, CANVAS_HEIGHT, cmds);
}

/// Builds the gear-comparison preview (see `GearPreview`) for a weapon/
/// armor/ring sitting at `idx` in the bag — shared by the Browsing bag list
/// (hovering an item, keyed off the live cursor) and the Give-to-member
/// picker (keyed off the item already chosen when that mode was entered).
pub(super) fn gear_preview_for(tab: InventoryTab, idx: usize, inventory: &Inventory) -> Option<GearPreview> {
    match tab {
        InventoryTab::Weapons => inventory.weapons.get(idx).map(|w| GearPreview::Weapon {
            attack_bonus: w.attack_bonus,
            defense_bonus: w.defense_bonus,
        }),
        InventoryTab::Armor => inventory.armors.get(idx).map(|a| GearPreview::Armor { defense_bonus: a.defense_bonus }),
        InventoryTab::Rings => inventory.rings.get(idx).map(|r| GearPreview::Ring {
            attack_bonus: r.attack_bonus,
            defense_bonus: r.defense_bonus,
            slot: None,
        }),
        InventoryTab::Items | InventoryTab::Materials => None,
    }
}

/// Builds the same uniform `GearPreview` for the piece currently being
/// relocated in the "Move to..." target picker, so each candidate's
/// ATK/DEF delta shows up on the right in exactly the same format as
/// hovering a bag item — instead of the picker drawing its own before/after
/// numbers under every name. Unlike `gear_preview_for`, a ring here is
/// pinned to the exact slot being moved (`move_gear_between_members` always
/// swaps like-for-like, R1-to-R1 or R2-to-R2), not whichever of the
/// receiver's two rings happens to be weaker.
fn move_target_preview(party: &Party, from_member: usize, slot: EquipSlot) -> Option<GearPreview> {
    let from = party.members.get(from_member)?;
    match slot {
        EquipSlot::Weapon => from.equipped_weapon.as_ref().map(|w| GearPreview::Weapon {
            attack_bonus: w.attack_bonus,
            defense_bonus: w.defense_bonus,
        }),
        EquipSlot::Armor => from.equipped_armor.as_ref().map(|a| GearPreview::Armor { defense_bonus: a.defense_bonus }),
        EquipSlot::Ring(rs) => {
            let idx = match rs {
                RingSlot::First => 0,
                RingSlot::Second => 1,
            };
            from.equipped_rings[idx].as_ref().map(|r| GearPreview::Ring {
                attack_bonus: r.attack_bonus,
                defense_bonus: r.defense_bonus,
                slot: Some(rs),
            })
        }
    }
}

fn draw_tabs(active: InventoryTab, cmds: &mut Vec<TextCmd>) {
    let tabs = [
        ("Items", InventoryTab::Items),
        ("Weapons", InventoryTab::Weapons),
        ("Armor", InventoryTab::Armor),
        ("Rings", InventoryTab::Rings),
        ("Materials", InventoryTab::Materials),
    ];
    let mut x = 4.0;
    for (label, tab) in tabs {
        let selected = tab == active;
        let color = if selected { YELLOW } else { LIGHTGRAY };
        let text = if selected { format!("[{label}]") } else { format!(" {label} ") };
        push_text(cmds, &text, x, TAB_Y + 9.0, 9.0, color);
        x += text.len() as f32 * 6.0 + 4.0;
    }
    push_text(
        cmds,
        "Tab/left-right switch, p party gear, r roster, Esc close",
        4.0,
        TAB_Y + TAB_H - 2.0,
        7.0,
        GRAY,
    );
}

/// Header row + header/body divider + continuous column gridlines for a gear
/// tab (Weapons/Armor/Rings) — see `shop::draw_gear_table`, same idea, just
/// keyed off this screen's own `LEFT_W`/`FOOTER_H` (the bag list has no
/// price column, so callers pass `show_price: false`).
fn draw_gear_table(cmds: &mut Vec<TextCmd>, text_pad: f32, y0: f32, show_price: bool) {
    push_gear_table_header(cmds, text_pad, y0 + 8.0, show_price);
    draw_gear_row_divider(0.0, LEFT_W, y0 + 12.0);
    draw_gear_col_dividers(text_pad, y0 + 12.0, CANVAS_HEIGHT - FOOTER_H, show_price);
}

fn draw_list(assets: &Assets, inv_ui: &InventoryUiState, inventory: &Inventory, y0: f32, cmds: &mut Vec<TextCmd>) {
    let pad = 4.0;
    let text_pad = pad + ICON_GAP;
    let visible = 6usize;
    // Weapons/Armor/Rings now carry a header row above the list (see
    // `draw_gear_table`), which eats into the same fixed panel height —
    // one fewer row fits than the header-less Items/Materials tabs without
    // the last row's description spilling past the panel border.
    let gear_visible = 5usize;
    match inv_ui.tab {
        InventoryTab::Items => {
            if inventory.items.is_empty() {
                push_text(cmds, "No items.", pad, y0 + 12.0, 8.0, GRAY);
                return;
            }
            let range = scroll_window(inventory.items.len(), inv_ui.cursor, visible);
            for (row, i) in range.enumerate() {
                let (item, qty) = &inventory.items[i];
                let selected = i == inv_ui.cursor;
                let ty = y0 + 10.0 + row as f32 * ROW_STRIDE;
                if selected {
                    draw_rectangle(0.0, ty - 8.0, LEFT_W, 28.0, Color::new(1.0, 1.0, 1.0, 0.12));
                }
                let color = if selected { YELLOW } else { WHITE };
                let marker = if selected { "> " } else { "  " };
                draw_icon(
                    &assets.tiles,
                    item_kind_icon_rect(&item.kind),
                    pad,
                    icon_y_for_text(ty, 8.0, ICON_SIZE),
                    ICON_SIZE,
                );
                push_text(cmds, format!("{marker}{} x{qty}", item.name), text_pad, ty, 8.0, color);
                for (li, line) in wrap_lines(&item.description, 44, 2).iter().enumerate() {
                    push_text(cmds, line, text_pad, ty + 11.0 + li as f32 * 7.0, 7.0, LIGHTGRAY);
                }
            }
        }
        InventoryTab::Weapons => {
            if inventory.weapons.is_empty() {
                push_text(cmds, "No spare weapons.", pad, y0 + 12.0, 8.0, GRAY);
                return;
            }
            // Rows here use the taller WEAPON_ROW_STRIDE (extra room for a
            // passive line), so fewer of them fit in the same panel height
            // than the other tabs' `visible` — without this, the last row
            // spills past the panel border into the footer's tip text.
            let weapon_visible = 4usize;
            let range = scroll_window(inventory.weapons.len(), inv_ui.cursor, weapon_visible);
            draw_gear_table(cmds, text_pad, y0, false);
            for (row, i) in range.enumerate() {
                let w = &inventory.weapons[i];
                let selected = i == inv_ui.cursor;
                let ty = y0 + 22.0 + row as f32 * WEAPON_ROW_STRIDE;
                if row > 0 {
                    draw_gear_row_divider(0.0, LEFT_W, ty - 9.0);
                }
                if selected {
                    draw_rectangle(0.0, ty - 8.0, LEFT_W, 28.0, Color::new(1.0, 1.0, 1.0, 0.12));
                }
                let marker = if selected { "> " } else { "  " };
                draw_icon(
                    &assets.characters,
                    weapon_icon_rect(),
                    pad,
                    icon_y_for_text(ty, 8.0, ICON_SIZE),
                    ICON_SIZE,
                );
                let name_color = rarity_color(w.rarity);
                push_gear_row(
                    cmds,
                    text_pad,
                    ty,
                    8.0,
                    format!("{marker}{} [{}]", w.display_name(), w.rarity),
                    name_color,
                    Some((w.attack_bonus, if w.attack_bonus == 0 { DARKGRAY } else { stat_color(AllocStat::Attack) })),
                    Some((w.defense_bonus, if w.defense_bonus == 0 { DARKGRAY } else { stat_color(AllocStat::Defense) })),
                    None::<(String, Color)>,
                );
                let detail_color = if selected { LIGHTGRAY } else { DARKGRAY };
                let desc_lines = wrap_lines_px(&assets.font, &format!("{} - {}", w.description, w.source), GEAR_DESC_COL_W, 7.0, 2);
                let mut ly = ty + 11.0;
                for line in &desc_lines {
                    push_text(cmds, line, text_pad, ly, 7.0, detail_color);
                    ly += 7.0;
                }
                if let Some(passive) = &w.passive {
                    for line in wrap_lines_px(&assets.font, &passive.description(), GEAR_DESC_COL_W, 7.0, 2) {
                        push_text(cmds, line, text_pad, ly, 7.0, YELLOW);
                        ly += 7.0;
                    }
                }
            }
        }
        InventoryTab::Armor => {
            if inventory.armors.is_empty() {
                push_text(cmds, "No spare armor.", pad, y0 + 12.0, 8.0, GRAY);
                return;
            }
            let range = scroll_window(inventory.armors.len(), inv_ui.cursor, gear_visible);
            draw_gear_table(cmds, text_pad, y0, false);
            for (row, i) in range.enumerate() {
                let a = &inventory.armors[i];
                let selected = i == inv_ui.cursor;
                let ty = y0 + 22.0 + row as f32 * ROW_STRIDE;
                if row > 0 {
                    draw_gear_row_divider(0.0, LEFT_W, ty - 9.0);
                }
                if selected {
                    draw_rectangle(0.0, ty - 8.0, LEFT_W, 28.0, Color::new(1.0, 1.0, 1.0, 0.12));
                }
                let marker = if selected { "> " } else { "  " };
                draw_icon(
                    &assets.characters,
                    armor_icon_rect(),
                    pad,
                    icon_y_for_text(ty, 8.0, ICON_SIZE),
                    ICON_SIZE,
                );
                let name_color = rarity_color(a.rarity);
                push_gear_row(
                    cmds,
                    text_pad,
                    ty,
                    8.0,
                    format!("{marker}{} [{}]", a.name, a.rarity),
                    name_color,
                    Some((0, DARKGRAY)),
                    Some((a.defense_bonus, if a.defense_bonus == 0 { DARKGRAY } else { stat_color(AllocStat::Defense) })),
                    None::<(String, Color)>,
                );
                let detail_color = if selected { LIGHTGRAY } else { DARKGRAY };
                for (li, line) in wrap_lines_px(&assets.font, &format!("{} - {}", a.description, a.source), GEAR_DESC_COL_W, 7.0, 2)
                    .iter()
                    .enumerate()
                {
                    push_text(cmds, line, text_pad, ty + 11.0 + li as f32 * 7.0, 7.0, detail_color);
                }
            }
        }
        InventoryTab::Rings => {
            if inventory.rings.is_empty() {
                push_text(cmds, "No spare rings.", pad, y0 + 12.0, 8.0, GRAY);
                return;
            }
            let range = scroll_window(inventory.rings.len(), inv_ui.cursor, gear_visible);
            draw_gear_table(cmds, text_pad, y0, false);
            for (row, i) in range.enumerate() {
                let r = &inventory.rings[i];
                let selected = i == inv_ui.cursor;
                let ty = y0 + 22.0 + row as f32 * ROW_STRIDE;
                if row > 0 {
                    draw_gear_row_divider(0.0, LEFT_W, ty - 9.0);
                }
                if selected {
                    draw_rectangle(0.0, ty - 8.0, LEFT_W, 28.0, Color::new(1.0, 1.0, 1.0, 0.12));
                }
                let marker = if selected { "> " } else { "  " };
                draw_icon(
                    &assets.tiles,
                    ring_icon_rect(),
                    pad,
                    icon_y_for_text(ty, 8.0, ICON_SIZE),
                    ICON_SIZE,
                );
                let name_color = rarity_color(r.rarity);
                push_gear_row(
                    cmds,
                    text_pad,
                    ty,
                    8.0,
                    format!("{marker}{} [{}]", r.name, r.rarity),
                    name_color,
                    Some((r.attack_bonus, if r.attack_bonus == 0 { DARKGRAY } else { stat_color(AllocStat::Attack) })),
                    Some((r.defense_bonus, if r.defense_bonus == 0 { DARKGRAY } else { stat_color(AllocStat::Defense) })),
                    None::<(String, Color)>,
                );
                let detail_color = if selected { LIGHTGRAY } else { DARKGRAY };
                for (li, line) in wrap_lines_px(&assets.font, &format!("{} - {}", r.description, r.source), GEAR_DESC_COL_W, 7.0, 2)
                    .iter()
                    .enumerate()
                {
                    push_text(cmds, line, text_pad, ty + 11.0 + li as f32 * 7.0, 7.0, detail_color);
                }
            }
        }
        InventoryTab::Materials => {
            if inventory.upgrade_materials == 0 {
                push_text(cmds, "No materials yet.", pad, y0 + 12.0, 8.0, GRAY);
            } else {
                let selected = inv_ui.cursor == 0;
                let color = if selected { YELLOW } else { WHITE };
                draw_icon(
                    &assets.tiles,
                    material_icon_rect(),
                    pad,
                    icon_y_for_text(y0 + 12.0, 8.0, ICON_SIZE),
                    ICON_SIZE,
                );
                push_text(
                    cmds,
                    format!("Titanite Shard x{}", inventory.upgrade_materials),
                    text_pad,
                    y0 + 12.0,
                    8.0,
                    color,
                );
            }
        }
    }
}

/// The item currently under the cursor in a buy/bag list, carrying just
/// enough of its stat bonuses to preview how it'd change each party member's
/// ATK/DEF if equipped — shared by the Shop buy screen and the Inventory
/// bag list so hovering a weapon/armor/ring (before actually equipping or
/// buying it) can show whether it's an upgrade.
pub enum GearPreview {
    Weapon { attack_bonus: i32, defense_bonus: i32 },
    Armor { defense_bonus: i32 },
    /// `slot: None` means "not bound to R1 or R2 yet" (a bag/shop ring
    /// hasn't been equipped anywhere), so the delta compares against
    /// whichever of the member's two rings is weaker. `Some(slot)` pins the
    /// comparison to that exact slot instead, used when the piece being
    /// previewed is already equipped in a specific slot (the "Move to..."
    /// target picker), since that flow always swaps like-for-like.
    Ring { attack_bonus: i32, defense_bonus: i32, slot: Option<RingSlot> },
}

/// ATK/DEF delta `m` would see if the previewed item replaced whatever's
/// currently in the slot it belongs to.
fn gear_preview_delta(m: &Character, preview: &GearPreview) -> (i32, i32) {
    match *preview {
        GearPreview::Weapon { attack_bonus, defense_bonus } => {
            let cur_atk = m.equipped_weapon.as_ref().map(|w| w.attack_bonus).unwrap_or(0);
            let cur_def = m.equipped_weapon.as_ref().map(|w| w.defense_bonus).unwrap_or(0);
            (attack_bonus - cur_atk, defense_bonus - cur_def)
        }
        GearPreview::Armor { defense_bonus } => {
            let cur_def = m.equipped_armor.as_ref().map(|a| a.defense_bonus).unwrap_or(0);
            (0, defense_bonus - cur_def)
        }
        GearPreview::Ring { attack_bonus, defense_bonus, slot } => {
            let replace = match slot {
                Some(RingSlot::First) => 0,
                Some(RingSlot::Second) => 1,
                None => {
                    let slot_value = |i: usize| {
                        m.equipped_rings[i].as_ref().map(|r| r.attack_bonus + r.defense_bonus).unwrap_or(0)
                    };
                    if slot_value(0) <= slot_value(1) {
                        0
                    } else {
                        1
                    }
                }
            };
            let cur_atk = m.equipped_rings[replace].as_ref().map(|r| r.attack_bonus).unwrap_or(0);
            let cur_def = m.equipped_rings[replace].as_ref().map(|r| r.defense_bonus).unwrap_or(0);
            (attack_bonus - cur_atk, defense_bonus - cur_def)
        }
    }
}

/// Compact "(+N)"/"(-N)" delta suffix drawn right after a stat number,
/// colored red/green — returns the rendered width so the caller can lay out
/// whatever comes next (e.g. the DEF icon after an ATK delta) without
/// overlapping it.
fn push_delta_suffix(cmds: &mut Vec<TextCmd>, font: &Font, delta: i32, x: f32, y: f32) -> f32 {
    if delta == 0 {
        return 0.0;
    }
    let sign = if delta > 0 { "+" } else { "" };
    let text = format!("({sign}{delta})");
    let w = measure_text(&text, Some(font), 6, 1.0).width;
    push_text(cmds, text, x, y, 6.0, delta_color(delta));
    w
}

fn delta_color(diff: i32) -> Color {
    if diff > 0 {
        GREEN
    } else if diff < 0 {
        RED
    } else {
        DARKGRAY
    }
}

/// Comparable "how good is this ring" key for deciding which of a member's
/// two ring slots is weaker — `None` (an empty slot) sorts below every
/// `Some`, so an empty slot naturally reads as the weaker one when the other
/// slot is occupied. Same rarity-then-stat-total tiebreak already used ad
/// hoc by `gear_preview_delta`'s `slot_value` closure, promoted to a named
/// helper so `draw_ring_slot_picker` can reuse it.
fn ring_power(ring: Option<&Ring>) -> Option<(Rarity, i32)> {
    ring.map(|r| (r.rarity, r.attack_bonus + r.defense_bonus))
}

fn draw_member_picker(
    party: &Party,
    inventory: &Inventory,
    tab: InventoryTab,
    idx: usize,
    member_cursor: usize,
    y0: f32,
    cmds: &mut Vec<TextCmd>,
) {
    let target_name = match tab {
        InventoryTab::Items => inventory.items.get(idx).map(|(i, _)| i.name.clone()),
        InventoryTab::Weapons => inventory.weapons.get(idx).map(|w| w.display_name()),
        InventoryTab::Armor => inventory.armors.get(idx).map(|a| a.name.clone()),
        InventoryTab::Rings => inventory.rings.get(idx).map(|r| r.name.clone()),
        InventoryTab::Materials => None,
    }
    .unwrap_or_else(|| "???".to_string());

    push_text(cmds, format!("Give {target_name} to..."), 4.0, y0 + 10.0, 8.0, WHITE);

    // Each candidate is just a name + what they currently have — the ATK/DEF
    // delta itself lives only on the right-side party panel (via the
    // `gear_preview_for` preview computed in `draw`), the same uniform
    // (+N)/(-N) indicator shown when hovering an item in the bag list.
    let mut ty = y0 + 24.0;
    for (i, m) in party.members.iter().enumerate() {
        let selected = i == member_cursor;
        let color = if selected { YELLOW } else { WHITE };
        let marker = if selected { "> " } else { "  " };
        let current = match tab {
            InventoryTab::Weapons => m.equipped_weapon.as_ref().map(|w| w.display_name()),
            InventoryTab::Armor => m.equipped_armor.as_ref().map(|a| a.name.clone()),
            InventoryTab::Rings => None,
            InventoryTab::Items | InventoryTab::Materials => None,
        };
        let label = match current {
            Some(current) => format!("{marker}{} (has: {current})", m.name),
            None => format!("{marker}{}", m.name),
        };
        push_text(cmds, label, 4.0, ty, 8.0, color);
        ty += 12.0;
    }
}

fn draw_ring_slot_picker(
    font: &Font,
    party: &Party,
    inventory: &Inventory,
    idx: usize,
    member_idx: usize,
    slot_cursor: usize,
    y0: f32,
    cmds: &mut Vec<TextCmd>,
) {
    let ring_name = inventory.rings.get(idx).map(|r| r.name.clone()).unwrap_or_else(|| "???".to_string());
    let member = party.members.get(member_idx);
    let member_name = member.map(|m| m.name.as_str()).unwrap_or("???");
    push_text(cmds, format!("Give {ring_name} to {member_name}'s..."), 4.0, y0 + 10.0, 8.0, WHITE);

    let slot_rings: [Option<&Ring>; 2] = [
        member.and_then(|m| m.equipped_rings[0].as_ref()),
        member.and_then(|m| m.equipped_rings[1].as_ref()),
    ];
    let powers = [ring_power(slot_rings[0]), ring_power(slot_rings[1])];
    let weaker_slot = if slot_rings[0].is_some() || slot_rings[1].is_some() {
        if powers[0] < powers[1] {
            Some(0)
        } else if powers[1] < powers[0] {
            Some(1)
        } else {
            None
        }
    } else {
        None
    };

    let labels = ["First ring slot", "Second ring slot"];
    for (i, label) in labels.iter().enumerate() {
        let selected = i == slot_cursor;
        let marker = if selected { "> " } else { "  " };
        let mut x = 4.0;
        let y = y0 + 24.0 + i as f32 * 12.0;
        let prefix_color = if selected { YELLOW } else { WHITE };
        let prefix = format!("{marker}{label} (has: ");
        push_text(cmds, prefix.clone(), x, y, 8.0, prefix_color);
        x += measure_text(&prefix, Some(font), 8, 1.0).width;

        let name_text = slot_rings[i].map(|r| r.name.as_str()).unwrap_or("empty");
        let name_color = if selected {
            YELLOW
        } else if let Some(ring) = slot_rings[i] {
            rarity_color(ring.rarity)
        } else {
            WHITE
        };
        push_text(cmds, name_text, x, y, 8.0, name_color);
        x += measure_text(name_text, Some(font), 8, 1.0).width;

        let suffix_color = if selected { YELLOW } else { WHITE };
        push_text(cmds, ")", x, y, 8.0, suffix_color);
        x += measure_text(")", Some(font), 8, 1.0).width;

        if weaker_slot == Some(i) {
            push_text(cmds, " (weaker)", x, y, 8.0, RED);
        }
    }
}

fn draw_party_gear_hint(y0: f32, cmds: &mut Vec<TextCmd>) {
    push_text(cmds, "Party Gear", 4.0, y0 + 10.0, 9.0, WHITE);
    let lines = [
        "up/down choose character",
        "left/right/Tab choose slot",
        "Enter to act on it",
        "Esc to leave",
    ];
    for (i, line) in lines.iter().enumerate() {
        push_text(cmds, *line, 4.0, y0 + 26.0 + i as f32 * 11.0, 8.0, LIGHTGRAY);
    }
}

fn draw_party_gear_action_menu(action_cursor: usize, y0: f32, cmds: &mut Vec<TextCmd>) {
    push_text(cmds, "Choose an action", 4.0, y0 + 10.0, 8.0, WHITE);
    let labels = ["Unequip to bag", "Move to another member"];
    for (i, label) in labels.iter().enumerate() {
        let selected = i == action_cursor;
        let color = if selected { YELLOW } else { WHITE };
        let marker = if selected { "> " } else { "  " };
        push_text(cmds, format!("{marker}{label}"), 4.0, y0 + 24.0 + i as f32 * 12.0, 8.0, color);
    }
}

fn draw_party_gear_target_picker(
    party: &Party,
    from_member: usize,
    slot: EquipSlot,
    to_cursor: usize,
    y0: f32,
    cmds: &mut Vec<TextCmd>,
) {
    // The piece/source half of the preview is the same for every candidate,
    // so any non-source member serves to fill in the header.
    let header = (0..party.members.len())
        .find(|&i| i != from_member)
        .and_then(|i| move_preview(party, from_member, i, slot))
        .map(|p| format!("Move {} ({}) from {} to...", p.piece_name, slot_label(slot), p.from_name))
        .unwrap_or_else(|| "Move to...".to_string());
    let header_lines = wrap_lines(&header, 44, 2);
    for (li, line) in header_lines.iter().enumerate() {
        push_text(cmds, line, 4.0, y0 + 10.0 + li as f32 * 9.0, 8.0, WHITE);
    }

    // Each candidate is just a name + what they currently have in the target
    // slot — the ATK/DEF delta lives only on the right-side party panel (via
    // `move_target_preview`), the same uniform (+N)/(-N) indicator shown
    // when hovering an item in the bag list.
    let mut ty = y0 + 15.0 + header_lines.len() as f32 * 9.0;
    for (i, m) in party.members.iter().enumerate() {
        if i == from_member {
            continue;
        }
        let selected = i == to_cursor;
        let color = if selected { YELLOW } else { WHITE };
        let marker = if selected { "> " } else { "  " };
        let current = match slot {
            EquipSlot::Weapon => m.equipped_weapon.as_ref().map(|w| w.display_name()),
            EquipSlot::Armor => m.equipped_armor.as_ref().map(|a| a.name.clone()),
            EquipSlot::Ring(RingSlot::First) => m.equipped_rings[0].as_ref().map(|r| r.name.clone()),
            EquipSlot::Ring(RingSlot::Second) => m.equipped_rings[1].as_ref().map(|r| r.name.clone()),
        }
        .unwrap_or_else(|| "empty".to_string());
        push_text(cmds, format!("{marker}{} (has: {current})", m.name), 4.0, ty, 8.0, color);
        ty += 12.0;
    }
}

/// The active-roster picker for `InventoryMode::Roster` — pick who to bench
/// to make room for a recruit. Bench membership itself is shown below the
/// active list so the player can see who's waiting before committing.
fn draw_roster_picker(party: &Party, active_cursor: usize, y0: f32, cmds: &mut Vec<TextCmd>) {
    push_text(cmds, "Roster: pick who to bench...", 4.0, y0 + 10.0, 8.0, WHITE);

    let mut ty = y0 + 24.0;
    for (i, m) in party.members.iter().enumerate() {
        let selected = i == active_cursor;
        let color = if selected { YELLOW } else { WHITE };
        let marker = if selected { "> " } else { "  " };
        push_text(cmds, format!("{marker}{} (Lv{})", m.name, m.level), 4.0, ty, 8.0, color);
        ty += 12.0;
    }

    ty += 6.0;
    if party.bench.is_empty() {
        push_text(cmds, "Bench: empty", 4.0, ty, 8.0, GRAY);
    } else {
        push_text(cmds, "Bench:", 4.0, ty, 8.0, LIGHTGRAY);
        ty += 12.0;
        for m in &party.bench {
            push_text(cmds, format!("  {} (Lv{})", m.name, m.level), 4.0, ty, 8.0, LIGHTGRAY);
            ty += 12.0;
        }
    }
}

/// The bench picker for `InventoryMode::RosterTarget` — who replaces the
/// active member picked in `Roster`.
fn draw_roster_target_picker(party: &Party, active_idx: usize, bench_cursor: usize, y0: f32, cmds: &mut Vec<TextCmd>) {
    let outgoing_name = party.members.get(active_idx).map(|m| m.name.as_str()).unwrap_or("???");
    push_text(cmds, format!("Bench {outgoing_name} for..."), 4.0, y0 + 10.0, 8.0, WHITE);

    let mut ty = y0 + 24.0;
    for (i, m) in party.bench.iter().enumerate() {
        let selected = i == bench_cursor;
        let color = if selected { YELLOW } else { WHITE };
        let marker = if selected { "> " } else { "  " };
        push_text(cmds, format!("{marker}{} (Lv{})", m.name, m.level), 4.0, ty, 8.0, color);
        ty += 12.0;
    }
}

fn slot_label(slot: EquipSlot) -> &'static str {
    match slot {
        EquipSlot::Weapon => "Wpn",
        EquipSlot::Armor => "Arm",
        EquipSlot::Ring(RingSlot::First) => "R1",
        EquipSlot::Ring(RingSlot::Second) => "R2",
    }
}

/// Shared with `shop::draw` (called there with `InventoryMode::Browsing`,
/// which yields no slot highlight — the shop never lets you move gear).
/// Takes its own `x0`/`w` rather than the module's `RIGHT_X`/`RIGHT_W`
/// because the shop uses a different left/right split (65/35) than the
/// inventory screen (55/45).
pub(super) fn draw_party_gear(
    assets: &Assets,
    party: &Party,
    mode: &InventoryMode,
    preview: Option<&GearPreview>,
    x0: f32,
    y0: f32,
    w: f32,
    y1: f32,
    cmds: &mut Vec<TextCmd>,
) {
    draw_rectangle_lines(x0, y0, w, y1 - y0, 1.0, WHITE);
    push_text(cmds, "Party (p to move/unequip)", x0 + 3.0, y0 + 9.0, 7.0, LIGHTGRAY);

    let highlight: Option<(usize, EquipSlot)> = match *mode {
        InventoryMode::PartyGear { member_cursor, slot_cursor } => Some((member_cursor, EQUIP_SLOTS[slot_cursor])),
        InventoryMode::PartyGearAction { member_idx, slot, .. } => Some((member_idx, slot)),
        InventoryMode::PartyGearTarget { to_cursor, slot, .. } => Some((to_cursor, slot)),
        _ => None,
    };
    // While picking a move target, also mark where the piece is coming FROM
    // (blue, `<`) so the yellow target cursor can't read as a desync.
    let source: Option<(usize, EquipSlot)> = match *mode {
        InventoryMode::PartyGearTarget { from_member, slot, .. } => Some((from_member, slot)),
        _ => None,
    };

    // Equip-slot rows below reserve a leading character for the ">"/"<"
    // cursor marker (blank when neither applies), which pushes their text
    // one glyph right of `bar_x`. The name/HP/MP header and the ATK/DEF
    // footer have no marker of their own, so without matching that same
    // indent here, every row in a member's block reads at a different left
    // edge — `text_x` is the shared column all of them line up on instead.
    let indent = measure_text(" ", Some(&assets.font), 6, 1.0).width;

    let member_h = (y1 - (y0 + 12.0)) / party.members.len().max(1) as f32;
    for (mi, m) in party.members.iter().enumerate() {
        let base_y = y0 + 12.0 + mi as f32 * member_h;
        let bar_x = x0 + 3.0;
        let bar_w = w - 6.0;
        let text_x = bar_x + indent;
        push_text(cmds, m.name.clone(), text_x, base_y + 6.0, 7.0, WHITE);
        let ratio = m.hp_ratio().clamp(0.0, 1.0) as f32;
        draw_rectangle(bar_x, base_y + 8.0, bar_w, 2.0, DARKGRAY);
        draw_rectangle(bar_x, base_y + 8.0, bar_w * ratio, 2.0, hp_color(m.hp_ratio()));
        push_text(
            cmds,
            format!("HP{}/{}", m.stats.hp, m.stats.max_hp),
            text_x,
            base_y + 15.0,
            6.0,
            stat_color(AllocStat::MaxHp),
        );
        push_text(
            cmds,
            format!("MP{}/{}", m.stats.mp, m.stats.max_mp),
            text_x + bar_w / 2.0,
            base_y + 15.0,
            6.0,
            stat_color(AllocStat::MaxMp),
        );

        let mut sy = base_y + 21.0;
        for &slot in EQUIP_SLOTS.iter() {
            let selected = highlight == Some((mi, slot));
            let (name, color) = match slot {
                EquipSlot::Weapon => m
                    .equipped_weapon
                    .as_ref()
                    .map(|w| (w.display_name(), rarity_color(w.rarity)))
                    .unwrap_or_else(|| ("(empty)".to_string(), DARKGRAY)),
                EquipSlot::Armor => m
                    .equipped_armor
                    .as_ref()
                    .map(|a| (a.name.clone(), rarity_color(a.rarity)))
                    .unwrap_or_else(|| ("(empty)".to_string(), DARKGRAY)),
                EquipSlot::Ring(RingSlot::First) => m.equipped_rings[0]
                    .as_ref()
                    .map(|r| (r.name.clone(), rarity_color(r.rarity)))
                    .unwrap_or_else(|| ("(empty)".to_string(), DARKGRAY)),
                EquipSlot::Ring(RingSlot::Second) => m.equipped_rings[1]
                    .as_ref()
                    .map(|r| (r.name.clone(), rarity_color(r.rarity)))
                    .unwrap_or_else(|| ("(empty)".to_string(), DARKGRAY)),
            };
            let is_source = source == Some((mi, slot));
            if selected {
                draw_rectangle(bar_x - 1.0, sy - 6.0, bar_w + 2.0, 8.0, Color::new(1.0, 1.0, 0.0, 0.25));
            } else if is_source {
                draw_rectangle(bar_x - 1.0, sy - 6.0, bar_w + 2.0, 8.0, Color::new(0.3, 0.55, 1.0, 0.25));
            }
            let name_color = if selected { YELLOW } else { color };
            let marker = if selected {
                ">"
            } else if is_source {
                "<"
            } else {
                " "
            };
            push_text(cmds, format!("{marker}{}:{name}", slot_label(slot)), bar_x, sy, 6.0, name_color);
            sy += 6.0;
        }
        let (delta_atk, delta_def) = preview.map(|p| gear_preview_delta(m, p)).unwrap_or((0, 0));
        let stat_icon_size = 8.0;
        draw_icon(
            &assets.characters,
            weapon_icon_rect(),
            text_x,
            icon_y_for_text(sy + 2.0, 6.0, stat_icon_size),
            stat_icon_size,
        );
        let atk_text = format!("ATK {}", m.total_attack());
        let atk_x = text_x + stat_icon_size + 2.0;
        push_text(cmds, atk_text.clone(), atk_x, sy + 2.0, 6.0, stat_color(AllocStat::Attack));
        // DEF is placed relative to the ATK text's (plus any delta suffix's)
        // actual rendered width rather than a fixed bar_w/2.0 half-column
        // split, so it neither collides with a wide ATK value nor leaves an
        // oversized gap after a narrow one.
        let atk_w = measure_text(&atk_text, Some(&assets.font), 6, 1.0).width;
        let delta_atk_w = push_delta_suffix(cmds, &assets.font, delta_atk, atk_x + atk_w + 2.0, sy + 2.0);
        let extra = if delta_atk_w > 0.0 { delta_atk_w + 2.0 } else { 0.0 };
        let def_x = atk_x + atk_w + extra + 6.0;
        draw_icon(
            &assets.characters,
            armor_icon_rect(),
            def_x,
            icon_y_for_text(sy + 2.0, 6.0, stat_icon_size),
            stat_icon_size,
        );
        let def_text = format!("DEF {}", m.total_defense());
        let def_text_x = def_x + stat_icon_size + 2.0;
        push_text(cmds, def_text.clone(), def_text_x, sy + 2.0, 6.0, stat_color(AllocStat::Defense));
        let def_w = measure_text(&def_text, Some(&assets.font), 6, 1.0).width;
        push_delta_suffix(cmds, &assets.font, delta_def, def_text_x + def_w + 2.0, sy + 2.0);
    }
}

fn draw_footer(message: Option<&str>, y0: f32, y1: f32, cmds: &mut Vec<TextCmd>) {
    draw_rectangle_lines(0.0, y0, CANVAS_WIDTH, y1 - y0, 1.0, WHITE);
    let text = message.unwrap_or("Rarer gear is stronger: Common < Uncommon < Rare < Epic < Legendary.");
    push_text(cmds, text, 4.0, y0 + 12.0, 7.0, WHITE);
}
