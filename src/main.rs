mod app;
mod game;
mod input;
mod render;

use macroquad::prelude::*;

use app::World;
use render::assets::Assets;

fn window_conf() -> Conf {
    Conf {
        window_title: "Bashborne".to_owned(),
        window_width: 960,
        window_height: 540,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let assets = Assets::load().await;
    // Start on the title screen; Continue/New Game decide the save's fate.
    let mut world = World::at_main_menu();
    match std::env::var("BB_SCREEN").as_deref() {
        Ok("explore") => world = World::new(),
        Ok("inventory") => {
            world = World::new();
            world.inventory.weapons.push(game::item::iron_sword());
            world.inventory.weapons.push(game::item::bandits_falchion());
            world.inventory.armors.push(game::item::padded_vest());
            world.inventory.rings.push(game::item::copper_band());
            world.inventory.rings.push(game::item::iron_loop());
            world.inventory.upgrade_materials = 3;
            let return_pos = game::map::Position { x: 0, y: 0 };
            let mut inv_ui = game::inventory_ui::InventoryUiState::new(return_pos);
            match std::env::var("BB_INV_MODE").as_deref() {
                Ok("weapons") => inv_ui.tab = game::inventory_ui::InventoryTab::Weapons,
                Ok("party_gear") => {
                    inv_ui.mode = game::inventory_ui::InventoryMode::PartyGear {
                        member_cursor: 1,
                        slot_cursor: 0,
                    }
                }
                _ => {}
            }
            world.state = game::state::GameState::Inventory(inv_ui);
        }
        _ => {}
    }

    while !world.should_quit {
        if let Some(key) = input::poll_key() {
            world.handle_key(key);
        }
        world.tick(get_frame_time());

        render::draw(&assets, &world);

        next_frame().await;
    }
}
