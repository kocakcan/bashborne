use crate::game::character::RingSlot;
use crate::game::map::Position;

/// Which list the inventory screen is currently showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryTab {
    Items,
    Weapons,
    Armor,
    Rings,
}

impl InventoryTab {
    pub fn next(self) -> Self {
        match self {
            InventoryTab::Items => InventoryTab::Weapons,
            InventoryTab::Weapons => InventoryTab::Armor,
            InventoryTab::Armor => InventoryTab::Rings,
            InventoryTab::Rings => InventoryTab::Items,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            InventoryTab::Items => InventoryTab::Rings,
            InventoryTab::Weapons => InventoryTab::Items,
            InventoryTab::Armor => InventoryTab::Weapons,
            InventoryTab::Rings => InventoryTab::Armor,
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

    #[test]
    fn tabs_cycle_forward_through_all_four_and_back_to_items() {
        let mut tab = InventoryTab::Items;
        tab = tab.next();
        assert_eq!(tab, InventoryTab::Weapons);
        tab = tab.next();
        assert_eq!(tab, InventoryTab::Armor);
        tab = tab.next();
        assert_eq!(tab, InventoryTab::Rings);
        tab = tab.next();
        assert_eq!(tab, InventoryTab::Items);
    }

    #[test]
    fn tabs_cycle_backward_symmetrically() {
        let mut tab = InventoryTab::Items;
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
