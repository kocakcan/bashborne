use macroquad::prelude::*;

use crate::app::World;
use crate::game::state::GameState;

pub mod assets;
mod combat;
mod common;
mod event;
mod explore;
mod inventory;
mod main_menu;

pub use assets::Assets;
use assets::{CANVAS_HEIGHT, CANVAS_WIDTH};
use common::{canvas_transform, flush_text, push_text, TextCmd};

mod hud;

pub fn draw(assets: &Assets, world: &World) {
    let mut camera = Camera2D::from_display_rect(Rect::new(0.0, 0.0, CANVAS_WIDTH, CANVAS_HEIGHT));
    camera.render_target = Some(assets.canvas.clone());
    set_camera(&camera);
    clear_background(Color::new(0.08, 0.08, 0.1, 1.0));

    let font = &assets.font;
    let mut text = Vec::new();
    match &world.state {
        GameState::MainMenu(menu) => main_menu::draw(font, menu, &mut text),
        GameState::Explore(explore) => explore::draw(assets, explore, &world.party, &mut text),
        GameState::Combat(combat) => {
            combat::draw(font, combat, &world.party, &world.inventory, &mut text)
        }
        GameState::Event(ev) => event::draw(ev, &mut text),
        GameState::Inventory(inv_ui) => {
            inventory::draw(inv_ui, &world.party, &world.inventory, &mut text)
        }
        GameState::Shop(_) => draw_placeholder(font, "Shop", &mut text),
        GameState::QuestLog(_) => draw_placeholder(font, "Quest Log", &mut text),
        GameState::LevelUp(_) => draw_placeholder(font, "Level Up", &mut text),
        GameState::Blacksmith(_) => draw_placeholder(font, "Blacksmith", &mut text),
        GameState::GameOver { victory } => draw_game_over(font, *victory, &mut text),
    }

    hud::draw_status_bar(world, &mut text);
    if world.show_help {
        hud::draw_help_overlay(&mut text);
    }

    set_default_camera();
    clear_background(BLACK);
    blit_canvas_to_window(assets);
    flush_text(font, &assets.text_material, &text);
}

fn draw_placeholder(font: &Font, name: &str, cmds: &mut Vec<TextCmd>) {
    let text = format!("{name} — not yet implemented in the new renderer");
    let d = measure_text(&text, Some(font), 12, 1.0);
    push_text(
        cmds,
        text,
        (CANVAS_WIDTH - d.width) / 2.0,
        CANVAS_HEIGHT / 2.0,
        12.0,
        LIGHTGRAY,
    );
    let hint = "Esc to go back";
    let hd = measure_text(hint, Some(font), 9, 1.0);
    push_text(
        cmds,
        hint,
        (CANVAS_WIDTH - hd.width) / 2.0,
        CANVAS_HEIGHT / 2.0 + 18.0,
        9.0,
        GRAY,
    );
}

fn draw_game_over(font: &Font, victory: bool, cmds: &mut Vec<TextCmd>) {
    let (msg, color) = if victory {
        ("Victory!", GREEN)
    } else {
        ("Your party has fallen...", RED)
    };
    let d = measure_text(msg, Some(font), 20, 1.0);
    push_text(
        cmds,
        msg,
        (CANVAS_WIDTH - d.width) / 2.0,
        CANVAS_HEIGHT / 2.0 - 10.0,
        20.0,
        color,
    );
    let footer = if victory {
        "Press N for New Game+  -  Enter to quit."
    } else {
        "Press Enter to quit."
    };
    let fd = measure_text(footer, Some(font), 10, 1.0);
    push_text(
        cmds,
        footer,
        (CANVAS_WIDTH - fd.width) / 2.0,
        CANVAS_HEIGHT / 2.0 + 16.0,
        10.0,
        LIGHTGRAY,
    );
}

/// Blits the fixed logical canvas to the real window at the largest clean
/// integer scale, letterboxed — keeps the pixel art crisp at any window size.
fn blit_canvas_to_window(assets: &Assets) {
    let (scale, x, y) = canvas_transform();
    let dest_w = CANVAS_WIDTH * scale;
    let dest_h = CANVAS_HEIGHT * scale;

    draw_texture_ex(
        &assets.canvas.texture,
        x,
        y,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(dest_w, dest_h)),
            flip_y: true,
            ..Default::default()
        },
    );
}
