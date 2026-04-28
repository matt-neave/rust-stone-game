//! Layout constants + helpers for the economy UI.
//!
//! Everything that takes a position in spec-coordinate space is
//! computed here, so the panel-spawning code and the hover/visibility
//! systems can share the same source of truth.

use bevy::prelude::*;

use crate::core::constants::{
    CAVE_W, CAVE_X, CAVE_Y, HUT_BEACHCOMBER_X, HUT_BEACHCOMBER_Y, HUT_FISHER_X, HUT_FISHER_Y,
    HUT_MINER_X, HUT_MINER_Y, HUT_ROOF_W, HUT_SKIMMER_X, HUT_SKIMMER_Y, HUT_STONEMASON_X,
    HUT_STONEMASON_Y, HUT_X, HUT_Y, PIER_X, PIER_Y, PORT_H, PORT_W, PORT_X, PORT_Y,
};

use super::{
    CAVE_PANEL_KINDS, HUT_BEACHCOMBER_KINDS, HUT_FISHER_KINDS, HUT_MINER_KINDS, HUT_PANEL_KINDS,
    HUT_SKIMMER_KINDS, HUT_STONEMASON_KINDS, PIER_PANEL_KINDS, PORT_PANEL_KINDS,
};

// ---------------------------------------------------------------------------
// Cave panel — sized dynamically from CAVE_PANEL_KINDS.len()
// ---------------------------------------------------------------------------

pub const CAVE_PANEL_W: f32 = 80.0;
pub const CAVE_DETAIL_W: f32 = 100.0;
pub const CAVE_DETAIL_GAP: f32 = 4.0;
/// Spec px between the cave's east edge and the cave panel's west
/// edge. Same gap the hut panels use to feel consistent.
pub const CAVE_PANEL_GAP: f32 = 4.0;

/// Anchor data for the cave panel — center vertically on the cave
/// (so the pointer-west arrow lines up with the cave's middle), and
/// place the panel just east of the cave's right edge. The dynamic
/// relayout system reads this and grows the panel symmetrically
/// around the y as rows are added.
pub struct CavePanelAnchor {
    pub center_x: f32,
    pub center_y: f32,
}

pub fn cave_panel_anchor() -> CavePanelAnchor {
    CavePanelAnchor {
        center_x: CAVE_X + CAVE_W * 0.5 + CAVE_PANEL_GAP + CAVE_PANEL_W * 0.5,
        center_y: CAVE_Y,
    }
}

/// Where the detail panel sits relative to the (just-computed) main
/// panel. East of the main panel, vertically centered. Detail height
/// matches the panel height with a `DETAIL_MIN_H` floor.
pub struct CaveDetailAnchor {
    pub x: f32,
    pub y: f32,
    pub height: f32,
}

pub fn cave_detail_anchor(panel_pos: Vec2, panel_h: f32) -> CaveDetailAnchor {
    CaveDetailAnchor {
        x: panel_pos.x + CAVE_PANEL_W * 0.5 + CAVE_DETAIL_GAP + CAVE_DETAIL_W * 0.5,
        y: panel_pos.y,
        height: panel_h.max(DETAIL_MIN_H),
    }
}

/// Initial spawn-time geometry for the cave panel — sized for the
/// max possible row count. The relayout system rewrites this every
/// frame, but spawn needs *some* position to attach entities to.
pub fn cave_panel_height() -> f32 {
    panel_height_for(CAVE_PANEL_KINDS.len())
}

pub fn cave_panel_pos() -> Vec2 {
    let a = cave_panel_anchor();
    Vec2::new(a.center_x, a.center_y)
}

pub fn cave_panel_size() -> Vec2 {
    Vec2::new(CAVE_PANEL_W, cave_panel_height())
}

pub fn cave_detail_pos() -> Vec2 {
    let main = cave_panel_pos();
    let h = cave_panel_height();
    let d = cave_detail_anchor(main, h);
    Vec2::new(d.x, d.y)
}

/// Generic panel-height helper — same `title strip + rows + pad`
/// formula every panel uses, parameterised by row count.
pub fn panel_height_for(count: usize) -> f32 {
    let title_strip = 5.0 + 2.0;
    let rows = count as f32 * ROW_HEIGHT;
    let pad = 4.0;
    title_strip + rows + pad
}

pub fn cave_detail_size() -> Vec2 {
    Vec2::new(CAVE_DETAIL_W, cave_panel_height().max(DETAIL_MIN_H))
}

/// Y position (in spec coords) for the Nth cave-panel buy row.
pub fn cave_buy_row_y(index: usize) -> f32 {
    let panel = cave_panel_pos();
    let panel_size = cave_panel_size();
    let panel_top = panel.y - panel_size.y * 0.5;
    panel_top + 7.0 + ROW_HEIGHT * 0.5 + index as f32 * ROW_HEIGHT
}

/// Hover rectangles for the cave's *structure* footprint only. The
/// hover system uses this to decide when to **open** the panel
/// (cursor over the building itself).
pub fn cave_building_rects() -> [(Vec2, Vec2); 1] {
    use crate::core::constants::CAVE_X as STRUCT_X;
    use crate::core::constants::CAVE_Y as STRUCT_Y;
    [building_rect(STRUCT_X, STRUCT_Y, 12.0)]
}

// Cave panel/detail hover rects come from the dynamic
// `CavePanelGeo` resource (computed each frame in
// `economy::relayout`), not from this module. The pre-existing
// `cave_panel_rects` static helper has been retired in favour of
// the resource-driven version in `economy::interaction`.

// ---------------------------------------------------------------------------
// Hut panel — width fixed, height derived from `HUT_PANEL_KINDS.len()`
// ---------------------------------------------------------------------------

pub const HUT_PANEL_W: f32 = 88.0;
pub const HUT_DETAIL_W: f32 = 100.0;
/// Minimum detail-panel height — guarantees the description fits even
/// when the main panel only has a couple of rows. Tuned tight so
/// vertically-stacked huts have enough breathing room between their
/// detail panels.
pub const DETAIL_MIN_H: f32 = 42.0;
pub const HUT_DETAIL_GAP: f32 = 4.0;

// ---------------------------------------------------------------------------
// Shared
// ---------------------------------------------------------------------------

/// Inset from each panel edge for content. The 1-px outer border +
/// a couple of pixels of breathing room.
pub const PANEL_INSET: f32 = 3.0;
/// Border thickness around panels.
pub const PANEL_BORDER_W: f32 = 1.0;
/// Height of one buy / count row.
pub const ROW_HEIGHT: f32 = 9.0;

// ---------------------------------------------------------------------------
// Hut panel — dynamically sized by `HUT_PANEL_KINDS.len()`. Three more
// specialist-hut panels are wired below using the same primitives.
// ---------------------------------------------------------------------------

/// Layout for one hut-style panel: where the buy panel sits relative
/// to its building, where its detail panel sits beside that, and the
/// resulting sizes derived from the panel's row count.
struct HutPanelGeo {
    building: Vec2,
    panel_pos: Vec2,
    panel_size: Vec2,
    detail_pos: Vec2,
    detail_size: Vec2,
}

/// Spec px between a building's right edge and the left edge of its
/// panel. Small enough that a fast cursor crossing the gap keeps the
/// panel "shown" via the building hit on entry.
const BUILDING_PANEL_GAP: f32 = 4.0;
/// Half-width assumed for any building's footprint when computing
/// where its panel sits. Matches the existing hut roof half-width.
const BUILDING_HALF_W: f32 = HUT_ROOF_W * 0.5;

/// Build a `HutPanelGeo` given the building's position and the row
/// count of its panel. The panel always sits **directly to the right**
/// of the building, with the detail panel further right of the panel.
/// Adding a new structure is just placing it — the panel layout
/// follows automatically.
fn hut_geo(building: Vec2, kinds_count: usize) -> HutPanelGeo {
    let panel_x = building.x + BUILDING_HALF_W + BUILDING_PANEL_GAP + HUT_PANEL_W * 0.5;
    let panel_pos = Vec2::new(panel_x, building.y);
    let panel_h = panel_height_for(kinds_count);
    let panel_size = Vec2::new(HUT_PANEL_W, panel_h);
    let detail_x = panel_pos.x + HUT_PANEL_W * 0.5 + HUT_DETAIL_GAP + HUT_DETAIL_W * 0.5;
    let detail_pos = Vec2::new(detail_x, building.y);
    let detail_size = Vec2::new(HUT_DETAIL_W, panel_h.max(DETAIL_MIN_H));
    HutPanelGeo {
        building,
        panel_pos,
        panel_size,
        detail_pos,
        detail_size,
    }
}

fn hut_building_rects_from(geo: &HutPanelGeo) -> [(Vec2, Vec2); 1] {
    [building_rect(geo.building.x, geo.building.y, 12.0)]
}

fn hut_panel_rects_from(geo: &HutPanelGeo) -> [(Vec2, Vec2); 2] {
    [
        panel_rect(geo.panel_pos, geo.panel_size),
        panel_rect(geo.detail_pos, geo.detail_size),
    ]
}

fn hut_buy_row_y_from(geo: &HutPanelGeo, index: usize) -> f32 {
    let panel_top = geo.panel_pos.y - geo.panel_size.y * 0.5;
    panel_top + 7.0 + ROW_HEIGHT * 0.5 + index as f32 * ROW_HEIGHT
}

// --- Foragers (worker) hut --------------------------------------------------

fn hut_foragers_geo() -> HutPanelGeo {
    hut_geo(Vec2::new(HUT_X, HUT_Y), HUT_PANEL_KINDS.len())
}

pub fn hut_panel_pos() -> Vec2 { hut_foragers_geo().panel_pos }
pub fn hut_panel_size() -> Vec2 { hut_foragers_geo().panel_size }
pub fn hut_detail_pos() -> Vec2 { hut_foragers_geo().detail_pos }
pub fn hut_detail_size() -> Vec2 { hut_foragers_geo().detail_size }
pub fn hut_building_rects() -> [(Vec2, Vec2); 1] { hut_building_rects_from(&hut_foragers_geo()) }
pub fn hut_panel_rects() -> [(Vec2, Vec2); 2] { hut_panel_rects_from(&hut_foragers_geo()) }
pub fn hut_buy_row_y(index: usize) -> f32 { hut_buy_row_y_from(&hut_foragers_geo(), index) }

// --- Miner hut --------------------------------------------------------------

fn hut_miner_geo() -> HutPanelGeo {
    hut_geo(Vec2::new(HUT_MINER_X, HUT_MINER_Y), HUT_MINER_KINDS.len())
}

pub fn hut_miner_panel_pos() -> Vec2 { hut_miner_geo().panel_pos }
pub fn hut_miner_panel_size() -> Vec2 { hut_miner_geo().panel_size }
pub fn hut_miner_detail_pos() -> Vec2 { hut_miner_geo().detail_pos }
pub fn hut_miner_detail_size() -> Vec2 { hut_miner_geo().detail_size }
pub fn hut_miner_building_rects() -> [(Vec2, Vec2); 1] { hut_building_rects_from(&hut_miner_geo()) }
pub fn hut_miner_panel_rects() -> [(Vec2, Vec2); 2] { hut_panel_rects_from(&hut_miner_geo()) }
pub fn hut_miner_buy_row_y(index: usize) -> f32 { hut_buy_row_y_from(&hut_miner_geo(), index) }

// --- Skimmer hut ------------------------------------------------------------

fn hut_skimmer_geo() -> HutPanelGeo {
    hut_geo(Vec2::new(HUT_SKIMMER_X, HUT_SKIMMER_Y), HUT_SKIMMER_KINDS.len())
}

pub fn hut_skimmer_panel_pos() -> Vec2 { hut_skimmer_geo().panel_pos }
pub fn hut_skimmer_panel_size() -> Vec2 { hut_skimmer_geo().panel_size }
pub fn hut_skimmer_detail_pos() -> Vec2 { hut_skimmer_geo().detail_pos }
pub fn hut_skimmer_detail_size() -> Vec2 { hut_skimmer_geo().detail_size }
pub fn hut_skimmer_building_rects() -> [(Vec2, Vec2); 1] { hut_building_rects_from(&hut_skimmer_geo()) }
pub fn hut_skimmer_panel_rects() -> [(Vec2, Vec2); 2] { hut_panel_rects_from(&hut_skimmer_geo()) }
pub fn hut_skimmer_buy_row_y(index: usize) -> f32 { hut_buy_row_y_from(&hut_skimmer_geo(), index) }

// --- Fisherman hut ----------------------------------------------------------

fn hut_fisher_geo() -> HutPanelGeo {
    hut_geo(Vec2::new(HUT_FISHER_X, HUT_FISHER_Y), HUT_FISHER_KINDS.len())
}

pub fn hut_fisher_panel_pos() -> Vec2 { hut_fisher_geo().panel_pos }
pub fn hut_fisher_panel_size() -> Vec2 { hut_fisher_geo().panel_size }
pub fn hut_fisher_detail_pos() -> Vec2 { hut_fisher_geo().detail_pos }
pub fn hut_fisher_detail_size() -> Vec2 { hut_fisher_geo().detail_size }
pub fn hut_fisher_building_rects() -> [(Vec2, Vec2); 1] { hut_building_rects_from(&hut_fisher_geo()) }
pub fn hut_fisher_panel_rects() -> [(Vec2, Vec2); 2] { hut_panel_rects_from(&hut_fisher_geo()) }
pub fn hut_fisher_buy_row_y(index: usize) -> f32 { hut_buy_row_y_from(&hut_fisher_geo(), index) }

// --- Beachcomber hut --------------------------------------------------------

fn hut_beachcomber_geo() -> HutPanelGeo {
    hut_geo(
        Vec2::new(HUT_BEACHCOMBER_X, HUT_BEACHCOMBER_Y),
        HUT_BEACHCOMBER_KINDS.len(),
    )
}

pub fn hut_beachcomber_panel_pos() -> Vec2 { hut_beachcomber_geo().panel_pos }
pub fn hut_beachcomber_panel_size() -> Vec2 { hut_beachcomber_geo().panel_size }
pub fn hut_beachcomber_detail_pos() -> Vec2 { hut_beachcomber_geo().detail_pos }
pub fn hut_beachcomber_detail_size() -> Vec2 { hut_beachcomber_geo().detail_size }
pub fn hut_beachcomber_building_rects() -> [(Vec2, Vec2); 1] {
    hut_building_rects_from(&hut_beachcomber_geo())
}
pub fn hut_beachcomber_panel_rects() -> [(Vec2, Vec2); 2] {
    hut_panel_rects_from(&hut_beachcomber_geo())
}
pub fn hut_beachcomber_buy_row_y(index: usize) -> f32 {
    hut_buy_row_y_from(&hut_beachcomber_geo(), index)
}

// --- Stonemason hut ---------------------------------------------------------

fn hut_stonemason_geo() -> HutPanelGeo {
    hut_geo(
        Vec2::new(HUT_STONEMASON_X, HUT_STONEMASON_Y),
        HUT_STONEMASON_KINDS.len(),
    )
}

pub fn hut_stonemason_panel_pos() -> Vec2 { hut_stonemason_geo().panel_pos }
pub fn hut_stonemason_panel_size() -> Vec2 { hut_stonemason_geo().panel_size }
pub fn hut_stonemason_detail_pos() -> Vec2 { hut_stonemason_geo().detail_pos }
pub fn hut_stonemason_detail_size() -> Vec2 { hut_stonemason_geo().detail_size }
pub fn hut_stonemason_building_rects() -> [(Vec2, Vec2); 1] {
    hut_building_rects_from(&hut_stonemason_geo())
}
pub fn hut_stonemason_panel_rects() -> [(Vec2, Vec2); 2] {
    hut_panel_rects_from(&hut_stonemason_geo())
}
pub fn hut_stonemason_buy_row_y(index: usize) -> f32 {
    hut_buy_row_y_from(&hut_stonemason_geo(), index)
}

// ---------------------------------------------------------------------------
// Pier panel — sits below the pier on the water
// ---------------------------------------------------------------------------

pub const PIER_PANEL_X: f32 = 240.0;
/// Pier panel sits just below the pier so the cursor can travel
/// from the pier into the panel without crossing dead space. With
/// the pier's padded hover rect ending at y ≈ 98 and the panel's
/// padded top at y = `PIER_PANEL_Y` − height/2 − 2, this `110` puts
/// the two rects edge-to-edge for any panel height up to ~22 px.
pub const PIER_PANEL_Y: f32 = 110.0;
pub const PIER_PANEL_W: f32 = 80.0;
pub const PIER_DETAIL_W: f32 = 100.0;
pub const PIER_DETAIL_GAP: f32 = 4.0;

pub fn pier_panel_height() -> f32 {
    let title_strip = 5.0 + 2.0;
    let rows = PIER_PANEL_KINDS.len() as f32 * ROW_HEIGHT;
    let pad = 4.0;
    title_strip + rows + pad
}

pub fn pier_panel_pos() -> Vec2 {
    Vec2::new(PIER_PANEL_X, PIER_PANEL_Y)
}

pub fn pier_panel_size() -> Vec2 {
    Vec2::new(PIER_PANEL_W, pier_panel_height())
}

pub fn pier_detail_pos() -> Vec2 {
    let main = pier_panel_pos();
    Vec2::new(
        main.x + PIER_PANEL_W * 0.5 + PIER_DETAIL_GAP + PIER_DETAIL_W * 0.5,
        main.y,
    )
}

pub fn pier_detail_size() -> Vec2 {
    Vec2::new(PIER_DETAIL_W, pier_panel_height().max(DETAIL_MIN_H))
}

pub fn pier_buy_row_y(index: usize) -> f32 {
    let panel = pier_panel_pos();
    let panel_size = pier_panel_size();
    let panel_top = panel.y - panel_size.y * 0.5;
    panel_top + 7.0 + ROW_HEIGHT * 0.5 + index as f32 * ROW_HEIGHT
}

/// Hover rectangles for the pier: structure + buy panel + detail panel.
/// The pier itself is wide and thin, so its rect uses an asymmetric
/// pad — wide in x, slim in y.
pub fn pier_building_rects() -> [(Vec2, Vec2); 1] {
    let pier_min = Vec2::new(PIER_X - 36.0, PIER_Y - 6.0);
    let pier_max = Vec2::new(PIER_X + 36.0, PIER_Y + 6.0);
    [(pier_min - Vec2::splat(2.0), pier_max + Vec2::splat(2.0))]
}

pub fn pier_panel_rects() -> [(Vec2, Vec2); 2] {
    [
        panel_rect(pier_panel_pos(), pier_panel_size()),
        panel_rect(pier_detail_pos(), pier_detail_size()),
    ]
}

// ---------------------------------------------------------------------------
// Port panel — sits above the port, mirroring the pier's panel-up layout
// ---------------------------------------------------------------------------

pub const PORT_PANEL_W: f32 = 80.0;
pub const PORT_DETAIL_W: f32 = 100.0;
pub const PORT_DETAIL_GAP: f32 = 4.0;

pub fn port_panel_height() -> f32 {
    let title_strip = 5.0 + 2.0;
    let rows = PORT_PANEL_KINDS.len() as f32 * ROW_HEIGHT;
    let pad = 4.0;
    title_strip + rows + pad
}

/// Port panel sits above the port, the same way the pier panel sits
/// above the pier (the port is far south on the canvas — placing the
/// panel below would push it off-screen).
pub fn port_panel_pos() -> Vec2 {
    let h = port_panel_height();
    Vec2::new(PORT_X, PORT_Y - PORT_H * 0.5 - 4.0 - h * 0.5)
}

pub fn port_panel_size() -> Vec2 {
    Vec2::new(PORT_PANEL_W, port_panel_height())
}

pub fn port_detail_pos() -> Vec2 {
    let main = port_panel_pos();
    Vec2::new(
        main.x + PORT_PANEL_W * 0.5 + PORT_DETAIL_GAP + PORT_DETAIL_W * 0.5,
        main.y,
    )
}

pub fn port_detail_size() -> Vec2 {
    Vec2::new(PORT_DETAIL_W, port_panel_height().max(DETAIL_MIN_H))
}

pub fn port_buy_row_y(index: usize) -> f32 {
    let panel = port_panel_pos();
    let panel_size = port_panel_size();
    let panel_top = panel.y - panel_size.y * 0.5;
    panel_top + 7.0 + ROW_HEIGHT * 0.5 + index as f32 * ROW_HEIGHT
}

pub fn port_building_rects() -> [(Vec2, Vec2); 1] {
    let port_min = Vec2::new(PORT_X - PORT_W * 0.5, PORT_Y - PORT_H * 0.5);
    let port_max = Vec2::new(PORT_X + PORT_W * 0.5, PORT_Y + PORT_H * 0.5);
    [(port_min - Vec2::splat(2.0), port_max + Vec2::splat(2.0))]
}

pub fn port_panel_rects() -> [(Vec2, Vec2); 2] {
    [
        panel_rect(port_panel_pos(), port_panel_size()),
        panel_rect(port_detail_pos(), port_detail_size()),
    ]
}

// ---------------------------------------------------------------------------
// Hover-rect helpers
// ---------------------------------------------------------------------------

/// Rectangle around a building footprint, padded by `pad` in both axes
/// and the standard 2-px hover-cushion on top of that.
fn building_rect(cx: f32, cy: f32, pad: f32) -> (Vec2, Vec2) {
    let p = pad + 2.0;
    (Vec2::new(cx - p, cy - p), Vec2::new(cx + p, cy + p))
}

/// Rectangle around a UI panel centered at `center` with the given
/// `size`, padded by 2 px on each side so the cursor can graze the
/// border without flickering the panel.
fn panel_rect(center: Vec2, size: Vec2) -> (Vec2, Vec2) {
    let half = size * 0.5;
    let pad = Vec2::splat(2.0);
    (center - half - pad, center + half + pad)
}
