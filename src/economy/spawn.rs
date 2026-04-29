//! Panel spawning — cave panel, hut panel, pier panel, plus their
//! detail panels.
//!
//! All three main panels are data-driven: each iterates its
//! `*_PANEL_KINDS` slice and spawns one buy row per kind, so adding
//! a new variant to a slice grows the panel automatically.
//!
//! Visibility split:
//!
//! * **Chrome** (BG, border, title, dividers, pointer, detail bg/text)
//!   carries the unified `PanelTag(PanelKind)` marker.
//!   `interaction::update_chrome_visibility` toggles every chrome
//!   entity from a single query, branching on its `PanelKind`.
//! * **Rows** (`PurchaseButton` + `ButtonLabel` + `ButtonCost` +
//!   `ButtonCount`) do **not** carry `PanelTag`. Each row's
//!   visibility is gated independently by `button_active(kind)` so a
//!   cave row for an already-built structure can hide while the panel
//!   chrome stays up for the next unbought option.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::text::{Justify, TextLayout};

use crate::core::assets::GameAssets;
use crate::core::colors;
use crate::core::common::{Layer, Pos};
use crate::core::constants::Z_BUTTON;
use crate::render::{UiText, UI_LAYER};

use super::layout::*;
use super::{
    ButtonCost, ButtonCount, ButtonLabel, DetailBody, DetailHeader, PanelChromePart, PanelKind,
    PanelTag, PurchaseButton, PurchaseKind, CAVE_PANEL_KINDS, HUT_AQUA_KINDS,
    HUT_BEACHCOMBER_KINDS, HUT_FISHER_KINDS, HUT_MINER_KINDS, HUT_PANEL_KINDS,
    HUT_RESEARCH_KINDS, HUT_SKIMMER_KINDS, HUT_STONEMASON_KINDS, HUT_TREE_STORAGE_KINDS,
    PIER_PANEL_KINDS, PORT_PANEL_KINDS,
};

pub(super) fn spawn_ui(mut commands: Commands, assets: Res<GameAssets>) {
    spawn_panel(
        &mut commands,
        &assets,
        PanelKind::Cave,
        "FORAGERS  CAVE",
        cave_panel_pos(),
        cave_panel_size(),
        cave_detail_pos(),
        cave_detail_size(),
        CAVE_PANEL_KINDS,
        &cave_buy_row_y,
    );
    spawn_panel(
        &mut commands,
        &assets,
        PanelKind::Hut,
        "FORAGERS  HUT",
        hut_panel_pos(),
        hut_panel_size(),
        hut_detail_pos(),
        hut_detail_size(),
        HUT_PANEL_KINDS,
        &hut_buy_row_y,
    );
    spawn_panel(
        &mut commands,
        &assets,
        PanelKind::HutMiner,
        "MINERS  HUT",
        hut_miner_panel_pos(),
        hut_miner_panel_size(),
        hut_miner_detail_pos(),
        hut_miner_detail_size(),
        HUT_MINER_KINDS,
        &hut_miner_buy_row_y,
    );
    spawn_panel(
        &mut commands,
        &assets,
        PanelKind::HutSkimmer,
        "SKIMMERS  HUT",
        hut_skimmer_panel_pos(),
        hut_skimmer_panel_size(),
        hut_skimmer_detail_pos(),
        hut_skimmer_detail_size(),
        HUT_SKIMMER_KINDS,
        &hut_skimmer_buy_row_y,
    );
    spawn_panel(
        &mut commands,
        &assets,
        PanelKind::HutFisher,
        "ANGLERS  HUT",
        hut_fisher_panel_pos(),
        hut_fisher_panel_size(),
        hut_fisher_detail_pos(),
        hut_fisher_detail_size(),
        HUT_FISHER_KINDS,
        &hut_fisher_buy_row_y,
    );
    spawn_panel(
        &mut commands,
        &assets,
        PanelKind::HutBeachcomber,
        "COMBERS  HUT",
        hut_beachcomber_panel_pos(),
        hut_beachcomber_panel_size(),
        hut_beachcomber_detail_pos(),
        hut_beachcomber_detail_size(),
        HUT_BEACHCOMBER_KINDS,
        &hut_beachcomber_buy_row_y,
    );
    spawn_panel(
        &mut commands,
        &assets,
        PanelKind::HutStonemason,
        "MASONS  HUT",
        hut_stonemason_panel_pos(),
        hut_stonemason_panel_size(),
        hut_stonemason_detail_pos(),
        hut_stonemason_detail_size(),
        HUT_STONEMASON_KINDS,
        &hut_stonemason_buy_row_y,
    );
    spawn_panel(
        &mut commands,
        &assets,
        PanelKind::HutResearch,
        "RESEARCH",
        hut_research_panel_pos(),
        hut_research_panel_size(),
        hut_research_detail_pos(),
        hut_research_detail_size(),
        HUT_RESEARCH_KINDS,
        &hut_research_buy_row_y,
    );
    spawn_panel(
        &mut commands,
        &assets,
        PanelKind::HutAqua,
        "AQUA  CENTER",
        hut_aqua_panel_pos(),
        hut_aqua_panel_size(),
        hut_aqua_detail_pos(),
        hut_aqua_detail_size(),
        HUT_AQUA_KINDS,
        &hut_aqua_buy_row_y,
    );
    spawn_panel(
        &mut commands,
        &assets,
        PanelKind::TreeStorage,
        "STORAGE",
        hut_tree_storage_panel_pos(),
        hut_tree_storage_panel_size(),
        hut_tree_storage_detail_pos(),
        hut_tree_storage_detail_size(),
        HUT_TREE_STORAGE_KINDS,
        &hut_tree_storage_buy_row_y,
    );
    spawn_panel(
        &mut commands,
        &assets,
        PanelKind::Pier,
        "PIER",
        pier_panel_pos(),
        pier_panel_size(),
        pier_detail_pos(),
        pier_detail_size(),
        PIER_PANEL_KINDS,
        &pier_buy_row_y,
    );
    spawn_panel(
        &mut commands,
        &assets,
        PanelKind::Port,
        "PORT",
        port_panel_pos(),
        port_panel_size(),
        port_detail_pos(),
        port_detail_size(),
        PORT_PANEL_KINDS,
        &port_buy_row_y,
    );
}

#[allow(clippy::too_many_arguments)]
fn spawn_panel(
    commands: &mut Commands,
    assets: &GameAssets,
    panel: PanelKind,
    title: &str,
    panel_pos: Vec2,
    panel_size: Vec2,
    detail_pos: Vec2,
    detail_size: Vec2,
    kinds: &[PurchaseKind],
    row_y: &dyn Fn(usize) -> f32,
) {
    spawn_main_panel_frame(commands, panel, panel_pos, panel_size);

    // Title — anchored just inside the top edge.
    let title_y = panel_pos.y - panel_size.y * 0.5 + 3.0;
    spawn_chrome_text(commands, assets, panel, panel_pos.x, title_y, title);

    // Divider under title.
    spawn_divider(
        commands,
        panel,
        panel_pos,
        panel_size,
        panel_size.y * -0.5 + 6.0,
    );

    // Buy rows.
    let row_w = panel_size.x - PANEL_INSET * 2.0;
    for (i, kind) in kinds.iter().enumerate() {
        spawn_buy_row(
            commands,
            assets,
            *kind,
            Vec2::new(panel_pos.x, row_y(i)),
            row_w,
        );
    }

    // Sibling detail panel.
    spawn_detail_panel(commands, assets, panel, detail_pos, detail_size);
}

fn spawn_chrome_text(
    commands: &mut Commands,
    assets: &GameAssets,
    panel: PanelKind,
    cx: f32,
    cy: f32,
    text: &str,
) {
    commands.spawn((
        PanelTag(panel),
        PanelChromePart::Title,
        UiText {
            spec_pos: Vec2::new(cx, cy),
            spec_font_size: 4.0,
            z: 0.0,
        },
        Text2d::new(text.to_string()),
        TextFont {
            font: assets.font.clone(),
            font_size: 4.0,
            font_smoothing: bevy::text::FontSmoothing::None,
            ..default()
        },
        TextColor(colors::FG),
        Transform::default(),
        RenderLayers::layer(UI_LAYER),
        Visibility::Hidden,
    ));
}

// ---------------------------------------------------------------------------
// Panel frame primitives
// ---------------------------------------------------------------------------

fn spawn_main_panel_frame(commands: &mut Commands, tag: PanelKind, pos: Vec2, size: Vec2) {
    let half = size * 0.5;
    let outer = size + Vec2::splat(PANEL_BORDER_W * 2.0);

    commands.spawn((
        PanelTag(tag),
        PanelChromePart::Border,
        Pos(pos),
        Layer(Z_BUTTON - 0.05),
        Sprite::from_color(colors::BUTTON_BORDER, outer),
        Transform::default(),
        Visibility::Hidden,
    ));
    commands.spawn((
        PanelTag(tag),
        PanelChromePart::Bg,
        Pos(pos),
        Layer(Z_BUTTON),
        Sprite::from_color(colors::BUTTON_BG, size),
        Transform::default(),
        Visibility::Hidden,
    ));

    // Pointer indicating which structure owns the panel. Cave + the
    // four hut panels all sit east of their building (point west);
    // pier sits below the pier (points up).
    match tag {
        PanelKind::Cave
        | PanelKind::Hut
        | PanelKind::HutMiner
        | PanelKind::HutSkimmer
        | PanelKind::HutFisher
        | PanelKind::HutBeachcomber
        | PanelKind::HutStonemason
        | PanelKind::HutResearch
        | PanelKind::HutAqua
        | PanelKind::TreeStorage => spawn_pointer_west(commands, tag, pos, half),
        // Pier panel sits below its structure; arrow points up
        // toward the pier from the panel's top edge.
        PanelKind::Pier => spawn_pointer_up(commands, tag, pos, half),
        // Port panel sits above its structure; arrow points down.
        PanelKind::Port => spawn_pointer_down(commands, tag, pos, half),
    }
}

fn spawn_detail_panel_frame(
    commands: &mut Commands,
    tag: PanelKind,
    pos: Vec2,
    size: Vec2,
) {
    let outer = size + Vec2::splat(PANEL_BORDER_W * 2.0);
    commands.spawn((
        PanelTag(tag),
        PanelChromePart::DetailBorder,
        Pos(pos),
        Layer(Z_BUTTON - 0.05),
        Sprite::from_color(colors::BUTTON_BORDER, outer),
        Transform::default(),
        Visibility::Hidden,
    ));
    commands.spawn((
        PanelTag(tag),
        PanelChromePart::DetailBg,
        Pos(pos),
        Layer(Z_BUTTON),
        Sprite::from_color(colors::BUTTON_BG, size),
        Transform::default(),
        Visibility::Hidden,
    ));
}

fn spawn_pointer_down(commands: &mut Commands, tag: PanelKind, pos: Vec2, half: Vec2) {
    let cx = pos.x;
    let bottom_y = pos.y + half.y + PANEL_BORDER_W;
    for (i, w) in [5.0, 3.0, 1.0].iter().enumerate() {
        commands.spawn((
            PanelTag(tag),
            PanelChromePart::Pointer(i as u8),
            Pos(Vec2::new(cx, bottom_y + 0.5 + i as f32)),
            Layer(Z_BUTTON + 0.02),
            Sprite::from_color(colors::BUTTON_BORDER, Vec2::new(*w, 1.0)),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
}

fn spawn_pointer_up(commands: &mut Commands, tag: PanelKind, pos: Vec2, half: Vec2) {
    let cx = pos.x;
    let top_y = pos.y - half.y - PANEL_BORDER_W;
    for (i, w) in [5.0, 3.0, 1.0].iter().enumerate() {
        commands.spawn((
            PanelTag(tag),
            PanelChromePart::Pointer(i as u8),
            Pos(Vec2::new(cx, top_y - 0.5 - i as f32)),
            Layer(Z_BUTTON + 0.02),
            Sprite::from_color(colors::BUTTON_BORDER, Vec2::new(*w, 1.0)),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
}

fn spawn_pointer_west(commands: &mut Commands, tag: PanelKind, pos: Vec2, half: Vec2) {
    let cy = pos.y;
    let left_x = pos.x - half.x - PANEL_BORDER_W;
    for (i, h) in [5.0, 3.0, 1.0].iter().enumerate() {
        commands.spawn((
            PanelTag(tag),
            PanelChromePart::Pointer(i as u8),
            Pos(Vec2::new(left_x - 0.5 - i as f32, cy)),
            Layer(Z_BUTTON + 0.02),
            Sprite::from_color(colors::BUTTON_BORDER, Vec2::new(1.0, *h)),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
}

fn spawn_divider(
    commands: &mut Commands,
    tag: PanelKind,
    panel_pos: Vec2,
    panel_size: Vec2,
    offset_y: f32,
) {
    let w = panel_size.x - PANEL_INSET * 2.0;
    commands.spawn((
        PanelTag(tag),
        PanelChromePart::Divider,
        Pos(Vec2::new(panel_pos.x, panel_pos.y + offset_y)),
        Layer(Z_BUTTON + 0.03),
        Sprite::from_color(colors::BUTTON_BORDER, Vec2::new(w, 1.0)),
        Transform::default(),
        Visibility::Hidden,
    ));
}

/// Stamp a chrome entity with the unified `PanelTag` marker. The
/// visibility/detail systems branch on `PanelTag.0` instead of needing
/// one filter component per panel type.
fn spawn_panel_tagged<B: Bundle>(commands: &mut Commands, panel: PanelKind, bundle: B) {
    commands.spawn((PanelTag(panel), bundle));
}

// ---------------------------------------------------------------------------
// Buy rows (no panel tag — visibility is per-button via button_active)
// ---------------------------------------------------------------------------

/// Spawn a buy row with name + live count + cost. The count text is
/// tagged `ButtonCount` so the count system can refresh it from the
/// matching resource each frame; one-time purchases write an empty
/// string into it so the column visually disappears for them.
fn spawn_buy_row(
    commands: &mut Commands,
    assets: &GameAssets,
    kind: PurchaseKind,
    pos: Vec2,
    width: f32,
) {
    let size = Vec2::new(width, ROW_HEIGHT);

    // Background — also the click hit-target.
    commands.spawn((
        PurchaseButton { kind, size },
        Pos(pos),
        Layer(Z_BUTTON + 0.04),
        Sprite::from_color(colors::BUTTON_BG_DIM, size),
        Transform::default(),
        Visibility::Hidden,
    ));

    // Name (left).
    let label_x = pos.x - width * 0.5 + 3.0;
    commands.spawn((
        ButtonLabel(kind),
        UiText {
            spec_pos: Vec2::new(label_x, pos.y),
            spec_font_size: 4.0,
            z: 0.0,
        },
        Text2d::new(kind.label().to_string()),
        TextFont {
            font: assets.font.clone(),
            font_size: 4.0,
            font_smoothing: bevy::text::FontSmoothing::None,
            ..default()
        },
        TextColor(colors::BUTTON_BORDER),
        Transform::default(),
        Anchor::CENTER_LEFT,
        RenderLayers::layer(UI_LAYER),
        Visibility::Hidden,
    ));

    // Live count (centre column).
    let count_x = pos.x + width * 0.5 - 26.0;
    commands.spawn((
        ButtonCount(kind),
        UiText {
            spec_pos: Vec2::new(count_x, pos.y),
            spec_font_size: 4.0,
            z: 0.0,
        },
        Text2d::new(String::new()),
        TextFont {
            font: assets.font.clone(),
            font_size: 4.0,
            font_smoothing: bevy::text::FontSmoothing::None,
            ..default()
        },
        TextColor(colors::FG_DIM),
        Transform::default(),
        Anchor::CENTER_RIGHT,
        RenderLayers::layer(UI_LAYER),
        Visibility::Hidden,
    ));

    // Cost (right).
    let cost_x = pos.x + width * 0.5 - 3.0;
    commands.spawn((
        ButtonCost(kind),
        UiText {
            spec_pos: Vec2::new(cost_x, pos.y),
            spec_font_size: 4.0,
            z: 0.0,
        },
        Text2d::new(kind.cost_label().to_string()),
        TextFont {
            font: assets.font.clone(),
            font_size: 4.0,
            font_smoothing: bevy::text::FontSmoothing::None,
            ..default()
        },
        TextColor(colors::YELLOW),
        Transform::default(),
        Anchor::CENTER_RIGHT,
        RenderLayers::layer(UI_LAYER),
        Visibility::Hidden,
    ));
}

// ---------------------------------------------------------------------------
// Detail panel
// ---------------------------------------------------------------------------

fn spawn_detail_panel(
    commands: &mut Commands,
    assets: &GameAssets,
    kind: PanelKind,
    pos: Vec2,
    size: Vec2,
) {
    spawn_detail_panel_frame(commands, kind, pos, size);

    let inner_left = pos.x - size.x * 0.5 + PANEL_INSET;
    let header_y = pos.y - size.y * 0.5 + PANEL_INSET + 2.0;
    let body_y = header_y + 7.0;

    // Header — colour gets recoloured each frame in
    // `interaction::update_detail_text` based on affordability.
    spawn_panel_tagged(
        commands,
        kind,
        (
            DetailHeader(kind),
            UiText {
                spec_pos: Vec2::new(inner_left, header_y),
                spec_font_size: 4.0,
                z: 0.0,
            },
            Text2d::new(String::new()),
            TextFont {
                font: assets.font.clone(),
                font_size: 4.0,
                font_smoothing: bevy::text::FontSmoothing::None,
                ..default()
            },
            TextColor(colors::DETAIL_OK),
            Transform::default(),
            Anchor::CENTER_LEFT,
            RenderLayers::layer(UI_LAYER),
            Visibility::Hidden,
        ),
    );

    // Body — multi-line via `\n`, top-left anchored so newlines flow
    // downward from the body_y baseline.
    spawn_panel_tagged(
        commands,
        kind,
        (
            DetailBody(kind),
            UiText {
                spec_pos: Vec2::new(inner_left, body_y),
                spec_font_size: 4.0,
                z: 0.0,
            },
            Text2d::new(String::new()),
            TextFont {
                font: assets.font.clone(),
                font_size: 4.0,
                font_smoothing: bevy::text::FontSmoothing::None,
                ..default()
            },
            TextLayout {
                justify: Justify::Left,
                ..default()
            },
            TextColor(colors::FG_DIM),
            Transform::default(),
            Anchor::TOP_LEFT,
            RenderLayers::layer(UI_LAYER),
            Visibility::Hidden,
        ),
    );
}
