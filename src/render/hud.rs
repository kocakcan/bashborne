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
    let gold_x = CANVAS_WIDTH - d.width - 4.0;
    push_text(cmds, gold_text, gold_x, 9.0, HUD_FONT, GOLD);

    // A one-shot combat-log line ("... press 'u'") is the only other signal
    // that points spending is available, and it scrolls away — keep a
    // persistent nudge here so it isn't forgotten mid-explore.
    if world.party.members.iter().any(|m| m.unspent_points > 0) {
        let nudge = "[!] Unspent points";
        let nd = measure_text(nudge, Some(font), HUD_FONT as u16, 1.0);
        push_text(cmds, nudge, gold_x - nd.width - 12.0, 9.0, HUD_FONT, GOLD);
    }
}

/// Two columns of keybind reference (left: the screens with the fewest
/// entries, right: the rest) plus a map legend, since a single column can no
/// longer fit every screen's keybinds in the overlay's fixed 230px height.
const HELP_LEFT: [&str; 14] = [
    "Exploration",
    "arrows/WASD  move",
    "i            inventory",
    "l            quest log",
    "u            level up",
    "e            interact (shop/NPC)",
    "S            save (works anywhere)",
    "q            quit (works anywhere)",
    "",
    "Combat",
    "up/down      choose action/target",
    "Enter        confirm",
    "Esc          back",
    "PageUp/Dn    scroll the log",
];

const HELP_RIGHT: [&str; 21] = [
    "Inventory",
    "up/down choose item",
    "left/right change tab",
    "p party gear, Enter act",
    "",
    "Shop",
    "left/right Buy/Sell",
    "Tab category, Enter act",
    "x sell all Common gear",
    "",
    "Level-Up",
    "up/down member, l/r stat",
    "Enter spend, f spend all",
    "",
    "Blacksmith",
    "up/down weapon, Enter upgrade",
    "B buy Titanite Shard",
    "",
    "Map Legend",
    "Town shop+NPCs, BossLair boss",
    "TallGrass random encounters",
];

pub fn draw_help_overlay(cmds: &mut Vec<TextCmd>) {
    const BOX_X: f32 = 40.0;
    const BOX_Y: f32 = 20.0;
    const BOX_W: f32 = CANVAS_WIDTH - 80.0;
    const BOX_H: f32 = 230.0;
    draw_rectangle(BOX_X, BOX_Y, BOX_W, BOX_H, Color::new(0.05, 0.05, 0.08, 0.95));
    draw_rectangle_lines(BOX_X, BOX_Y, BOX_W, BOX_H, 1.0, WHITE);

    for (i, line) in HELP_LEFT.iter().enumerate() {
        push_text(cmds, *line, BOX_X + 8.0, BOX_Y + 14.0 + i as f32 * 13.0, HUD_FONT, WHITE);
    }
    let right_x = BOX_X + 210.0;
    for (i, line) in HELP_RIGHT.iter().enumerate() {
        push_text(cmds, *line, right_x, BOX_Y + 14.0 + i as f32 * 10.0, 8.0, WHITE);
    }
    push_text(cmds, "? / Esc to close", BOX_X + 8.0, BOX_Y + BOX_H - 6.0, HUD_FONT, LIGHTGRAY);
}

/// Modal "quit without saving?" prompt — reachable from Explore and every
/// sub-screen that carries a `return_pos` (see `World::current_player_pos`),
/// not just Explore, so `q` works consistently no matter what the player has
/// open.
pub fn draw_confirm_quit(cmds: &mut Vec<TextCmd>) {
    draw_rectangle(
        90.0,
        90.0,
        CANVAS_WIDTH - 180.0,
        90.0,
        Color::new(0.05, 0.05, 0.08, 0.97),
    );
    draw_rectangle_lines(90.0, 90.0, CANVAS_WIDTH - 180.0, 90.0, 1.0, YELLOW);
    push_text(cmds, "Quit without saving?", 100.0, 110.0, 12.0, YELLOW);
    push_text(
        cmds,
        "Progress since your last save",
        100.0,
        128.0,
        9.0,
        LIGHTGRAY,
    );
    push_text(cmds, "will be lost.", 100.0, 140.0, 9.0, LIGHTGRAY);
    push_text(cmds, "Enter/y - quit    Esc/n - stay", 100.0, 160.0, 9.0, WHITE);
}
