mod app;
mod game;
mod input;
mod render;

use macroquad::prelude::*;

use app::World;
use input::Input;
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
    let mut input = Input::new();

    while !world.should_quit {
        let dt = get_frame_time();
        if let Some(key) = input.poll(dt) {
            world.handle_key(key);
        }
        world.tick(dt);

        render::draw(&assets, &world);

        next_frame().await;
    }
}
