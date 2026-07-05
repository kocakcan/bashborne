use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::game::inventory_ui::{InventoryMode, InventoryTab, InventoryUiState};
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
    }

    draw_party_gear(frame, mid[1], party);
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
    ]);
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Inventory (Tab/←→ switch tabs, Esc close)");
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
                    let style = cursor_style(i == inv_ui.cursor);
                    ListItem::new(Line::from(Span::styled(
                        format!("{} x{qty}", item.name),
                        style,
                    )))
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
                            format!("{:<20}", w.name),
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
                    let detail = Line::from(Span::styled(
                        format!("     {} — {}", w.description, w.source),
                        Style::default().fg(Color::DarkGray),
                    ));
                    let bg = if i == inv_ui.cursor {
                        Style::default().bg(Color::DarkGray)
                    } else {
                        Style::default()
                    };
                    ListItem::new(Text::from(vec![header, detail])).style(bg)
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
        InventoryTab::Weapons => inventory.weapons.get(idx).map(|w| w.name.clone()),
    }
    .unwrap_or_else(|| "???".to_string());

    let items: Vec<ListItem> = party
        .members
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let style = cursor_style(i == member_cursor);
            let current_weapon = m
                .equipped_weapon
                .as_ref()
                .map(|w| w.name.as_str())
                .unwrap_or("unarmed");
            ListItem::new(Line::from(Span::styled(
                format!("{:<8} (currently: {current_weapon})", m.name),
                style,
            )))
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!("Give {target_name} to... (↑↓ Enter, Esc cancel)"));
    frame.render_widget(List::new(items).block(block), area);
}

fn draw_party_gear(frame: &mut Frame, area: Rect, party: &Party) {
    let mut lines = Vec::new();
    for m in &party.members {
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
        if let Some(w) = &m.equipped_weapon {
            let color = crate::ui::rarity_color(w.rarity);
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(w.name.clone(), Style::default().fg(color)),
                Span::styled(format!(" [{}]", w.rarity), Style::default().fg(color)),
            ]));
            let mut bonus_line = format!("  ATK +{}", w.attack_bonus);
            if w.defense_bonus > 0 {
                bonus_line.push_str(&format!(" DEF +{}", w.defense_bonus));
            }
            lines.push(Line::from(bonus_line));
        } else {
            lines.push(Line::from("  (unarmed)"));
        }
        lines.push(Line::from(""));
    }
    let block = Block::default().borders(Borders::ALL).title("Party");
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn draw_footer(frame: &mut Frame, area: Rect, inv_ui: &InventoryUiState) {
    let text = inv_ui.message.clone().unwrap_or_else(|| {
        "Rarer weapons hit harder: Common < Uncommon < Rare < Epic < Legendary.".to_string()
    });
    let block = Block::default().borders(Borders::ALL).title("Status");
    frame.render_widget(Paragraph::new(text).block(block), area);
}
