# Future Plans

A backlog of next-improvement ideas for Bashborne. Nothing here is
scheduled — it's a reference for prioritizing future work. Items are struck
once shipped rather than deleted, so this file stays a record of what's
already been decided rather than just what's left.

## Done

- **2nd/3rd ability per class** — every playable class now has a 3-ability
  kit (`class_abilities`, `character.rs`).
- **Difficulty setting at game start** — a main-menu difficulty pick reuses
  `Character::apply_ng_plus`'s cycle math as a starting offset.
- **Bestiary/codex screen** — `b` from Explore opens a read-only codex
  enumerating every species/boss (`bestiary_ui.rs`, `render/bestiary.rs`).
- **Chapter/boss #4** — "The Drowned Cathedral," guarded by the Drowned
  King, is now the game's final chapter (`chapter.rs`, `map.rs::chapter_four`,
  `combat::resolve_boss_move`'s `BossKind::DrownedKing` arm). Chapter Three's
  Ashen Sovereign is no longer the last fight.
- **NG+-gated loot tables** — shop stock unlocks a dedicated Epic
  weapon/armor/ring plus `sovereign_elixir` from Chapter Two onward
  (`game/shop.rs`).
- **Full respec system** — the "Rite of Undoing" consumable
  (`item.rs::rite_of_undoing`, sold from Chapter Two onward) calls
  `Character::full_respec`, refunding every hand-spent level-up point
  regardless of when it was spent — distinct from the level-up screen's
  existing single-visit Backspace-undo.
- **Multi-stage quests** — `QuestObjective::KillCount` (`quest.rs`) tracks
  cumulative kills via `QuestLog::record_kill`; the Exiled Knight's quest in
  Chapter Four uses it. True multi-step quest chains (several sequential
  objectives per quest) remain out of scope.
- **Recruitable roster** — the Wounded Scout, Ashen Pilgrim, and Exiled
  Knight can be recruited onto a bench (`Party::bench`) the moment their
  quest is turned in, and swapped into the active 4-member roster from the
  inventory screen's `r` Roster mode (`Party::swap`). The active combat
  party stays capped at 4 (the combat party panel's layout is hardcoded to
  fit exactly that many rows); the bench itself is uncapped.

## Remaining

- **Audio** — nothing exists yet: no sound/music assets, no
  `macroquad::audio` usage anywhere in `src/`. Bounded scope but needs new
  assets and a check that the audio feature exists at the pinned
  `macroquad = "=0.4.15"`.
- **Controller support** — lowest priority; `input.rs` only wraps keyboard
  polling, and macroquad's own gamepad support is thin, so this would
  likely mean pulling in `gilrs`.
