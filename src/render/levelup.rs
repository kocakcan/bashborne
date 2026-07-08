use macroquad::prelude::*;

use crate::game::character::{xp_to_next_level, AllocPreview, ALLOC_STATS};
use crate::game::levelup::LevelUpUiState;
use crate::game::party::Party;
use crate::render::assets::{CANVAS_HEIGHT, CANVAS_WIDTH};
use crate::render::common::{push_text, TextCmd};

const HEADER_Y: f32 = 12.0;
const HEADER_H: f32 = 20.0;
const FOOTER_H: f32 = 26.0;

pub fn draw(ui: &LevelUpUiState, party: &Party, cmds: &mut Vec<TextCmd>) {
    let content_y0 = HEADER_Y + HEADER_H;
    let content_y1 = CANVAS_HEIGHT - FOOTER_H;

    draw_header(ui, party, cmds);
    draw_stat_list(ui, party, content_y0, content_y1, cmds);
    draw_footer(ui.message.as_deref(), content_y1, cmds);
}

fn draw_header(ui: &LevelUpUiState, party: &Party, cmds: &mut Vec<TextCmd>) {
    draw_rectangle_lines(0.0, HEADER_Y, CANVAS_WIDTH, HEADER_H, 1.0, WHITE);
    if let Some(m) = party.members.get(ui.member_cursor) {
        push_text(
            cmds,
            format!("{} - Level {}", m.name, m.level),
            4.0,
            HEADER_Y + 9.0,
            9.0,
            WHITE,
        );
        push_text(
            cmds,
            format!("XP {}/{}   Unspent points: {}", m.xp, xp_to_next_level(m.level), m.unspent_points),
            4.0,
            HEADER_Y + HEADER_H - 2.0,
            7.0,
            LIGHTGRAY,
        );
    }
}

fn draw_stat_list(ui: &LevelUpUiState, party: &Party, y0: f32, y1: f32, cmds: &mut Vec<TextCmd>) {
    draw_rectangle_lines(0.0, y0, CANVAS_WIDTH, y1 - y0, 1.0, WHITE);
    let Some(member) = party.members.get(ui.member_cursor) else {
        push_text(cmds, "No party member selected.", 4.0, y0 + 12.0, 8.0, GRAY);
        return;
    };

    for (i, &stat) in ALLOC_STATS.iter().enumerate() {
        let current = member.base_stat(stat);
        let preview = member.alloc_preview(stat);
        let increment = match preview {
            AllocPreview::Full(n) => format!("+{n}/pt"),
            AllocPreview::Diminished(n) => format!("+{n}/pt, soft cap"),
            AllocPreview::Capped => "MAX".to_string(),
        };
        let selected = i == ui.stat_cursor;
        let ty = y0 + 12.0 + i as f32 * 14.0;
        if selected {
            draw_rectangle(0.0, ty - 8.0, CANVAS_WIDTH, 13.0, Color::new(1.0, 1.0, 1.0, 0.12));
        }
        let color = if selected {
            YELLOW
        } else if matches!(preview, AllocPreview::Capped) {
            DARKGRAY
        } else {
            WHITE
        };
        let marker = if selected { "> " } else { "  " };
        push_text(
            cmds,
            format!("{marker}{:<10} {:>4}   ({increment})", stat.to_string(), current),
            4.0,
            ty,
            9.0,
            color,
        );
    }
}

fn draw_footer(message: Option<&str>, y0: f32, cmds: &mut Vec<TextCmd>) {
    draw_rectangle_lines(0.0, y0, CANVAS_WIDTH, CANVAS_HEIGHT - y0, 1.0, WHITE);
    let text = message.unwrap_or("up/down pick member, left/right pick stat, Enter to spend a point, Esc to leave");
    push_text(cmds, text, 4.0, y0 + 12.0, 7.0, WHITE);
}
