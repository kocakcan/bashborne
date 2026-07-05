use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::game::character::{xp_to_next_level, ALLOC_STATS};
use crate::game::levelup::LevelUpUiState;
use crate::game::party::Party;

pub fn draw(frame: &mut Frame, ui: &LevelUpUiState, party: &Party) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(frame.size());

    draw_header(frame, outer[0], ui, party);
    draw_stat_list(frame, outer[1], ui, party);
    draw_footer(frame, outer[2], ui);
}

fn draw_header(frame: &mut Frame, area: Rect, ui: &LevelUpUiState, party: &Party) {
    let text = party
        .members
        .get(ui.member_cursor)
        .map(|m| {
            format!(
                "{} — Level {}   XP {}/{}   Unspent points: {}",
                m.name,
                m.level,
                m.xp,
                xp_to_next_level(m.level),
                m.unspent_points
            )
        })
        .unwrap_or_default();
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Level Up (↑↓ member, ←→ stat, Enter to spend, Esc leave)");
    frame.render_widget(Paragraph::new(text).block(block), area);
}

fn draw_stat_list(frame: &mut Frame, area: Rect, ui: &LevelUpUiState, party: &Party) {
    let Some(member) = party.members.get(ui.member_cursor) else {
        frame.render_widget(
            Paragraph::new("No party member selected.")
                .block(Block::default().borders(Borders::ALL)),
            area,
        );
        return;
    };
    let items: Vec<ListItem> = ALLOC_STATS
        .iter()
        .enumerate()
        .map(|(i, stat)| {
            let current = match stat {
                crate::game::character::AllocStat::MaxHp => member.stats.max_hp,
                crate::game::character::AllocStat::MaxMp => member.stats.max_mp,
                crate::game::character::AllocStat::Attack => member.stats.attack,
                crate::game::character::AllocStat::Defense => member.stats.defense,
                crate::game::character::AllocStat::Speed => member.stats.speed,
                crate::game::character::AllocStat::Luck => member.stats.luck,
            };
            let increment = match stat {
                crate::game::character::AllocStat::MaxHp => "+5/pt",
                crate::game::character::AllocStat::MaxMp => "+3/pt",
                crate::game::character::AllocStat::Attack => "+2/pt",
                crate::game::character::AllocStat::Defense => "+2/pt",
                crate::game::character::AllocStat::Speed => "+1/pt",
                crate::game::character::AllocStat::Luck => "+1/pt",
            };
            let style = crate::ui::cursor_style(i == ui.stat_cursor);
            ListItem::new(Line::from(Span::styled(
                format!("{:<10} {:>4}   ({increment})", stat.to_string(), current),
                style,
            )))
        })
        .collect();
    let block = Block::default().borders(Borders::ALL).title("Stats");
    frame.render_widget(List::new(items).block(block), area);
}

fn draw_footer(frame: &mut Frame, area: Rect, ui: &LevelUpUiState) {
    let text = ui.message.clone().unwrap_or_else(|| {
        "↑↓ pick member, ←→ pick stat, Enter to spend a point, Esc to leave".to_string()
    });
    let block = Block::default().borders(Borders::ALL).title("Status");
    frame.render_widget(Paragraph::new(text).block(block), area);
}
