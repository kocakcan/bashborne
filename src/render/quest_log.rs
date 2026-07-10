use macroquad::prelude::*;

use crate::game::item::Inventory;
use crate::game::npc::npc_def;
use crate::game::quest::{quest_def, QuestId, QuestLog, QuestObjective};
use crate::game::quest_ui::QuestLogUiState;
use crate::render::assets::{CANVAS_HEIGHT, CANVAS_WIDTH};
use crate::render::common::{push_text, TextCmd};

const CONTENT_Y0: f32 = 12.0;
const FOOTER_H: f32 = 26.0;
const LEFT_W: f32 = CANVAS_WIDTH * 0.5;
const RIGHT_X: f32 = LEFT_W;
const RIGHT_W: f32 = CANVAS_WIDTH - LEFT_W;

pub fn draw(ui: &QuestLogUiState, quest_log: &QuestLog, inventory: &Inventory, cmds: &mut Vec<TextCmd>) {
    let content_y1 = CANVAS_HEIGHT - FOOTER_H;

    draw_active(&quest_log.active, ui.cursor, inventory, content_y1, cmds);
    draw_completed(&quest_log.completed, content_y1, cmds);
    draw_footer(content_y1, cmds);
}

fn draw_active(ids: &[QuestId], cursor: usize, inventory: &Inventory, y1: f32, cmds: &mut Vec<TextCmd>) {
    draw_rectangle_lines(0.0, CONTENT_Y0, LEFT_W, y1 - CONTENT_Y0, 1.0, WHITE);
    push_text(cmds, "Active Quests", 4.0, CONTENT_Y0 + 9.0, 8.0, WHITE);

    if ids.is_empty() {
        push_text(cmds, "No active quests.", 4.0, CONTENT_Y0 + 24.0, 8.0, GRAY);
        return;
    }
    for (i, &id) in ids.iter().enumerate() {
        let quest = quest_def(id);
        let selected = i == cursor;
        let ty = CONTENT_Y0 + 24.0 + i as f32 * 20.0;
        if selected {
            draw_rectangle(0.0, ty - 8.0, LEFT_W, 18.0, Color::new(1.0, 1.0, 1.0, 0.12));
        }
        let color = if selected { YELLOW } else { WHITE };
        let marker = if selected { "> " } else { "  " };
        push_text(cmds, format!("{marker}{}", quest.title), 4.0, ty, 8.0, color);
        let detail_color = if selected { LIGHTGRAY } else { DARKGRAY };
        let detail = match quest.objective {
            QuestObjective::DeliverItem { item_name, qty } => {
                let have = inventory
                    .items
                    .iter()
                    .find(|(item, _)| item.name == item_name)
                    .map(|(_, have)| *have)
                    .unwrap_or(0);
                format!(
                    "Deliver {qty}x {item_name} (have {have}) - From: {}",
                    npc_def(quest.giver).name
                )
            }
            QuestObjective::DefeatBoss(_) => format!("From: {}", npc_def(quest.giver).name),
        };
        push_text(cmds, detail, 12.0, ty + 10.0, 7.0, detail_color);
    }
}

fn draw_completed(ids: &[QuestId], y1: f32, cmds: &mut Vec<TextCmd>) {
    draw_rectangle_lines(RIGHT_X, CONTENT_Y0, RIGHT_W, y1 - CONTENT_Y0, 1.0, WHITE);
    push_text(cmds, "Completed Quests", RIGHT_X + 4.0, CONTENT_Y0 + 9.0, 8.0, WHITE);

    if ids.is_empty() {
        push_text(cmds, "None yet.", RIGHT_X + 4.0, CONTENT_Y0 + 24.0, 8.0, GRAY);
        return;
    }
    for (i, &id) in ids.iter().enumerate() {
        let quest = quest_def(id);
        let ty = CONTENT_Y0 + 24.0 + i as f32 * 12.0;
        push_text(cmds, quest.title, RIGHT_X + 4.0, ty, 8.0, GREEN);
    }
}

fn draw_footer(y0: f32, cmds: &mut Vec<TextCmd>) {
    draw_rectangle_lines(0.0, y0, CANVAS_WIDTH, CANVAS_HEIGHT - y0, 1.0, WHITE);
    push_text(cmds, "up/down select an active quest, Esc to close", 4.0, y0 + 12.0, 7.0, WHITE);
}
