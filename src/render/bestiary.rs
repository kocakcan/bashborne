use macroquad::prelude::*;

use std::collections::HashSet;

use crate::game::bestiary_ui::{bestiary_entries, BestiaryUiState};
use crate::game::combat::loot_notes;
use crate::render::assets::{CANVAS_HEIGHT, CANVAS_WIDTH};
use crate::render::common::{push_text, scroll_window, TextCmd};
use crate::render::inventory::wrap_lines;

const CONTENT_Y0: f32 = 12.0;
const FOOTER_H: f32 = 26.0;
const LEFT_W: f32 = CANVAS_WIDTH * 0.5;
const RIGHT_X: f32 = LEFT_W;
const RIGHT_W: f32 = CANVAS_WIDTH - LEFT_W;

/// Bosses don't go through `species_color`'s name-hash palette — a fixed
/// dark red marks them apart from the rabble.
const BOSS_COLOR: Color = Color::new(0.75, 0.2, 0.2, 1.0);

pub fn draw(ui: &BestiaryUiState, seen: &HashSet<String>, cmds: &mut Vec<TextCmd>) {
    let content_y1 = CANVAS_HEIGHT - FOOTER_H;
    let entries = bestiary_entries();

    draw_list(ui.cursor, &entries, seen, content_y1, cmds);
    draw_detail(ui.cursor, &entries, seen, content_y1, cmds);
    draw_footer(content_y1, cmds);
}

fn draw_list(
    cursor: usize,
    entries: &[crate::game::bestiary_ui::BestiaryEntry],
    seen: &HashSet<String>,
    y1: f32,
    cmds: &mut Vec<TextCmd>,
) {
    draw_rectangle_lines(0.0, CONTENT_Y0, LEFT_W, y1 - CONTENT_Y0, 1.0, WHITE);
    let seen_count = entries.iter().filter(|e| seen.contains(e.name)).count();
    push_text(
        cmds,
        format!("Bestiary ({seen_count}/{})", entries.len()),
        4.0,
        CONTENT_Y0 + 9.0,
        8.0,
        WHITE,
    );

    let visible = 14usize;
    let range = scroll_window(entries.len(), cursor, visible);
    for (row, i) in range.enumerate() {
        let entry = &entries[i];
        let selected = i == cursor;
        let ty = CONTENT_Y0 + 24.0 + row as f32 * 12.0;
        if selected {
            draw_rectangle(0.0, ty - 8.0, LEFT_W, 11.0, Color::new(1.0, 1.0, 1.0, 0.12));
        }
        let marker = if selected { "> " } else { "  " };
        if seen.contains(entry.name) {
            let color = if entry.is_boss {
                BOSS_COLOR
            } else {
                super::combat::species_color(entry.name, false)
            };
            let tag = if entry.is_boss { " [Boss]" } else { "" };
            push_text(cmds, format!("{marker}{}{tag}", entry.name), 4.0, ty, 8.0, color);
        } else {
            push_text(cmds, format!("{marker}???"), 4.0, ty, 8.0, DARKGRAY);
        }
    }
}

fn draw_detail(
    cursor: usize,
    entries: &[crate::game::bestiary_ui::BestiaryEntry],
    seen: &HashSet<String>,
    y1: f32,
    cmds: &mut Vec<TextCmd>,
) {
    draw_rectangle_lines(RIGHT_X, CONTENT_Y0, RIGHT_W, y1 - CONTENT_Y0, 1.0, WHITE);
    let pad = RIGHT_X + 4.0;
    let Some(entry) = entries.get(cursor) else {
        return;
    };
    if !seen.contains(entry.name) {
        push_text(cmds, "???", pad, CONTENT_Y0 + 12.0, 9.0, DARKGRAY);
        push_text(
            cmds,
            "You have not faced this creature yet.",
            pad,
            CONTENT_Y0 + 26.0,
            7.0,
            GRAY,
        );
        return;
    }

    let color = if entry.is_boss {
        BOSS_COLOR
    } else {
        super::combat::species_color(entry.name, false)
    };
    push_text(cmds, entry.name, pad, CONTENT_Y0 + 12.0, 9.0, color);

    // Baseline (level 1 / pre-scaling) stats straight from the factory —
    // chapter level scaling and NG+ multiply these in the field.
    let sample = (entry.factory)(entry.name);
    push_text(
        cmds,
        format!(
            "HP {}  ATK {}  DEF {}  SPD {}",
            sample.stats.max_hp, sample.stats.attack, sample.stats.defense, sample.stats.speed
        ),
        pad,
        CONTENT_Y0 + 26.0,
        7.0,
        LIGHTGRAY,
    );
    push_text(cmds, "(baseline; scales with chapter and NG+)", pad, CONTENT_Y0 + 35.0, 6.0, DARKGRAY);

    let mut ty = CONTENT_Y0 + 50.0;
    for line in wrap_lines(entry.signature, 40, 3) {
        push_text(cmds, line, pad, ty, 7.0, WHITE);
        ty += 8.0;
    }

    ty += 8.0;
    push_text(cmds, "Notable spoils:", pad, ty, 7.0, GOLD);
    ty += 9.0;
    if entry.is_boss {
        push_text(cmds, "Its Legendary arm, yours by right", pad, ty, 7.0, GRAY);
        push_text(cmds, "of conquest - guaranteed.", pad, ty + 8.0, 7.0, GRAY);
    } else {
        for note in loot_notes(entry.name) {
            push_text(cmds, note, pad, ty, 7.0, GRAY);
            ty += 8.0;
        }
    }
}

fn draw_footer(y0: f32, cmds: &mut Vec<TextCmd>) {
    draw_rectangle_lines(0.0, y0, CANVAS_WIDTH, CANVAS_HEIGHT - y0, 1.0, WHITE);
    push_text(cmds, "up/down browse, Esc to close", 4.0, y0 + 12.0, 7.0, WHITE);
}
