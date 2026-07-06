use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::game::state::MainMenuState;

/// Block-letter pieces for the title, in "BASHBORNE" order. Every piece is
/// exactly `LETTER_WIDTH` chars so the assembled rows all come out the same
/// width — the paragraph centers each line independently, so a ragged row
/// would visibly drift sideways (same invariant as the combat sprites).
const LETTER_WIDTH: usize = 7;
const TITLE_LETTERS: [[&str; 5]; 9] = [
    // B
    ["██████ ", "██   ██", "██████ ", "██   ██", "██████ "],
    // A
    [" █████ ", "██   ██", "███████", "██   ██", "██   ██"],
    // S
    ["███████", "██     ", "███████", "     ██", "███████"],
    // H
    ["██   ██", "██   ██", "███████", "██   ██", "██   ██"],
    // B
    ["██████ ", "██   ██", "██████ ", "██   ██", "██████ "],
    // O
    [" █████ ", "██   ██", "██   ██", "██   ██", " █████ "],
    // R
    ["██████ ", "██   ██", "██████ ", "██  ██ ", "██   ██"],
    // N
    ["██   ██", "███  ██", "██ █ ██", "██  ███", "██   ██"],
    // E
    ["███████", "██     ", "█████  ", "██     ", "███████"],
];

fn banner_rows() -> [String; 5] {
    std::array::from_fn(|row| {
        TITLE_LETTERS
            .iter()
            .map(|letter| letter[row])
            .collect::<Vec<_>>()
            .join(" ")
    })
}

pub fn draw(frame: &mut Frame, menu: &MainMenuState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(22), Constraint::Min(0)])
        .split(frame.size());

    let dim = Style::default().fg(Color::DarkGray);
    let mut lines: Vec<Line> = banner_rows()
        .into_iter()
        .map(|row| Line::from(Span::styled(row, Style::default().fg(Color::Gray))))
        .collect();
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "The ash remembers.",
        dim.add_modifier(Modifier::ITALIC),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(""));

    if menu.confirm_overwrite {
        lines.push(Line::from(Span::styled(
            "A previous journey lingers here.",
            Style::default().fg(Color::Gray),
        )));
        lines.push(Line::from(Span::styled(
            "Begin anew and let it fade?",
            Style::default().fg(Color::Gray),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Enter — begin anew      Esc — turn back",
            dim,
        )));
    } else {
        for (i, entry) in menu.entries().iter().enumerate() {
            let line = if i == menu.cursor {
                Line::from(Span::styled(
                    format!("— {} —", entry.label()),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(Span::styled(entry.label(), dim))
            };
            lines.push(line);
            lines.push(Line::from(""));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "↑/↓ choose   Enter confirm   q quit",
            dim,
        )));
    }

    frame.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center),
        chunks[1],
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_title_letter_piece_is_the_declared_width() {
        for letter in TITLE_LETTERS {
            for row in letter {
                assert_eq!(
                    row.chars().count(),
                    LETTER_WIDTH,
                    "ragged piece {row:?} would make the centered banner drift"
                );
            }
        }
    }

    #[test]
    fn every_banner_row_is_the_same_width() {
        let rows = banner_rows();
        let width = rows[0].chars().count();
        for row in &rows {
            assert_eq!(row.chars().count(), width, "ragged row: {row:?}");
        }
    }
}
