use macroquad::prelude::*;

use crate::app::World;
use crate::game::chapter::chapter_def;
use crate::game::state::GameState;
use crate::render::assets::CANVAS_WIDTH;
use crate::render::common::{draw_screen_rect, draw_screen_rect_lines, push_text, TextCmd};

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
    if world.difficulty > 0 {
        text.push_str("   [Accursed]");
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
const HELP_LEFT: [&str; 15] = [
    "Exploration",
    "arrows/WASD  move",
    "i            inventory",
    "l            quest log",
    "b            bestiary",
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

const HELP_BOX_X: f32 = 40.0;
const HELP_BOX_Y: f32 = 20.0;
const HELP_BOX_W: f32 = CANVAS_WIDTH - 80.0;
const HELP_BOX_H: f32 = 230.0;

pub fn draw_help_overlay(cmds: &mut Vec<TextCmd>) {
    draw_rectangle(
        HELP_BOX_X,
        HELP_BOX_Y,
        HELP_BOX_W,
        HELP_BOX_H,
        Color::new(0.05, 0.05, 0.08, 0.95),
    );
    draw_rectangle_lines(HELP_BOX_X, HELP_BOX_Y, HELP_BOX_W, HELP_BOX_H, 1.0, WHITE);

    for (i, line) in HELP_LEFT.iter().enumerate() {
        push_text(
            cmds,
            *line,
            HELP_BOX_X + 8.0,
            HELP_BOX_Y + 14.0 + i as f32 * 13.0,
            HUD_FONT,
            WHITE,
        );
    }
    let right_x = HELP_BOX_X + 210.0;
    for (i, line) in HELP_RIGHT.iter().enumerate() {
        push_text(cmds, *line, right_x, HELP_BOX_Y + 14.0 + i as f32 * 10.0, 8.0, WHITE);
    }
    push_text(
        cmds,
        "? / Esc to close",
        HELP_BOX_X + 8.0,
        HELP_BOX_Y + HELP_BOX_H - 6.0,
        HUD_FONT,
        LIGHTGRAY,
    );
}

/// Redraws the help overlay's opaque panel directly in real screen space
/// (post-canvas-blit) so it can sit between the base text flush and the
/// overlay's own text flush — see `render::mod::draw`. Without this, the
/// panel only exists baked into the canvas beneath a single later,
/// undifferentiated text pass, so it can occlude canvas art but never the
/// underlying screen's text, which bleeds through the overlay's own text.
pub fn redraw_help_overlay_panel() {
    draw_screen_rect(
        HELP_BOX_X,
        HELP_BOX_Y,
        HELP_BOX_W,
        HELP_BOX_H,
        Color::new(0.05, 0.05, 0.08, 0.95),
    );
    draw_screen_rect_lines(HELP_BOX_X, HELP_BOX_Y, HELP_BOX_W, HELP_BOX_H, 1.0, WHITE);
}

/// Modal "quit without saving?" prompt — reachable from Explore and every
/// sub-screen that carries a `return_pos` (see `World::current_player_pos`),
/// not just Explore, so `q` works consistently no matter what the player has
/// open.
const QUIT_BOX_X: f32 = 90.0;
const QUIT_BOX_Y: f32 = 90.0;
const QUIT_BOX_W: f32 = CANVAS_WIDTH - 180.0;
const QUIT_BOX_H: f32 = 90.0;

pub fn draw_confirm_quit(cmds: &mut Vec<TextCmd>) {
    draw_rectangle(
        QUIT_BOX_X,
        QUIT_BOX_Y,
        QUIT_BOX_W,
        QUIT_BOX_H,
        Color::new(0.05, 0.05, 0.08, 0.97),
    );
    draw_rectangle_lines(QUIT_BOX_X, QUIT_BOX_Y, QUIT_BOX_W, QUIT_BOX_H, 1.0, YELLOW);
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

/// Screen-space repaint of the confirm-quit panel, see
/// `redraw_help_overlay_panel`.
pub fn redraw_confirm_quit_panel() {
    draw_screen_rect(
        QUIT_BOX_X,
        QUIT_BOX_Y,
        QUIT_BOX_W,
        QUIT_BOX_H,
        Color::new(0.05, 0.05, 0.08, 0.97),
    );
    draw_screen_rect_lines(QUIT_BOX_X, QUIT_BOX_Y, QUIT_BOX_W, QUIT_BOX_H, 1.0, YELLOW);
}
