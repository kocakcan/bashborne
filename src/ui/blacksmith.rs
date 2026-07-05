use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::game::blacksmith::{weapon_for, weapon_ref_label, weapon_refs, BlacksmithUiState};
use crate::game::item::Inventory;
use crate::game::party::Party;

pub fn draw(frame: &mut Frame, bs: &BlacksmithUiState, party: &Party, inventory: &Inventory) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(frame.size());

    draw_header(frame, outer[0], party, inventory);

    let mid = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(outer[1]);

    draw_weapon_list(frame, mid[0], bs, party, inventory);
    crate::ui::draw_party_gear(frame, mid[1], party);

    draw_footer(frame, outer[2], bs);
}

fn draw_header(frame: &mut Frame, area: Rect, party: &Party, inventory: &Inventory) {
    let line = Line::from(format!(
        "Gold: {}   Titanite Shards: {}",
        party.gold, inventory.upgrade_materials
    ));
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Andre of Astora — Weapon Upgrades (↑↓ select, Enter to upgrade, Esc leave)");
    frame.render_widget(Paragraph::new(line).block(block), area);
}

fn draw_weapon_list(
    frame: &mut Frame,
    area: Rect,
    bs: &BlacksmithUiState,
    party: &Party,
    inventory: &Inventory,
) {
    let refs = weapon_refs(inventory, party);
    let items: Vec<ListItem> = if refs.is_empty() {
        Vec::new()
    } else {
        refs.iter()
            .enumerate()
            .map(|(i, &r)| {
                let weapon = weapon_for(r, inventory, party).expect("weapon_refs stays in sync");
                let color = crate::ui::rarity_color(weapon.rarity);
                let marker = if i == bs.cursor { "> " } else { "  " };
                let base_style = if i == bs.cursor {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };
                let cost_label = match crate::game::blacksmith::upgrade_cost(
                    weapon.rarity,
                    weapon.upgrade_level,
                ) {
                    Some((gold, shards)) => {
                        let affordable =
                            party.gold >= gold && inventory.upgrade_materials >= shards;
                        // Selected rows get a DarkGray background (below), so an
                        // unaffordable label styled DarkGray-on-DarkGray used to be
                        // effectively invisible on the highlighted row. Red reads
                        // clearly against both the normal and highlighted background.
                        let style = if affordable {
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::Red)
                        };
                        Span::styled(format!("Upgrade: {gold}g + {shards} shards"), style)
                    }
                    None => Span::styled("MAX", Style::default().fg(Color::Yellow)),
                };
                let header = Line::from(vec![
                    Span::raw(marker),
                    Span::styled(
                        format!("{:<22}", weapon.display_name()),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!("[{}] ", weapon.rarity), Style::default().fg(color)),
                    Span::raw(format!(
                        "ATK +{} DEF +{}  {}  ",
                        weapon.attack_bonus,
                        weapon.defense_bonus,
                        weapon_ref_label(r, party)
                    )),
                    cost_label,
                ]);
                let mut lines = vec![header];
                if let Some(passive) = weapon.passive {
                    lines.push(Line::from(Span::styled(
                        format!("     {}", passive.description()),
                        Style::default().fg(Color::Yellow),
                    )));
                }
                ListItem::new(Text::from(lines)).style(base_style)
            })
            .collect()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Weapons (bag + equipped)");
    if items.is_empty() {
        frame.render_widget(
            Paragraph::new("No weapons to upgrade.").block(block),
            area,
        );
    } else {
        frame.render_widget(List::new(items).block(block), area);
    }
}

fn draw_footer(frame: &mut Frame, area: Rect, bs: &BlacksmithUiState) {
    let text = bs.message.clone().unwrap_or_else(|| {
        "Upgrading raises ATK (and DEF, for weapons that grant it) using gold and Titanite Shards."
            .to_string()
    });
    let block = Block::default().borders(Borders::ALL).title("Status");
    frame.render_widget(Paragraph::new(text).block(block), area);
}
