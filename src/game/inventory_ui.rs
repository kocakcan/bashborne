use crate::game::map::Position;

/// Which list the inventory screen is currently showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryTab {
    Items,
    Weapons,
}

impl InventoryTab {
    pub fn toggled(self) -> Self {
        match self {
            InventoryTab::Items => InventoryTab::Weapons,
            InventoryTab::Weapons => InventoryTab::Items,
        }
    }
}

/// What the inventory screen is currently doing: browsing the active tab's
/// list, or picking which party member should receive the highlighted
/// item/weapon.
#[derive(Debug, Clone, Copy)]
pub enum InventoryMode {
    Browsing,
    SelectMember {
        tab: InventoryTab,
        /// Index into `inventory.items` or `inventory.weapons`, depending on `tab`.
        idx: usize,
        member_cursor: usize,
    },
}

/// Out-of-combat inventory/equipment screen. Lets the player quaff potions
/// and equip weapons found in the world or won from enemies onto any party
/// member, entirely outside of battle.
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
