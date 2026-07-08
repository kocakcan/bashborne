use macroquad::prelude::*;

use crate::game::character::{AllocStat, RingSlot};
use crate::game::inventory_ui::{EquipSlot, InventoryMode, InventoryTab, InventoryUiState, EQUIP_SLOTS};
use crate::game::item::Inventory;
use crate::game::party::Party;
use crate::render::assets::{
    armor_icon_rect, item_kind_icon_rect, material_icon_rect, ring_icon_rect, weapon_icon_rect, Assets,
    CANVAS_HEIGHT, CANVAS_WIDTH,
};
use crate::render::common::{draw_icon, hp_color, push_text, rarity_color, scroll_window, stat_color, TextCmd};

/// Icons are drawn at this size, with row text shifted this many px to the
/// right of where it used to start, to leave room for them.
const ICON_SIZE: f32 = 8.0;
const ICON_GAP: f32 = 10.0;

// Leaves room for the persistent status bar chrome drawn at y 0-12 (see
// `hud::draw_status_bar`) — the same convention `explore::MAP_TOP` and
// `combat::draw`'s enemy-panel `y` use.
const TAB_Y: f32 = 12.0;
const TAB_H: f32 = 18.0;
const FOOTER_H: f32 = 26.0;
const LEFT_W: f32 = CANVAS_WIDTH * 0.55;
const RIGHT_X: f32 = LEFT_W;
const RIGHT_W: f32 = CANVAS_WIDTH - LEFT_W;

/// Clips a description/source line to fit the left panel's width at the 7px
/// detail-line font size — the left panel is only ~260 logical px wide, far
/// short of the terminal width the original ratatui text assumed.
fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max_chars.saturating_sub(3)).collect();
        out.push_str("...");
        out
    }
}

pub fn draw(assets: &Assets, inv_ui: &InventoryUiState, party: &Party, inventory: &Inventory, cmds: &mut Vec<TextCmd>) {
    let content_y0 = TAB_Y + TAB_H;
    let content_y1 = CANVAS_HEIGHT - FOOTER_H;

    draw_tabs(inv_ui.tab, cmds);
    draw_rectangle_lines(0.0, content_y0, CANVAS_WIDTH, content_y1 - content_y0, 1.0, WHITE);

    match &inv_ui.mode {
        InventoryMode::Browsing => draw_list(assets, inv_ui, inventory, content_y0, cmds),
        InventoryMode::SelectMember { tab, idx, member_cursor } => {
            draw_member_picker(party, inventory, *tab, *idx, *member_cursor, content_y0, cmds)
        }
        InventoryMode::SelectRingSlot { idx, member_idx, slot_cursor } => {
            draw_ring_slot_picker(party, inventory, *idx, *member_idx, *slot_cursor, content_y0, cmds)
        }
        InventoryMode::PartyGear { .. } => draw_party_gear_hint(content_y0, cmds),
        InventoryMode::PartyGearAction { action_cursor, .. } => {
            draw_party_gear_action_menu(*action_cursor, content_y0, cmds)
        }
        InventoryMode::PartyGearTarget { from_member, to_cursor, .. } => {
            draw_party_gear_target_picker(party, *from_member, *to_cursor, content_y0, cmds)
        }
    }

    draw_party_gear(party, &inv_ui.mode, RIGHT_X, content_y0, RIGHT_W, content_y1, cmds);
    draw_footer(inv_ui.message.as_deref(), content_y1, CANVAS_HEIGHT, cmds);
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
        "Tab/left-right switch, p party gear, Esc close",
        4.0,
        TAB_Y + TAB_H - 2.0,
        7.0,
        GRAY,
    );
}

fn draw_list(assets: &Assets, inv_ui: &InventoryUiState, inventory: &Inventory, y0: f32, cmds: &mut Vec<TextCmd>) {
    let pad = 4.0;
    let text_pad = pad + ICON_GAP;
    let visible = 6usize;
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
                let ty = y0 + 10.0 + row as f32 * 30.0;
                if selected {
                    draw_rectangle(0.0, ty - 8.0, LEFT_W, 28.0, Color::new(1.0, 1.0, 1.0, 0.12));
                }
                let color = if selected { YELLOW } else { WHITE };
                let marker = if selected { "> " } else { "  " };
                draw_icon(&assets.tiles, item_kind_icon_rect(&item.kind), pad, ty - 7.0, ICON_SIZE);
                push_text(cmds, format!("{marker}{} x{qty}", item.name), text_pad, ty, 8.0, color);
                push_text(cmds, truncate(&item.description, 44), text_pad + 4.0, ty + 11.0, 7.0, LIGHTGRAY);
            }
        }
        InventoryTab::Weapons => {
            if inventory.weapons.is_empty() {
                push_text(cmds, "No spare weapons.", pad, y0 + 12.0, 8.0, GRAY);
                return;
            }
            let range = scroll_window(inventory.weapons.len(), inv_ui.cursor, visible);
            for (row, i) in range.enumerate() {
                let w = &inventory.weapons[i];
                let selected = i == inv_ui.cursor;
                let ty = y0 + 10.0 + row as f32 * 30.0;
                if selected {
                    draw_rectangle(0.0, ty - 8.0, LEFT_W, 28.0, Color::new(1.0, 1.0, 1.0, 0.12));
                }
                let marker = if selected { "> " } else { "  " };
                let mut stats = format!("ATK+{}", w.attack_bonus);
                if w.defense_bonus > 0 {
                    stats.push_str(&format!(" DEF+{}", w.defense_bonus));
                }
                draw_icon(&assets.characters, weapon_icon_rect(), pad, ty - 7.0, ICON_SIZE);
                push_text(
                    cmds,
                    format!("{marker}{} [{}] {stats}", w.display_name(), w.rarity),
                    text_pad,
                    ty,
                    8.0,
                    rarity_color(w.rarity),
                );
                let detail_color = if selected { LIGHTGRAY } else { DARKGRAY };
                push_text(
                    cmds,
                    truncate(&format!("{} - {}", w.description, w.source), 44),
                    text_pad + 4.0,
                    ty + 11.0,
                    7.0,
                    detail_color,
                );
                if let Some(passive) = &w.passive {
                    push_text(cmds, truncate(&passive.description(), 44), text_pad + 4.0, ty + 20.0, 7.0, YELLOW);
                }
            }
        }
        InventoryTab::Armor => {
            if inventory.armors.is_empty() {
                push_text(cmds, "No spare armor.", pad, y0 + 12.0, 8.0, GRAY);
                return;
            }
            let range = scroll_window(inventory.armors.len(), inv_ui.cursor, visible);
            for (row, i) in range.enumerate() {
                let a = &inventory.armors[i];
                let selected = i == inv_ui.cursor;
                let ty = y0 + 10.0 + row as f32 * 30.0;
                if selected {
                    draw_rectangle(0.0, ty - 8.0, LEFT_W, 28.0, Color::new(1.0, 1.0, 1.0, 0.12));
                }
                let marker = if selected { "> " } else { "  " };
                draw_icon(&assets.characters, armor_icon_rect(), pad, ty - 7.0, ICON_SIZE);
                push_text(
                    cmds,
                    format!("{marker}{} [{}] DEF+{}", a.name, a.rarity, a.defense_bonus),
                    text_pad,
                    ty,
                    8.0,
                    rarity_color(a.rarity),
                );
                let detail_color = if selected { LIGHTGRAY } else { DARKGRAY };
                push_text(
                    cmds,
                    truncate(&format!("{} - {}", a.description, a.source), 44),
                    text_pad + 4.0,
                    ty + 11.0,
                    7.0,
                    detail_color,
                );
            }
        }
        InventoryTab::Rings => {
            if inventory.rings.is_empty() {
                push_text(cmds, "No spare rings.", pad, y0 + 12.0, 8.0, GRAY);
                return;
            }
            let range = scroll_window(inventory.rings.len(), inv_ui.cursor, visible);
            for (row, i) in range.enumerate() {
                let r = &inventory.rings[i];
                let selected = i == inv_ui.cursor;
                let ty = y0 + 10.0 + row as f32 * 30.0;
                if selected {
                    draw_rectangle(0.0, ty - 8.0, LEFT_W, 28.0, Color::new(1.0, 1.0, 1.0, 0.12));
                }
                let marker = if selected { "> " } else { "  " };
                let mut bonus = String::new();
                if r.attack_bonus > 0 {
                    bonus.push_str(&format!("ATK+{} ", r.attack_bonus));
                }
                if r.defense_bonus > 0 {
                    bonus.push_str(&format!("DEF+{}", r.defense_bonus));
                }
                draw_icon(&assets.tiles, ring_icon_rect(), pad, ty - 7.0, ICON_SIZE);
                push_text(
                    cmds,
                    format!("{marker}{} [{}] {bonus}", r.name, r.rarity),
                    text_pad,
                    ty,
                    8.0,
                    rarity_color(r.rarity),
                );
                let detail_color = if selected { LIGHTGRAY } else { DARKGRAY };
                push_text(
                    cmds,
                    truncate(&format!("{} - {}", r.description, r.source), 44),
                    text_pad + 4.0,
                    ty + 11.0,
                    7.0,
                    detail_color,
                );
            }
        }
        InventoryTab::Materials => {
            if inventory.upgrade_materials == 0 {
                push_text(cmds, "No materials yet.", pad, y0 + 12.0, 8.0, GRAY);
            } else {
                let selected = inv_ui.cursor == 0;
                let color = if selected { YELLOW } else { WHITE };
                draw_icon(&assets.tiles, material_icon_rect(), pad, y0 + 5.0, ICON_SIZE);
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

fn delta_color(diff: i32) -> Color {
    if diff > 0 {
        GREEN
    } else if diff < 0 {
        RED
    } else {
        DARKGRAY
    }
}

fn push_delta_line(cmds: &mut Vec<TextCmd>, label: &str, current: i32, new: i32, x: f32, y: f32) {
    let diff = new - current;
    let sign = if diff > 0 { format!("+{diff}") } else { diff.to_string() };
    push_text(
        cmds,
        format!("{label} {current} -> {new} ({sign})"),
        x,
        y,
        7.0,
        delta_color(diff),
    );
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

    let mut ty = y0 + 24.0;
    for (i, m) in party.members.iter().enumerate() {
        let selected = i == member_cursor;
        let color = if selected { YELLOW } else { WHITE };
        let marker = if selected { "> " } else { "  " };
        match tab {
            InventoryTab::Weapons => {
                let (cur_atk, cur_def, current) = m
                    .equipped_weapon
                    .as_ref()
                    .map(|w| (w.attack_bonus, w.defense_bonus, w.display_name()))
                    .unwrap_or((0, 0, "unarmed".to_string()));
                let (new_atk, new_def) =
                    inventory.weapons.get(idx).map(|w| (w.attack_bonus, w.defense_bonus)).unwrap_or((0, 0));
                push_text(cmds, format!("{marker}{} (has: {current})", m.name), 4.0, ty, 8.0, color);
                push_delta_line(cmds, "ATK", cur_atk, new_atk, 12.0, ty + 10.0);
                ty += if cur_def != 0 || new_def != 0 {
                    push_delta_line(cmds, "DEF", cur_def, new_def, 12.0, ty + 19.0);
                    28.0
                } else {
                    18.0
                };
            }
            InventoryTab::Armor => {
                let current = m.equipped_armor.as_ref().map(|a| a.name.as_str()).unwrap_or("no armor");
                let cur_def = m.equipped_armor.as_ref().map(|a| a.defense_bonus).unwrap_or(0);
                let new_def = inventory.armors.get(idx).map(|a| a.defense_bonus).unwrap_or(0);
                push_text(cmds, format!("{marker}{} (has: {current})", m.name), 4.0, ty, 8.0, color);
                push_delta_line(cmds, "DEF", cur_def, new_def, 12.0, ty + 10.0);
                ty += 18.0;
            }
            InventoryTab::Rings => {
                let (new_atk, new_def) =
                    inventory.rings.get(idx).map(|r| (r.attack_bonus, r.defense_bonus)).unwrap_or((0, 0));
                push_text(cmds, format!("{marker}{}", m.name), 4.0, ty, 8.0, color);
                let mut ly = ty + 10.0;
                for (slot_label, ring) in [("R1", &m.equipped_rings[0]), ("R2", &m.equipped_rings[1])] {
                    let (cur_atk, cur_def) = ring.as_ref().map(|r| (r.attack_bonus, r.defense_bonus)).unwrap_or((0, 0));
                    push_delta_line(cmds, &format!("{slot_label} ATK"), cur_atk, new_atk, 12.0, ly);
                    ly += 9.0;
                    push_delta_line(cmds, &format!("{slot_label} DEF"), cur_def, new_def, 12.0, ly);
                    ly += 9.0;
                }
                ty = ly;
            }
            InventoryTab::Items | InventoryTab::Materials => {
                push_text(cmds, format!("{marker}{}", m.name), 4.0, ty, 8.0, color);
                ty += 12.0;
            }
        }
    }
}

fn draw_ring_slot_picker(
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

    let labels = ["First ring slot", "Second ring slot"];
    for (i, label) in labels.iter().enumerate() {
        let current = member
            .and_then(|m| m.equipped_rings[i].as_ref())
            .map(|r| r.name.as_str())
            .unwrap_or("empty");
        let selected = i == slot_cursor;
        let color = if selected { YELLOW } else { WHITE };
        let marker = if selected { "> " } else { "  " };
        push_text(
            cmds,
            format!("{marker}{label} (has: {current})"),
            4.0,
            y0 + 24.0 + i as f32 * 12.0,
            8.0,
            color,
        );
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

fn draw_party_gear_target_picker(party: &Party, from_member: usize, to_cursor: usize, y0: f32, cmds: &mut Vec<TextCmd>) {
    push_text(cmds, "Move to...", 4.0, y0 + 10.0, 8.0, WHITE);
    let mut row = 0.0;
    for (i, m) in party.members.iter().enumerate() {
        if i == from_member {
            continue;
        }
        let selected = i == to_cursor;
        let color = if selected { YELLOW } else { WHITE };
        let marker = if selected { "> " } else { "  " };
        push_text(cmds, format!("{marker}{}", m.name), 4.0, y0 + 24.0 + row * 12.0, 8.0, color);
        row += 1.0;
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
pub(super) fn draw_party_gear(party: &Party, mode: &InventoryMode, x0: f32, y0: f32, w: f32, y1: f32, cmds: &mut Vec<TextCmd>) {
    draw_rectangle_lines(x0, y0, w, y1 - y0, 1.0, WHITE);
    push_text(cmds, "Party (p to move/unequip)", x0 + 3.0, y0 + 9.0, 7.0, LIGHTGRAY);

    let highlight: Option<(usize, EquipSlot)> = match *mode {
        InventoryMode::PartyGear { member_cursor, slot_cursor } => Some((member_cursor, EQUIP_SLOTS[slot_cursor])),
        InventoryMode::PartyGearAction { member_idx, slot, .. } => Some((member_idx, slot)),
        InventoryMode::PartyGearTarget { to_cursor, slot, .. } => Some((to_cursor, slot)),
        _ => None,
    };

    let member_h = (y1 - (y0 + 12.0)) / party.members.len().max(1) as f32;
    for (mi, m) in party.members.iter().enumerate() {
        let base_y = y0 + 12.0 + mi as f32 * member_h;
        let bar_x = x0 + 3.0;
        let bar_w = w - 6.0;
        push_text(cmds, m.name.clone(), bar_x, base_y + 6.0, 7.0, WHITE);
        let ratio = m.hp_ratio().clamp(0.0, 1.0) as f32;
        draw_rectangle(bar_x, base_y + 8.0, bar_w, 2.0, DARKGRAY);
        draw_rectangle(bar_x, base_y + 8.0, bar_w * ratio, 2.0, hp_color(m.hp_ratio()));
        push_text(
            cmds,
            format!("HP{}/{}", m.stats.hp, m.stats.max_hp),
            bar_x,
            base_y + 15.0,
            6.0,
            stat_color(AllocStat::MaxHp),
        );
        push_text(
            cmds,
            format!("MP{}/{}", m.stats.mp, m.stats.max_mp),
            bar_x + bar_w / 2.0,
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
            if selected {
                draw_rectangle(bar_x - 1.0, sy - 6.0, bar_w + 2.0, 8.0, Color::new(1.0, 1.0, 0.0, 0.25));
            }
            let name_color = if selected { YELLOW } else { color };
            let marker = if selected { ">" } else { " " };
            push_text(cmds, format!("{marker}{}:{name}", slot_label(slot)), bar_x, sy, 6.0, name_color);
            sy += 6.0;
        }
        push_text(
            cmds,
            format!("ATK {}", m.total_attack()),
            bar_x,
            sy + 2.0,
            6.0,
            stat_color(AllocStat::Attack),
        );
        push_text(
            cmds,
            format!("DEF {}", m.total_defense()),
            bar_x + bar_w / 2.0,
            sy + 2.0,
            6.0,
            stat_color(AllocStat::Defense),
        );
    }
}

fn draw_footer(message: Option<&str>, y0: f32, y1: f32, cmds: &mut Vec<TextCmd>) {
    draw_rectangle_lines(0.0, y0, CANVAS_WIDTH, y1 - y0, 1.0, WHITE);
    let text = message.unwrap_or("Rarer gear is stronger: Common < Uncommon < Rare < Epic < Legendary.");
    push_text(cmds, text, 4.0, y0 + 12.0, 7.0, WHITE);
}
