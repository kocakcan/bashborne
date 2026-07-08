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
}

/// Polls for at most one key this frame. Arrow/Enter/Esc/Tab/Page keys are
/// checked first so they win over a same-frame character press; letters and
/// digits come from `get_char_pressed()`, which already debounces to one
/// event per physical keypress.
pub fn poll_key() -> Option<Key> {
    use macroquad::input::{is_key_down, is_key_pressed, KeyCode as MqKey};

    let shift = is_key_down(MqKey::LeftShift) || is_key_down(MqKey::RightShift);

    let key = if is_key_pressed(MqKey::Up) || is_key_pressed(MqKey::W) {
        Some(Key::Up)
    } else if is_key_pressed(MqKey::Down) || (is_key_pressed(MqKey::S) && !shift) {
        Some(Key::Down)
    } else if is_key_pressed(MqKey::Left) || is_key_pressed(MqKey::A) {
        Some(Key::Left)
    } else if is_key_pressed(MqKey::Right) || is_key_pressed(MqKey::D) {
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
    } else {
        macroquad::input::get_char_pressed().map(Key::Char)
    };

    // `chars_pressed_queue` isn't cleared automatically each frame (unlike
    // keys_pressed) — only draining it here stops a same-frame char event
    // from surviving to leak out as a phantom keypress on a later frame.
    macroquad::input::clear_input_queue();
    key
}
