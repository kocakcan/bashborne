use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::game::item::Inventory;
use crate::game::party::Party;
use crate::game::shop::{
    shop_armor_stock, shop_item_stock, shop_ring_stock, shop_weapon_stock, ShopMode, ShopTab,
    ShopUiState,
};

pub fn draw(frame: &mut Frame, shop: &ShopUiState, party: &Party, inventory: &Inventory) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(frame.size());

    draw_header(frame, outer[0], shop, party);

    let mid = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(outer[1]);

    match shop.mode {
        ShopMode::Buy => draw_buy_list(frame, mid[0], shop, party, inventory),
        ShopMode::Sell => draw_sell_list(frame, mid[0], shop, inventory),
    }
    crate::ui::draw_party_gear(frame, mid[1], party);

    draw_footer(frame, outer[2], shop);
}

fn draw_header(frame: &mut Frame, area: Rect, shop: &ShopUiState, party: &Party) {
    let mode_span = |label: &str, active: bool| {
        let style = if active {
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        Span::styled(format!(" {label} "), style)
    };
    let tab_span = |label: &str, active: bool| {
        let style = if active {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        Span::styled(format!(" {label} "), style)
    };
    let line = Line::from(vec![
        mode_span("Buy", shop.mode == ShopMode::Buy),
        Span::raw(" "),
        mode_span("Sell", shop.mode == ShopMode::Sell),
        Span::raw("   |  "),
        tab_span("Items", shop.tab == ShopTab::Items),
        tab_span("Weapons", shop.tab == ShopTab::Weapons),
        tab_span("Armor", shop.tab == ShopTab::Armor),
        tab_span("Rings", shop.tab == ShopTab::Rings),
        Span::raw(format!("   Gold: {}", party.gold)),
    ]);
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Town Shop (←→ Buy/Sell, Tab cycles tabs, Esc leave)");
    frame.render_widget(Paragraph::new(line).block(block), area);
}

fn draw_buy_list(
    frame: &mut Frame,
    area: Rect,
    shop: &ShopUiState,
    party: &Party,
    inventory: &Inventory,
) {
    let items: Vec<ListItem> = match shop.tab {
        ShopTab::Items => shop_item_stock()
            .into_iter()
            .enumerate()
            .map(|(i, (factory, price))| {
                let sample = factory();
                let affordable = party.gold >= price;
                let owned = inventory
                    .items
                    .iter()
                    .find(|(item, _)| item.name == sample.name)
                    .map(|(_, qty)| *qty)
                    .unwrap_or(0);
                let style = if i == shop.cursor {
                    crate::ui::cursor_style(true)
                } else if !affordable {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(Span::styled(
                    format!("{:<16} {price} gold   (have x{owned})", sample.name),
                    style,
                )))
            })
            .collect(),
        ShopTab::Weapons => shop_weapon_stock()
            .into_iter()
            .enumerate()
            .map(|(i, (factory, price))| {
                let sample = factory();
                let affordable = party.gold >= price;
                let color = crate::ui::rarity_color(sample.rarity);
                let base_style = if i == shop.cursor {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };
                let name_style = if !affordable {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default().fg(color).add_modifier(Modifier::BOLD)
                };
                let marker = if i == shop.cursor { "> " } else { "  " };
                ListItem::new(Line::from(vec![
                    Span::raw(marker),
                    Span::styled(format!("{:<20}", sample.name), name_style),
                    Span::styled(format!("[{}] ", sample.rarity), Style::default().fg(color)),
                    Span::raw(format!("ATK +{}  {price} gold", sample.attack_bonus)),
                ]))
                .style(base_style)
            })
            .collect(),
        ShopTab::Armor => shop_armor_stock()
            .into_iter()
            .enumerate()
            .map(|(i, (factory, price))| {
                let sample = factory();
                let affordable = party.gold >= price;
                let color = crate::ui::rarity_color(sample.rarity);
                let base_style = if i == shop.cursor {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };
                let name_style = if !affordable {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default().fg(color).add_modifier(Modifier::BOLD)
                };
                let marker = if i == shop.cursor { "> " } else { "  " };
                ListItem::new(Line::from(vec![
                    Span::raw(marker),
                    Span::styled(format!("{:<20}", sample.name), name_style),
                    Span::styled(format!("[{}] ", sample.rarity), Style::default().fg(color)),
                    Span::raw(format!("DEF +{}  {price} gold", sample.defense_bonus)),
                ]))
                .style(base_style)
            })
            .collect(),
        ShopTab::Rings => shop_ring_stock()
            .into_iter()
            .enumerate()
            .map(|(i, (factory, price))| {
                let sample = factory();
                let affordable = party.gold >= price;
                let color = crate::ui::rarity_color(sample.rarity);
                let base_style = if i == shop.cursor {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };
                let name_style = if !affordable {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default().fg(color).add_modifier(Modifier::BOLD)
                };
                let marker = if i == shop.cursor { "> " } else { "  " };
                let mut bonus = String::new();
                if sample.attack_bonus > 0 {
                    bonus.push_str(&format!("ATK +{} ", sample.attack_bonus));
                }
                if sample.defense_bonus > 0 {
                    bonus.push_str(&format!("DEF +{} ", sample.defense_bonus));
                }
                ListItem::new(Line::from(vec![
                    Span::raw(marker),
                    Span::styled(format!("{:<20}", sample.name), name_style),
                    Span::styled(format!("[{}] ", sample.rarity), Style::default().fg(color)),
                    Span::raw(format!("{bonus} {price} gold")),
                ]))
                .style(base_style)
            })
            .collect(),
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title("For sale (↑↓ select, Enter to buy)");
    frame.render_widget(List::new(items).block(block), area);
}

fn draw_sell_list(frame: &mut Frame, area: Rect, shop: &ShopUiState, inventory: &Inventory) {
    let items: Vec<ListItem> = match shop.tab {
        ShopTab::Items => inventory
            .items
            .iter()
            .enumerate()
            .map(|(i, (item, qty))| {
                let style = crate::ui::cursor_style(i == shop.cursor);
                ListItem::new(Line::from(Span::styled(
                    format!("{:<16} x{qty}   sells for {} gold", item.name, item.value / 2),
                    style,
                )))
            })
            .collect(),
        ShopTab::Weapons => inventory
            .weapons
            .iter()
            .enumerate()
            .map(|(i, w)| {
                let color = crate::ui::rarity_color(w.rarity);
                let marker = if i == shop.cursor { "> " } else { "  " };
                let base_style = if i == shop.cursor {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(vec![
                    Span::raw(marker),
                    Span::styled(
                        format!("{:<20}", w.display_name()),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!("[{}] ", w.rarity), Style::default().fg(color)),
                    Span::raw(format!("sells for {} gold", w.rarity.base_value() / 2)),
                ]))
                .style(base_style)
            })
            .collect(),
        ShopTab::Armor => inventory
            .armors
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let color = crate::ui::rarity_color(a.rarity);
                let marker = if i == shop.cursor { "> " } else { "  " };
                let base_style = if i == shop.cursor {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(vec![
                    Span::raw(marker),
                    Span::styled(
                        format!("{:<20}", a.name),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!("[{}] ", a.rarity), Style::default().fg(color)),
                    Span::raw(format!("sells for {} gold", a.rarity.base_value() / 2)),
                ]))
                .style(base_style)
            })
            .collect(),
        ShopTab::Rings => inventory
            .rings
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let color = crate::ui::rarity_color(r.rarity);
                let marker = if i == shop.cursor { "> " } else { "  " };
                let base_style = if i == shop.cursor {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(vec![
                    Span::raw(marker),
                    Span::styled(
                        format!("{:<20}", r.name),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!("[{}] ", r.rarity), Style::default().fg(color)),
                    Span::raw(format!("sells for {} gold", r.rarity.base_value() / 2)),
                ]))
                .style(base_style)
            })
            .collect(),
    };
    let title = match shop.tab {
        ShopTab::Items => "Your items (↑↓ select, Enter to sell)",
        ShopTab::Weapons => "Your spare weapons (↑↓ select, Enter to sell)",
        ShopTab::Armor => "Your spare armor (↑↓ select, Enter to sell)",
        ShopTab::Rings => "Your spare rings (↑↓ select, Enter to sell)",
    };
    let block = Block::default().borders(Borders::ALL).title(title);
    if items.is_empty() {
        let msg = match shop.tab {
            ShopTab::Items => "Nothing to sell.",
            ShopTab::Weapons => {
                "No spare weapons to sell. Unequip one in the inventory screen first."
            }
            ShopTab::Armor => {
                "No spare armor to sell. Unequip some in the inventory screen first."
            }
            ShopTab::Rings => {
                "No spare rings to sell. Unequip some in the inventory screen first."
            }
        };
        frame.render_widget(Paragraph::new(msg).block(block), area);
    } else {
        frame.render_widget(List::new(items).block(block), area);
    }
}

fn draw_footer(frame: &mut Frame, area: Rect, shop: &ShopUiState) {
    let text = shop
        .message
        .clone()
        .unwrap_or_else(|| "Epic and Legendary gear can't be bought — you'll have to earn it.".to_string());
    let block = Block::default().borders(Borders::ALL).title("Status");
    frame.render_widget(Paragraph::new(text).block(block), area);
}
