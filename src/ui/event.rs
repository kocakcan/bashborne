use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::game::state::EventState;

pub fn draw(frame: &mut Frame, area: Rect, ev: &EventState) {
    let title_color = if ev.title.contains("Curse") {
        Color::Magenta
    } else if ev.title.contains("Blessing") {
        Color::Cyan
    } else {
        Color::Yellow
    };

    let mut lines = vec![
        Line::from(ratatui::text::Span::styled(
            ev.title.clone(),
            Style::default()
                .fg(title_color)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    for l in &ev.lines {
        lines.push(Line::from(l.as_str()));
    }
    lines.push(Line::from(""));
    lines.push(Line::from("Press Enter to continue."));

    let block = Block::default().borders(Borders::ALL).title("Encounter");
    let p = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false });
    frame.render_widget(p, area);
}
