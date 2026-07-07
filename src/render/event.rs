use macroquad::prelude::*;

use crate::game::state::EventState;
use crate::render::assets::{CANVAS_HEIGHT, CANVAS_WIDTH};
use crate::render::common::{push_text, TextCmd};

pub fn draw(ev: &EventState, cmds: &mut Vec<TextCmd>) {
    let title_color = if ev.title.contains("Curse") {
        MAGENTA
    } else if ev.title.contains("Blessing") {
        SKYBLUE
    } else {
        YELLOW
    };

    draw_rectangle(
        20.0,
        20.0,
        CANVAS_WIDTH - 40.0,
        CANVAS_HEIGHT - 40.0,
        Color::new(0.05, 0.05, 0.08, 0.95),
    );
    draw_rectangle_lines(
        20.0,
        20.0,
        CANVAS_WIDTH - 40.0,
        CANVAS_HEIGHT - 40.0,
        1.0,
        WHITE,
    );

    push_text(cmds, ev.title.clone(), 32.0, 42.0, 14.0, title_color);
    let mut y = 62.0;
    for line in &ev.lines {
        push_text(cmds, line.clone(), 32.0, y, 10.0, WHITE);
        y += 14.0;
    }
    push_text(
        cmds,
        "Press Enter to continue.",
        32.0,
        CANVAS_HEIGHT - 32.0,
        10.0,
        LIGHTGRAY,
    );
}
