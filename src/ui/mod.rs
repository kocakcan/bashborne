use ratatui::style::Color;
use ratatui::Frame;

use crate::app::World;
use crate::game::item::Rarity;
use crate::game::state::GameState;

mod combat;
mod event;
mod explore;
mod inventory;
mod quest_log;
mod shop;

pub fn draw(frame: &mut Frame, world: &World) {
    match &world.state {
        GameState::Explore(explore) => explore::draw(frame, explore, &world.party),
        GameState::Combat(combat) => combat::draw(frame, combat, &world.party, &world.inventory),
        GameState::Event(ev) => event::draw(frame, ev),
        GameState::Inventory(inv) => inventory::draw(frame, inv, &world.party, &world.inventory),
        GameState::Shop(shop_ui) => shop::draw(frame, shop_ui, &world.party, &world.inventory),
        GameState::QuestLog(ui) => quest_log::draw(frame, ui, &world.quest_log),
        GameState::GameOver { victory } => draw_game_over(frame, *victory),
    }
}

/// Color for a weapon's rarity tier — shared so the inventory screen and the
/// combat victory panel display rarity consistently. Climbs from a plain
/// gray (Common) to a striking gold (Legendary).
pub(crate) fn rarity_color(rarity: Rarity) -> Color {
    match rarity {
        Rarity::Common => Color::Gray,
        Rarity::Uncommon => Color::Green,
        Rarity::Rare => Color::Cyan,
        Rarity::Epic => Color::Magenta,
        Rarity::Legendary => Color::Yellow,
    }
}

/// Renders a Pokémon-style block HP bar, e.g. "████████░░" for 80%.
pub(crate) fn hp_bar(current: i32, max: i32, width: usize) -> String {
    let ratio = if max > 0 {
        (current as f64 / max as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let filled = ((ratio * width as f64).round() as usize).min(width);
    let mut s = String::with_capacity(width);
    for _ in 0..filled {
        s.push('█');
    }
    for _ in filled..width {
        s.push('░');
    }
    s
}

/// Color for an HP bar/number based on remaining fraction — shared so party
/// and enemy displays stay visually consistent.
pub(crate) fn hp_color(ratio: f64) -> Color {
    if ratio > 0.5 {
        Color::Green
    } else if ratio > 0.2 {
        Color::Yellow
    } else {
        Color::Red
    }
}

fn draw_game_over(frame: &mut Frame, victory: bool) {
    use ratatui::layout::Alignment;
    use ratatui::style::Style;
    use ratatui::text::Line;
    use ratatui::widgets::{Block, Borders, Paragraph};

    let area = frame.size();
    let msg = if victory {
        "Victory!"
    } else {
        "Your party has fallen..."
    };
    let color = if victory { Color::Green } else { Color::Red };
    let text = vec![
        Line::from(msg).style(Style::default().fg(color)),
        Line::from(""),
        Line::from("Press Enter to quit."),
    ];
    let block = Block::default().borders(Borders::ALL).title("Game Over");
    let p = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center);
    frame.render_widget(p, area);
}
