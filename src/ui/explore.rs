use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::game::map::{Position, Tile};
use crate::game::party::Party;
use crate::game::state::ExploreState;

pub fn draw(frame: &mut Frame, explore: &ExploreState, party: &Party) {
    let outer = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(frame.size());

    draw_map(frame, outer[0], explore);

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(outer[1]);

    draw_party_panel(frame, right[0], party);
    draw_log(frame, right[1], &explore.log);
}

fn draw_map(frame: &mut Frame, area: Rect, explore: &ExploreState) {
    let mut lines = Vec::with_capacity(explore.map.height as usize);
    for y in 0..explore.map.height {
        let mut spans = Vec::with_capacity(explore.map.width as usize);
        for x in 0..explore.map.width {
            let pos = Position { x, y };
            if pos == explore.player_pos {
                spans.push(Span::styled(
                    "@",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ));
                continue;
            }
            if let Some(id) = explore.map.npc_at(pos) {
                let ch = crate::game::npc::glyph_for(id).to_string();
                let color = crate::game::npc::color_for(id);
                spans.push(Span::styled(ch, Style::default().fg(color).add_modifier(Modifier::BOLD)));
                continue;
            }
            let (ch, style) = match explore.map.tile_at(pos) {
                Tile::Wall => ("#", Style::default().fg(Color::DarkGray)),
                Tile::Floor => (".", Style::default().fg(Color::Gray)),
                Tile::TallGrass => (",", Style::default().fg(Color::Green)),
                Tile::Town => ("T", Style::default().fg(Color::Cyan)),
                Tile::BossLair => (
                    "B",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
            };
            spans.push(Span::styled(ch, style));
        }
        lines.push(Line::from(spans));
    }
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Fields — arrows/WASD to move, i inventory, e shop/NPC, u level up, q to quit");
    let p = Paragraph::new(lines).block(block);
    frame.render_widget(p, area);
}

fn draw_party_panel(frame: &mut Frame, area: Rect, party: &Party) {
    let mut lines = Vec::new();
    for m in &party.members {
        let color = crate::ui::hp_color(m.hp_ratio());
        let bar = crate::ui::hp_bar(m.stats.hp, m.stats.max_hp, 10);
        let mut spans = vec![
            Span::styled(
                format!("{:<8}Lv{:<3}", m.name, m.level),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" {bar} "), Style::default().fg(color)),
            Span::raw(format!("{:>3}/{:<3}", m.stats.hp, m.stats.max_hp)),
            Span::raw(format!("  MP {:>3}/{:<3}", m.stats.mp, m.stats.max_mp)),
            Span::styled(
                format!(
                    "  XP {:>3}/{:<3}",
                    m.xp,
                    crate::game::character::xp_to_next_level(m.level)
                ),
                Style::default().fg(Color::Cyan),
            ),
        ];
        if m.unspent_points > 0 {
            spans.push(Span::styled(
                format!("  [+{} pts]", m.unspent_points),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ));
        }
        lines.push(Line::from(spans));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(format!("Gold: {}", party.gold)));
    if !party.effects.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Active effects:",
            Style::default().add_modifier(Modifier::BOLD),
        )));
        for e in &party.effects {
            let color = if e.delta >= 0 { Color::Cyan } else { Color::Magenta };
            lines.push(Line::from(Span::styled(
                format!(
                    "{} ({:+} {}, {} left)",
                    e.name, e.delta, e.target, e.encounters_remaining
                ),
                Style::default().fg(color),
            )));
        }
    }
    let block = Block::default().borders(Borders::ALL).title("Party");
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn draw_log(frame: &mut Frame, area: Rect, log: &[String]) {
    let visible_rows = area.height.saturating_sub(2) as usize;
    let start = log.len().saturating_sub(visible_rows.max(1));
    let lines: Vec<Line> = log[start..].iter().map(|s| Line::from(s.as_str())).collect();
    let block = Block::default().borders(Borders::ALL).title("Log");
    frame.render_widget(Paragraph::new(lines).block(block), area);
}
