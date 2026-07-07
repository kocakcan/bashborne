use macroquad::prelude::*;

use crate::game::state::MainMenuState;
use crate::render::assets::{CANVAS_HEIGHT, CANVAS_WIDTH};
use crate::render::common::{push_text, TextCmd};

pub fn draw(font: &Font, menu: &MainMenuState, cmds: &mut Vec<TextCmd>) {
    let title = "BASHBORNE";
    let title_size = 28.0;
    let dims = measure_text(title, Some(font), title_size as u16, 1.0);
    push_text(
        cmds,
        title,
        (CANVAS_WIDTH - dims.width) / 2.0,
        70.0,
        title_size,
        Color::new(0.85, 0.85, 0.85, 1.0),
    );
    let subtitle = "The ash remembers.";
    let sub_dims = measure_text(subtitle, Some(font), 12, 1.0);
    push_text(
        cmds,
        subtitle,
        (CANVAS_WIDTH - sub_dims.width) / 2.0,
        95.0,
        12.0,
        GRAY,
    );

    let mut y = 140.0;
    if menu.confirm_overwrite {
        for line in [
            "A previous journey lingers here.",
            "Begin anew and let it fade?",
            "",
            "Enter - begin anew      Esc - turn back",
        ] {
            let d = measure_text(line, Some(font), 12, 1.0);
            push_text(cmds, line, (CANVAS_WIDTH - d.width) / 2.0, y, 12.0, LIGHTGRAY);
            y += 16.0;
        }
        return;
    }

    for (i, entry) in menu.entries().iter().enumerate() {
        let selected = i == menu.cursor;
        let label = if selected {
            format!("- {} -", entry.label())
        } else {
            entry.label().to_string()
        };
        let color = if selected { YELLOW } else { GRAY };
        let d = measure_text(&label, Some(font), 14, 1.0);
        push_text(cmds, label, (CANVAS_WIDTH - d.width) / 2.0, y, 14.0, color);
        y += 22.0;
    }

    let hint = "up/down choose   Enter confirm   q quit";
    let hd = measure_text(hint, Some(font), 10, 1.0);
    push_text(
        cmds,
        hint,
        (CANVAS_WIDTH - hd.width) / 2.0,
        CANVAS_HEIGHT - 20.0,
        10.0,
        DARKGRAY,
    );
}
