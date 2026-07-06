use ratatui::layout::{Constraint, Direction, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::World;
use crate::game::item::Rarity;
use crate::game::party::Party;
use crate::game::state::GameState;

mod blacksmith;
mod combat;
mod event;
mod explore;
mod inventory;
mod levelup;
mod main_menu;
mod quest_log;
mod shop;

/// Smallest terminal size every screen in this codebase assumes it has —
/// below this, panels don't just look cramped, they clip mid-line/mid-span
/// (no `Paragraph` reflows fixed-width ASCII sprites or map rows). Rather
/// than let that render as garbled text, `draw` bails out to a friendly
/// notice instead.
const MIN_COLS: u16 = 80;
const MIN_ROWS: u16 = 24;

pub fn draw(frame: &mut Frame, world: &World) {
    let size = frame.size();
    if size.width < MIN_COLS || size.height < MIN_ROWS {
        draw_too_small(frame, size.width, size.height);
        return;
    }

    match &world.state {
        GameState::MainMenu(menu) => main_menu::draw(frame, menu),
        GameState::Explore(explore) => explore::draw(frame, explore, &world.party),
        GameState::Combat(combat) => combat::draw(
            frame,
            combat,
            &world.party,
            &world.inventory,
            world.anim_frame(),
        ),
        GameState::Event(ev) => event::draw(frame, ev),
        GameState::Inventory(inv) => inventory::draw(frame, inv, &world.party, &world.inventory),
        GameState::Shop(shop_ui) => shop::draw(frame, shop_ui, &world.party, &world.inventory),
        GameState::QuestLog(ui) => quest_log::draw(frame, ui, &world.quest_log),
        GameState::LevelUp(ui) => levelup::draw(frame, ui, &world.party),
        GameState::Blacksmith(bs) => blacksmith::draw(frame, bs, &world.party, &world.inventory),
        GameState::GameOver { victory } => draw_game_over(frame, *victory),
    }

    if world.show_help {
        draw_help_overlay(frame);
    }
}

/// Standard ratatui "centered popup" recipe: carves a `percent_x` ×
/// `percent_y` rectangle out of the middle of `area`.
pub(crate) fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = ratatui::layout::Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    ratatui::layout::Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn draw_too_small(frame: &mut Frame, width: u16, height: u16) {
    use ratatui::layout::Alignment;

    let area = frame.size();
    let text = vec![
        Line::from(Span::styled(
            "Terminal too small",
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(format!(
            "Resize to at least {MIN_COLS}x{MIN_ROWS} (currently {width}x{height})."
        )),
    ];
    let block = Block::default().borders(Borders::ALL).title("Bashborne");
    let p = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center);
    frame.render_widget(p, area);
}

fn draw_help_overlay(frame: &mut Frame) {
    use ratatui::layout::Alignment;
    use ratatui::widgets::Wrap;

    let area = centered_rect(70, 80, frame.size());
    let dim = Style::default().fg(Color::DarkGray);
    let heading = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);

    let lines = vec![
        Line::from(Span::styled("Exploration", heading)),
        Line::from("arrows/WASD  move"),
        Line::from("i            inventory"),
        Line::from("l            quest log"),
        Line::from("u            level up"),
        Line::from("e            interact (shop/NPC)"),
        Line::from("S            save"),
        Line::from("PageUp/Down  scroll log"),
        Line::from("q            quit (confirm)"),
        Line::from(""),
        Line::from(Span::styled("Combat", heading)),
        Line::from("up/down      choose action/target"),
        Line::from("Enter        confirm"),
        Line::from("Esc          back"),
        Line::from("PageUp/Down  scroll log"),
        Line::from(""),
        Line::from(Span::styled("? / Esc to close", dim)),
    ];

    frame.render_widget(Clear, area);
    let block = Block::default().borders(Borders::ALL).title("Help");
    let p = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(p, area);
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

/// Highlight style for the currently-selected row in a cursor-driven list —
/// shared by the shop, blacksmith, and level-up screens.
pub(crate) fn cursor_style(selected: bool) -> Style {
    if selected {
        Style::default()
            .fg(Color::Black)
            .bg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    }
}

/// A compact "each party member's HP/MP and equipped gear" panel, shared by
/// the shop and blacksmith screens (both want to show the party's current
/// loadout alongside whatever they're buying/upgrading).
pub(crate) fn draw_party_gear(frame: &mut Frame, area: Rect, party: &Party) {
    let mut lines = Vec::new();
    for m in &party.members {
        let hp_color = hp_color(m.hp_ratio());
        let hp_bar = hp_bar(m.stats.hp, m.stats.max_hp, 10);
        lines.push(Line::from(Span::styled(
            format!("{} (Lv {})", m.name, m.level),
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
            let color = rarity_color(w.rarity);
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(w.display_name(), Style::default().fg(color)),
                Span::styled(format!(" [{}]", w.rarity), Style::default().fg(color)),
            ]));
        } else {
            lines.push(Line::from("  (unarmed)"));
        }
        if let Some(a) = &m.equipped_armor {
            let color = rarity_color(a.rarity);
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(a.name.clone(), Style::default().fg(color)),
                Span::styled(format!(" [{}]", a.rarity), Style::default().fg(color)),
            ]));
        }
        for ring in m.equipped_rings.iter().flatten() {
            let color = rarity_color(ring.rarity);
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(ring.name.clone(), Style::default().fg(color)),
                Span::styled(format!(" [{}]", ring.rarity), Style::default().fg(color)),
            ]));
        }
        lines.push(Line::from(""));
    }
    let block = Block::default().borders(Borders::ALL).title("Party");
    frame.render_widget(Paragraph::new(lines).block(block), area);
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
