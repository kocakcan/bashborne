# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Bashborne is a Dark Souls–flavored, turn-based RPG that runs entirely in a
terminal, built in Rust with `ratatui` + `crossterm`. No other runtime,
network, or build tooling is involved — it's a single-binary TUI game.

## Commands

```
cargo run             # play the game (needs a real terminal, not piped stdin/stdout)
cargo build            # compile without running
cargo test              # run all unit tests
cargo test <name>        # run a single test by name (e.g. cargo test resolve_enemy_action)
cargo test --lib module::   # run all tests within one module, e.g. cargo test game::combat::
```

Dependency versions in `Cargo.toml` are pinned exactly
(`ratatui = "=0.26.3"`, `crossterm = "=0.27.0"`, `rand = "=0.8.5"`) because
the project was originally built against an old rustc. Don't loosen these
without checking the toolchain actually in use — nothing in the code
requires the older versions.

There is no lint/format config checked in (no `rustfmt.toml`/`.clippy.toml`);
use standard `cargo fmt` / `cargo clippy` defaults if asked to clean up code.

## Architecture

```
src/
  main.rs          — terminal setup/teardown (raw mode, alt screen, panic hook), run loop
  event.rs         — thin crossterm key-poll wrapper
  app.rs           — World (Party + Inventory + GameState + rng), all input handling
  ui/
    mod.rs         — dispatches to the right screen based on GameState; shared HP-bar/rarity-color helpers
    explore.rs     — map viewport + party panel (incl. active effects) + log
    combat.rs      — enemy sprites/HP bars + party panel + action/ability menu + log
    event.rs       — blessing/curse/treasure narrative screen
    inventory.rs   — out-of-combat inventory screen (equip weapons, use items)
    shop.rs        — town shop screen (buy/sell)
  game/
    character.rs   — Stats, Class, Character (incl. equipped_weapon), Ability, full roster + boss factory fns
    party.rs       — Party (Vec<Character> + gold + active status effects)
    item.rs        — Item, Rarity, Weapon, Inventory, item/weapon factory functions
    inventory_ui.rs — InventoryUiState/Tab/Mode backing the out-of-combat inventory screen
    shop.rs        — fixed buy stock, ShopUiState, buy/sell pricing
    map.rs         — Tile, Map, Position (hand-authored ASCII layout, incl. the boss lair)
    combat.rs      — CombatState: turn order, action resolution, loot, boss AI, win/lose detection
    sprites.rs     — ASCII art + per-species color for the combat screen
    status.rs      — StatusEffect: buffs/curses that persist across encounters
    state.rs       — GameState enum (Explore | Combat | Event | Inventory | Shop | GameOver), field-event table
```

### State machine

`GameState` is a plain enum (`Explore | Combat | Event | Inventory | Shop |
GameOver`) rather than `Box<dyn Screen>` trait objects — for a fixed, known
set of screens this gives exhaustive `match` checking at compile time with
no dynamic dispatch. `CombatPhase` is the same idea one level down:
`SelectAction → SelectAbility → SelectTarget → {Victory, Defeat, Fled}`, all
exhaustively matched wherever it's read. `CombatPhase::Resolving` exists but
is currently unused (reserved for adding a delay between an action and the
log update).

### Turn order & combat

`CombatState::new` builds a `Vec<ActorRef>` (player/enemy indices) sorted by
speed once at combat start. `resolve_current_turn` handles both
player-selected and AI-driven turns, then `advance_turn` walks the cursor
forward, skipping dead actors, and checks win/lose conditions.

Eight regular monster species live in `character.rs` (Slime, Goblin, Bat,
Wolf, Skeleton, Orc, Wraith, Mimic), each with a distinct stat profile.
`state.rs`'s `roll_encounter` picks from ten hand-tuned compositions. The
**Wraith** has a signature move (30% of turns, curses the party via
`roll_curse`) handled in `resolve_enemy_action`. The **Mimic** never appears
in the normal encounter table — it's a 1-in-6 chance hiding inside what
looked like a treasure find.

The **Barrow Knight** boss (`character.rs`) is reachable only via the fixed
`Tile::BossLair` tile, not a random roll, and has two scripted moves in
`resolve_enemy_action` alongside the Wraith's curse: **Rending Cleave**
(~1.8x damage, ~35% of turns) and a one-time **Second Wind** (heals 20% max
HP, permanently +6 attack, triggers once at ≤30% HP). Beating it guarantees
**Knightsbane**, the strongest weapon in the game. `World.boss_defeated`
prevents the lair from re-triggering. Additional bosses would follow the
same name-checked pattern inside `resolve_enemy_action`.

### Weapons, rarity, inventory, shop

Every playable character always has exactly one `equipped_weapon`
(`Character::total_attack`/`total_defense` fold its bonuses into combat
math); monsters leave it `None`. `Rarity` (`item.rs`) is a five-tier enum
(Common < Uncommon < Rare < Epic < Legendary) with a derived `Ord`, plus
`base_value()` used for both shop pricing and loot value. Weapons come from
field-treasure rolls, per-species drop chances (`combat::loot_profile`),
the shop (Common/Uncommon/Rare only), or the boss (Legendary, guaranteed).

`i` from Explore opens the out-of-combat inventory (`ui/inventory.rs`,
state in `game/inventory_ui.rs`) to equip weapons or use consumables on any
party member — displaced weapons return to the bag. `e` on a town tile
opens the shop (`ui/shop.rs`, `game/shop.rs`); selling only works on spare
(unequipped) weapons, so unequip via the inventory screen first.
**In-combat item use is hardcoded to inventory slot 0** —
`resolve_pending_target` in `app.rs` always uses
`self.inventory.items.first()`; there's no item submenu yet.

### Field events & status effects

Stepping into tall grass rolls a weighted outcome via `roll_field_event`
(`game/state.rs`): 55% combat, 15% blessing, 15% curse, 15% treasure (which
itself has a small chance of being a Mimic ambush, or of burying a bonus
weapon). Blessings/curses (`game/status.rs`) are party-wide stat deltas
(`StatusEffect { target, delta, encounters_remaining }`) that persist for a
fixed number of *encounters*, not turns — `Party::tick_effects()` counts one
down each time a combat encounter concludes (win or flee). `CombatState`
reads `Party::stat_delta(target)` for attack/defense/speed, so these
actually affect combat math. Stacking the same named effect merges it
(magnitude adds, duration refreshes to the longer of the two). Field events
and combat both remember the tile you stepped on (`return_pos`) so
dismissing a notice or finishing a fight drops you back exactly where you
were.

### Loot

`CombatState::loot` is rolled once, on victory, via `roll_loot` in
`game/combat.rs`, which looks up a per-species gold range and optional
item/weapon-drop chance from `loot_profile`. `World::conclude_combat`
applies it to `Party::gold`/`Inventory` and shows a summary on both the
victory screen and the exploration log.

## Testing

Unit tests live alongside the modules they cover (`app.rs`, `game/combat.rs`,
`game/character.rs`, `game/item.rs`, `game/map.rs`, `game/shop.rs`) and
exercise game logic directly (turn order, damage, weapon/rarity math, loot
and shop pricing, boss scripted moves, inventory/shop input handling)
without needing a terminal. UI rendering code has no automated tests.

**If you need to verify the TUI itself interactively**: naive ANSI-stripping
of the terminal output stream doesn't work, because ratatui only
retransmits *changed* cells with explicit cursor-positioning escape codes —
stripped-and-concatenated text from multiple frames comes out jumbled. Feed
the real PTY output through a proper terminal emulator (e.g. the `pyte`
Python package) to reconstruct an accurate screen buffer before asserting
against it.

## Known stubs / deliberately unfinished seams

- `Class::Rogue` exists but has no factory function yet.
- Only two blessings and two curses exist in `status.rs`'s pools
  (`roll_blessing`, `roll_curse`).
- No leveling: `Character.level` is hardcoded to `1` everywhere; gear is the
  only progression axis.
- No armor/accessory slot — `equipped_weapon` is the only equipment slot.
- No save/load — nothing implements `Serialize`/`Deserialize` yet.
- Single overworld map (`Map::starting_area()`), single boss.
- In-combat item selection is hardcoded to inventory slot 0 (see above).
