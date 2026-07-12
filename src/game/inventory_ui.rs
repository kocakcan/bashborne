use crate::game::character::{Character, RingSlot};
use crate::game::map::Position;
use crate::game::party::Party;

/// Which list the inventory screen is currently showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryTab {
    Items,
    Weapons,
    Armor,
    Rings,
    /// Fungible/non-equippable resources — currently just Titanite Shards,
    /// the blacksmith's upgrade material. A natural home for quest items too,
    /// once those exist; browsing-only, nothing here is equipped on a member.
    Materials,
}

impl InventoryTab {
    pub fn next(self) -> Self {
        match self {
            InventoryTab::Items => InventoryTab::Weapons,
            InventoryTab::Weapons => InventoryTab::Armor,
            InventoryTab::Armor => InventoryTab::Rings,
            InventoryTab::Rings => InventoryTab::Materials,
            InventoryTab::Materials => InventoryTab::Items,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            InventoryTab::Items => InventoryTab::Materials,
            InventoryTab::Weapons => InventoryTab::Items,
            InventoryTab::Armor => InventoryTab::Weapons,
            InventoryTab::Rings => InventoryTab::Armor,
            InventoryTab::Materials => InventoryTab::Rings,
        }
    }
}

/// Which equip slot on a character is being addressed — the weapon slot,
/// the armor slot, or one of the two ring slots. Used by the interactive
/// party-gear view to move/unequip gear directly between characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipSlot {
    Weapon,
    Armor,
    Ring(RingSlot),
}

/// The fixed, ordered set of a character's equip slots, cycled through by
/// the party-gear view's left/right navigation.
pub const EQUIP_SLOTS: [EquipSlot; 4] = [
    EquipSlot::Weapon,
    EquipSlot::Armor,
    EquipSlot::Ring(RingSlot::First),
    EquipSlot::Ring(RingSlot::Second),
];

/// What the inventory screen is currently doing.
#[derive(Debug, Clone, Copy)]
pub enum InventoryMode {
    /// Browsing the active tab's bag list.
    Browsing,
    /// Picking which party member should receive the highlighted item/weapon/armor.
    SelectMember {
        tab: InventoryTab,
        /// Index into the bag list for `tab`.
        idx: usize,
        member_cursor: usize,
    },
    /// Picking which of `member_idx`'s two ring slots receives the ring at `idx`.
    SelectRingSlot {
        idx: usize,
        member_idx: usize,
        slot_cursor: usize,
    },
    /// Browsing the party's currently-equipped gear directly, cell by cell —
    /// the entry point for moving or unequipping gear without going through
    /// the shared bag.
    PartyGear {
        member_cursor: usize,
        slot_cursor: usize,
    },
    /// Choosing what to do with `member_idx`'s `slot`: unequip it to the bag,
    /// or move it to another party member.
    PartyGearAction {
        member_idx: usize,
        slot: EquipSlot,
        action_cursor: usize,
    },
    /// Picking which other party member should receive `from_member`'s `slot`.
    PartyGearTarget {
        from_member: usize,
        slot: EquipSlot,
        to_cursor: usize,
    },
    /// Browsing the active roster to pick who to bench, for the recruit
    /// swap screen (`r` from `Browsing`). A no-op mode if the bench is
    /// empty — there's nothing to swap in.
    Roster { active_cursor: usize },
    /// Picking which benched recruit replaces `active_idx` once `Roster`
    /// has picked an active member to swap out.
    RosterTarget { active_idx: usize, bench_cursor: usize },
}

/// What moving one member's equipped piece onto another member would do to
/// the receiver's totals — computed ahead of time so the "Move to..." picker
/// can show each candidate's before/after ATK/DEF instead of bare names.
pub struct MovePreview {
    /// Display name of the piece being moved (includes upgrade suffix for weapons).
    pub piece_name: String,
    /// Name of the member the piece is coming from.
    pub from_name: String,
    /// Receiver's (current, after-move) total attack.
    pub atk: (i32, i32),
    /// Receiver's (current, after-move) total defense.
    pub def: (i32, i32),
}

fn slot_bonuses(member: &Character, slot: EquipSlot) -> Option<(String, i32, i32)> {
    match slot {
        EquipSlot::Weapon => member
            .equipped_weapon
            .as_ref()
            .map(|w| (w.display_name(), w.attack_bonus, w.defense_bonus)),
        EquipSlot::Armor => member
            .equipped_armor
            .as_ref()
            .map(|a| (a.name.clone(), 0, a.defense_bonus)),
        EquipSlot::Ring(rs) => {
            let idx = match rs {
                RingSlot::First => 0,
                RingSlot::Second => 1,
            };
            member.equipped_rings[idx]
                .as_ref()
                .map(|r| (r.name.clone(), r.attack_bonus, r.defense_bonus))
        }
    }
}

/// Previews `move_gear_between_members` for the target picker: what
/// `to_member`'s totals become if `from_member`'s `slot` lands on them
/// (displacing whatever they had in the same slot). Totals are linear sums
/// of stat + gear bonuses, so this is pure bonus arithmetic — no cloning or
/// trial equips. `None` if the source slot is empty or an index is bad.
pub fn move_preview(
    party: &Party,
    from_member: usize,
    to_member: usize,
    slot: EquipSlot,
) -> Option<MovePreview> {
    let from = party.members.get(from_member)?;
    let to = party.members.get(to_member)?;
    let (piece_name, moved_atk, moved_def) = slot_bonuses(from, slot)?;
    let (_, cur_slot_atk, cur_slot_def) = slot_bonuses(to, slot).unwrap_or((String::new(), 0, 0));
    let cur_atk = to.total_attack();
    let cur_def = to.total_defense();
    Some(MovePreview {
        piece_name,
        from_name: from.name.clone(),
        atk: (cur_atk, cur_atk - cur_slot_atk + moved_atk),
        def: (cur_def, cur_def - cur_slot_def + moved_def),
    })
}

/// Out-of-combat inventory/equipment screen. Lets the player quaff potions,
/// equip weapons/armor/rings found in the world or won from enemies onto
/// any party member, and move or unequip gear directly between party
/// members, entirely outside of battle.
pub struct InventoryUiState {
    pub tab: InventoryTab,
    pub cursor: usize,
    pub mode: InventoryMode,
    /// Last action's result, shown in the footer until the next action replaces it.
    pub message: Option<String>,
    /// Where to place the player back on the map once this screen is closed.
    pub return_pos: Position,
}

impl InventoryUiState {
    pub fn new(return_pos: Position) -> Self {
        Self {
            tab: InventoryTab::Items,
            cursor: 0,
            mode: InventoryMode::Browsing,
            message: None,
            return_pos,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::character::{rogue, warrior};
    use crate::game::item::{copper_band, iron_sword, padded_vest, iron_loop};

    fn two_member_party() -> Party {
        Party::new(vec![warrior("Bram"), rogue("Wren")])
    }

    #[test]
    fn weapon_move_preview_reports_both_totals_shifting() {
        let mut party = two_member_party();
        party.members[0].equip_weapon(iron_sword());
        let moved = iron_sword();
        let target = &party.members[1];
        let old_weapon_atk = target.equipped_weapon.as_ref().map(|w| w.attack_bonus).unwrap_or(0);
        let old_weapon_def = target.equipped_weapon.as_ref().map(|w| w.defense_bonus).unwrap_or(0);
        let (cur_atk, cur_def) = (target.total_attack(), target.total_defense());

        let p = move_preview(&party, 0, 1, EquipSlot::Weapon).expect("source weapon exists");
        assert_eq!(p.piece_name, moved.display_name());
        assert_eq!(p.from_name, "Bram");
        assert_eq!(p.atk, (cur_atk, cur_atk - old_weapon_atk + moved.attack_bonus));
        assert_eq!(p.def, (cur_def, cur_def - old_weapon_def + moved.defense_bonus));
    }

    #[test]
    fn move_into_empty_slot_adds_the_full_bonus() {
        let mut party = two_member_party();
        party.members[0].equip_armor(padded_vest());
        assert!(party.members[1].equipped_armor.is_none());
        let (cur_atk, cur_def) = (party.members[1].total_attack(), party.members[1].total_defense());

        let p = move_preview(&party, 0, 1, EquipSlot::Armor).expect("source armor exists");
        assert_eq!(p.atk, (cur_atk, cur_atk));
        assert_eq!(p.def, (cur_def, cur_def + padded_vest().defense_bonus));
    }

    #[test]
    fn ring_move_diffs_against_the_specific_ring_slot() {
        let mut party = two_member_party();
        party.members[0].equip_ring(RingSlot::Second, copper_band());
        // Target wears a different ring in each slot; only Second should be displaced.
        party.members[1].equip_ring(RingSlot::First, copper_band());
        party.members[1].equip_ring(RingSlot::Second, iron_loop());
        let (cur_atk, cur_def) = (party.members[1].total_attack(), party.members[1].total_defense());

        let p = move_preview(&party, 0, 1, EquipSlot::Ring(RingSlot::Second)).expect("source ring exists");
        let displaced = iron_loop();
        let moved = copper_band();
        assert_eq!(p.atk, (cur_atk, cur_atk - displaced.attack_bonus + moved.attack_bonus));
        assert_eq!(p.def, (cur_def, cur_def - displaced.defense_bonus + moved.defense_bonus));
    }

    #[test]
    fn empty_source_slot_yields_no_preview() {
        let party = two_member_party();
        assert!(party.members[0].equipped_armor.is_none());
        assert!(move_preview(&party, 0, 1, EquipSlot::Armor).is_none());
        assert!(move_preview(&party, 0, 1, EquipSlot::Ring(RingSlot::First)).is_none());
        assert!(move_preview(&party, 9, 1, EquipSlot::Weapon).is_none());
    }

    #[test]
    fn tabs_cycle_forward_through_all_five_and_back_to_items() {
        let mut tab = InventoryTab::Items;
        tab = tab.next();
        assert_eq!(tab, InventoryTab::Weapons);
        tab = tab.next();
        assert_eq!(tab, InventoryTab::Armor);
        tab = tab.next();
        assert_eq!(tab, InventoryTab::Rings);
        tab = tab.next();
        assert_eq!(tab, InventoryTab::Materials);
        tab = tab.next();
        assert_eq!(tab, InventoryTab::Items);
    }

    #[test]
    fn tabs_cycle_backward_symmetrically() {
        let mut tab = InventoryTab::Items;
        tab = tab.prev();
        assert_eq!(tab, InventoryTab::Materials);
        tab = tab.prev();
        assert_eq!(tab, InventoryTab::Rings);
        tab = tab.prev();
        assert_eq!(tab, InventoryTab::Armor);
        tab = tab.prev();
        assert_eq!(tab, InventoryTab::Weapons);
        tab = tab.prev();
        assert_eq!(tab, InventoryTab::Items);
    }
}
