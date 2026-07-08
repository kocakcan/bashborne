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
        window_width: 1440,
        window_height: 810,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let assets = Assets::load().await;
    // Start on the title screen; Continue/New Game decide the save's fate.
    let mut world = World::at_main_menu();

    while !world.should_quit {
        if let Some(key) = input::poll_key() {
            world.handle_key(key);
        }
        world.tick(get_frame_time());

        render::draw(&assets, &world);

        next_frame().await;
    }
}
