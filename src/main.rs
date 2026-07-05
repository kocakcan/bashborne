mod app;
mod event;
mod game;
mod ui;

use std::io::stdout;
use std::panic;
use std::time::Duration;

use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, SetTitle,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::World;

fn main() -> anyhow::Result<()> {
    install_panic_hook();

    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen, SetTitle("Bashborne"))?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> anyhow::Result<()> {
    let mut world = World::new();

    while !world.should_quit {
        terminal.draw(|frame| ui::draw(frame, &world))?;

        if let Some(key) = event::poll_key(Duration::from_millis(100))? {
            world.handle_key(key);
        }
        world.tick();
    }
    Ok(())
}

/// Ensures raw mode / alternate screen are torn down even if we panic,
/// otherwise the user's terminal is left in a broken state.
fn install_panic_hook() {
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
        default_hook(info);
    }));
}
