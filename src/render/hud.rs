use macroquad::prelude::*;

use crate::app::World;
use crate::game::chapter::chapter_def;
use crate::game::state::GameState;
use crate::render::assets::CANVAS_WIDTH;
use crate::render::common::{push_text, TextCmd};

const HUD_FONT: f32 = 10.0;

/// Thin, always-visible chrome row showing the current chapter and (once
/// unlocked) the New Game+ cycle. Skipped on the main menu, where neither
/// value means anything yet.
pub fn draw_status_bar(world: &World, font: &Font, cmds: &mut Vec<TextCmd>) {
    if matches!(world.state, GameState::MainMenu(_)) {
        return;
    }
    draw_rectangle(0.0, 0.0, CANVAS_WIDTH, 12.0, Color::new(0.0, 0.0, 0.0, 0.85));
    let def = chapter_def(world.current_chapter);
    let mut text = format!("Chapter {}: {}", world.current_chapter.number(), def.name);
    if world.ng_plus > 0 {
        text.push_str(&format!("   NG+{}", world.ng_plus));
    }
    push_text(cmds, text, 4.0, 9.0, HUD_FONT, WHITE);

    let gold_text = format!("Gold: {}", world.party.gold);
    let d = measure_text(&gold_text, Some(font), HUD_FONT as u16, 1.0);
    push_text(cmds, gold_text, CANVAS_WIDTH - d.width - 4.0, 9.0, HUD_FONT, GOLD);
}

pub fn draw_help_overlay(cmds: &mut Vec<TextCmd>) {
    draw_rectangle(
        40.0,
        20.0,
        CANVAS_WIDTH - 80.0,
        230.0,
        Color::new(0.05, 0.05, 0.08, 0.95),
    );
    draw_rectangle_lines(40.0, 20.0, CANVAS_WIDTH - 80.0, 230.0, 1.0, WHITE);
    let lines = [
        "Exploration",
        "arrows/WASD  move",
        "i            inventory",
        "l            quest log",
        "u            level up",
        "e            interact (shop/NPC)",
        "S            save",
        "q            quit (confirm)",
        "",
        "Combat",
        "up/down      choose action/target",
        "Enter        confirm",
        "Esc          back",
        "",
        "? / Esc to close",
    ];
    for (i, line) in lines.iter().enumerate() {
        push_text(cmds, *line, 48.0, 34.0 + i as f32 * 13.0, HUD_FONT, WHITE);
    }
}
