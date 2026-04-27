//! Per-frame dynamic layout for the cave panel.
//!
//! The cave panel is the only one whose row count varies at runtime
//! (rows reveal themselves as progression unlocks each one). Every
//! other panel has a fixed row count and stays at the geometry it
//! was spawned with.
//!
//! What this module does, every frame:
//!
//! * Computes which cave-panel rows are currently `row_visible` — the
//!   set of rows that should appear at all (buyable or sold out, but
//!   not gated behind unmet prereqs).
//! * Picks a panel size that fits exactly that many rows, anchors the
//!   panel's **bottom** at a fixed y so it grows upward (the pointer
//!   stays put on the cave), and writes the result into the
//!   [`CavePanelGeo`] resource for other systems (hover detection,
//!   visibility) to read.
//! * Re-positions all cave chrome entities (border, BG, title,
//!   divider, pointer segments, detail border/BG) and the cave row
//!   entities (button background + 3 row texts each) so the visible
//!   rows are tightly packed in the new panel.

use bevy::prelude::*;

use crate::core::common::Pos;
use crate::render::UiText;

use super::layout::{
    cave_detail_anchor, cave_panel_anchor, panel_height_for, CAVE_DETAIL_GAP, CAVE_DETAIL_W,
    CAVE_PANEL_W, PANEL_BORDER_W, PANEL_INSET, ROW_HEIGHT,
};
use super::purchase::row_visible;
use super::{
    ButtonCost, ButtonCount, ButtonLabel, CavePanelGeo, DetailBody, DetailHeader, FisherHut, Hut,
    MinerHut, PanelChromePart, PanelKind, PanelTag, Pier, PurchaseButton, PurchaseKind, SkimmerHut,
    CAVE_PANEL_KINDS,
};

/// Run the cave panel relayout — recompute geometry, write
/// `CavePanelGeo`, and reposition every cave chrome / row entity.
#[allow(clippy::too_many_arguments)]
pub(super) fn relayout_cave_panel(
    hut: Res<Hut>,
    miner_hut: Res<MinerHut>,
    skimmer_hut: Res<SkimmerHut>,
    fisher_hut: Res<FisherHut>,
    pier: Res<Pier>,
    mut geo: ResMut<CavePanelGeo>,
    mut chrome_sprites: Query<
        (&PanelTag, &PanelChromePart, &mut Pos, &mut Sprite),
        Without<PurchaseButton>,
    >,
    mut chrome_text: Query<
        (&PanelTag, &PanelChromePart, &mut UiText),
        (Without<Sprite>, Without<ButtonLabel>, Without<ButtonCost>, Without<ButtonCount>),
    >,
    mut row_buttons: Query<
        (&PurchaseButton, &mut Pos, &mut Sprite),
        Without<PanelChromePart>,
    >,
    mut row_labels: Query<
        (&ButtonLabel, &mut UiText),
        (Without<PanelChromePart>, Without<ButtonCost>, Without<ButtonCount>),
    >,
    mut row_costs: Query<
        (&ButtonCost, &mut UiText),
        (Without<PanelChromePart>, Without<ButtonLabel>, Without<ButtonCount>),
    >,
    mut row_counts: Query<
        (&ButtonCount, &mut UiText),
        (Without<PanelChromePart>, Without<ButtonLabel>, Without<ButtonCost>),
    >,
    mut detail_headers: Query<
        (&DetailHeader, &mut UiText),
        (Without<PanelChromePart>, Without<DetailBody>, Without<ButtonLabel>, Without<ButtonCost>, Without<ButtonCount>),
    >,
    mut detail_bodies: Query<
        (&DetailBody, &mut UiText),
        (Without<PanelChromePart>, Without<DetailHeader>, Without<ButtonLabel>, Without<ButtonCost>, Without<ButtonCount>),
    >,
) {
    // Skip the work entirely if no resource that affects the cave's
    // visible-row set has changed since last frame.
    if !hut.is_changed()
        && !miner_hut.is_changed()
        && !skimmer_hut.is_changed()
        && !fisher_hut.is_changed()
        && !pier.is_changed()
    {
        // First-frame initialisation: the resource starts at zeroed
        // defaults, so still run once if the count is zero.
        if geo.row_count > 0 {
            return;
        }
    }

    let visible: Vec<PurchaseKind> = CAVE_PANEL_KINDS
        .iter()
        .copied()
        .filter(|k| row_visible(*k, &hut, &miner_hut, &skimmer_hut, &fisher_hut, &pier))
        .collect();
    let count = visible.len();
    if count == 0 {
        return;
    }

    // Compute the new geometry. The cave panel sits east of the
    // cave with a westward pointer (matching the hut panels), and
    // grows symmetrically around the cave's vertical center as rows
    // are added.
    let panel_h = panel_height_for(count);
    let panel_size = Vec2::new(CAVE_PANEL_W, panel_h);
    let anchor = cave_panel_anchor();
    let panel_pos = Vec2::new(anchor.center_x, anchor.center_y);
    let panel_top = panel_pos.y - panel_h * 0.5;
    let row_w = panel_size.x - PANEL_INSET * 2.0;

    // Detail panel sits east of the main panel, vertically centered
    // on it. Detail height matches the main panel height, with a
    // sensible minimum so single-row panels still have legible body
    // text below the header.
    let detail = cave_detail_anchor(panel_pos, panel_h);
    let detail_size = Vec2::new(CAVE_DETAIL_W, detail.height);
    let detail_pos = Vec2::new(detail.x, detail.y);

    geo.panel_pos = panel_pos;
    geo.panel_size = panel_size;
    geo.detail_pos = detail_pos;
    geo.detail_size = detail_size;
    geo.row_count = count as u8;

    let outer_size = panel_size + Vec2::splat(PANEL_BORDER_W * 2.0);
    let detail_outer = detail_size + Vec2::splat(PANEL_BORDER_W * 2.0);

    // Sprite-bearing chrome: position + custom_size.
    for (tag, role, mut pos, mut sprite) in &mut chrome_sprites {
        if tag.0 != PanelKind::Cave {
            continue;
        }
        match role {
            PanelChromePart::Border => {
                pos.0 = panel_pos;
                sprite.custom_size = Some(outer_size);
            }
            PanelChromePart::Bg => {
                pos.0 = panel_pos;
                sprite.custom_size = Some(panel_size);
            }
            PanelChromePart::Divider => {
                pos.0 = Vec2::new(panel_pos.x, panel_top + 6.0);
                sprite.custom_size = Some(Vec2::new(row_w, 1.0));
            }
            PanelChromePart::Pointer(seg) => {
                // Cave's pointer points west — anchored to the
                // panel's left edge at the panel's vertical center.
                let heights = [5.0, 3.0, 1.0];
                let i = (*seg as usize).min(heights.len() - 1);
                let h = heights[i];
                let left_x = panel_pos.x - panel_size.x * 0.5 - PANEL_BORDER_W;
                pos.0 = Vec2::new(left_x - 0.5 - i as f32, panel_pos.y);
                sprite.custom_size = Some(Vec2::new(1.0, h));
            }
            PanelChromePart::DetailBorder => {
                pos.0 = detail_pos;
                sprite.custom_size = Some(detail_outer);
            }
            PanelChromePart::DetailBg => {
                pos.0 = detail_pos;
                sprite.custom_size = Some(detail_size);
            }
            PanelChromePart::DetailPointer(_) | PanelChromePart::Title => {}
        }
    }

    // Title text: tracks the panel's top edge.
    for (tag, role, mut ui) in &mut chrome_text {
        if tag.0 != PanelKind::Cave {
            continue;
        }
        if matches!(role, PanelChromePart::Title) {
            ui.spec_pos = Vec2::new(panel_pos.x, panel_top + 3.0);
        }
    }

    // Rows — slot each visible kind to its index in the visible list,
    // converting that to a y position inside the panel. Hidden rows
    // get parked at a sentinel y far off-screen so they don't catch
    // clicks (their visibility is also gated via the row visibility
    // system, but two layers of safety here matter for clicks since
    // those hit-test off `Pos` directly).
    for (i, kind) in visible.iter().enumerate() {
        let row_y = panel_top + 7.0 + ROW_HEIGHT * 0.5 + i as f32 * ROW_HEIGHT;
        update_row_positions(
            *kind,
            Vec2::new(panel_pos.x, row_y),
            row_w,
            &mut row_buttons,
            &mut row_labels,
            &mut row_costs,
            &mut row_counts,
        );
    }

    // Park hidden rows off-screen.
    for kind in CAVE_PANEL_KINDS.iter().copied() {
        if visible.contains(&kind) {
            continue;
        }
        update_row_positions(
            kind,
            Vec2::new(-9999.0, -9999.0),
            row_w,
            &mut row_buttons,
            &mut row_labels,
            &mut row_costs,
            &mut row_counts,
        );
    }

    // Detail header + body positions — anchored just inside the
    // detail panel's top-left corner. As the detail panel shrinks
    // they shift up to stay inside.
    let inner_left = detail_pos.x - detail_size.x * 0.5 + PANEL_INSET;
    let detail_top = detail_pos.y - detail_size.y * 0.5;
    let header_y = detail_top + PANEL_INSET + 2.0;
    let body_y = header_y + 7.0;
    for (h, mut ui) in &mut detail_headers {
        if h.0 != PanelKind::Cave {
            continue;
        }
        ui.spec_pos = Vec2::new(inner_left, header_y);
    }
    for (b, mut ui) in &mut detail_bodies {
        if b.0 != PanelKind::Cave {
            continue;
        }
        ui.spec_pos = Vec2::new(inner_left, body_y);
    }
}

fn update_row_positions(
    kind: PurchaseKind,
    row_pos: Vec2,
    row_w: f32,
    row_buttons: &mut Query<
        (&PurchaseButton, &mut Pos, &mut Sprite),
        Without<PanelChromePart>,
    >,
    row_labels: &mut Query<
        (&ButtonLabel, &mut UiText),
        (Without<PanelChromePart>, Without<ButtonCost>, Without<ButtonCount>),
    >,
    row_costs: &mut Query<
        (&ButtonCost, &mut UiText),
        (Without<PanelChromePart>, Without<ButtonLabel>, Without<ButtonCount>),
    >,
    row_counts: &mut Query<
        (&ButtonCount, &mut UiText),
        (Without<PanelChromePart>, Without<ButtonLabel>, Without<ButtonCost>),
    >,
) {
    let label_x = row_pos.x - row_w * 0.5 + 3.0;
    let cost_x = row_pos.x + row_w * 0.5 - 3.0;
    let count_x = row_pos.x + row_w * 0.5 - 26.0;
    for (btn, mut pos, mut sprite) in row_buttons {
        if btn.kind != kind {
            continue;
        }
        pos.0 = row_pos;
        sprite.custom_size = Some(Vec2::new(row_w, ROW_HEIGHT));
    }
    for (label, mut ui) in row_labels {
        if label.0 != kind {
            continue;
        }
        ui.spec_pos = Vec2::new(label_x, row_pos.y);
    }
    for (cost, mut ui) in row_costs {
        if cost.0 != kind {
            continue;
        }
        ui.spec_pos = Vec2::new(cost_x, row_pos.y);
    }
    for (count, mut ui) in row_counts {
        if count.0 != kind {
            continue;
        }
        ui.spec_pos = Vec2::new(count_x, row_pos.y);
    }
    let _ = CAVE_DETAIL_GAP; // silence unused-import lint when constants change.
}
