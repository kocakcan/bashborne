use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};

/// Polls for a key press within the given timeout. Returns None on timeout or
/// on non-key/non-press events (so held-key repeats on some terminals don't
/// fire twice).
pub fn poll_key(timeout: Duration) -> anyhow::Result<Option<KeyCode>> {
    if event::poll(timeout)? {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                return Ok(Some(key.code));
            }
        }
    }
    Ok(None)
}
