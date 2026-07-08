use macroquad::prelude::*;

use crate::game::character::xp_to_next_level;
use crate::game::map::Position;
use crate::game::party::Party;
use crate::game::state::ExploreState;
use crate::render::assets::{npc_rect, player_rect, tile_rect, Assets, CANVAS_HEIGHT, CANVAS_WIDTH};
use crate::render::common::{hp_color, push_text, TextCmd};

const TILE: f32 = 16.0;
const MAP_TOP: f32 = 12.0;

pub fn draw(assets: &Assets, explore: &ExploreState, party: &Party, cmds: &mut Vec<TextCmd>) {
    let map = &explore.map;
    let map_px_w = map.width as f32 * TILE;
    let map_px_h = map.height as f32 * TILE;
    let offset_x = ((CANVAS_WIDTH - map_px_w) / 2.0).max(0.0);
    let offset_y = MAP_TOP;

    for y in 0..map.height {
        for x in 0..map.width {
            let rect = tile_rect(map.tile_at(Position { x, y }));
            draw_texture_ex(
                &assets.tiles,
                offset_x + x as f32 * TILE,
                offset_y + y as f32 * TILE,
                WHITE,
                DrawTextureParams {
                    source: Some(rect),
                    ..Default::default()
                },
            );
        }
    }

    for (pos, id) in &map.npcs {
        draw_texture_ex(
            &assets.characters,
            offset_x + pos.x as f32 * TILE,
            offset_y + pos.y as f32 * TILE,
            WHITE,
            DrawTextureParams {
                source: Some(npc_rect(*id)),
                ..Default::default()
            },
        );
    }

    draw_texture_ex(
        &assets.characters,
        offset_x + explore.player_pos.x as f32 * TILE,
        offset_y + explore.player_pos.y as f32 * TILE,
        WHITE,
        DrawTextureParams {
            source: Some(player_rect()),
            ..Default::default()
        },
    );

    draw_bottom_strip(explore, party, offset_y + map_px_h, cmds);

    if explore.confirm_quit {
        draw_confirm_quit(cmds);
    }
}

fn draw_bottom_strip(explore: &ExploreState, party: &Party, strip_y: f32, cmds: &mut Vec<TextCmd>) {
    draw_rectangle(
        0.0,
        strip_y,
        CANVAS_WIDTH,
        CANVAS_HEIGHT - strip_y,
        Color::new(0.05, 0.05, 0.08, 0.95),
    );
    draw_rectangle_lines(0.0, strip_y, CANVAS_WIDTH, CANVAS_HEIGHT - strip_y, 1.0, WHITE);

    let member_count = party.members.len().max(1);
    let member_w = CANVAS_WIDTH / member_count as f32;
    for (i, m) in party.members.iter().enumerate() {
        let x = i as f32 * member_w + 4.0;
        let y = strip_y + 4.0;
        push_text(cmds, format!("{} Lv{}", m.name, m.level), x, y + 8.0, 8.0, WHITE);
        let bar_w = member_w - 8.0;
        let ratio = m.hp_ratio().clamp(0.0, 1.0) as f32;
        draw_rectangle(x, y + 12.0, bar_w, 4.0, DARKGRAY);
        draw_rectangle(x, y + 12.0, bar_w * ratio, 4.0, hp_color(m.hp_ratio()));
        push_text(
            cmds,
            format!("{}/{}", m.stats.hp, m.stats.max_hp),
            x,
            y + 24.0,
            8.0,
            LIGHTGRAY,
        );
        let next = xp_to_next_level(m.level);
        let xp_ratio = (m.xp as f32 / next as f32).clamp(0.0, 1.0);
        draw_rectangle(x, y + 30.0, bar_w, 3.0, DARKGRAY);
        draw_rectangle(x, y + 30.0, bar_w * xp_ratio, 3.0, SKYBLUE);
    }

    push_text(cmds, format!("Gold: {}", party.gold), 4.0, strip_y + 40.0, 9.0, GOLD);

    let log_y = strip_y + 54.0;
    let visible = 3;
    let start = explore.log.len().saturating_sub(visible);
    for (i, line) in explore.log[start..].iter().enumerate() {
        push_text(cmds, line.clone(), 4.0, log_y + i as f32 * 12.0, 9.0, WHITE);
    }
}

fn draw_confirm_quit(cmds: &mut Vec<TextCmd>) {
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
