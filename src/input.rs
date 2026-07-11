/// The small, fixed set of keys the game actually reads. Kept separate from
/// macroquad's own `KeyCode` so `app.rs`'s input handling stays engine-agnostic
/// (mirrors how it never depended on crossterm's `KeyCode` beyond the name).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Up,
    Down,
    Left,
    Right,
    Enter,
    Esc,
    Tab,
    PageUp,
    PageDown,
    Backspace,
}

/// Seconds a direction must be held before it starts auto-repeating.
const INITIAL_REPEAT_DELAY: f32 = 0.35;
/// Seconds between repeats once auto-repeat has kicked in.
const REPEAT_INTERVAL: f32 = 0.08;

/// Edge-triggers immediately when a key transitions from up to held, then
/// repeats at a fixed cadence — the same shape as OS key-repeat, but applied
/// uniformly by us so arrows and their WASD equivalents move at identical
/// speed when held. `poll_key`'s old approach relied on macroquad's own
/// repeat behavior, which only exists for `get_char_pressed()` (letters) and
/// not for `is_key_pressed()` (arrows) — holding an arrow key fired once and
/// then nothing until release, while holding W/A/S/D free-rode the OS's
/// character-repeat timer, so WASD visibly outran the arrow keys.
#[derive(Default)]
struct KeyRepeat {
    held: bool,
    timer: f32,
}

impl KeyRepeat {
    fn poll(&mut self, down: bool, dt: f32) -> bool {
        if !down {
            self.held = false;
            self.timer = 0.0;
            return false;
        }
        if !self.held {
            self.held = true;
            self.timer = INITIAL_REPEAT_DELAY;
            return true;
        }
        self.timer -= dt;
        if self.timer <= 0.0 {
            self.timer = REPEAT_INTERVAL;
            true
        } else {
            false
        }
    }
}

/// Stateful key poller. Directional input (arrows and WASD alike) is driven
/// by our own held-key timer rather than macroquad's raw pressed/char
/// events, so both input schemes repeat at the same rate while held.
#[derive(Default)]
pub struct Input {
    up: KeyRepeat,
    down: KeyRepeat,
    left: KeyRepeat,
    right: KeyRepeat,
}

impl Input {
    pub fn new() -> Self {
        Self::default()
    }

    /// Polls for at most one key this frame. Directional keys are checked
    /// first via the held-key repeat timers so they win over a same-frame
    /// character press; everything else falls back to
    /// `get_char_pressed()`, which already debounces to one event per
    /// physical keypress.
    pub fn poll(&mut self, dt: f32) -> Option<Key> {
        use macroquad::input::{is_key_down, is_key_pressed, KeyCode as MqKey};

        let shift = is_key_down(MqKey::LeftShift) || is_key_down(MqKey::RightShift);

        let up_down = is_key_down(MqKey::Up) || is_key_down(MqKey::W);
        let down_down = is_key_down(MqKey::Down) || (is_key_down(MqKey::S) && !shift);
        let left_down = is_key_down(MqKey::Left) || is_key_down(MqKey::A);
        let right_down = is_key_down(MqKey::Right) || is_key_down(MqKey::D);

        // Poll every direction every frame (not just the winner) so a
        // released key always resets its own timer.
        let up = self.up.poll(up_down, dt);
        let down = self.down.poll(down_down, dt);
        let left = self.left.poll(left_down, dt);
        let right = self.right.poll(right_down, dt);

        let key = if up {
            Some(Key::Up)
        } else if down {
            Some(Key::Down)
        } else if left {
            Some(Key::Left)
        } else if right {
            Some(Key::Right)
        } else if is_key_pressed(MqKey::Enter) || is_key_pressed(MqKey::KpEnter) {
            Some(Key::Enter)
        } else if is_key_pressed(MqKey::Escape) {
            Some(Key::Esc)
        } else if is_key_pressed(MqKey::Tab) {
            Some(Key::Tab)
        } else if is_key_pressed(MqKey::PageUp) {
            Some(Key::PageUp)
        } else if is_key_pressed(MqKey::PageDown) {
            Some(Key::PageDown)
        } else if is_key_pressed(MqKey::Backspace) {
            Some(Key::Backspace)
        } else {
            // w/a/s/d are already handled above via the repeat timers; if we
            // let them fall through here too, macroquad's own OS-driven
            // character-repeat would sneak in extra movement events between
            // our timer's ticks, reintroducing the exact speed mismatch this
            // struct exists to fix. Drain the queue looking for a char that
            // isn't one of those four.
            let mut found = None;
            while let Some(c) = macroquad::input::get_char_pressed() {
                if matches!(c, 'w' | 'a' | 's' | 'd') {
                    continue;
                }
                found = Some(c);
                break;
            }
            found.map(Key::Char)
        };

        // `chars_pressed_queue` isn't cleared automatically each frame (unlike
        // keys_pressed) — only draining it here stops a same-frame char event
        // from surviving to leak out as a phantom keypress on a later frame.
        macroquad::input::clear_input_queue();
        key
    }
}
