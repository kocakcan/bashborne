use macroquad::prelude::*;

use crate::game::state::MainMenuState;
use crate::render::assets::{CANVAS_HEIGHT, CANVAS_WIDTH};
use crate::render::common::{push_text, TextCmd};

/// One-line summary shown on an occupied slot's row: "New Game" for an empty
/// slot, otherwise chapter/average level/NG+ pulled straight out of the
/// loaded `SaveData` (no NG+ suffix on a first playthrough).
fn slot_summary(menu: &MainMenuState, slot_idx: usize) -> String {
    match &menu.slots[slot_idx] {
        None => "New Game".to_string(),
        Some(data) => {
            let avg_level = data.party.average_level();
            let mut summary = format!(
                "Chapter {} - Avg Lv {avg_level}",
                data.current_chapter.number()
            );
            if data.ng_plus > 0 {
                summary.push_str(&format!(" - NG+{}", data.ng_plus));
            }
            summary
        }
    }
}

fn slot_label(slot_idx: usize) -> &'static str {
    match slot_idx {
        0 => "Slot 1",
        1 => "Slot 2",
        2 => "Slot 3 (Dev)",
        _ => unreachable!("only 3 save slots exist"),
    }
}

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
    if let Some(slot) = menu.confirm_overwrite {
        for line in [
            format!("A previous journey lingers in Slot {slot}."),
            "Begin anew and let it fade?".to_string(),
            String::new(),
            "Enter - begin anew      Esc - turn back".to_string(),
        ] {
            let d = measure_text(&line, Some(font), 12, 1.0);
            push_text(cmds, line, (CANVAS_WIDTH - d.width) / 2.0, y, 12.0, LIGHTGRAY);
            y += 16.0;
        }
        return;
    }

    for slot_idx in 0..3 {
        let selected = menu.cursor == slot_idx;
        let label = format!("{}: {}", slot_label(slot_idx), slot_summary(menu, slot_idx));
        let label = if selected { format!("- {label} -") } else { label };
        let color = if selected { YELLOW } else { GRAY };
        let d = measure_text(&label, Some(font), 12, 1.0);
        push_text(cmds, label, (CANVAS_WIDTH - d.width) / 2.0, y, 12.0, color);
        y += 18.0;
    }

    y += 6.0;
    let quit_selected = menu.cursor == 3;
    let quit_label = if quit_selected { "- Quit -" } else { "Quit" };
    let quit_color = if quit_selected { YELLOW } else { GRAY };
    let d = measure_text(quit_label, Some(font), 14, 1.0);
    push_text(cmds, quit_label, (CANVAS_WIDTH - d.width) / 2.0, y, 14.0, quit_color);

    let hint = "up/down choose   Enter select/load   n force New Game   q quit";
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
