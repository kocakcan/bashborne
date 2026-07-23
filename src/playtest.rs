//! Scripted-input driver used by the `playtest` skill (see
//! `.claude/skills/playtest/SKILL.md`) to drive the game and capture
//! screenshots in-process, without a human at the keyboard. Only active when
//! `BASHBORNE_PLAYTEST_DIR` is set (see `main.rs`) — normal play never
//! touches this module.
//!
//! Protocol: the caller writes one command per line to `<dir>/cmd.txt`
//! (atomically, via a temp file + rename). Each command is applied, the game
//! is ticked/drawn until it reaches a stable, input-awaiting state (or a
//! hard timeout), then the offscreen canvas is exported to `<dir>/out.png`
//! and `<dir>/ready.txt` is written with an incrementing counter so the
//! caller knows a fresh frame is available.
//!
//! Command vocabulary (one per line in `cmd.txt`):
//!   Up | Down | Left | Right | Enter | Esc | Tab | PageUp | PageDown | Backspace
//!   Char:<c>      -- a single character key, e.g. `Char:i`, `Char:S`
//!   Wait:<secs>   -- no key; just keep ticking for `<secs>` simulated seconds
//!   Quit          -- exit the process

use std::path::{Path, PathBuf};

use macroquad::prelude::*;

use crate::app::World;
use crate::game::combat::{ActorRef, CombatPhase};
use crate::game::state::GameState;
use crate::input::Key;
use crate::render::{self, Assets};

const FIXED_DT: f32 = 1.0 / 60.0;
/// Minimum frames to settle even outside combat, covering the cosmetic
/// walk-step/cursor animations that don't gate on any state field.
const MIN_SETTLE_FRAMES: u32 = 8;
/// Hard cap so a genuine softlock (something that never reaches a stable
/// state) still produces a screenshot instead of hanging the driver forever.
const MAX_SETTLE_SECONDS: f32 = 5.0;

enum Cmd {
    Key(Key),
    Wait(f32),
    Quit,
}

fn parse_cmd(line: &str) -> Option<Cmd> {
    let line = line.trim();
    match line {
        "" => None,
        "Up" => Some(Cmd::Key(Key::Up)),
        "Down" => Some(Cmd::Key(Key::Down)),
        "Left" => Some(Cmd::Key(Key::Left)),
        "Right" => Some(Cmd::Key(Key::Right)),
        "Enter" => Some(Cmd::Key(Key::Enter)),
        "Esc" => Some(Cmd::Key(Key::Esc)),
        "Tab" => Some(Cmd::Key(Key::Tab)),
        "PageUp" => Some(Cmd::Key(Key::PageUp)),
        "PageDown" => Some(Cmd::Key(Key::PageDown)),
        "Backspace" => Some(Cmd::Key(Key::Backspace)),
        "Quit" => Some(Cmd::Quit),
        _ => {
            if let Some(c) = line.strip_prefix("Char:") {
                c.chars().next().map(|ch| Cmd::Key(Key::Char(ch)))
            } else if let Some(secs) = line.strip_prefix("Wait:") {
                secs.trim().parse::<f32>().ok().map(Cmd::Wait)
            } else {
                None
            }
        }
    }
}

/// Pulls one command out of `cmd.txt` if present, deleting the file so the
/// caller's next write is unambiguous. `cmd.txt` is single-writer (the
/// caller only ever `mv`s it into place once the previous one is gone), so a
/// plain read+remove is enough — no lock file needed.
fn take_command(cmd_path: &Path) -> Option<Cmd> {
    let content = std::fs::read_to_string(cmd_path).ok()?;
    let _ = std::fs::remove_file(cmd_path);
    parse_cmd(&content)
}

/// True while the game is mid-animation/enemy-AI and a screenshot would show
/// a transient frame rather than the settled result of the last input — the
/// same `resolving_timer`/enemy-`SelectAction` gate `World::tick` itself uses
/// to decide when to run `resolve_current_turn` (see `app.rs::tick`).
fn is_unsettled(world: &World) -> bool {
    let GameState::Combat(combat) = &world.state else {
        return false;
    };
    matches!(combat.phase, CombatPhase::Resolving)
        || matches!(
            combat.phase,
            CombatPhase::SelectAction {
                actor: ActorRef::Enemy(_)
            }
        )
}

/// Ticks/draws at a fixed synthetic `dt` until the game reaches a stable,
/// input-awaiting state (subject to `MIN_SETTLE_FRAMES`/`MAX_SETTLE_SECONDS`),
/// then captures and returns *without* swapping that final frame away —
/// see `capture`'s doc comment for why the swap has to wait.
async fn settle_and_capture(assets: &Assets, world: &mut World, dir: &Path, counter: u64) {
    let mut elapsed = 0.0f32;
    let mut frame = 0u32;
    loop {
        world.tick(FIXED_DT);
        elapsed += FIXED_DT;
        frame += 1;
        render::draw(assets, world);
        let done = elapsed >= MAX_SETTLE_SECONDS || (frame >= MIN_SETTLE_FRAMES && !is_unsettled(world));
        if done {
            capture(dir, counter);
            next_frame().await;
            return;
        }
        next_frame().await;
    }
}

/// Ticks/draws for at least `seconds` of simulated time, unconditionally,
/// then captures — backs the `Wait:<secs>` command, for e.g. letting a
/// status message linger on screen longer than the default settle window.
async fn wait_and_capture(assets: &Assets, world: &mut World, dir: &Path, counter: u64, seconds: f32) {
    let mut elapsed = 0.0f32;
    loop {
        world.tick(FIXED_DT);
        elapsed += FIXED_DT;
        render::draw(assets, world);
        if elapsed >= seconds {
            capture(dir, counter);
            next_frame().await;
            return;
        }
        next_frame().await;
    }
}

/// Exports the real window framebuffer to `out.png` (via a same-directory
/// temp file + rename, so a concurrent reader never sees a half-written
/// PNG) and bumps `ready.txt` so the caller knows a fresh frame is
/// available.
///
/// Two things this depends on, both easy to get wrong:
/// - Deliberately `get_screen_data()` (the real window), not
///   `assets.canvas.texture.get_texture_data()` (the offscreen canvas): per
///   CLAUDE.md's rendering model, `flush_text` draws every screen's text
///   directly onto the real window framebuffer in its own pass *after* the
///   canvas is blitted, so a canvas-only capture would show sprites/tiles
///   but silently drop all text (names, HP numbers, menu labels, dialogue).
/// - Must be called after `render::draw` but *before* that frame's
///   `next_frame().await`. macroquad double-buffers; `next_frame` swaps and
///   leaves the just-drawn content behind, so calling `get_screen_data()`
///   after the swap reads the new (undrawn) back buffer and silently
///   captures a solid-black frame instead.
fn capture(dir: &Path, counter: u64) {
    let image = get_screen_data();
    let tmp_path = dir.join("out.tmp.png");
    image.export_png(tmp_path.to_str().expect("scratch dir path must be utf8"));
    let _ = std::fs::rename(&tmp_path, dir.join("out.png"));
    let _ = std::fs::write(dir.join("ready.txt"), counter.to_string());
}

pub async fn run(assets: Assets, dir: PathBuf) {
    let _ = std::fs::create_dir_all(&dir);
    let mut world = World::at_main_menu();
    let cmd_path = dir.join("cmd.txt");
    let mut frame_counter: u64 = 0;

    // The very first frame has no command yet to react to — capture it
    // immediately so the caller can see the main menu without needing to
    // send a throwaway command first.
    frame_counter += 1;
    settle_and_capture(&assets, &mut world, &dir, frame_counter).await;

    loop {
        match take_command(&cmd_path) {
            Some(Cmd::Quit) => break,
            Some(Cmd::Key(key)) => {
                world.handle_key(key);
                frame_counter += 1;
                settle_and_capture(&assets, &mut world, &dir, frame_counter).await;
            }
            Some(Cmd::Wait(secs)) => {
                frame_counter += 1;
                wait_and_capture(&assets, &mut world, &dir, frame_counter, secs).await;
            }
            None => {
                // Idle: no command yet — keep the window alive at real dt
                // rather than busy-looping the OS thread.
                let dt = get_frame_time();
                world.tick(dt);
                render::draw(&assets, &world);
                next_frame().await;
            }
        }
        if world.should_quit {
            break;
        }
    }
}
