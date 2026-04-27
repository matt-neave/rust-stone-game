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
use super::purchase::{is_sold_out, row_visible};
use super::{
    ButtonCost, ButtonCount, ButtonLabel, CavePanelGeo, DetailBody, DetailHeader, FisherHut,
    Fishermen, Fishes, HoverState, Hut, MinerHut, Miners, PanelChromePart, PanelKind, PanelTag,
    Pier, PurchaseButton, PurchaseKind, SkimUpgrades, SkimmerHut, Skimmers, Workers,
    CAVE_PANEL_KINDS, HUT_FISHER_KINDS, HUT_MINER_KINDS, HUT_PANEL_KINDS, HUT_SKIMMER_KINDS,
    PIER_PANEL_KINDS,
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
    pier: Res<Pier>,
    cave_geo: Res<CavePanelGeo>,
    mut hover: ResMut<HoverState>,
) {
    // In docked mode the game plays out without UI — clear all hover
    // state and bail before reading the cursor.
    if *mode == DisplayMode::Docked {
        if hover.cave || hover.hut || hover.pier || hover.row.is_some() {
            *hover = HoverState::default();
        }
        return;
    }
    let Ok(window) = windows.single() else {
        if hover.cave || hover.hut || hover.pier || hover.row.is_some() {
            *hover = HoverState::default();
        }
        return;
    };
    let cursor = cursor_to_spec(window, display_scale.0);
    // Two-stage hover model with mouse blocking:
    //   * A panel **opens** only when the cursor is on its
    //     structure footprint. Hovering empty space where the
    //     panel *would* render never opens it.
    //   * A panel **persists** while the cursor sits over its
    //     own panel/detail chrome.
    //   * When the cursor is on the chrome of any open panel,
    //     that panel claims the cursor — building hovers under
    //     the chrome are suppressed, so a chrome overlay can't
    //     leak hover events through to a building beneath it.
    let cave_dynamic_rects = cave_panel_rects_from_geo(&cave_geo);

    let cave_chrome = match cursor {
        Some(s) => hover.cave && in_any_box(s, &cave_dynamic_rects),
        None => false,
    };
    let hut_chrome = match cursor {
        Some(s) => hover.hut && in_any_box(s, &hut_panel_rects()),
        None => false,
    };
    let miner_chrome = match cursor {
        Some(s) => hover.hut_miner && in_any_box(s, &hut_miner_panel_rects()),
        None => false,
    };
    let skimmer_chrome = match cursor {
        Some(s) => hover.hut_skimmer && in_any_box(s, &hut_skimmer_panel_rects()),
        None => false,
    };
    let fisher_chrome = match cursor {
        Some(s) => hover.hut_fisher && in_any_box(s, &hut_fisher_panel_rects()),
        None => false,
    };
    let pier_chrome = match cursor {
        Some(s) => hover.pier && in_any_box(s, &pier_panel_rects()),
        None => false,
    };

    let chrome_claim = cave_chrome
        || hut_chrome
        || miner_chrome
        || skimmer_chrome
        || fisher_chrome
        || pier_chrome;

    let cave_building = match cursor {
        Some(s) => in_any_box(s, &cave_building_rects()),
        None => false,
    };
    let hut_building = match cursor {
        Some(s) => in_any_box(s, &hut_building_rects()),
        None => false,
    };
    let miner_building = match cursor {
        Some(s) => in_any_box(s, &hut_miner_building_rects()),
        None => false,
    };
    let skimmer_building = match cursor {
        Some(s) => in_any_box(s, &hut_skimmer_building_rects()),
        None => false,
    };
    let fisher_building = match cursor {
        Some(s) => in_any_box(s, &hut_fisher_building_rects()),
        None => false,
    };
    let pier_building = match cursor {
        Some(s) => in_any_box(s, &pier_building_rects()),
        None => false,
    };

    let cave_zone = if chrome_claim { cave_chrome } else { cave_building };
    let hut_zone = if chrome_claim { hut_chrome } else { hut_building };
    let miner_zone = if chrome_claim { miner_chrome } else { miner_building };
    let skimmer_zone = if chrome_claim { skimmer_chrome } else { skimmer_building };
    let fisher_zone = if chrome_claim { fisher_chrome } else { fisher_building };
    let pier_zone = if chrome_claim { pier_chrome } else { pier_building };

    // Row detection — only meaningful when the panel that owns the
    // row is currently active. The cave's row layout is dynamic, so
    // we test against the live `CavePanelGeo` + visible-row list
    // rather than the static `cave_buy_row_y` indices.
    let row = match cursor {
        None => None,
        Some(spec) => {
            if cave_zone {
                cave_row_at(spec, &cave_geo, &cave_visible_kinds(&hut, &miner_hut, &skimmer_hut, &fisher_hut, &pier))
            } else if hut.owned && hut_zone {
                row_at_hut(spec)
            } else if hut.owned && miner_zone {
                row_at_hut_miner(spec)
            } else if hut.owned && skimmer_zone {
                row_at_hut_skimmer(spec)
            } else if hut.owned && fisher_zone {
                row_at_hut_fisher(spec)
            } else if pier.owned && pier_zone {
                row_at_pier(spec)
            } else {
                None
            }
        }
    };

    if hover.cave != cave_zone {
        hover.cave = cave_zone;
    }
    if hover.hut != hut_zone {
        hover.hut = hut_zone;
    }
    if hover.hut_miner != miner_zone {
        hover.hut_miner = miner_zone;
    }
    if hover.hut_skimmer != skimmer_zone {
        hover.hut_skimmer = skimmer_zone;
    }
    if hover.hut_fisher != fisher_zone {
        hover.hut_fisher = fisher_zone;
    }
    if hover.pier != pier_zone {
        hover.pier = pier_zone;
    }
    if hover.row != row {
        hover.row = row;
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

fn cave_visible_kinds(
    hut: &Hut,
    miner_hut: &MinerHut,
    skimmer_hut: &SkimmerHut,
    fisher_hut: &FisherHut,
    pier: &Pier,
) -> Vec<PurchaseKind> {
    CAVE_PANEL_KINDS
        .iter()
        .copied()
        .filter(|k| row_visible(*k, hut, miner_hut, skimmer_hut, fisher_hut, pier))
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

pub(super) fn update_chrome_visibility(
    hut: Res<Hut>,
    miner_hut: Res<MinerHut>,
    skimmer_hut: Res<SkimmerHut>,
    fisher_hut: Res<FisherHut>,
    pier: Res<Pier>,
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
        && !pier.is_changed()
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
            PanelKind::Pier => pier.owned && hover.pier,
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
    pier: Res<Pier>,
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
        && !pier.is_changed()
        && !workers.is_changed()
        && !skims.is_changed()
        && !hover.is_changed()
    {
        return;
    }
    // A row is *shown* (visible chrome — possibly darkened) when it
    // passes `row_visible` AND its panel is currently hovered. The
    // narrower `button_active` predicate decides whether clicking
    // the row should fire a purchase, but doesn't affect visibility
    // any more — sold-out and locked rows still render so the
    // player sees what they've built / what's coming.
    let panel_open = |k: PurchaseKind| -> bool {
        match panel_for(k) {
            PanelKind::Cave => hover.cave,
            PanelKind::Hut => hut.owned && hover.hut,
            PanelKind::HutMiner => miner_hut.owned && hover.hut_miner,
            PanelKind::HutSkimmer => skimmer_hut.owned && hover.hut_skimmer,
            PanelKind::HutFisher => fisher_hut.owned && hover.hut_fisher,
            PanelKind::Pier => pier.owned && hover.pier,
        }
    };
    let visible = |k: PurchaseKind| -> bool {
        panel_open(k)
            && row_visible(k, &hut, &miner_hut, &skimmer_hut, &fisher_hut, &pier)
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
        | PurchaseKind::Pier => PanelKind::Cave,
        PurchaseKind::Worker => PanelKind::Hut,
        PurchaseKind::Miner => PanelKind::HutMiner,
        PurchaseKind::Skimmer | PurchaseKind::SkimUpgrade => PanelKind::HutSkimmer,
        PurchaseKind::Fisherman => PanelKind::HutFisher,
        PurchaseKind::Fish => PanelKind::Pier,
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
    pier: Res<Pier>,
    workers: Res<Workers>,
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
        && !pier.is_changed()
        && !workers.is_changed()
        && !hover.is_changed()
    {
        return;
    }
    let active = |k: PurchaseKind| -> bool {
        button_active(k, &hut, &miner_hut, &skimmer_hut, &fisher_hut, &pier, &hover)
    };
    let afford = |k: PurchaseKind| -> bool {
        can_afford(
            k,
            &skims,
            &hut,
            &miner_hut,
            &skimmer_hut,
            &fisher_hut,
            &pier,
            &workers,
        )
    };
    let visible = |k: PurchaseKind| -> bool {
        row_visible(k, &hut, &miner_hut, &skimmer_hut, &fisher_hut, &pier)
    };
    let sold_out = |k: PurchaseKind| -> bool {
        is_sold_out(k, &hut, &miner_hut, &skimmer_hut, &fisher_hut, &pier)
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

pub(super) fn update_count_text(
    workers: Res<Workers>,
    miners: Res<Miners>,
    skimmers: Res<Skimmers>,
    fishermen: Res<Fishermen>,
    fishes: Res<Fishes>,
    upgrades: Res<SkimUpgrades>,
    mut q: Query<(&ButtonCount, &mut Text2d)>,
) {
    if !workers.is_changed()
        && !miners.is_changed()
        && !skimmers.is_changed()
        && !fishermen.is_changed()
        && !fishes.is_changed()
        && !upgrades.is_changed()
    {
        return;
    }
    for (tag, mut text) in &mut q {
        text.0 = match tag.0 {
            PurchaseKind::Worker => format!("x{}", workers.count),
            PurchaseKind::Miner => format!("x{}", miners.count),
            PurchaseKind::Skimmer => format!("x{}", skimmers.count),
            PurchaseKind::SkimUpgrade => format!("L{}", upgrades.level),
            PurchaseKind::Fisherman => format!("x{}", fishermen.count),
            PurchaseKind::Fish => format!("x{}", fishes.count),
            // One-time structures don't show a count column.
            PurchaseKind::Hut
            | PurchaseKind::HutMiner
            | PurchaseKind::HutSkimmer
            | PurchaseKind::HutFisher
            | PurchaseKind::Pier => String::new(),
        };
    }
}

// ---------------------------------------------------------------------------
// Detail panel content
// ---------------------------------------------------------------------------

struct Detail {
    header: &'static str,
    body: &'static str,
}

fn detail_for(kind: PurchaseKind, afford: bool) -> Detail {
    match kind {
        PurchaseKind::Hut => Detail {
            header: if afford { "Buy!" } else { "Locked" },
            body: "Foragers hut.\n\n- 2 starter workers\n- Unlocks worker\n  buys",
        },
        PurchaseKind::HutMiner => Detail {
            header: if afford { "Buy!" } else { "Locked" },
            body: "Miners hut.\n\n- 2 starter workers\n- Unlocks miners\n- Gates the next\n  two huts",
        },
        PurchaseKind::HutSkimmer => Detail {
            header: if afford { "Buy!" } else { "Locked" },
            body: "Skimmers hut.\n\n- 2 starter workers\n- Unlocks skimmers\n- Sells skim\n  upgrades",
        },
        PurchaseKind::HutFisher => Detail {
            header: if afford { "Buy!" } else { "Locked" },
            body: "Anglers hut.\n\n- 2 starter workers\n- Unlocks fishermen",
        },
        PurchaseKind::Worker => Detail {
            header: if afford { "Buy!" } else { "Need 10 skims" },
            body: "A forager who idles\nnear the hut.\n\n- Adds 1 worker\n- Convertible to any\n  specialist role",
        },
        PurchaseKind::Miner => Detail {
            header: if afford { "Buy!" } else { "Need 1 worker" },
            body: "Pickaxe-throwing\nspecialist.\n\n- Hits the big rock\n- 3 damage per throw\n- ~10s cycle",
        },
        PurchaseKind::Skimmer => Detail {
            header: if afford { "Buy!" } else { "Need 1 worker" },
            body: "Picks idle stones up\nand skims them.\n\n- 25% bounce chance\n  (worse than you!)\n- ~15s cycle",
        },
        PurchaseKind::SkimUpgrade => Detail {
            header: if afford { "Buy!" } else { "Need 25 skims" },
            body: "Better technique.\n\n- +5% bounce chance\n  for player + skimmer\n- Stacks linearly\n- Caps at 95%",
        },
        PurchaseKind::Fisherman => Detail {
            header: if afford { "Buy!" } else { "Need 1 worker" },
            body: "Fishes stones from\nthe sea.\n\n- 7-13s per cast\n- 50% catch chance",
        },
        PurchaseKind::Pier => Detail {
            header: if afford { "Buy!" } else { "Need 30 skims" },
            body: "A wooden pier into\nthe water.\n\n- Unlocks fish buys\n- Comes with 1 fish",
        },
        PurchaseKind::Fish => Detail {
            header: if afford { "Buy!" } else { "Need 5 skims" },
            body: "A bucket of 10 fish.\n\n- Each fish saves 1\n  failing bounce\n- Consumed on rescue\n- Restock anytime",
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
    pier: Res<Pier>,
    workers: Res<Workers>,
    mut header_q: Query<(&DetailHeader, &mut Text2d, &mut TextColor), Without<DetailBody>>,
    mut body_q: Query<(&DetailBody, &mut Text2d, &mut TextColor), Without<DetailHeader>>,
) {
    if !hover.is_changed()
        && !skims.is_changed()
        && !hut.is_changed()
        && !miner_hut.is_changed()
        && !skimmer_hut.is_changed()
        && !fisher_hut.is_changed()
        && !pier.is_changed()
        && !workers.is_changed()
    {
        return;
    }

    // Each row's purchase kind belongs to exactly one panel — that's
    // the panel whose detail box should show its description.
    let kind_to_panel = |k: PurchaseKind| match k {
        PurchaseKind::Hut
        | PurchaseKind::HutMiner
        | PurchaseKind::HutSkimmer
        | PurchaseKind::HutFisher
        | PurchaseKind::Pier => PanelKind::Cave,
        PurchaseKind::Worker => PanelKind::Hut,
        PurchaseKind::Miner => PanelKind::HutMiner,
        PurchaseKind::Skimmer | PurchaseKind::SkimUpgrade => PanelKind::HutSkimmer,
        PurchaseKind::Fisherman => PanelKind::HutFisher,
        PurchaseKind::Fish => PanelKind::Pier,
    };

    let detail_for_panel = |panel: PanelKind| -> Option<Detail> {
        let kind = hover.row?;
        if kind_to_panel(kind) != panel {
            return None;
        }
        Some(detail_for(
            kind,
            can_afford(
                kind,
                &skims,
                &hut,
                &miner_hut,
                &skimmer_hut,
                &fisher_hut,
                &pier,
                &workers,
            ),
        ))
    };

    for panel in [
        PanelKind::Cave,
        PanelKind::Hut,
        PanelKind::HutMiner,
        PanelKind::HutSkimmer,
        PanelKind::HutFisher,
        PanelKind::Pier,
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
