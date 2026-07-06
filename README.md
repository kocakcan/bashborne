# Bashborne — terminal turn-based RPG (Rust + ratatui)

A Dark Souls–flavored, turn-based RPG that runs entirely in your terminal.

## Running it

```
cargo run
```

Requires a real terminal (not piped stdin/stdout).

**Platform notes:** the game is cross-platform by construction — no manual
ANSI, no platform-specific code anywhere in the repo, and the save file path
is separator-agnostic. On Windows, use Windows Terminal (or another
UTF-8-locale terminal); legacy `conhost`/plain `cmd.exe` may mis-render the
box-drawing HP bars and ASCII-art title (run `chcp 65001` first if you must
use one). Either platform needs an 80x24 terminal at minimum.

**Note on dependency versions:** `Cargo.toml` pins `ratatui = "=0.26.3"` and
`crossterm = "=0.27.0"` because this was built in a sandbox stuck on rustc
1.75. If you're on a normal, up-to-date toolchain (`rustup update`), feel
free to loosen those to `"0.28"` / `"0.28"` (or run `cargo update`) to get
the latest features — nothing here depends on anything version-specific.

## Controls

**Exploring:** arrow keys / WASD to move, `i` for inventory, `q` to quit.
Step into the `,` tall-grass tiles enough times and you'll trigger a field
event — usually a fight, but sometimes a blessing, a curse, or treasure
(occasionally a trap). Standing on a `T` (town) tile and pressing `e` opens
the shop. The lone red `B` tile is the boss lair — walking onto it always
starts a fight, once, until you win.

**Combat:** `↑↓` to move the menu cursor, `Enter` to confirm. Picking
"Ability" opens a submenu of that character's abilities (with MP cost shown,
grayed out if you can't afford it); `Esc` there goes back to the main menu.
After picking an action, `←→` cycles the target, `Enter` confirms, `Esc`
backs out (to the ability submenu if that's where you came from). `Enter`
again to dismiss the victory/defeat/flee screen.

**Inventory (`i` from Explore):** `Tab` / `←→` switches between the Items
and Weapons tabs, `↑↓` selects, `Enter` opens a party-member picker to use
the item or equip the weapon (swapping out whatever that member had on),
`Esc` backs out one level at a time.

**Shop (`e` on a town tile):** `←→` toggles Buy/Sell, `Tab` toggles
Items/Weapons, `↑↓` selects, `Enter` buys or sells, `Esc` leaves. The shop's
buy stock tops out at Rare — Epic and Legendary gear can only be found or
won, never bought.

**Blessing / Curse / Treasure screens:** `Enter` to continue back to
exploring, right back where you left off.

## Architecture

```
src/
  main.rs             — terminal setup/teardown (raw mode, alt screen, panic hook), run loop
  event.rs            — thin crossterm key-poll wrapper
  app.rs              — World (Party + Inventory + GameState + rng), all input handling
  ui/
    mod.rs            — dispatches to the right screen based on GameState; shared HP-bar/rarity-color helpers
    explore.rs        — map viewport + party panel (incl. active effects) + log
    combat.rs         — enemy sprites/HP bars + party panel + action/ability menu + log
    event.rs          — blessing/curse/treasure narrative screen
    inventory.rs      — out-of-combat inventory screen (equip weapons, use items)
    shop.rs           — town shop screen (buy/sell)
  game/
    character.rs      — Stats, Class, Character (incl. equipped_weapon), Ability, full roster + boss factory fns
    party.rs           — Party (Vec<Character> + gold + active status effects)
    item.rs             — Item, Rarity, Weapon, Inventory, item/weapon factory functions
    inventory_ui.rs      — InventoryUiState/Tab/Mode backing the out-of-combat inventory screen
    shop.rs               — fixed buy stock, ShopUiState, buy/sell pricing
    map.rs                 — Tile, Map, Position (hand-authored ASCII layout, incl. the boss lair)
    combat.rs               — CombatState: turn order, action resolution, loot, boss AI, win/lose detection
    sprites.rs               — ASCII art + per-species color for the combat screen
    status.rs                 — StatusEffect: buffs/curses that persist across encounters
    state.rs                   — GameState enum (Explore | Combat | Event | Inventory | Shop | GameOver), field-event table
```

**State machine:** `GameState` is a plain enum (`Explore | Combat | Event |
Inventory | Shop | GameOver`) rather than `Box<dyn Screen>` trait objects.
For a fixed, known set of screens this gives exhaustive `match` checking at
compile time with no dynamic dispatch. If you later want plugin-style or
scripted screens, swap in a trait-object `Screen` trait — the rest of the
architecture doesn't need to change to support that. `CombatPhase` is the
same idea one level down: `SelectAction → SelectAbility → SelectTarget →
{Victory, Defeat, Fled}`, all exhaustively matched wherever it's read.

**Turn order:** `CombatState::new` builds a `Vec<ActorRef>` (player/enemy
indices) sorted by speed once at combat start. `resolve_current_turn` handles
both player-selected and AI-driven turns, then `advance_turn` walks the
cursor forward, skipping dead actors, and checks win/lose conditions.

**Monster roster:** sixteen regular species (`character.rs`), each with a
distinct stat profile (Bat is fast/fragile, Skeleton is slow/tanky, Orc hits
hard, etc), plus a chance per encounter of an Elite variant (tougher,
better-paying). `state.rs`'s `roll_encounter` picks from twenty hand-tuned
compositions. Several species have signature moves in `resolve_enemy_action`
— the **Wraith** curses the party 30% of turns instead of attacking, and
others (Goblin/Wolf targeting, Skeleton/Orc/Bandit/Barrow Sentinel/Forsaken
Knight scripted moves) add more variety. The **Mimic** never appears in the
normal encounter table; it's a 1-in-5 chance hiding inside what looked like
a treasure find (see Field events below), with loot to match the scare.

**The boss:** the **Barrow Knight** (also in `character.rs`) is the one
hand-placed encounter, reachable via a fixed `Tile::BossLair` tile rather
than a random roll. It clearly outclasses every regular enemy on raw stats,
and has two scripted moves living alongside the Wraith's curse in
`resolve_enemy_action`: **Rending Cleave** (a ~1.8x damage swing, ~35% of
turns) and a one-time **Second Wind** (heals 20% max HP and permanently adds
+6 attack the first time it drops to ≤30% HP). Beating it is the only way to
get **Knightsbane**, the strongest weapon in the game — that drop is
guaranteed, not rolled, since the fight itself is the gate. `World.
boss_defeated` keeps the lair from re-triggering afterward.

**Weapons & rarity:** every playable character always has exactly one
`equipped_weapon` (see `Character::total_attack`/`total_defense`, which fold
its bonuses into combat math); monsters leave it `None`. `Rarity` (`item.rs`)
is a five-tier enum — Common < Uncommon < Rare < Epic < Legendary — with a
derived `Ord` so "rarer" is always literally "stronger," and a `base_value()`
used for both shop pricing and loot value. Weapons are found in the world
(rare field-treasure rolls), won from specific enemies (per-species drop
chances in `combat::loot_profile`), bought from the shop (Common/Uncommon/
Rare only), or earned from the boss (Legendary, guaranteed).

**Inventory & shop screens:** `i` from Explore opens the out-of-combat
inventory (`ui/inventory.rs`, state in `game/inventory_ui.rs`) to equip
weapons or use consumables on any party member — displaced weapons return to
the bag rather than vanishing. `e` on a town tile opens the shop
(`ui/shop.rs`, `game/shop.rs`) to spend or recoup gold; selling only works on
spare (unequipped) weapons, so you unequip via the inventory screen first.

**Field events & status effects:** stepping into tall grass doesn't always
mean a fight. `roll_field_event` (in `game/state.rs`) rolls a weighted
outcome — 55% combat, 15% blessing, 15% curse, 15% treasure (which itself has
a small chance of being a Mimic ambush instead, and a smaller chance of
burying a bonus weapon). Blessings and curses (`game/status.rs`) are
party-wide stat deltas (`StatusEffect { target, delta, encounters_remaining
}`) that persist for a fixed number of *encounters*, not turns —
`Party::tick_effects()` counts one down each time a combat encounter
concludes (win or flee), and `CombatState` reads `Party::stat_delta(target)`
when computing attack, defense, and turn-order speed so buffs/curses actually
move the numbers rather than just being cosmetic. Stacking the same
named effect merges it (magnitude adds, duration refreshes to the longer of
the two) instead of cluttering the list with duplicate entries. Active
effects are listed in both the explore and combat party panels. Field
events (and combat) remember the tile you stepped on (`return_pos`) so
dismissing a blessing/curse/treasure notice — or finishing a fight — drops
you back exactly where you were, not back at the map's spawn point.

**Loot:** `CombatState::loot` is rolled once, when victory is detected, via
`roll_loot` in `game/combat.rs`, which looks up a per-species gold range and
optional item/weapon-drop chance from `loot_profile`. `World::conclude_combat`
applies it to `Party::gold` / `Inventory` and shows a summary both on the
victory screen and as the opening log line back in exploration.

**Testing:** unit tests live alongside the modules they cover (`combat.rs`,
`character.rs`, `item.rs`, `map.rs`, `shop.rs`, `app.rs`) and exercise the
game logic directly — turn order, damage, the weapon/rarity math, loot and
shop pricing, the boss's scripted moves, and the inventory/shop input
handling — without needing a terminal. Run with `cargo test`. UI rendering
code is thin enough to eyeball (though see the note below on how it was
actually verified interactively).

**A note on testing the TUI itself:** naive ANSI-stripping of the terminal
output stream doesn't work for verifying screen contents — ratatui only
retransmits *changed* cells with explicit cursor-positioning escape codes,
so stripped-and-concatenated text from multiple frames comes out jumbled.
This was actually caught mid-session (looked like "movement isn't working"
when it was really "movement is working, the test's screen-reading isn't").
The fix was feeding the real PTY output through a proper terminal emulator
(the `pyte` Python package) to get an accurate reconstructed screen buffer
before asserting against it — worth knowing if you want to script your own
interactive tests against this UI.

## What's here vs. what's a stub

Working: exploration with collision, weighted random field events (combat /
blessing / curse / treasure / mimic ambush), full turn-based combat with a
per-class ability submenu (2 abilities each), item / flee, HP/MP tracking,
victory/defeat/flee outcomes, loot (gold + item/weapon drops), a five-tier
weapon rarity system with an equip slot on every character, an out-of-combat
inventory screen, a town shop (buy/sell, gold-gated), a hand-placed boss
fight with two scripted moves and a guaranteed unique-weapon reward,
party-wide status effects that persist across encounters and actually affect
combat math (with stacking), Pokémon-style ASCII sprites and HP bars in
combat, an 8-species regular monster roster, and position-preserving
transitions between exploring, fighting, shopping, and field events.

Deliberately minimal, marked as seams for you to extend:
- **`Class::Rogue`** exists but has no factory function yet.
- **Six blessings, six curses, tier-gated by NG+.** The pools in `status.rs`
  (`roll_blessing`, `roll_curse`) unlock two of their stronger options at
  NG+1/NG+2 — add more `(name, target, delta, min_ng_plus)` entries to widen
  the variety further, or add new `StatEffectTarget` variants (e.g.
  `Evasion`) if you want effects beyond attack/defense/speed.
- **No leveling.** `Character.level` exists but is hardcoded to `1`
  everywhere — there's no XP gain or stat growth yet, so gear is currently
  the only progression axis.
- **No armor/accessory slot.** Only one equipment slot (`equipped_weapon`)
  exists; `defense_bonus` on `Weapon` is the only way to boost defense via
  gear right now.
- **No save/load.** Nothing here is `Serialize`/`Deserialize` yet; adding
  `serde` + `serde_json` and deriving on `Character`/`Party`/`Inventory` is
  the whole job.
- **Single overworld map, single boss.** `Map::starting_area()` is one
  hand-authored layout with one `BossLair` tile; swapping to loading maps
  from text/RON files, or adding a second area gated behind the first boss,
  is a natural next step. Additional bosses would follow the same
  name-checked pattern as the Barrow Knight in `resolve_enemy_action`.
- **`CombatPhase::Resolving`** is unused — reserved for adding a brief delay
  between an action and the log update if you want actions to feel less
  instant (currently everything resolves in the same frame).
- **In-combat item selection is hardcoded to slot 0.** `resolve_pending_target`
  in `app.rs` always uses `self.inventory.items.first()`; a proper item
  submenu (mirroring the ability submenu, or the item tab already built for
  the out-of-combat inventory screen) is a natural next step if you want to
  choose *which* potion/ether to use mid-fight.

## Suggested next steps, roughly in order of payoff

1. A leveling/XP system — the biggest open gap, and it plugs directly into
   stats and combat that already exist.
2. Save/load via `serde`.
3. An item submenu in combat (same pattern as the ability submenu) so you
   can pick which item to use instead of always consuming slot 0.
4. A second area + a door/transition tile, gated behind defeating the Barrow
   Knight.
5. An armor/accessory equipment slot alongside weapons.
