# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Bashborne is a Dark Souls–flavored, turn-based RPG built in Rust with
`macroquad`, a real 2D-canvas/OpenGL game engine — it opens an actual window
(not a terminal UI). Everything is drawn onto a fixed low-res offscreen
canvas (480x270) using bundled Kenney pixel-art sprite sheets and a pixel
font, then blitted to the real window at integer scale. No other runtime,
network, or build tooling is involved — it's a single-binary game.

**Note:** earlier versions of this project (and of this file) used
`ratatui`/`crossterm` to render as a terminal UI (`src/ui/`, `src/event.rs`).
That was fully replaced by the macroquad rewrite; those paths no longer
exist. If you find other docs/comments still describing a terminal UI,
they're stale — trust the source tree over old prose.

## Commands

```
cargo run             # play the game (opens a real window; not a terminal UI)
cargo build            # compile without running
cargo test              # run all unit tests
cargo test <name>        # run a single test by name (e.g. cargo test resolve_enemy_action)
cargo test --lib module::   # run all tests within one module, e.g. cargo test game::combat::
```

Dependency versions in `Cargo.toml` are pinned exactly
(`macroquad = "=0.4.15"`, `rand = "=0.8.5"`). Don't loosen these without
checking the toolchain/engine version actually in use. `anyhow`, `serde`,
and `serde_json` are ordinary caret deps.

There is no lint/format config checked in (no `rustfmt.toml`/`.clippy.toml`);
use standard `cargo fmt` / `cargo clippy` defaults if asked to clean up code.

## Architecture

```
src/
  main.rs          — window_conf (title/size), Assets::load, the macroquad main loop (poll key -> tick -> draw -> next_frame)
  input.rs         — engine-agnostic Key enum; stateful Input::poll wraps macroquad's raw KeyCode polling with its own key-repeat timer
  app.rs           — World (Party + Inventory + GameState + chapter progress + rng + anim timer), all input handling, save/load hooks
  render/
    mod.rs         — sets up the offscreen camera/canvas, dispatches to the right screen based on GameState, blits the canvas to the window, then flushes queued text
    assets.rs      — Assets (tile/character textures, font, canvas render target, text shader material), the shared 17px-pitch sprite-atlas `cell()` helper, and per-tile/NPC/weapon/armor/ring/item-kind icon-rect lookups
    common.rs      — TextCmd queue (push_text/flush_text), canvas_transform, hp_color/rarity_color/stat_color helpers, draw_icon, scroll_window
    hud.rs         — persistent chapter/NG+ status bar chrome + the `?` keybind help overlay
    explore.rs     — map viewport + party panel (levels/HP/XP bars) + log
    combat.rs      — enemy panel (flat species-colored boxes + HP bars) + party panel + action/ability/item menus + log
    event.rs       — blessing/curse/treasure/NPC-dialogue narrative screen
    inventory.rs   — out-of-combat inventory screen (equip gear, use items, move gear between members); also exports draw_party_gear, shared with the shop screen
    shop.rs        — town shop screen (buy/sell)
    quest_log.rs   — active/completed quest list
    levelup.rs     — stat-point allocation screen
    blacksmith.rs  — weapon-upgrade screen
    main_menu.rs   — title screen (Continue/New Game/Quit)
  game/
    character.rs   — Stats, Class, Character, Ability, per-class LevelGrowth, XP curve, roster + boss factory fns
    party.rs       — Party (Vec<Character> + gold + active status effects)
    item.rs        — Item, Rarity, Weapon/Armor/Ring, WeaponPassive, Inventory, gear factory functions
    inventory_ui.rs — InventoryUiState/Tab/Mode backing the out-of-combat inventory screen
    shop.rs        — fixed buy stock, ShopUiState, buy/sell pricing
    map.rs         — Tile, Map, Position (hand-authored tile layouts, one per chapter, incl. boss lairs)
    combat.rs      — CombatState: turn order, action resolution, crits, loot/XP, boss AI, win/lose detection
    chapter.rs     — ChapterId/ChapterDef registry (map, boss, NPCs, enemy_level), BossKind
    npc.rs         — NpcId registry: dialogue, map glyphs, quest hooks
    quest.rs       — QuestId registry, objectives, rewards, QuestLog
    quest_ui.rs    — QuestLogUiState backing the quest-log screen
    levelup.rs     — LevelUpUiState backing the allocation screen
    blacksmith.rs  — upgrade costs/increments, BlacksmithUiState
    status.rs      — StatusEffect: buffs/curses that persist across encounters
    save.rs        — SaveData (serde/JSON) + slot-aware read/write (3 save slots)
    state.rs       — GameState enum, ExploreState, field-event table
```

### Rendering model

Every screen draws into a fixed 480x270 logical canvas (`render::assets::
CANVAS_WIDTH`/`CANVAS_HEIGHT`), which `render::mod::draw` then blits to the
real window at the largest clean integer scale (letterboxed) so pixel art
stays crisp at any window size. Sprites/tiles/icons are drawn directly onto
the canvas via `draw_texture_ex` in canvas space. Text is handled
differently: screens queue it via `push_text` into a `Vec<TextCmd>` instead
of rasterizing into the low-res canvas (a pixel font at 7-10px canvas-space
sizes is illegible), and `flush_text` renders it afterward in real screen
space at its true final size, using a hard-edge alpha-threshold shader so
glyphs read crisp like the Nearest-filtered tile art instead of carrying
fontdue's usual AA fringe. Anything new that should look like the existing
pixel art (icons, tiles, sprites) belongs in the canvas-space `draw_texture_ex`
pass, not the `TextCmd`/`flush_text` path.

Both bundled Kenney sheets (`assets/roguelike_rpg_pack.png` for tiles/items,
`assets/roguelike_characters.png` for the player/NPCs/weapons/armor) share a
17px-pitch/16px-tile grid, indexed via `assets::cell(col, row) -> Rect`.
Weapons/armor/rings/materials each get one generic icon (the underlying
structs have no sub-type field to key finer art off of); consumables get one
icon per `ItemKind` variant.

### State machine

`GameState` is a plain enum (`Explore | Combat | Event | Inventory | Shop |
QuestLog | LevelUp | Blacksmith | GameOver`) rather than `Box<dyn Screen>`
trait objects — for a fixed, known
set of screens this gives exhaustive `match` checking at compile time with
no dynamic dispatch. `CombatPhase` is the same idea one level down:
`SelectAction → SelectAbility → SelectTarget → {Victory, Defeat, Fled}`, all
exhaustively matched wherever it's read. `CombatPhase::Resolving` is the
turn-resolution hold state: `app.rs::begin_resolving_hold` switches into it
and stashes the real next phase in `pending_phase` behind a
`RESOLVING_HOLD_SECONDS` timer, and `render/combat.rs` reads it to drive the
lunge/flash/idle-bob animation beat before the log updates and the phase
advances for real.

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
`roll_curse`), **Fell Acolytes** can chant a Withering Prayer (25%, drains a
member's MP to heal themselves), **Goblins**/**Wolves** prioritize the
lowest-HP%/lowest-max-HP target 60% of the time, **Skeletons** can Bone
Guard (skip the attack for +3 defense), **Orcs** can Reckless Swing (1.6x
damage with self-recoil), **Bandits** can Coin Grab (steal gold instead of
attacking), **Barrow Sentinels** can Warcry (party-wide defense curse), and
**Forsaken Knights** can land Knight's Judgment (1.7x damage, no drawback).
For those seven probability-gated moves, an Elite variant additionally gets
one guaranteed shot at its species' move on its first eligible turn
(`Character.elite_signature_used`, checked as `(is_elite &&
!elite_signature_used) || rng.gen_bool(p)` in `resolve_enemy_action`) before
reverting to the normal per-turn odds — a mechanical difference from a plain
enemy, not just bigger numbers. The **Mimic** never appears in the normal
encounter table — it's a 1-in-5 chance hiding inside what looked like a
treasure find. Enemy display names (`Character::display_name`, e.g. "Elite
Orc") are cosmetic only — every species-keyed comparison always matches
against the raw `Character.name`.

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
the shop, or the boss (Legendary, guaranteed).

`i` from Explore opens the out-of-combat inventory (`render/inventory.rs`,
state in `game/inventory_ui.rs`) to equip gear or use consumables on any
party member — displaced gear returns to the bag, and the `p` party-gear
mode can unequip or move pieces between members directly. `e` on a town
tile opens the shop (`render/shop.rs`, `game/shop.rs`); selling only works on
spare (unequipped) gear. Shop stock is gated by `World.current_chapter`
(threaded into `shop_item_stock`/`shop_weapon_stock`/`shop_armor_stock`/
`shop_ring_stock`): Chapter One is Common→Rare only, and from Chapter Two
onward it additionally carries one dedicated shop-exclusive Epic
weapon/armor/ring (`coinwrought_blade`/`coinwrought_plate`/
`merchants_blessing` — deliberately not reused from `loot_profile` or the
treasure-tile roll) plus `sovereign_elixir`. Legendary always stays
boss-exclusive, at every chapter. In combat, "Item" opens a submenu
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

Unit tests live alongside the modules they cover (`app.rs` and most of
`game/*.rs`: `blacksmith`, `chapter`, `character`, `combat`, `inventory_ui`,
`item`, `map`, `npc`, `party`, `quest`, `save`, `shop`, `state`, `status`) and
exercise game logic directly (turn order, damage, weapon/rarity math, loot
and shop pricing, boss scripted moves, inventory/shop input handling,
save/load round-tripping) without needing a window. `render/*.rs` has no
automated tests.

**If you need to verify rendering itself**: macroquad opens a real OS
window, so this can't be driven headlessly like a terminal UI — there's no
PTY/ANSI stream to capture. Run `cargo run` and check visually (or take a
screenshot of the window) after a change to `render/*.rs` or `input.rs`.

## Save/load

Three save slots (`game/save.rs::SAVE_SLOTS`), each its own JSON file in the
working directory (`bashborne_save_1.json`/`_2.json`/`_3.json`,
`save::save_path(slot)`). Slot 3 is labeled "(Dev)" on the main menu by pure
UI convention (`render/main_menu.rs`) — mechanically identical to slots 1-2,
just an obvious sandbox to playtest in without touching real progress. A
one-time migration (`save::migrate_legacy_save`, run on any read of slot 1)
renames a pre-3-slot `bashborne_save.json`, if found, into slot 1 rather
than orphaning it.

The game never auto-loads: it always boots to the main menu
(`World::at_main_menu`), which loads all three slots up front
(`save::read_all_slots`) into `MainMenuState.slots` for the picker. Up/Down
move across the 3 slot rows plus a Quit row (`MAIN_MENU_ROWS`); Enter on a
slot loads it if occupied or starts a fresh game there if empty, either way
setting `World.active_slot`; `n` forces "New Game" on the highlighted slot,
arming `confirm_overwrite` (now `Option<u8>`, the slot awaiting
confirmation) if it's occupied. Shift+S while exploring writes back to
whichever slot `World.active_slot` holds via `game/save.rs::write`. Plain
`s`/Down-arrow both move the player south — `Input::poll` only lets the
physical S key fall through to `Key::Char('S')` (the save shortcut `app.rs`
checks for) when Shift is held; otherwise it's folded into `Key::Down`
alongside the arrow key and `w`/`a`/`d`. Deleting a slot's save file is how
a player makes that slot's row show "New Game" again. Only exploration
state persists (party, inventory, chapter/boss/NPC/quest progress,
position); combat and menus are never saved. `SaveData.version` gates
loading: any mismatch or parse failure silently falls back to no save being
offered for that slot.

## Known stubs / deliberately unfinished seams

- The playable-class roster (Warrior/Mage/Cleric/Rogue) is fixed at game
  start; there's no party selection or recruitment.

## Platform notes

The game is cross-platform by construction — macroquad itself targets
Windows/macOS/Linux (and web/mobile) uniformly, nothing in the repo has
platform `#[cfg]`s, and `game/save.rs`'s save path is a plain,
separator-agnostic `PathBuf`. `cargo run`/`cargo build` are identical on all
desktop platforms.
