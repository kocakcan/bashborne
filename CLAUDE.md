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
requires the older versions. `anyhow`, `serde`, and `serde_json` are
ordinary caret deps.

There is no lint/format config checked in (no `rustfmt.toml`/`.clippy.toml`);
use standard `cargo fmt` / `cargo clippy` defaults if asked to clean up code.

## Architecture

```
src/
  main.rs          — terminal setup/teardown (raw mode, alt screen, panic hook), run loop
  event.rs         — thin crossterm key-poll wrapper
  app.rs           — World (Party + Inventory + GameState + chapter progress + rng), all input handling, save/load hooks
  ui/
    mod.rs         — persistent chapter/NG+ status bar chrome, then dispatches to the right screen based on GameState; shared HP-bar/rarity-color helpers
    explore.rs     — map viewport + party panel (incl. levels/XP and active effects) + log
    combat.rs      — animated enemy sprites/HP bars + party panel + action/ability/item menus + log
    event.rs       — blessing/curse/treasure/NPC-dialogue narrative screen
    inventory.rs   — out-of-combat inventory screen (equip gear, use items, move gear between members)
    shop.rs        — town shop screen (buy/sell)
    quest_log.rs   — active/completed quest list
    levelup.rs     — stat-point allocation screen
    blacksmith.rs  — weapon-upgrade screen
  game/
    character.rs   — Stats, Class, Character, Ability, per-class LevelGrowth, XP curve, roster + boss factory fns
    party.rs       — Party (Vec<Character> + gold + active status effects)
    item.rs        — Item, Rarity, Weapon/Armor/Ring, WeaponPassive, Inventory, gear factory functions
    inventory_ui.rs — InventoryUiState/Tab/Mode backing the out-of-combat inventory screen
    shop.rs        — fixed buy stock, ShopUiState, buy/sell pricing
    map.rs         — Tile, Map, Position (hand-authored ASCII layouts, one per chapter, incl. boss lairs)
    combat.rs      — CombatState: turn order, action resolution, crits, loot/XP, boss AI, win/lose detection
    chapter.rs     — ChapterId/ChapterDef registry (map, boss, NPCs, enemy_level), BossKind
    npc.rs         — NpcId registry: dialogue, map glyphs, quest hooks
    quest.rs       — QuestId registry, objectives, rewards, QuestLog
    quest_ui.rs    — QuestLogUiState backing the quest-log screen
    levelup.rs     — LevelUpUiState backing the allocation screen
    blacksmith.rs  — upgrade costs/increments, BlacksmithUiState
    sprites.rs     — two-frame animated ASCII art + per-species color for the combat screen
    status.rs      — StatusEffect: buffs/curses that persist across encounters
    save.rs        — SaveData (serde/JSON) + read/write of bashborne_save.json
    state.rs       — GameState enum, ExploreState, field-event table
```

### State machine

`GameState` is a plain enum (`Explore | Combat | Event | Inventory | Shop |
QuestLog | LevelUp | Blacksmith | GameOver`) rather than `Box<dyn Screen>`
trait objects — for a fixed, known
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

Sixteen regular monster species live in `character.rs` (Slime, Goblin, Bat,
Wolf, Skeleton, Orc, Wraith, Mimic, Hollow, Rat, Carrion Crow, Bandit, Fell
Acolyte, Grave Ghoul, Barrow Sentinel, Forsaken Knight), each with a distinct
stat profile. `state.rs`'s `roll_encounter` picks from twenty hand-tuned
compositions and then has a chance (`Character::elite_chance`, 10% baseline
rising to 24% at NG+7) to promote exactly one enemy in the roll to an Elite
variant (`Character::apply_elite`) — tougher, better-paying, and never a
Mimic or boss. Several species have signature moves handled in
`resolve_enemy_action`: the **Wraith** curses the party (30% of turns, via
`roll_curse`), **Goblins**/**Wolves** prioritize the lowest-HP%/lowest-max-HP
target 60% of the time, **Skeletons** can Bone Guard (skip the attack for
+3 defense), **Orcs** can Reckless Swing (1.6x damage with self-recoil),
**Bandits** can Coin Grab (steal gold instead of attacking), **Barrow
Sentinels** can Warcry (party-wide defense curse), and **Forsaken Knights**
can land Knight's Judgment (1.7x damage, no drawback). The **Mimic** never
appears in the normal encounter table — it's a 1-in-5 chance hiding inside
what looked like a treasure find. Enemy display names (`Character::
display_name`, e.g. "Elite Orc") are cosmetic only — every species-keyed
comparison always matches against the raw `Character.name`.

Each chapter has one boss, reachable only via its fixed `Tile::BossLair`
tile, not a random roll. Bosses are identified by `Character::boss_kind`
(`BossKind` in `chapter.rs`) and their scripted moves live in
`combat::resolve_boss_move`, one match arm per boss — the **Barrow Knight**
(Rending Cleave + one-time Second Wind at ≤30% HP), the **Wyrmscale Warden**
(party-wide Tail Sweep + one-time defense rally at ≤40% HP), and the
**Ashen Sovereign** (Cinder Nova + two-stage Ashen Rebirth at ≤50%/≤20% HP).
Beating a boss guarantees its Legendary signature weapon, marks the chapter
in `World.bosses_defeated` (so the lair never re-triggers), and advances
`World.current_chapter`; beating the final boss ends the game in victory.
Before a boss's own NG+ scaling is applied, `Character::scale_boss_to_party`
also toughens it (+6% max_hp/attack/defense per level the party's average
level exceeds `ChapterDef::boss_baseline_level()`, capped at 20 excess
levels) so a party that over-leveled before reaching the lair still gets a
real fight.

### Leveling & chapter difficulty

Victory XP (`combat::xp_value`, derived from the enemy's stats; 3x for
bosses) goes to every party member. Each level-up banks `POINTS_PER_LEVEL`
allocatable points (spent on the `u` screen) *and* applies the class's
automatic `level_growth` profile — Warriors toughen, Mages deepen their MP
pool, Rogues gain speed/luck — so party members diverge even before any
points are spent. That automatic growth is tapered by
`Character::growth_multiplier`: full strength through level 10, linearly
down to 0.5x by level 25, then floored there — hand-allocated points are
unaffected, so grinding levels stops compounding forever without capping
player choice. Regular monsters use the same machinery in reverse:
`ChapterDef::enemy_level` (1/6/11) is applied to every tall-grass roll via
`Character::scale_to_level`, so later chapters field stronger specimens of
the same species, which in turn raises the XP and gold they pay out.

### Weapons, rarity, inventory, shop

Every playable character always has exactly one `equipped_weapon`
(`Character::total_attack`/`total_defense` fold its bonuses into combat
math); monsters leave it `None`. `Rarity` (`item.rs`) is a five-tier enum
(Common < Uncommon < Rare < Epic < Legendary) with a derived `Ord`, plus
`base_value()` used for both shop pricing and loot value. Weapons come from
field-treasure rolls, per-species drop chances (`combat::loot_profile`),
the shop (Common/Uncommon/Rare only), or the boss (Legendary, guaranteed).

`i` from Explore opens the out-of-combat inventory (`ui/inventory.rs`,
state in `game/inventory_ui.rs`) to equip gear or use consumables on any
party member — displaced gear returns to the bag, and the `p` party-gear
mode can unequip or move pieces between members directly. `e` on a town
tile opens the shop (`ui/shop.rs`, `game/shop.rs`); selling only works on
spare (unequipped) gear. In combat, "Item" opens a submenu
(`CombatPhase::SelectItem`) to pick which consumable to use.

### Field events & status effects

Stepping into tall grass rolls a weighted outcome via `roll_field_event`
(`game/state.rs`): 50% combat, 15% blessing, 20% curse, 15% treasure (which
itself has a 1-in-5 chance of being a Mimic ambush, or a smaller chance of
burying a bonus weapon). All rolled enemies — Mimic included — are scaled to
the current chapter's `enemy_level` before the fight starts. Blessings/
curses (`game/status.rs`) are party-wide stat deltas (`StatusEffect {
target, delta, encounters_remaining }`) drawn from a 6-option pool each
(`roll_blessing`/`roll_curse`), 4 of which are always available and 2 of
which are tier-gated behind the current NG+ cycle (one unlocks at NG+1, one
at NG+2) so New Game+ surfaces new effects rather than just bigger monster
stats. They persist for a fixed number of *encounters*, not turns —
`Party::tick_effects()` counts one down each time a combat encounter
concludes (win or flee). `CombatState` reads `Party::stat_delta(target)` for
attack/defense/speed, so these actually affect combat math. Stacking the
same named effect merges it (magnitude adds, duration refreshes to the
longer of the two). Field events and combat both remember the tile you
stepped on (`return_pos`) so dismissing a notice or finishing a fight drops
you back exactly where you were. Fleeing combat (`CombatState::flee_chance`)
favors a party faster than the enemy pack and tightens by 2 percentage
points per NG+ cycle (capped at NG+7), clamped to a 15%–85% range.

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

## Save/load

`S` while exploring writes `bashborne_save.json` (serde/JSON, in the
working directory) via `game/save.rs`; startup (`World::load_or_new`)
resumes from it automatically if present — deleting the file is how a
player declines the continue. Only exploration state persists (party,
inventory, chapter/boss/NPC/quest progress, position); combat and menus are
never saved. `SaveData.version` gates loading: any mismatch or parse
failure silently falls back to a fresh game.

## Sprite animation

Combat sprites have two idle frames (`sprites::ANIM_FRAMES`), flipped about
twice a second by `World::anim_tick` (stepped once per ~100ms app-loop
tick). Two invariants, enforced by unit tests in `sprites.rs`: a species'
frames must share a line count (so the HP bar doesn't jump), and every line
within a frame must be the same width — the combat screen centers lines
independently, so a ragged line visibly drifts sideways.

## Known stubs / deliberately unfinished seams

- The playable-class roster (Warrior/Mage/Cleric/Rogue) is fixed at game
  start; there's no party selection or recruitment.

## Platform notes

The game is cross-platform by construction — `main.rs`/`event.rs` only use
crossterm/ratatui abstractions (no manual ANSI, no platform `#[cfg]`s
anywhere in the repo), and `game/save.rs`'s save path is a plain,
separator-agnostic `PathBuf`. On Windows, use Windows Terminal (or another
UTF-8-locale terminal) — legacy `conhost`/plain `cmd.exe` may mis-render the
box-drawing HP bars (`ui/mod.rs::hp_bar`, using `█`/`░`), the ASCII-art
title (`ui/main_menu.rs`), and arrow-key hint glyphs scattered across
`ui/*.rs`; run `chcp 65001` first if you must use a legacy console. Either
platform needs an 80x24 terminal at minimum (`ui/mod.rs::MIN_COLS`/
`MIN_ROWS`); `cargo run`/`cargo build` are identical on both.
