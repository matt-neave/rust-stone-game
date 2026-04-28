//! Foragers economy — resources, purchase events, and the in-world UI
//! for buying upgrades.
//!
//! ## UI shape (gnorp-style)
//!
//! Each in-world structure has a hover-activated **main panel** + a
//! **detail panel** that appears next to it. The main panel holds rows
//! (counts, buttons) separated by horizontal dividers; hovering a row
//! highlights it and surfaces a description in the detail panel.
//!
//! * The **cave** is on the sand from the start. Hovering it shows a
//!   small panel with a single BUY HUT row + a side panel describing
//!   what the hut does.
//! * Once the hut exists, hovering it shows a larger panel with a
//!   buy row per [`PurchaseKind`] in [`HUT_PANEL_KINDS`] and a
//!   re-targeting detail panel beside it.
//!
//! Adding a new buyable role requires:
//!
//! 1. A new variant on [`PurchaseKind`] (+ its `label`, `cost_label`).
//! 2. A new entry in [`HUT_PANEL_KINDS`] (panel auto-grows).
//! 3. A new branch in [`purchase::can_afford`] / [`cost_for`].
//! 4. A new branch in [`interaction::detail_for`].
//! 5. The crew-side spawn handler (`crate::crew::<role>`).
//!
//! No edits to the panel spawn or hover system are required — they
//! drive themselves from the data above.
//!
//! Purchases flow through [`PurchaseEvent`]. The button system emits
//! these on click; `crew` and `structures::hut` consume them.

use bevy::prelude::*;

mod interaction;
mod layout;
mod purchase;
mod relayout;
mod spawn;

pub use purchase::{
    PurchaseEvent, PurchaseKind, CAVE_PANEL_KINDS, HUT_BEACHCOMBER_KINDS, HUT_FISHER_KINDS,
    HUT_MINER_KINDS, HUT_PANEL_KINDS, HUT_SKIMMER_KINDS, HUT_STONEMASON_KINDS, PIER_PANEL_KINDS,
    PORT_PANEL_KINDS,
};

// =====================================================================
// Resources
// =====================================================================

#[derive(Resource, Default, Debug)]
pub struct Hut {
    pub owned: bool,
}

/// Miner hut — second cave purchase, gates the skimmer + angler huts.
#[derive(Resource, Default, Debug)]
pub struct MinerHut {
    pub owned: bool,
}

/// Skimmer hut — gated behind the miner hut.
#[derive(Resource, Default, Debug)]
pub struct SkimmerHut {
    pub owned: bool,
}

/// Anglers hut — gated behind the miner hut.
#[derive(Resource, Default, Debug)]
pub struct FisherHut {
    pub owned: bool,
}

/// Beachcombers hut — gated behind foragers hut. Sells beachcomber
/// conversions.
#[derive(Resource, Default, Debug)]
pub struct BeachcomberHut {
    pub owned: bool,
}

/// Stonemasons hut — gated behind miner hut. Sells stonemason
/// conversions.
#[derive(Resource, Default, Debug)]
pub struct StonemasonHut {
    pub owned: bool,
}

#[derive(Resource, Default, Debug)]
pub struct Pier {
    pub owned: bool,
}

/// Port — water-side structure unlocked from the pier. Boatmen
/// launch from the port to fish stones out of the ocean.
#[derive(Resource, Default, Debug)]
pub struct Port {
    pub owned: bool,
}

#[derive(Resource, Default, Debug)]
pub struct Workers {
    /// Currently-alive workers idling around the foragers hut.
    /// Decremented when one is converted into a specialist.
    pub count: u32,
    /// Cumulative count of workers ever bought from the cave's
    /// Worker row — never decremented. Drives the dynamic worker
    /// price (`current_worker_cost`).
    pub purchased: u32,
}

#[derive(Resource, Default, Debug)]
pub struct Miners {
    pub count: u32,
}

#[derive(Resource, Default, Debug)]
pub struct Skimmers {
    pub count: u32,
}

#[derive(Resource, Default, Debug)]
pub struct Fishermen {
    pub count: u32,
}

#[derive(Resource, Default, Debug)]
pub struct Fishes {
    pub count: u32,
}

#[derive(Resource, Default, Debug)]
pub struct Beachcombers {
    pub count: u32,
}

#[derive(Resource, Default, Debug)]
pub struct Stonemasons {
    pub count: u32,
}

#[derive(Resource, Default, Debug)]
pub struct Boatmen {
    pub count: u32,
}

/// Number of `Skim Up` upgrades purchased. Each level adds
/// `SKIM_UPGRADE_DELTA` to the bounce chance applied when the player
/// or a skimmer throws a rock.
#[derive(Resource, Default, Debug)]
pub struct SkimUpgrades {
    pub level: u32,
}

/// Repeatable miner upgrades. `damage_level` adds +1 damage per
/// pickaxe throw, on top of the base [`MINER_PICKAXE_DAMAGE`].
#[derive(Resource, Default, Debug)]
pub struct MinerUpgrades {
    pub damage_level: u32,
}

/// Cursor-over-structure state. One flag per building (and each
/// covers the structure + its panel + its detail panel as a union of
/// rects, so panels stay visible while the cursor is over the panel).
/// `row` is the specific button row currently under the cursor.
#[derive(Resource, Default, Debug)]
pub struct HoverState {
    pub cave: bool,
    pub hut: bool,
    pub hut_miner: bool,
    pub hut_skimmer: bool,
    pub hut_fisher: bool,
    pub hut_beachcomber: bool,
    pub hut_stonemason: bool,
    pub pier: bool,
    pub port: bool,
    pub row: Option<PurchaseKind>,
}

// =====================================================================
// UI components
// =====================================================================

#[derive(Component)]
pub struct PurchaseButton {
    pub kind: PurchaseKind,
    pub size: Vec2,
}

#[derive(Component)]
pub struct ButtonLabel(pub PurchaseKind);

#[derive(Component)]
pub struct ButtonCost(pub PurchaseKind);

/// Live count for a buy row — the "x3" between name and cost.
/// Updated from the matching resource each frame.
#[derive(Component)]
pub struct ButtonCount(pub PurchaseKind);

/// Which structure a UI part belongs to. Drives visibility gating.
/// The four hut sub-types (`Hut` for the worker hut and the three
/// `HutXxx` variants for specialist buildings) all unlock together
/// when the hut is purchased from the cave.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PanelKind {
    Cave,
    Hut,
    HutMiner,
    HutSkimmer,
    HutFisher,
    HutBeachcomber,
    HutStonemason,
    Pier,
    Port,
}

/// Single tag for every chrome entity (panel BG, border, title,
/// pointer, detail panel, …). Replaces the per-kind tag components
/// — visibility/detail systems branch on `PanelKind` instead of
/// filtering by component type.
#[derive(Component, Clone, Copy)]
pub struct PanelTag(pub PanelKind);

/// Which sub-element of a panel a chrome entity is. The cave panel's
/// relayout system reads this to know what to update each frame as
/// the panel grows or shrinks with its visible row count.
#[derive(Component, Clone, Copy, Debug)]
pub enum PanelChromePart {
    /// Outer border rectangle — slightly larger than the inner BG.
    Border,
    /// Inner background rectangle.
    Bg,
    /// Title text along the panel's top.
    Title,
    /// Divider line under the title.
    Divider,
    /// One segment of the multi-pixel pointer arrow (index 0 = widest).
    Pointer(u8),
    /// Detail panel's outer border.
    DetailBorder,
    /// Detail panel's inner background.
    DetailBg,
    /// Detail panel pointer segment (currently unused — none of the
    /// detail panels have pointers, but reserved for symmetry).
    #[allow(dead_code)]
    DetailPointer(u8),
}

/// Computed cave panel geometry — refreshed each frame by the cave
/// relayout system. Other systems (hover detection, chrome
/// visibility, button visuals) read this resource so they always
/// reflect the panel's current size and position, not the static
/// max-sized layout.
#[derive(Resource, Clone, Copy, Debug)]
pub struct CavePanelGeo {
    pub panel_pos: Vec2,
    pub panel_size: Vec2,
    pub detail_pos: Vec2,
    pub detail_size: Vec2,
    /// Visible row count this frame, in display order.
    pub row_count: u8,
}

impl Default for CavePanelGeo {
    fn default() -> Self {
        Self {
            panel_pos: Vec2::ZERO,
            panel_size: Vec2::ZERO,
            detail_pos: Vec2::ZERO,
            detail_size: Vec2::ZERO,
            row_count: 0,
        }
    }
}

/// Header line of a detail panel ("Buy!" etc).
#[derive(Component)]
pub struct DetailHeader(pub PanelKind);

/// Body lines of a detail panel — one Text2d entity for the whole
/// description block, multi-line via `\n`.
#[derive(Component)]
pub struct DetailBody(pub PanelKind);

// =====================================================================
// Plugin
// =====================================================================

pub struct EconomyPlugin;

impl Plugin for EconomyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Hut>()
            .init_resource::<MinerHut>()
            .init_resource::<SkimmerHut>()
            .init_resource::<FisherHut>()
            .init_resource::<BeachcomberHut>()
            .init_resource::<StonemasonHut>()
            .init_resource::<Pier>()
            .init_resource::<Port>()
            .init_resource::<Beachcombers>()
            .init_resource::<Stonemasons>()
            .init_resource::<Boatmen>()
            .init_resource::<Workers>()
            .init_resource::<Miners>()
            .init_resource::<Skimmers>()
            .init_resource::<Fishermen>()
            .init_resource::<Fishes>()
            .init_resource::<SkimUpgrades>()
            .init_resource::<MinerUpgrades>()
            .init_resource::<HoverState>()
            .init_resource::<CavePanelGeo>()
            .add_message::<PurchaseEvent>()
            .add_systems(Startup, spawn::spawn_ui)
            .add_systems(
                Update,
                (
                    // Relayout runs first — it writes the dynamic
                    // CavePanelGeo resource that hover detection
                    // reads when computing the cave's panel rects.
                    relayout::relayout_cave_panel,
                    interaction::update_hover,
                    interaction::update_chrome_visibility,
                    interaction::update_row_visibility,
                    interaction::update_detail_text,
                    purchase::handle_button_clicks,
                    interaction::update_button_visuals,
                    interaction::update_count_text,
                    interaction::update_dynamic_cost_text,
                )
                    .chain(),
            );
    }
}
