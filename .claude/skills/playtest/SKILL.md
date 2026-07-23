---
name: playtest
description: Drive Bashborne (the macroquad game window) programmatically and capture real screenshots, to playtest any screen — exploration, combat, inventory, shop, quest log, bestiary, level-up, blacksmith, NPC dialogue — without a human relaying keypresses/screenshots.
---

# Playtesting Bashborne

Bashborne opens a real OS window (macroquad), not a terminal UI, so it can't
be driven the way a CLI tool can. This skill uses a driver mode built into
the game itself (`src/playtest.rs`, gated behind the `BASHBORNE_PLAYTEST_DIR`
env var — completely inert in normal play) that accepts scripted keypresses
via a command file and exports a real screenshot after each one.

This is a general-purpose driver, not a combat-only or movement-only tool:
it works identically on every `GameState` screen (Explore, Combat, Inventory
— including its Browsing/PartyGear/Roster/ring-slot sub-modes — Shop,
QuestLog, Bestiary, LevelUp, Blacksmith, Event dialogue, GameOver,
MainMenu), because it operates at the same `World::handle_key`/`World::tick`
level the real input loop does. Use it for any playtest ask, not just "walk
around and fight something."

## Setup

1. Build once so the binary is warm: `cargo build` (from the repo root).
2. Pick a scratch directory for the IPC files, e.g.
   `/tmp/bashborne-playtest` or your session scratchpad.
3. Launch the game in playtest mode, detached from this shell so it
   survives after the launching command returns. Backgrounding with plain
   `&` is **not** enough — when the Bash tool's subshell exits, an
   un-disowned child gets SIGHUP'd and the game dies with it. Use
   `nohup ... & disown`:

   ```bash
   DIR=/tmp/bashborne-playtest
   mkdir -p "$DIR"
   rm -f "$DIR"/cmd.txt "$DIR"/ready.txt "$DIR"/out.png
   cd /path/to/bashborne
   nohup env BASHBORNE_PLAYTEST_DIR="$DIR" cargo run > "$DIR/game.log" 2>&1 &
   disown
   ```

4. Confirm it's actually alive and has produced the first frame (main menu)
   before sending commands:

   ```bash
   sleep 3
   pgrep -fl "target/debug/bashborne"
   ls -la "$DIR"          # expect out.png + ready.txt
   ```

## Protocol

The driver watches `<dir>/cmd.txt` for one command at a time. Each command
is applied via `World::handle_key`, the game ticks/draws until it reaches a
stable, input-awaiting state (see "Settling" below), then it exports the
**real window framebuffer** to `<dir>/out.png` and bumps `<dir>/ready.txt`
(an incrementing counter) so you know a fresh frame landed.

Send a command with an atomic write (temp file + `mv`, matching what the
driver expects — it just reads-then-deletes `cmd.txt`), then poll
`ready.txt` for its value to change before reading `out.png`. A tiny helper
function makes this a one-liner per step — `timeout` isn't available on
stock macOS, so poll with a bounded loop instead:

```bash
send() {
  local DIR=/tmp/bashborne-playtest
  local prev; prev=$(cat "$DIR/ready.txt" 2>/dev/null || echo 0)
  echo -n "$1" > "$DIR/cmd.tmp"
  mv "$DIR/cmd.tmp" "$DIR/cmd.txt"
  local i=0
  while [ "$(cat "$DIR/ready.txt" 2>/dev/null)" == "$prev" ]; do
    sleep 0.1
    i=$((i+1))
    [ "$i" -gt 80 ] && { echo "TIMEOUT waiting on $1" >&2; break; }
  done
}
send "Down"
```

Then use the Read tool on `<dir>/out.png` to actually look at the frame and
decide the next command. **This has to be a real observe/decide loop, not a
blind script** — RNG isn't seeded (encounters, loot, blessings/curses are
genuinely random), so the same command sequence can land somewhere different
each run. Screenshot, look, then decide the next keypress, the same way a
human tester would.

### Command vocabulary (one per line/write to `cmd.txt`)

| Command | Effect |
|---|---|
| `Up` `Down` `Left` `Right` | Arrow keys (movement, menu navigation) |
| `Enter` | Confirm/select |
| `Esc` | Back/cancel |
| `Tab` | Cycle tabs (Inventory/Shop) |
| `PageUp` `PageDown` | Scroll a log |
| `Backspace` | Undo (level-up screen) |
| `Char:<c>` | Any single character key, e.g. `Char:i`, `Char:e`, `Char:S`, `Char:y`, `Char:?` |
| `Wait:<secs>` | No key — just keep ticking for `<secs>` simulated seconds (e.g. to let a status message linger, or pad extra settle time) |
| `Quit` | Exit the game process |

### Settling

A screenshot taken the instant a key is applied often shows a transient
frame — mid lunge/flash animation, or a still-thinking enemy turn — not the
actual result. The driver already handles this: after applying a key, it
keeps ticking (fixed `dt = 1/60`) until `World`'s state stops being
"unsettled" — defined as `GameState::Combat` sitting in
`CombatPhase::Resolving` or an enemy's `SelectAction` turn (the same gate
`app.rs::tick` itself uses to decide when to run enemy AI) — subject to a
hard 5-simulated-second cap so a genuine softlock still produces a
screenshot instead of hanging the driver forever. Outside combat it always
settles at least 8 frames to clear cosmetic walk-step/cursor animation. You
don't need to add manual `Wait:` padding for normal turns; reach for it only
when you want a message to linger longer than the default settle window, or
you suspect something is still mid-transition.

If a screenshot ever looks "in-between" (an attack animation frozen
mid-flight, text partially formed) immediately after a fresh screen
transition, it may be a one-frame font-glyph-cache warm-up artifact rather
than a real bug — send one more `Wait:0.1` and re-screenshot before
concluding it's broken. (This did turn up a real, reproducible one-frame HUD
text-clipping glitch on the very first frame after `Explore` starts — see
"What to look for" below. Confirm anything like that against a second
capture, and ideally an OS-level `screencapture` of the live window, before
reporting it.)

### Identifying what you're looking at — don't guess positions from pixels

Map coordinates are hand-authored per chapter in `src/game/chapter.rs`
(`ChapterDef::spawn`, `ChapterDef::npcs: Vec<(Position, NpcId)>`) and
`src/game/map.rs` (`Map::starting_area` etc., ASCII layout — `T` = Town,
`B` = BossLair, `,` = tall grass, `.` = floor, `#` = wall). **Read these
before assuming which on-screen sprite is the player vs. an NPC** — multiple
similarly-colored humanoid sprites can sit close together, and eyeballing
canvas pixel coordinates against a 28x10 grid is unreliable and easy to get
backwards. The player is the sprite that moves when you send movement
commands; anything that doesn't move between two captures despite a
movement command is a static NPC. When in doubt, cross-reference the
NPC's fixed `Position` from `chapter_def(...).npcs` against the map's ASCII
layout rather than trusting a pixel guess.

## Worked example: main menu → fresh game on the dev slot

Slot 3 is the "(Dev)" sandbox slot (see CLAUDE.md's save docs) — safe to
overwrite freely. From a fresh boot (menu cursor starts at row 0):

```
send "Down"    # cursor -> slot 2 row
send "Down"    # cursor -> slot 3 row
send "Enter"   # empty slot -> pending_new_game, opens difficulty picker
send "Enter"   # confirms difficulty cursor 0 ("Normal") -> starts the game
```

**Always screenshot first and confirm slot 3 is actually empty** before
assuming this sequence — if it already holds a save, `Enter` loads it
instead of starting fresh, and forcing a new game over it needs `n` plus a
confirm (`Enter`/`y`) instead.

## Reaching other screens from Explore

All of these are single keys from `GameState::Explore`, no menu needed:

| Key | Screen |
|---|---|
| `Char:i` | Inventory (Browsing tab; `Tab`/arrows to switch tabs, `p` for party-gear mode, `r` for roster, `Esc` closes) |
| `Char:l` | Quest log |
| `Char:u` | Level-up |
| `Char:b` | Bestiary |
| `Char:e` | Context-sensitive: talks to an NPC standing on that tile, opens the Blacksmith screen if it's the Blacksmith NPC, or opens the Shop if you're on a `Town` tile with no NPC underfoot |
| `Char:S` | Quicksave to the active slot |
| `Char:q` | Quit-confirm prompt (`Enter`/`y` confirms, `Esc`/`n` cancels) |
| `Char:?` | Keybind help overlay (`Esc` or `Char:?` again closes) |

Combat starts automatically (not via a keypress) when a movement command
lands on `TallGrass` and the RNG roll hits (`roll_field_event`, ~1-in-4 of
grass steps produce *some* event — not always a fight), or when you walk
onto a chapter's `BossLair` tile. In combat: `Up`/`Down` move the action
menu cursor, `Enter` selects/confirms, `Esc` backs up a step, arrows cycle
targets.

## Cleanup

Send `Quit` when done:

```bash
send "Quit"
```

or, if the driver is unresponsive, kill it directly:

```bash
pkill -f "target/debug/bashborne"
```

## What to look for

- Text clipping/overflow, especially right after a screen transition (a
  transient one-frame version is a known glyph-cache warm-up quirk — see
  "Settling" above; anything that persists past a couple of frames is real).
- Misaligned HP/MP/XP bars, wrong colors (`hp_color`/`rarity_color`/
  `stat_color` in `render/common.rs` drive most of this).
- Broken screen transitions (wrong return position, stale log content,
  cursor pointing past the end of a now-shorter list after selling/using
  the last of something).
- Softlocks — the settle loop hitting its 5-second cap with nothing
  advancing is itself a signal worth reporting, not just a driver quirk.
- Combat/menu logic that doesn't match what CLAUDE.md documents (e.g. an
  Elite enemy's guaranteed-first-turn signature move, boss rally
  thresholds, overkill bonus gold, sell-confirm gating on Rare+ gear only).

## Verification checklist for changes to this skill itself

If you edit `src/playtest.rs`:

- `cargo build` / `cargo test` still pass — the module is additive and
  gated behind an env var unset in normal play, so it should never affect
  the existing test suite.
- Re-run the main-menu → new-game → move → open-every-screen → combat →
  `Quit` sequence above end-to-end and confirm each capture matches what
  you'd expect from a manual `cargo run`.
- Specifically re-check the two easy-to-regress details in `capture()`:
  it must call `get_screen_data()` (the real window — `flush_text` draws
  every screen's text there, in a pass *after* the canvas is blitted, so
  the offscreen canvas alone is missing all text), and it must run
  *before* that frame's `next_frame().await` (macroquad double-buffers;
  capturing after the swap silently reads the new, undrawn back buffer and
  produces a solid-black image).
