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
- **Elite unique mechanics** — elites (`Character::apply_elite`) currently
  just get bigger stats (1.5x HP/ATK, 1.3x DEF). Giving them a guaranteed
  shot at their species' signature move (Wraith curse, Orc Reckless Swing,
  etc.) would make the elite roll feel distinct rather than just
  numerically bigger.

## Moderate, touches a few systems

- **Respec system** — a "Purging Stone"-style consumable refunding
  `unspent_points` (not automatic `level_growth`), fitting the existing
  `game/item.rs` consumable pattern. Pairs naturally with the hard-cap
  removal already shipped, since points are cheap to reallocate once earned.
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

- **NG+ scaling omits speed/luck** — `Character::apply_ng_plus` only
  multiplies max_hp/attack/defense (its own test asserts speed/luck are
  left untouched). At NG+7 monsters hit harder and survive longer but never
  act first or crit more. Worth revisiting now that the leveling numbers
  have changed with the hard-cap removal.
- **Chapter-gated shop/loot tiers** — shop stock (`game/shop.rs`) is flat
  Common→Rare across all 3 chapters. Unlocking Epic-tier goods later in the
  story, or NG+-gated loot tables, would give replay more to chase beyond
  bigger stat numbers.

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

## Docs cleanup

- `CLAUDE.md`'s "Known stubs / deliberately unfinished seams" section is
  stale: it claims `CombatPhase::Resolving` and `World.anim_timer` are
  unused/dead code. They're not — `Resolving` is wired via
  `app.rs::begin_resolving_hold`/`RESOLVING_HOLD_SECONDS`, and `anim_timer`
  drives real idle-bob/lunge/flash sprite animation in `render/combat.rs`.
  Worth a quick correction independent of any item above.
