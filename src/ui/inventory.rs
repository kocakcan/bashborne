use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::game::character::RingSlot;
use crate::game::inventory_ui::{
    EquipSlot, InventoryMode, InventoryTab, InventoryUiState, EQUIP_SLOTS,
};
use crate::game::item::Inventory;
use crate::game::party::Party;

pub fn draw(frame: &mut Frame, inv_ui: &InventoryUiState, party: &Party, inventory: &Inventory) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(frame.size());

    draw_tabs(frame, outer[0], inv_ui.tab);

    let mid = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(outer[1]);

    match &inv_ui.mode {
        InventoryMode::Browsing => draw_list(frame, mid[0], inv_ui, inventory),
        InventoryMode::SelectMember {
            tab,
            idx,
            member_cursor,
        } => draw_member_picker(frame, mid[0], party, *tab, *idx, *member_cursor, inventory),
        InventoryMode::SelectRingSlot {
            idx,
            member_idx,
            slot_cursor,
        } => draw_ring_slot_picker(
            frame,
            mid[0],
            party,
            inventory,
            *idx,
            *member_idx,
            *slot_cursor,
        ),
        InventoryMode::PartyGear { .. } => draw_party_gear_hint(frame, mid[0]),
        InventoryMode::PartyGearAction { action_cursor, .. } => {
            draw_party_gear_action_menu(frame, mid[0], *action_cursor)
        }
        InventoryMode::PartyGearTarget {
            from_member,
            to_cursor,
            ..
        } => draw_party_gear_target_picker(frame, mid[0], party, *from_member, *to_cursor),
    }

    draw_party_gear(frame, mid[1], party, &inv_ui.mode);
    draw_footer(frame, outer[2], inv_ui);
}

fn draw_tabs(frame: &mut Frame, area: Rect, active: InventoryTab) {
    let make_span = |label: &str, tab: InventoryTab| {
        let style = if tab == active {
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        Span::styled(format!(" {label} "), style)
    };
    let line = Line::from(vec![
        make_span("Items", InventoryTab::Items),
        Span::raw("  "),
        make_span("Weapons", InventoryTab::Weapons),
        Span::raw("  "),
        make_span("Armor", InventoryTab::Armor),
        Span::raw("  "),
        make_span("Rings", InventoryTab::Rings),
        Span::raw("  "),
        make_span("Materials", InventoryTab::Materials),
    ]);
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Inventory (Tab/←→ switch tabs, p party gear, Esc close)");
    frame.render_widget(Paragraph::new(line).block(block), area);
}

fn cursor_style(selected: bool) -> Style {
    if selected {
        Style::default()
            .fg(Color::Black)
            .bg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    }
}

fn draw_list(frame: &mut Frame, area: Rect, inv_ui: &InventoryUiState, inventory: &Inventory) {
    match inv_ui.tab {
        InventoryTab::Items => {
            let items: Vec<ListItem> = inventory
                .items
                .iter()
                .enumerate()
                .map(|(i, (item, qty))| {
                    let selected = i == inv_ui.cursor;
                    let header = Line::from(Span::styled(
                        format!("{} x{qty}", item.name),
                        cursor_style(selected),
                    ));
                    let detail_style = if selected {
                        Style::default().fg(Color::White)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };
                    let detail = Line::from(Span::styled(
                        format!("     {}", item.description),
                        detail_style,
                    ));
                    ListItem::new(Text::from(vec![header, detail]))
                })
                .collect();
            let block = Block::default()
                .borders(Borders::ALL)
                .title("Items (↑↓ select, Enter to use)");
            if items.is_empty() {
                frame.render_widget(Paragraph::new("No items.").block(block), area);
            } else {
                frame.render_widget(List::new(items).block(block), area);
            }
        }
        InventoryTab::Weapons => {
            let items: Vec<ListItem> = inventory
                .weapons
                .iter()
                .enumerate()
                .map(|(i, w)| {
                    let color = crate::ui::rarity_color(w.rarity);
                    let marker = if i == inv_ui.cursor { "> " } else { "  " };
                    let header = Line::from(vec![
                        Span::raw(marker),
                        Span::styled(
                            format!("{:<20}", w.display_name()),
                            Style::default().fg(color).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(format!("[{}] ", w.rarity), Style::default().fg(color)),
                        Span::raw(format!("ATK +{}", w.attack_bonus)),
                        if w.defense_bonus > 0 {
                            Span::raw(format!(" DEF +{}", w.defense_bonus))
                        } else {
                            Span::raw("")
                        },
                    ]);
                    let selected = i == inv_ui.cursor;
                    let detail_style = if selected {
                        Style::default().fg(Color::White)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };
                    let detail = Line::from(Span::styled(
                        format!("     {} — {}", w.description, w.source),
                        detail_style,
                    ));
                    let bg = if selected {
                        Style::default().bg(Color::DarkGray)
                    } else {
                        Style::default()
                    };
                    let mut lines = vec![header, detail];
                    if let Some(passive) = w.passive {
                        lines.push(Line::from(Span::styled(
                            format!("     {}", passive.description()),
                            Style::default().fg(Color::Yellow),
                        )));
                    }
                    ListItem::new(Text::from(lines)).style(bg)
                })
                .collect();
            let block = Block::default()
                .borders(Borders::ALL)
                .title("Weapons (↑↓ select, Enter to equip)");
            if items.is_empty() {
                frame.render_widget(
                    Paragraph::new("No spare weapons. Find some in the field or from battle!")
                        .block(block),
                    area,
                );
            } else {
                frame.render_widget(List::new(items).block(block), area);
            }
        }
        InventoryTab::Armor => {
            let items: Vec<ListItem> = inventory
                .armors
                .iter()
                .enumerate()
                .map(|(i, a)| {
                    let color = crate::ui::rarity_color(a.rarity);
                    let marker = if i == inv_ui.cursor { "> " } else { "  " };
                    let header = Line::from(vec![
                        Span::raw(marker),
                        Span::styled(
                            format!("{:<20}", a.name),
                            Style::default().fg(color).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(format!("[{}] ", a.rarity), Style::default().fg(color)),
                        Span::raw(format!("DEF +{}", a.defense_bonus)),
                    ]);
                    let selected = i == inv_ui.cursor;
                    let detail_style = if selected {
                        Style::default().fg(Color::White)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };
                    let detail = Line::from(Span::styled(
                        format!("     {} — {}", a.description, a.source),
                        detail_style,
                    ));
                    let bg = if selected {
                        Style::default().bg(Color::DarkGray)
                    } else {
                        Style::default()
                    };
                    ListItem::new(Text::from(vec![header, detail])).style(bg)
                })
                .collect();
            let block = Block::default()
                .borders(Borders::ALL)
                .title("Armor (↑↓ select, Enter to equip)");
            if items.is_empty() {
                frame.render_widget(
                    Paragraph::new("No spare armor. Find some in the field or from battle!")
                        .block(block),
                    area,
                );
            } else {
                frame.render_widget(List::new(items).block(block), area);
            }
        }
        InventoryTab::Rings => {
            let items: Vec<ListItem> = inventory
                .rings
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    let color = crate::ui::rarity_color(r.rarity);
                    let marker = if i == inv_ui.cursor { "> " } else { "  " };
                    let mut bonus = String::new();
                    if r.attack_bonus > 0 {
                        bonus.push_str(&format!("ATK +{} ", r.attack_bonus));
                    }
                    if r.defense_bonus > 0 {
                        bonus.push_str(&format!("DEF +{}", r.defense_bonus));
                    }
                    let header = Line::from(vec![
                        Span::raw(marker),
                        Span::styled(
                            format!("{:<20}", r.name),
                            Style::default().fg(color).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(format!("[{}] ", r.rarity), Style::default().fg(color)),
                        Span::raw(bonus),
                    ]);
                    let selected = i == inv_ui.cursor;
                    let detail_style = if selected {
                        Style::default().fg(Color::White)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };
                    let detail = Line::from(Span::styled(
                        format!("     {} — {}", r.description, r.source),
                        detail_style,
                    ));
                    let bg = if selected {
                        Style::default().bg(Color::DarkGray)
                    } else {
                        Style::default()
                    };
                    ListItem::new(Text::from(vec![header, detail])).style(bg)
                })
                .collect();
            let block = Block::default()
                .borders(Borders::ALL)
                .title("Rings (↑↓ select, Enter to equip)");
            if items.is_empty() {
                frame.render_widget(
                    Paragraph::new("No spare rings. Find some in the field or from battle!")
                        .block(block),
                    area,
                );
            } else {
                frame.render_widget(List::new(items).block(block), area);
            }
        }
        InventoryTab::Materials => {
            let block = Block::default()
                .borders(Borders::ALL)
                .title("Materials (used by the blacksmith)");
            if inventory.upgrade_materials == 0 {
                frame.render_widget(Paragraph::new("No materials yet.").block(block), area);
            } else {
                let style = cursor_style(inv_ui.cursor == 0);
                let item = ListItem::new(Line::from(Span::styled(
                    format!("Titanite Shard x{}", inventory.upgrade_materials),
                    style,
                )));
                frame.render_widget(List::new(vec![item]).block(block), area);
            }
        }
    }
}

fn draw_member_picker(
    frame: &mut Frame,
    area: Rect,
    party: &Party,
    tab: InventoryTab,
    idx: usize,
    member_cursor: usize,
    inventory: &Inventory,
) {
    let target_name = match tab {
        InventoryTab::Items => inventory.items.get(idx).map(|(i, _)| i.name.clone()),
        InventoryTab::Weapons => inventory.weapons.get(idx).map(|w| w.display_name()),
        InventoryTab::Armor => inventory.armors.get(idx).map(|a| a.name.clone()),
        InventoryTab::Rings => inventory.rings.get(idx).map(|r| r.name.clone()),
        // Materials never reach the member-picker (see app.rs's Browsing Enter
        // handler), but the match must stay exhaustive.
        InventoryTab::Materials => None,
    }
    .unwrap_or_else(|| "???".to_string());

    let items: Vec<ListItem> = party
        .members
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let style = cursor_style(i == member_cursor);
            let label = match tab {
                InventoryTab::Weapons => {
                    let current = m
                        .equipped_weapon
                        .as_ref()
                        .map(|w| w.display_name())
                        .unwrap_or_else(|| "unarmed".to_string());
                    format!("{:<8} (currently: {current})", m.name)
                }
                InventoryTab::Armor => {
                    let current = m
                        .equipped_armor
                        .as_ref()
                        .map(|a| a.name.as_str())
                        .unwrap_or("no armor");
                    format!("{:<8} (currently: {current})", m.name)
                }
                InventoryTab::Items | InventoryTab::Rings | InventoryTab::Materials => {
                    format!("{:<8}", m.name)
                }
            };
            ListItem::new(Line::from(Span::styled(label, style)))
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!("Give {target_name} to... (↑↓ Enter, Esc cancel)"));
    frame.render_widget(List::new(items).block(block), area);
}

fn draw_ring_slot_picker(
    frame: &mut Frame,
    area: Rect,
    party: &Party,
    inventory: &Inventory,
    idx: usize,
    member_idx: usize,
    slot_cursor: usize,
) {
    let ring_name = inventory
        .rings
        .get(idx)
        .map(|r| r.name.clone())
        .unwrap_or_else(|| "???".to_string());
    let member = party.members.get(member_idx);
    let member_name = member.map(|m| m.name.as_str()).unwrap_or("???");
    let labels = ["First ring slot", "Second ring slot"];
    let items: Vec<ListItem> = labels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let current = member
                .and_then(|m| m.equipped_rings[i].as_ref())
                .map(|r| r.name.as_str())
                .unwrap_or("empty");
            let style = cursor_style(i == slot_cursor);
            ListItem::new(Line::from(Span::styled(
                format!("{label} (currently: {current})"),
                style,
            )))
        })
        .collect();
    let block = Block::default().borders(Borders::ALL).title(format!(
        "Give {ring_name} to {member_name}'s... (↑↓ Enter, Esc cancel)"
    ));
    frame.render_widget(List::new(items).block(block), area);
}

fn draw_party_gear_hint(frame: &mut Frame, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title("Party Gear");
    let text = "↑↓ choose character\n←→/Tab choose slot\nEnter to act on it\nEsc to leave";
    frame.render_widget(Paragraph::new(text).block(block), area);
}

fn draw_party_gear_action_menu(frame: &mut Frame, area: Rect, action_cursor: usize) {
    let labels = ["Unequip to bag", "Move to another member"];
    let items: Vec<ListItem> = labels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            ListItem::new(Line::from(Span::styled(
                *label,
                cursor_style(i == action_cursor),
            )))
        })
        .collect();
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Choose an action (↑↓ Enter, Esc cancel)");
    frame.render_widget(List::new(items).block(block), area);
}

fn draw_party_gear_target_picker(
    frame: &mut Frame,
    area: Rect,
    party: &Party,
    from_member: usize,
    to_cursor: usize,
) {
    let items: Vec<ListItem> = party
        .members
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != from_member)
        .map(|(i, m)| {
            ListItem::new(Line::from(Span::styled(
                m.name.clone(),
                cursor_style(i == to_cursor),
            )))
        })
        .collect();
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Move to... (↑↓ Enter, Esc cancel)");
    frame.render_widget(List::new(items).block(block), area);
}

fn slot_label(slot: EquipSlot) -> &'static str {
    match slot {
        EquipSlot::Weapon => "Weapon",
        EquipSlot::Armor => "Armor",
        EquipSlot::Ring(RingSlot::First) => "Ring 1",
        EquipSlot::Ring(RingSlot::Second) => "Ring 2",
    }
}

fn draw_party_gear(frame: &mut Frame, area: Rect, party: &Party, mode: &InventoryMode) {
    let highlight: Option<(usize, EquipSlot)> = match *mode {
        InventoryMode::PartyGear {
            member_cursor,
            slot_cursor,
        } => Some((member_cursor, EQUIP_SLOTS[slot_cursor])),
        InventoryMode::PartyGearAction {
            member_idx, slot, ..
        } => Some((member_idx, slot)),
        InventoryMode::PartyGearTarget {
            to_cursor, slot, ..
        } => Some((to_cursor, slot)),
        _ => None,
    };

    let mut lines = Vec::new();
    for (mi, m) in party.members.iter().enumerate() {
        let hp_color = crate::ui::hp_color(m.hp_ratio());
        let hp_bar = crate::ui::hp_bar(m.stats.hp, m.stats.max_hp, 10);
        lines.push(Line::from(Span::styled(
            m.name.clone(),
            Style::default().add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(vec![
            Span::styled(format!("  {hp_bar} "), Style::default().fg(hp_color)),
            Span::raw(format!("{:>3}/{:<3} HP", m.stats.hp, m.stats.max_hp)),
        ]));
        lines.push(Line::from(format!(
            "  MP {:>3}/{:<3}",
            m.stats.mp, m.stats.max_mp
        )));

        for &slot in EQUIP_SLOTS.iter() {
            let selected = highlight == Some((mi, slot));
            let (name, color) = match slot {
                EquipSlot::Weapon => m
                    .equipped_weapon
                    .as_ref()
                    .map(|w| (w.display_name(), crate::ui::rarity_color(w.rarity)))
                    .unwrap_or_else(|| ("(empty)".to_string(), Color::DarkGray)),
                EquipSlot::Armor => m
                    .equipped_armor
                    .as_ref()
                    .map(|a| (a.name.clone(), crate::ui::rarity_color(a.rarity)))
                    .unwrap_or_else(|| ("(empty)".to_string(), Color::DarkGray)),
                EquipSlot::Ring(RingSlot::First) => m.equipped_rings[0]
                    .as_ref()
                    .map(|r| (r.name.clone(), crate::ui::rarity_color(r.rarity)))
                    .unwrap_or_else(|| ("(empty)".to_string(), Color::DarkGray)),
                EquipSlot::Ring(RingSlot::Second) => m.equipped_rings[1]
                    .as_ref()
                    .map(|r| (r.name.clone(), crate::ui::rarity_color(r.rarity)))
                    .unwrap_or_else(|| ("(empty)".to_string(), Color::DarkGray)),
            };
            let name_style = if selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(color)
            };
            let marker = if selected { "> " } else { "  " };
            lines.push(Line::from(vec![
                Span::raw(marker),
                Span::styled(
                    format!("{:<8}", slot_label(slot)),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(name, name_style),
            ]));
        }
        lines.push(Line::from(format!(
            "  ATK {} DEF {}",
            m.total_attack(),
            m.total_defense()
        )));
        lines.push(Line::from(""));
    }
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Party (p to move/unequip gear)");
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn draw_footer(frame: &mut Frame, area: Rect, inv_ui: &InventoryUiState) {
    let text = inv_ui.message.clone().unwrap_or_else(|| {
        "Rarer gear is stronger: Common < Uncommon < Rare < Epic < Legendary.".to_string()
    });
    let block = Block::default().borders(Borders::ALL).title("Status");
    frame.render_widget(Paragraph::new(text).block(block), area);
}
