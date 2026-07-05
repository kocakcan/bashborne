use ratatui::style::Color;

use crate::game::quest::QuestId;

/// Identifies a specific NPC. Each variant has a fixed registry entry
/// (name, dialogue, and the quest it offers, if any) in `npc_def` — a
/// plain enum + exhaustive match, the same pattern the rest of this
/// codebase uses instead of string-keyed lookups (see how fragile the
/// boss's name-string checks are).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NpcId {
    OldHerbalist,
    WoundedScout,
    AshenPilgrim,
}

/// Static data describing an NPC. Dialogue is plain linear text — no
/// branching — picked by a flat match on quest state:
/// - `intro`: the first time this NPC is ever talked to (also when its
///   quest, if any, is offered).
/// - `reminder`: later visits while its quest (if any) is still active and
///   unsatisfied — or just "what it repeats," for a quest-less NPC.
/// - `turn_in`: shown once, on the visit where its quest's objective
///   becomes satisfied (the reward is granted at the same time).
/// - `after`: later visits once its quest is completed.
pub struct NpcDef {
    pub name: &'static str,
    pub intro: Vec<&'static str>,
    pub reminder: Vec<&'static str>,
    pub turn_in: Vec<&'static str>,
    pub after: Vec<&'static str>,
    pub quest: Option<QuestId>,
}

pub fn npc_def(id: NpcId) -> NpcDef {
    match id {
        NpcId::OldHerbalist => NpcDef {
            name: "Old Herbalist",
            intro: vec![
                "The old herbalist looks up from her satchel of dried leaves.",
                "\"Travelers, is it? Mind the tall grass — it isn't only grass.\"",
                "\"Say — if you're carrying a spare Potion, I'd pay well for one.\"",
            ],
            reminder: vec!["\"Still hoping for that Potion, if you can spare it.\""],
            turn_in: vec![
                "\"Ah, a Potion! Just what these old bones needed.\"",
                "\"Here — take this for your trouble.\"",
            ],
            after: vec!["\"Safe travels out there.\""],
            quest: Some(QuestId::HerbalistsRequest),
        },
        NpcId::WoundedScout => NpcDef {
            name: "Wounded Scout",
            intro: vec![
                "A scout leans against a broken signpost, one arm in a sling.",
                "\"You're the ones who cleared the barrow? I owe you for that —",
                " that road was murder while the Knight still stalked it.\"",
            ],
            reminder: vec!["\"Still can't believe you took down the Barrow Knight.\""],
            turn_in: vec![
                "\"Word of the Knight's fall reached us fast. Well earned.\"",
                "\"Take this — least I can do.\"",
            ],
            after: vec!["\"Mind yourself in the marsh. The Warden doesn't share the road's manners.\""],
            quest: Some(QuestId::ScoutsCommendation),
        },
        NpcId::AshenPilgrim => NpcDef {
            name: "Ashen Pilgrim",
            intro: vec![
                "A robed pilgrim kneels at the edge of the ash-fall, murmuring.",
                "\"So the marsh finally yielded. The Warden's scales don't rot quietly.\"",
                "\"You've the look of those who mean to finish the climb.\"",
            ],
            reminder: vec!["\"The Sovereign's throne is close now. Closer than any of us have come.\""],
            turn_in: vec![
                "\"Then it's true — the Warden is silenced.\"",
                "\"Carry this. You'll need every edge you can get, further up.\"",
            ],
            after: vec!["\"Go carefully. Ash remembers who disturbs it.\""],
            quest: Some(QuestId::PilgrimsBlessing),
        },
    }
}

/// The glyph drawn on the map for this NPC, distinct from every `Tile`
/// glyph and from the player's `@` marker.
pub fn glyph_for(id: NpcId) -> char {
    match id {
        NpcId::OldHerbalist => 'h',
        NpcId::WoundedScout => 's',
        NpcId::AshenPilgrim => 'p',
    }
}

pub fn color_for(id: NpcId) -> Color {
    match id {
        NpcId::OldHerbalist => Color::LightYellow,
        NpcId::WoundedScout => Color::LightCyan,
        NpcId::AshenPilgrim => Color::Gray,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_npc_has_non_empty_dialogue_for_every_state() {
        for id in [NpcId::OldHerbalist, NpcId::WoundedScout, NpcId::AshenPilgrim] {
            let def = npc_def(id);
            assert!(!def.name.is_empty());
            assert!(!def.intro.is_empty());
            assert!(!def.reminder.is_empty());
            if def.quest.is_some() {
                assert!(!def.turn_in.is_empty());
                assert!(!def.after.is_empty());
            }
        }
    }
}
