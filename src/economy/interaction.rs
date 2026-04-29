//! Per-frame systems: hover detection, panel/row visibility, button
//! visuals, count text, detail-panel content.
//!
//! Visibility model:
//!
//! * **Chrome** entities (BG / border / title / divider / pointer /
//!   detail bg+header+body) carry a single `PanelTag(PanelKind)`
//!   marker. The chrome-visibility system queries it and toggles each
//!   entity based on its panel's hover + availability state.
//! * **Row** entities (`PurchaseButton` / `ButtonLabel` / `ButtonCost`
//!   / `ButtonCount`) do not carry `PanelTag`. Each row's visibility
//!   is gated independently by `button_active(kind)` so already-built
//!   one-time purchases hide while the next unbought option keeps the
//!   panel chrome up.

use bevy::prelude::*;

use crate::core::colors;
use crate::core::input::cursor_to_spec;
use crate::currency::Skims;
use crate::render::{DisplayMode, DisplayScale};

use super::layout::*;
use super::purchase::{button_active, can_afford};
use super::purchase::{current_worker_cost, is_sold_out, row_visible};
use super::{
    BeachcomberHut, Beachcombers, Boatmen, ButtonCost, ButtonCount, ButtonLabel, CavePanelGeo,
    DetailBody, DetailHeader, FisherHut, Fishermen, Fishes, HoverState, Hut, MinerHut,
    MinerUpgrades, Miners, PanelChromePart, PanelKind, PanelTag, Pier, Port, PurchaseButton,
    PurchaseKind, SkimUpgrades, SkimmerHut, Skimmers, StonemasonHut, Stonemasons, UpgradeRes,
    Workers, CAVE_PANEL_KINDS, HUT_BEACHCOMBER_KINDS, HUT_FISHER_KINDS, HUT_MINER_KINDS,
    HUT_PANEL_KINDS, HUT_SKIMMER_KINDS, HUT_STONEMASON_KINDS, PIER_PANEL_KINDS, PORT_PANEL_KINDS,
};

// ---------------------------------------------------------------------------
// Hover
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub(super) fn update_hover(
    windows: Query<&Window>,
    display_scale: Res<DisplayScale>,
    mode: Res<DisplayMode>,
    hut: Res<Hut>,
    miner_hut: Res<MinerHut>,
    skimmer_hut: Res<SkimmerHut>,
    fisher_hut: Res<FisherHut>,
    bc_hut: Res<BeachcomberHut>,
    sm_hut: Res<StonemasonHut>,
    pier: Res<Pier>,
    port: Res<Port>,
    cave_geo: Res<CavePanelGeo>,
    scroll: Res<crate::render::CameraScroll>,
    mut hover: ResMut<HoverState>,
) {
    if *mode == DisplayMode::Docked {
        if any_hover(&hover) {
            *hover = HoverState::default();
        }
        return;
    }
    let Ok(window) = windows.single() else {
        if any_hover(&hover) {
            *hover = HoverState::default();
        }
        return;
    };
    let cursor = cursor_to_spec(window, display_scale.0, scroll.x);
    let cave_dynamic_rects = cave_panel_rects_from_geo(&cave_geo);

    let cave_chrome = chrome_hit(cursor, hover.cave, &cave_dynamic_rects);
    let hut_chrome = chrome_hit(cursor, hover.hut, &hut_panel_rects());
    let miner_chrome = chrome_hit(cursor, hover.hut_miner, &hut_miner_panel_rects());
    let skimmer_chrome = chrome_hit(cursor, hover.hut_skimmer, &hut_skimmer_panel_rects());
    let fisher_chrome = chrome_hit(cursor, hover.hut_fisher, &hut_fisher_panel_rects());
    let bc_chrome = chrome_hit(cursor, hover.hut_beachcomber, &hut_beachcomber_panel_rects());
    let sm_chrome = chrome_hit(cursor, hover.hut_stonemason, &hut_stonemason_panel_rects());
    let pier_chrome = chrome_hit(cursor, hover.pier, &pier_panel_rects());
    let port_chrome = chrome_hit(cursor, hover.port, &port_panel_rects());

    let chrome_claim = cave_chrome
        || hut_chrome
        || miner_chrome
        || skimmer_chrome
        || fisher_chrome
        || bc_chrome
        || sm_chrome
        || pier_chrome
        || port_chrome;

    let building_hit = |rects: &[(Vec2, Vec2)]| match cursor {
        Some(s) => in_any_box(s, rects),
        None => false,
    };
    let cave_building = building_hit(&cave_building_rects());
    let hut_building = building_hit(&hut_building_rects());
    let miner_building = building_hit(&hut_miner_building_rects());
    let skimmer_building = building_hit(&hut_skimmer_building_rects());
    let fisher_building = building_hit(&hut_fisher_building_rects());
    let bc_building = building_hit(&hut_beachcomber_building_rects());
    let sm_building = building_hit(&hut_stonemason_building_rects());
    let pier_building = building_hit(&pier_building_rects());
    let port_building = building_hit(&port_building_rects());

    let resolve = |claim: bool, chrome: bool, building: bool| if claim { chrome } else { building };
    let cave_zone = resolve(chrome_claim, cave_chrome, cave_building);
    let hut_zone = resolve(chrome_claim, hut_chrome, hut_building);
    let miner_zone = resolve(chrome_claim, miner_chrome, miner_building);
    let skimmer_zone = resolve(chrome_claim, skimmer_chrome, skimmer_building);
    let fisher_zone = resolve(chrome_claim, fisher_chrome, fisher_building);
    let bc_zone = resolve(chrome_claim, bc_chrome, bc_building);
    let sm_zone = resolve(chrome_claim, sm_chrome, sm_building);
    let pier_zone = resolve(chrome_claim, pier_chrome, pier_building);
    let port_zone = resolve(chrome_claim, port_chrome, port_building);

    let row = match cursor {
        None => None,
        Some(spec) => {
            if cave_zone {
                cave_row_at(
                    spec,
                    &cave_geo,
                    &cave_visible_kinds(&hut, &miner_hut, &skimmer_hut, &fisher_hut, &pier, &port),
                )
            } else if hut.owned && hut_zone {
                row_at_hut(spec)
            } else if miner_hut.owned && miner_zone {
                row_at_hut_miner(spec)
            } else if skimmer_hut.owned && skimmer_zone {
                row_at_hut_skimmer(spec)
            } else if fisher_hut.owned && fisher_zone {
                row_at_hut_fisher(spec)
            } else if bc_hut.owned && bc_zone {
                row_at_hut_beachcomber(spec)
            } else if sm_hut.owned && sm_zone {
                row_at_hut_stonemason(spec)
            } else if pier.owned && pier_zone {
                row_at_pier(spec)
            } else if port.owned && port_zone {
                row_at_port(spec)
            } else {
                None
            }
        }
    };

    if hover.cave != cave_zone { hover.cave = cave_zone; }
    if hover.hut != hut_zone { hover.hut = hut_zone; }
    if hover.hut_miner != miner_zone { hover.hut_miner = miner_zone; }
    if hover.hut_skimmer != skimmer_zone { hover.hut_skimmer = skimmer_zone; }
    if hover.hut_fisher != fisher_zone { hover.hut_fisher = fisher_zone; }
    if hover.hut_beachcomber != bc_zone { hover.hut_beachcomber = bc_zone; }
    if hover.hut_stonemason != sm_zone { hover.hut_stonemason = sm_zone; }
    if hover.pier != pier_zone { hover.pier = pier_zone; }
    if hover.port != port_zone { hover.port = port_zone; }
    if hover.row != row { hover.row = row; }
}

fn any_hover(h: &HoverState) -> bool {
    h.cave
        || h.hut
        || h.hut_miner
        || h.hut_skimmer
        || h.hut_fisher
        || h.hut_beachcomber
        || h.hut_stonemason
        || h.pier
        || h.port
        || h.row.is_some()
}

fn chrome_hit(cursor: Option<Vec2>, currently_open: bool, rects: &[(Vec2, Vec2)]) -> bool {
    match cursor {
        Some(s) => currently_open && in_any_box(s, rects),
        None => false,
    }
}

fn in_box(p: Vec2, min: Vec2, max: Vec2) -> bool {
    p.x >= min.x && p.x <= max.x && p.y >= min.y && p.y <= max.y
}

/// True if `p` falls inside any of the given rectangles. Lets a panel's
/// hover region be a union of disjoint boxes (building + panel +
/// detail) so the empty space between them isn't sticky.
pub(super) fn in_any_box(p: Vec2, rects: &[(Vec2, Vec2)]) -> bool {
    rects.iter().any(|(min, max)| in_box(p, *min, *max))
}

/// Cave panel + detail rects derived from the live `CavePanelGeo`.
/// Falls back to a no-op rect when the geo hasn't been initialised
/// yet (first frame before `relayout_cave_panel` runs).
fn cave_panel_rects_from_geo(geo: &CavePanelGeo) -> [(Vec2, Vec2); 2] {
    if geo.row_count == 0 {
        return [
            (Vec2::ZERO, Vec2::ZERO),
            (Vec2::ZERO, Vec2::ZERO),
        ];
    }
    let pad = Vec2::splat(2.0);
    let main = (
        geo.panel_pos - geo.panel_size * 0.5 - pad,
        geo.panel_pos + geo.panel_size * 0.5 + pad,
    );
    let detail = (
        geo.detail_pos - geo.detail_size * 0.5 - pad,
        geo.detail_pos + geo.detail_size * 0.5 + pad,
    );
    [main, detail]
}

#[allow(clippy::too_many_arguments)]
fn cave_visible_kinds(
    hut: &Hut,
    miner_hut: &MinerHut,
    skimmer_hut: &SkimmerHut,
    fisher_hut: &FisherHut,
    pier: &Pier,
    port: &Port,
) -> Vec<PurchaseKind> {
    CAVE_PANEL_KINDS
        .iter()
        .copied()
        .filter(|k| row_visible(*k, hut, miner_hut, skimmer_hut, fisher_hut, pier, port))
        .collect()
}

/// Hit-test the cave's dynamically-laid-out rows. Slot index in the
/// visible-kinds list maps directly to a y inside the live panel.
fn cave_row_at(
    spec: Vec2,
    geo: &CavePanelGeo,
    visible: &[PurchaseKind],
) -> Option<PurchaseKind> {
    if geo.row_count == 0 {
        return None;
    }
    let half_h = ROW_HEIGHT * 0.5;
    let half_w = (geo.panel_size.x - PANEL_INSET * 2.0) * 0.5;
    if (spec.x - geo.panel_pos.x).abs() > half_w {
        return None;
    }
    let panel_top = geo.panel_pos.y - geo.panel_size.y * 0.5;
    for (i, kind) in visible.iter().enumerate() {
        let row_y = panel_top + 7.0 + ROW_HEIGHT * 0.5 + i as f32 * ROW_HEIGHT;
        if (spec.y - row_y).abs() <= half_h {
            return Some(*kind);
        }
    }
    None
}

fn row_at_hut(spec: Vec2) -> Option<PurchaseKind> {
    row_at_panel(spec, hut_panel_pos(), HUT_PANEL_W, HUT_PANEL_KINDS, &hut_buy_row_y)
}

fn row_at_hut_miner(spec: Vec2) -> Option<PurchaseKind> {
    row_at_panel(
        spec,
        hut_miner_panel_pos(),
        HUT_PANEL_W,
        HUT_MINER_KINDS,
        &hut_miner_buy_row_y,
    )
}

fn row_at_hut_skimmer(spec: Vec2) -> Option<PurchaseKind> {
    row_at_panel(
        spec,
        hut_skimmer_panel_pos(),
        HUT_PANEL_W,
        HUT_SKIMMER_KINDS,
        &hut_skimmer_buy_row_y,
    )
}

fn row_at_hut_fisher(spec: Vec2) -> Option<PurchaseKind> {
    row_at_panel(
        spec,
        hut_fisher_panel_pos(),
        HUT_PANEL_W,
        HUT_FISHER_KINDS,
        &hut_fisher_buy_row_y,
    )
}

fn row_at_pier(spec: Vec2) -> Option<PurchaseKind> {
    row_at_panel(spec, pier_panel_pos(), PIER_PANEL_W, PIER_PANEL_KINDS, &pier_buy_row_y)
}

fn row_at_port(spec: Vec2) -> Option<PurchaseKind> {
    row_at_panel(spec, port_panel_pos(), PORT_PANEL_W, PORT_PANEL_KINDS, &port_buy_row_y)
}

fn row_at_hut_beachcomber(spec: Vec2) -> Option<PurchaseKind> {
    row_at_panel(
        spec,
        hut_beachcomber_panel_pos(),
        HUT_PANEL_W,
        HUT_BEACHCOMBER_KINDS,
        &hut_beachcomber_buy_row_y,
    )
}

fn row_at_hut_stonemason(spec: Vec2) -> Option<PurchaseKind> {
    row_at_panel(
        spec,
        hut_stonemason_panel_pos(),
        HUT_PANEL_W,
        HUT_STONEMASON_KINDS,
        &hut_stonemason_buy_row_y,
    )
}

fn row_at_panel(
    spec: Vec2,
    panel_pos: Vec2,
    panel_w: f32,
    kinds: &[PurchaseKind],
    row_y: &dyn Fn(usize) -> f32,
) -> Option<PurchaseKind> {
    let half_h = ROW_HEIGHT * 0.5;
    let half_w = (panel_w - PANEL_INSET * 2.0) * 0.5;
    if (spec.x - panel_pos.x).abs() > half_w {
        return None;
    }
    for (i, kind) in kinds.iter().enumerate() {
        if (spec.y - row_y(i)).abs() <= half_h {
            return Some(*kind);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Visibility — chrome (per-panel) and rows (per-button)
//
// Bevy 0.18's borrow checker complains when a single system holds too
// many `Query<&mut Visibility>` parameters even with disjoint filters,
// so chrome and rows are split into two systems. They run in order so
// the row pass can override row-level visibility after chrome has
// blanket-set it.
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub(super) fn update_chrome_visibility(
    hut: Res<Hut>,
    miner_hut: Res<MinerHut>,
    skimmer_hut: Res<SkimmerHut>,
    fisher_hut: Res<FisherHut>,
    bc_hut: Res<BeachcomberHut>,
    sm_hut: Res<StonemasonHut>,
    pier: Res<Pier>,
    port: Res<Port>,
    hover: Res<HoverState>,
    mut chrome_q: Query<(
        &PanelTag,
        Option<&PanelChromePart>,
        Option<&DetailHeader>,
        Option<&DetailBody>,
        &mut Visibility,
    )>,
) {
    if !hut.is_changed()
        && !miner_hut.is_changed()
        && !skimmer_hut.is_changed()
        && !fisher_hut.is_changed()
        && !bc_hut.is_changed()
        && !sm_hut.is_changed()
        && !pier.is_changed()
        && !port.is_changed()
        && !hover.is_changed()
    {
        return;
    }
    let panel_open = |k: PanelKind| -> bool {
        match k {
            PanelKind::Cave => hover.cave,
            PanelKind::Hut => hut.owned && hover.hut,
            PanelKind::HutMiner => miner_hut.owned && hover.hut_miner,
            PanelKind::HutSkimmer => skimmer_hut.owned && hover.hut_skimmer,
            PanelKind::HutFisher => fisher_hut.owned && hover.hut_fisher,
            PanelKind::HutBeachcomber => bc_hut.owned && hover.hut_beachcomber,
            PanelKind::HutStonemason => sm_hut.owned && hover.hut_stonemason,
            PanelKind::Pier => pier.owned && hover.pier,
            PanelKind::Port => port.owned && hover.port,
        }
    };
    // Detail container shows only when a row in the *same panel* is
    // currently hovered — no row hover ⇒ no detail box rendered.
    let row_panel = hover.row.map(panel_for);
    for (tag, part, header, body, mut v) in &mut chrome_q {
        let panel_v = panel_open(tag.0);
        let is_detail_chrome = matches!(
            part,
            Some(
                PanelChromePart::DetailBg
                    | PanelChromePart::DetailBorder
                    | PanelChromePart::DetailPointer(_)
            ),
        ) || header.is_some()
            || body.is_some();
        let target = vis(if is_detail_chrome {
            panel_v && row_panel == Some(tag.0)
        } else {
            panel_v
        });
        if *v != target {
            *v = target;
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn update_row_visibility(
    hut: Res<Hut>,
    miner_hut: Res<MinerHut>,
    skimmer_hut: Res<SkimmerHut>,
    fisher_hut: Res<FisherHut>,
    bc_hut: Res<BeachcomberHut>,
    sm_hut: Res<StonemasonHut>,
    pier: Res<Pier>,
    port: Res<Port>,
    workers: Res<Workers>,
    skims: Res<Skims>,
    hover: Res<HoverState>,
    mut rows: ParamSet<(
        Query<(&PurchaseButton, &mut Visibility), Without<PanelTag>>,
        Query<(&ButtonLabel, &mut Visibility), Without<PanelTag>>,
        Query<(&ButtonCost, &mut Visibility), Without<PanelTag>>,
        Query<(&ButtonCount, &mut Visibility), Without<PanelTag>>,
    )>,
) {
    if !hut.is_changed()
        && !miner_hut.is_changed()
        && !skimmer_hut.is_changed()
        && !fisher_hut.is_changed()
        && !bc_hut.is_changed()
        && !sm_hut.is_changed()
        && !pier.is_changed()
        && !port.is_changed()
        && !workers.is_changed()
        && !skims.is_changed()
        && !hover.is_changed()
    {
        return;
    }
    let panel_open = |k: PurchaseKind| -> bool {
        match panel_for(k) {
            PanelKind::Cave => hover.cave,
            PanelKind::Hut => hut.owned && hover.hut,
            PanelKind::HutMiner => miner_hut.owned && hover.hut_miner,
            PanelKind::HutSkimmer => skimmer_hut.owned && hover.hut_skimmer,
            PanelKind::HutFisher => fisher_hut.owned && hover.hut_fisher,
            PanelKind::HutBeachcomber => bc_hut.owned && hover.hut_beachcomber,
            PanelKind::HutStonemason => sm_hut.owned && hover.hut_stonemason,
            PanelKind::Pier => pier.owned && hover.pier,
            PanelKind::Port => port.owned && hover.port,
        }
    };
    let visible = |k: PurchaseKind| -> bool {
        panel_open(k)
            && row_visible(k, &hut, &miner_hut, &skimmer_hut, &fisher_hut, &pier, &port)
    };
    for (btn, mut v) in &mut rows.p0() {
        let target = vis(visible(btn.kind));
        if *v != target { *v = target; }
    }
    for (label, mut v) in &mut rows.p1() {
        let target = vis(visible(label.0));
        if *v != target { *v = target; }
    }
    for (cost, mut v) in &mut rows.p2() {
        let target = vis(visible(cost.0));
        if *v != target { *v = target; }
    }
    for (count, mut v) in &mut rows.p3() {
        let target = vis(visible(count.0));
        if *v != target { *v = target; }
    }
}

/// Which panel does a purchase row live in.
fn panel_for(k: PurchaseKind) -> PanelKind {
    match k {
        PurchaseKind::Hut
        | PurchaseKind::HutMiner
        | PurchaseKind::HutSkimmer
        | PurchaseKind::HutFisher
        | PurchaseKind::HutBeachcomber
        | PurchaseKind::HutStonemason
        | PurchaseKind::Pier => PanelKind::Cave,
        PurchaseKind::Worker => PanelKind::Hut,
        PurchaseKind::Miner | PurchaseKind::MinerDamage => PanelKind::HutMiner,
        PurchaseKind::Skimmer | PurchaseKind::SkimUpgrade => PanelKind::HutSkimmer,
        PurchaseKind::Fisherman => PanelKind::HutFisher,
        PurchaseKind::Beachcomber => PanelKind::HutBeachcomber,
        PurchaseKind::Stonemason => PanelKind::HutStonemason,
        PurchaseKind::Fish | PurchaseKind::Port => PanelKind::Pier,
        PurchaseKind::Boatman => PanelKind::Port,
    }
}

fn vis(b: bool) -> Visibility {
    if b {
        Visibility::Visible
    } else {
        Visibility::Hidden
    }
}

// ---------------------------------------------------------------------------
// Button visuals + count text
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub(super) fn update_button_visuals(
    skims: Res<Skims>,
    hut: Res<Hut>,
    miner_hut: Res<MinerHut>,
    skimmer_hut: Res<SkimmerHut>,
    fisher_hut: Res<FisherHut>,
    bc_hut: Res<BeachcomberHut>,
    sm_hut: Res<StonemasonHut>,
    pier: Res<Pier>,
    port: Res<Port>,
    workers: Res<Workers>,
    upgrades: UpgradeRes,
    hover: Res<HoverState>,
    mut bg_q: Query<(&PurchaseButton, &mut Sprite)>,
    mut label_q: Query<(&ButtonLabel, &mut TextColor), Without<ButtonCost>>,
    mut cost_q: Query<(&ButtonCost, &mut TextColor), Without<ButtonLabel>>,
) {
    if !skims.is_changed()
        && !hut.is_changed()
        && !miner_hut.is_changed()
        && !skimmer_hut.is_changed()
        && !fisher_hut.is_changed()
        && !bc_hut.is_changed()
        && !sm_hut.is_changed()
        && !pier.is_changed()
        && !port.is_changed()
        && !workers.is_changed()
        && !upgrades.skim.is_changed()
        && !upgrades.miner.is_changed()
        && !hover.is_changed()
    {
        return;
    }
    let active = |k: PurchaseKind| -> bool {
        button_active(
            k, &hut, &miner_hut, &skimmer_hut, &fisher_hut, &bc_hut, &sm_hut, &pier, &port, &hover,
        )
    };
    let afford = |k: PurchaseKind| -> bool {
        can_afford(
            k, &skims, &hut, &miner_hut, &skimmer_hut, &fisher_hut, &bc_hut, &sm_hut, &pier, &port,
            &workers, &upgrades.skim, &upgrades.miner,
        )
    };
    let visible = |k: PurchaseKind| -> bool {
        row_visible(k, &hut, &miner_hut, &skimmer_hut, &fisher_hut, &pier, &port)
    };
    let sold_out = |k: PurchaseKind| -> bool {
        is_sold_out(
            k, &hut, &miner_hut, &skimmer_hut, &fisher_hut, &bc_hut, &sm_hut, &pier, &port,
            &upgrades.skim, &upgrades.miner,
        )
    };
    for (btn, mut sprite) in &mut bg_q {
        if !visible(btn.kind) { continue; }
        let row_hovered = hover.row == Some(btn.kind);
        sprite.color = if sold_out(btn.kind) {
            // Already-bought one-time row — darkened so it reads as
            // "owned" rather than "available".
            colors::BUTTON_BG_DIM
        } else if !active(btn.kind) {
            // Visible but locked (prereq somewhere not met).
            colors::BUTTON_BG_DIM
        } else if row_hovered && afford(btn.kind) {
            colors::BUTTON_BG_HOVER
        } else if afford(btn.kind) {
            colors::BUTTON_BG
        } else {
            colors::BUTTON_BG_DIM
        };
    }
    for (label, mut color) in &mut label_q {
        if !visible(label.0) { continue; }
        color.0 = if sold_out(label.0) || !active(label.0) || !afford(label.0) {
            colors::BUTTON_DIM_TEXT
        } else {
            colors::BUTTON_BORDER
        };
    }
    for (cost, mut color) in &mut cost_q {
        if !visible(cost.0) { continue; }
        color.0 = if sold_out(cost.0) || !active(cost.0) || !afford(cost.0) {
            colors::BUTTON_DIM_TEXT
        } else {
            colors::YELLOW
        };
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn update_count_text(
    workers: Res<Workers>,
    miners: Res<Miners>,
    skimmers: Res<Skimmers>,
    fishermen: Res<Fishermen>,
    fishes: Res<Fishes>,
    upgrades: Res<SkimUpgrades>,
    miner_upgrades: Res<MinerUpgrades>,
    beachcombers: Res<Beachcombers>,
    stonemasons: Res<Stonemasons>,
    boatmen: Res<Boatmen>,
    mut q: Query<(&ButtonCount, &mut Text2d)>,
) {
    if !workers.is_changed()
        && !miners.is_changed()
        && !skimmers.is_changed()
        && !fishermen.is_changed()
        && !fishes.is_changed()
        && !upgrades.is_changed()
        && !miner_upgrades.is_changed()
        && !beachcombers.is_changed()
        && !stonemasons.is_changed()
        && !boatmen.is_changed()
    {
        return;
    }
    for (tag, mut text) in &mut q {
        text.0 = match tag.0 {
            PurchaseKind::Worker => format!("x{}", workers.count),
            PurchaseKind::Miner => format!("x{}", miners.count),
            PurchaseKind::MinerDamage => format!("L{}", miner_upgrades.damage_level),
            PurchaseKind::Skimmer => format!("x{}", skimmers.count),
            PurchaseKind::SkimUpgrade => format!("L{}", upgrades.level),
            PurchaseKind::Fisherman => format!("x{}", fishermen.count),
            PurchaseKind::Beachcomber => format!("x{}", beachcombers.count),
            PurchaseKind::Stonemason => format!("x{}", stonemasons.count),
            PurchaseKind::Boatman => format!("x{}", boatmen.count),
            PurchaseKind::Fish => format!("x{}", fishes.count),
            // One-time structures don't show a count column.
            PurchaseKind::Hut
            | PurchaseKind::HutMiner
            | PurchaseKind::HutSkimmer
            | PurchaseKind::HutFisher
            | PurchaseKind::HutBeachcomber
            | PurchaseKind::HutStonemason
            | PurchaseKind::Pier
            | PurchaseKind::Port => String::new(),
        };
    }
}

/// Refresh dynamic cost text. Most rows have static costs and use
/// the `kind.cost_label()` baked in at spawn. Worker is the only
/// one whose price scales (1.2× per previous purchase), so we
/// rewrite its cost text whenever `Workers` changes.
pub(super) fn update_dynamic_cost_text(
    workers: Res<Workers>,
    mut q: Query<(&ButtonCost, &mut Text2d)>,
) {
    if !workers.is_changed() {
        return;
    }
    let worker_label = format!("{}", current_worker_cost(&workers));
    for (cost, mut text) in &mut q {
        if cost.0 == PurchaseKind::Worker && text.0 != worker_label {
            text.0 = worker_label.clone();
        }
    }
}

// ---------------------------------------------------------------------------
// Detail panel content
// ---------------------------------------------------------------------------

struct Detail {
    header: &'static str,
    body: &'static str,
}

fn building_header(afford: bool, sold_out: bool) -> &'static str {
    if sold_out {
        "Purchased"
    } else if afford {
        "Buy!"
    } else {
        "Locked"
    }
}

fn detail_for(kind: PurchaseKind, afford: bool, sold_out: bool) -> Detail {
    match kind {
        PurchaseKind::Hut => Detail {
            header: building_header(afford, sold_out),
            body: "Foragers hut.\n\n- 2 starter workers\n- Unlocks worker\n  buys",
        },
        PurchaseKind::HutMiner => Detail {
            header: building_header(afford, sold_out),
            body: "Miners hut.\n\n- 2 starter workers\n- Unlocks miners\n- Gates the next\n  two huts",
        },
        PurchaseKind::HutSkimmer => Detail {
            header: building_header(afford, sold_out),
            body: "Skimmers hut.\n\n- 2 starter workers\n- Unlocks skimmers\n- Sells skim\n  upgrades",
        },
        PurchaseKind::HutFisher => Detail {
            header: building_header(afford, sold_out),
            body: "Anglers hut.\n\n- 2 starter workers\n- Unlocks fishermen",
        },
        PurchaseKind::HutBeachcomber => Detail {
            header: building_header(afford, sold_out),
            body: "Combers hut.\n\n- 2 starter workers\n- Unlocks combers",
        },
        PurchaseKind::HutStonemason => Detail {
            header: building_header(afford, sold_out),
            body: "Masons hut.\n\n- 2 starter workers\n- Unlocks masons",
        },
        PurchaseKind::Worker => Detail {
            header: if afford { "Buy!" } else { "Need 10 skims" },
            body: "A forager who idles\nnear the hut.\n\n- Adds 1 worker\n- Convertible to any\n  specialist role",
        },
        PurchaseKind::Miner => Detail {
            header: if afford { "Buy!" } else { "Need 1 worker" },
            body: "Pickaxe-throwing\nspecialist.\n\n- Hits the big rock\n- 3 damage per throw\n- ~10s cycle",
        },
        PurchaseKind::MinerDamage => Detail {
            header: if afford { "Buy!" } else { "Need 30 skims" },
            body: "Sharper pickaxes.\n\n- +1 damage per\n  miner throw\n- Stacks linearly\n- All miners benefit",
        },
        PurchaseKind::Skimmer => Detail {
            header: if afford { "Buy!" } else { "Need 1 worker" },
            body: "Picks idle stones up\nand skims them.\n\n- 25% bounce chance\n  (worse than you!)\n- ~15s cycle",
        },
        PurchaseKind::SkimUpgrade => Detail {
            header: if afford { "Buy!" } else { "Need 25 skims" },
            body: "Better technique.\n\n- +5% bounce chance\n  for skimmers only\n- Stacks linearly\n- Caps at 95%",
        },
        PurchaseKind::Fisherman => Detail {
            header: if afford { "Buy!" } else { "Need 1 worker" },
            body: "Fishes stones from\nthe sea.\n\n- 7-13s per cast\n- 50% catch chance",
        },
        PurchaseKind::Pier => Detail {
            header: if sold_out {
                "Purchased"
            } else if afford {
                "Buy!"
            } else {
                "Need 30 skims"
            },
            body: "A wooden pier into\nthe water.\n\n- Unlocks fish buys\n- Comes with 1 fish",
        },
        PurchaseKind::Fish => Detail {
            header: if afford { "Buy!" } else { "Need 5 skims" },
            body: "A bucket of 10 fish.\n\n- Each fish saves 1\n  failing bounce\n- Consumed on rescue\n- Restock anytime",
        },
        PurchaseKind::Beachcomber => Detail {
            header: if afford { "Buy!" } else { "Need 1 worker" },
            body: "Walks the sand with\na shovel digging up\nstones.\n\n- Free rocks every\n  ~8s",
        },
        PurchaseKind::Stonemason => Detail {
            header: if afford { "Buy!" } else { "Need 1 worker" },
            body: "Sharpens idle stones.\nMasoned stones get\n+1 guaranteed bounce.",
        },
        PurchaseKind::Boatman => Detail {
            header: if afford { "Buy!" } else { "Need 1 worker" },
            body: "Sails out from the\nport, ferrying sunken\nstones back to shore.\n\n- Carries up to 5",
        },
        PurchaseKind::Port => Detail {
            header: if sold_out {
                "Purchased"
            } else if afford {
                "Buy!"
            } else {
                "Need 50 skims"
            },
            body: "Wooden dock east of\nthe pier. Unlocks the\nBoatman conversion.",
        },
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn update_detail_text(
    hover: Res<HoverState>,
    skims: Res<Skims>,
    hut: Res<Hut>,
    miner_hut: Res<MinerHut>,
    skimmer_hut: Res<SkimmerHut>,
    fisher_hut: Res<FisherHut>,
    bc_hut: Res<BeachcomberHut>,
    sm_hut: Res<StonemasonHut>,
    pier: Res<Pier>,
    port: Res<Port>,
    workers: Res<Workers>,
    upgrades: UpgradeRes,
    mut header_q: Query<(&DetailHeader, &mut Text2d, &mut TextColor), Without<DetailBody>>,
    mut body_q: Query<(&DetailBody, &mut Text2d, &mut TextColor), Without<DetailHeader>>,
) {
    if !hover.is_changed()
        && !skims.is_changed()
        && !hut.is_changed()
        && !miner_hut.is_changed()
        && !skimmer_hut.is_changed()
        && !fisher_hut.is_changed()
        && !bc_hut.is_changed()
        && !sm_hut.is_changed()
        && !pier.is_changed()
        && !port.is_changed()
        && !workers.is_changed()
        && !upgrades.skim.is_changed()
        && !upgrades.miner.is_changed()
    {
        return;
    }

    let detail_for_panel = |panel: PanelKind| -> Option<Detail> {
        let kind = hover.row?;
        if panel_for(kind) != panel {
            return None;
        }
        let afford = can_afford(
            kind, &skims, &hut, &miner_hut, &skimmer_hut, &fisher_hut, &bc_hut, &sm_hut, &pier,
            &port, &workers, &upgrades.skim, &upgrades.miner,
        );
        let sold_out = is_sold_out(
            kind, &hut, &miner_hut, &skimmer_hut, &fisher_hut, &bc_hut, &sm_hut, &pier, &port,
            &upgrades.skim, &upgrades.miner,
        );
        Some(detail_for(kind, afford, sold_out))
    };

    for panel in [
        PanelKind::Cave,
        PanelKind::Hut,
        PanelKind::HutMiner,
        PanelKind::HutSkimmer,
        PanelKind::HutFisher,
        PanelKind::HutBeachcomber,
        PanelKind::HutStonemason,
        PanelKind::Pier,
        PanelKind::Port,
    ] {
        apply_detail(&mut header_q, &mut body_q, panel, detail_for_panel(panel).as_ref());
    }
}

fn apply_detail(
    header_q: &mut Query<(&DetailHeader, &mut Text2d, &mut TextColor), Without<DetailBody>>,
    body_q: &mut Query<(&DetailBody, &mut Text2d, &mut TextColor), Without<DetailHeader>>,
    panel: PanelKind,
    detail: Option<&Detail>,
) {
    for (h, mut text, mut color) in header_q.iter_mut() {
        if h.0 != panel {
            continue;
        }
        match detail {
            Some(d) => {
                text.0 = d.header.to_string();
                color.0 = if d.header == "Buy!" {
                    colors::DETAIL_OK
                } else if d.header == "Purchased" {
                    colors::YELLOW
                } else {
                    colors::DETAIL_LOCKED
                };
            }
            None => text.0.clear(),
        }
    }
    for (b, mut text, mut color) in body_q.iter_mut() {
        if b.0 != panel {
            continue;
        }
        match detail {
            Some(d) => {
                text.0 = d.body.to_string();
                color.0 = colors::FG_DIM;
            }
            None => text.0.clear(),
        }
    }
}
