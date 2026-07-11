# Future Plans

A backlog of next-improvement ideas for Bashborne, gathered while auditing
the leveling system and QoL pass (see git history around the hard-cap
removal). Nothing here is scheduled — it's a reference for prioritizing
future work, roughly ordered cheapest/highest-value first within each tier.

## Cheap, mostly content (no new plumbing needed)

- **2nd/3rd ability per class** — `AbilityKind`/`Vec<Ability>`
  (`src/game/character.rs`) already support more than one ability; nothing
  in combat resolution assumes a single ability per class. Probably the
  best next investment for the effort involved.

## Moderate, touches a few systems

- **Full respec system** — the level-up screen now supports Backspace-undo
  within a single visit to the screen (`LevelUpUiState::history`,
  `Character::deallocate_point`), which covers "I picked the wrong stat
  just now." A persistent, any-time respec — refunding *all* banked points
  on a member regardless of when they were spent, e.g. via a consumable —
  is still a distinct, unimplemented feature.
- **Difficulty setting at game start** — `Character::apply_ng_plus` already
  takes a `u32` cycle multiplier; a main-menu difficulty pick could reuse
  that math as a starting offset instead of always beginning at NG+0.
- **Bestiary/codex screen** — all 16 species + elites + 3 bosses already
  have loot/sprite/move data keyed by name (`combat::loot_profile`,
  `render::combat::species_color`, etc.); a codex screen could mostly just
  enumerate the existing tables rather than needing new content.
- **Chapter/boss #4** — mechanical but not free: a new `Tile` map layout,
  a new `BossKind` variant, scripted moves added to
  `combat::resolve_boss_move`, and a tied NPC/quest.

## Balance-only, no new content

- **NG+-gated loot tables** — shop stock (`game/shop.rs`) now unlocks a
  dedicated Epic weapon/armor/ring plus `sovereign_elixir` from Chapter Two
  onward (see git history), but `combat::loot_profile`'s field-drop odds
  don't scale with NG+ cycle at all. Higher-NG+ drop chances (or a
  higher-still tier gated behind a specific NG+ cycle) would give replay
  more to chase beyond bigger stat numbers.

## Bigger lifts

- **Multi-stage quests** — `QuestObjective` (`game/quest.rs`) is
  deliberately single-stage per its own doc comment; a `KillCount`/
  `ReachChapter` variant is a cheap extension, but true multi-stage quests
  are a bigger structural change.
- **Recruitable/customizable roster** — the fixed 4-character party is a
  documented stub (see `CLAUDE.md`'s "Known stubs" section);
  `Party.members: Vec<Character>` doesn't fight it, but this is the biggest
  architectural lift on this list (main-menu flow, save format, UI).
- **Audio** — nothing exists yet: no sound/music assets, no
  `macroquad::audio` usage anywhere in `src/`. Bounded scope but needs new
  assets and a check that the audio feature exists at the pinned
  `macroquad = "=0.4.15"`.
- **Controller support** — lowest priority; `input.rs` only wraps keyboard
  polling, and macroquad's own gamepad support is thin, so this would
  likely mean pulling in `gilrs`.
