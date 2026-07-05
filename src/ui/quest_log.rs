use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::game::quest::{quest_def, QuestId, QuestLog};
use crate::game::quest_ui::QuestLogUiState;

pub fn draw(frame: &mut Frame, ui: &QuestLogUiState, quest_log: &QuestLog) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(frame.size());

    let mid = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(outer[0]);

    draw_active(frame, mid[0], &quest_log.active, ui.cursor);
    draw_completed(frame, mid[1], &quest_log.completed);

    let block = Block::default().borders(Borders::ALL).title("Status");
    frame.render_widget(
        Paragraph::new("↑↓ select an active quest, Esc to close").block(block),
        outer[1],
    );
}

fn draw_active(frame: &mut Frame, area: Rect, ids: &[QuestId], cursor: usize) {
    let items: Vec<ListItem> = ids
        .iter()
        .enumerate()
        .map(|(i, &id)| {
            let quest = quest_def(id);
            let selected = i == cursor;
            let style = if selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().add_modifier(Modifier::BOLD)
            };
            let header = Line::from(Span::styled(quest.title, style));
            let detail_style = if selected {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let detail = Line::from(Span::styled(
                format!("  From: {}", crate::game::npc::npc_def(quest.giver).name),
                detail_style,
            ));
            ListItem::new(ratatui::text::Text::from(vec![header, detail]))
        })
        .collect();
    let block = Block::default().borders(Borders::ALL).title("Active Quests");
    if items.is_empty() {
        frame.render_widget(Paragraph::new("No active quests.").block(block), area);
    } else {
        frame.render_widget(List::new(items).block(block), area);
    }
}

fn draw_completed(frame: &mut Frame, area: Rect, ids: &[QuestId]) {
    let items: Vec<ListItem> = ids
        .iter()
        .map(|&id| {
            let quest = quest_def(id);
            ListItem::new(Line::from(Span::styled(
                quest.title,
                Style::default().fg(Color::Green),
            )))
        })
        .collect();
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Completed Quests");
    if items.is_empty() {
        frame.render_widget(Paragraph::new("None yet.").block(block), area);
    } else {
        frame.render_widget(List::new(items).block(block), area);
    }
}
