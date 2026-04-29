//! Top-of-screen HUD — SKIMS currency counter centered on top, with a
//! smaller WORKERS counter pinned beside it so the player can see
//! both at a glance.

use bevy::camera::visibility::RenderLayers;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::core::assets::GameAssets;
use crate::core::colors;
use crate::core::common::{Layer, Pos};
use crate::core::constants::{INTERNAL_WIDTH, Z_BUTTON, Z_UI};
use crate::currency::{Skims, Wood};
use crate::economy::{ResearchMission, Workers};
use crate::render::shapes::Shapes;
use crate::render::{ScreenAnchored, ScreenFixedText, UiText, UI_LAYER};

#[derive(Component)]
pub struct SkimsLabel;

#[derive(Component)]
pub struct SkimsValue;

#[derive(Component)]
pub struct WorkersLabel;

#[derive(Component)]
pub struct WorkersValue;

#[derive(Component)]
pub struct FpsCounter;

#[derive(Component)]
pub struct WoodLabel;

#[derive(Component)]
pub struct WoodValue;

/// Rolling per-second counters for the top-left HUD readouts. Other
/// modules push events into the `*_window` in-flight accumulators;
/// the UI system folds those into a 60-bucket ring (one bucket per
/// second) and reports the sliding 60-second average.
///
/// Skims are sampled by diffing `Skims.total` so we don't have to
/// patch every site that grants skims; produced and thrown stones
/// are bumped at their (single) spawn / toss chokepoints.
#[derive(Resource)]
pub struct Rates {
    pub stones_produced_window: u32,
    pub stones_thrown_window: u32,
    pub skims_added_window: u64,
    pub last_skims_total: u64,
    pub skims_per_sec: f32,
    pub stones_produced_per_sec: f32,
    pub stones_thrown_per_sec: f32,
    /// 60 one-second buckets (ring buffer) for each tracked counter.
    /// Skims are stored as u32-per-second; 60 s of normal play stays
    /// well under u32::MAX so saturating conversions are fine.
    pub skim_buckets: [u32; 60],
    pub produced_buckets: [u32; 60],
    pub thrown_buckets: [u32; 60],
    pub bucket_index: usize,
    pub bucket_time: f32,
}

impl Default for Rates {
    fn default() -> Self {
        Self {
            stones_produced_window: 0,
            stones_thrown_window: 0,
            skims_added_window: 0,
            last_skims_total: 0,
            skims_per_sec: 0.0,
            stones_produced_per_sec: 0.0,
            stones_thrown_per_sec: 0.0,
            skim_buckets: [0; 60],
            produced_buckets: [0; 60],
            thrown_buckets: [0; 60],
            bucket_index: 0,
            bucket_time: 0.0,
        }
    }
}

#[derive(Component)]
pub struct SkimsRateValue;

#[derive(Component)]
pub struct ProBar;

#[derive(Component)]
pub struct ThrBar;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .init_resource::<Rates>()
            .add_systems(Startup, spawn_ui)
            .add_systems(
                Update,
                (
                    update_skims_text,
                    update_workers_text,
                    update_fps_text,
                    update_rates,
                    update_wood_text,
                    update_wood_visibility,
                ),
            );
    }
}

/// Top-left HUD layout (spec coords, Y-down). Sized to fit FPS, the
/// SKM/S text, and the PRO / THR rows with their bars.
const HUD_X: f32 = 3.0;
const HUD_Y: f32 = 3.0;
const HUD_W: f32 = 80.0;
const HUD_H: f32 = 26.0;
/// Left edge of each PRO/THR row's bar. The bar sprite is anchored
/// CENTER_LEFT so its left edge stays glued to this X and it grows
/// rightward as its width increases. Picked so the bar track sits
/// in the right half of the container with a few pixels of margin
/// from the right edge.
const BAR_LEFT_X: f32 = HUD_X + 25.0;
/// Maximum bar width — reached when one of the two rates carries
/// 100% of the (PRO+THR) total. Equal rates → both bars at 50%.
const BAR_MAX_W: f32 = 50.0;
const BAR_HEIGHT: f32 = 2.0;

fn spawn_ui(mut commands: Commands, assets: Res<GameAssets>, shapes: Res<Shapes>) {
    let cx = INTERNAL_WIDTH * 0.5;

    // SKIMS — primary currency, big yellow value at top center.
    commands.spawn((
        SkimsLabel,
        ScreenFixedText,
        UiText {
            spec_pos: Vec2::new(cx, 6.0),
            spec_font_size: 4.0,
            z: Z_UI,
        },
        Text2d::new("SKIMS"),
        TextFont {
            font: assets.font.clone(),
            font_size: 4.0,
            font_smoothing: bevy::text::FontSmoothing::None,
            ..default()
        },
        TextColor(colors::FG_DIM),
        Transform::default(),
        RenderLayers::layer(UI_LAYER),
    ));
    commands.spawn((
        SkimsValue,
        ScreenFixedText,
        UiText {
            spec_pos: Vec2::new(cx, 14.0),
            spec_font_size: 8.0,
            z: Z_UI,
        },
        Text2d::new("0"),
        TextFont {
            font: assets.font.clone(),
            font_size: 8.0,
            font_smoothing: bevy::text::FontSmoothing::None,
            ..default()
        },
        TextColor(colors::YELLOW),
        Transform::default(),
        RenderLayers::layer(UI_LAYER),
    ));

    // Container — dark translucent box behind the FPS / rates HUD
    // so the text reads against any background. Uses the panel BG
    // colour to match the buy panels visually. ScreenAnchored keeps
    // it pinned to the screen as the camera scrolls.
    commands.spawn((
        ScreenAnchored {
            spec_x: HUD_X + HUD_W * 0.5,
        },
        Pos(Vec2::new(HUD_X + HUD_W * 0.5, HUD_Y + HUD_H * 0.5)),
        Layer(Z_BUTTON - 0.05),
        Sprite::from_color(colors::BUTTON_BG, Vec2::new(HUD_W, HUD_H)),
        Transform::default(),
    ));

    // FPS — top of the container.
    commands.spawn((
        FpsCounter,
        ScreenFixedText,
        UiText {
            spec_pos: Vec2::new(HUD_X + 3.0, HUD_Y + 3.0),
            spec_font_size: 4.0,
            z: Z_UI,
        },
        Text2d::new("FPS --"),
        TextFont {
            font: assets.font.clone(),
            font_size: 4.0,
            font_smoothing: bevy::text::FontSmoothing::None,
            ..default()
        },
        TextColor(colors::FG_DIM),
        Transform::default(),
        Anchor::CENTER_LEFT,
        RenderLayers::layer(UI_LAYER),
    ));

    // SKM/S — plain text row; no bar.
    commands.spawn((
        SkimsRateValue,
        ScreenFixedText,
        UiText {
            spec_pos: Vec2::new(HUD_X + 3.0, HUD_Y + 8.0),
            spec_font_size: 4.0,
            z: Z_UI,
        },
        Text2d::new("SKM/S 0.0"),
        TextFont {
            font: assets.font.clone(),
            font_size: 4.0,
            font_smoothing: bevy::text::FontSmoothing::None,
            ..default()
        },
        TextColor(colors::FG_DIM),
        Transform::default(),
        Anchor::CENTER_LEFT,
        RenderLayers::layer(UI_LAYER),
    ));

    // PRO + THR rows — sign + stone sprite + "/s" text + a white bar
    // whose width tracks the row's share of (pro + thr). Equal
    // rates → both bars 50% of BAR_MAX_W and centered, so the two
    // bars line up visually.
    spawn_rate_row(
        &mut commands,
        &assets,
        &shapes,
        HUD_Y + 14.0,
        "+",
        true,
    );
    spawn_rate_row(
        &mut commands,
        &assets,
        &shapes,
        HUD_Y + 20.0,
        "-",
        false,
    );

    // WORKERS — smaller secondary readout sitting just below the
    // skims value. Tighter font so it doesn't compete visually.
    commands.spawn((
        WorkersLabel,
        ScreenFixedText,
        UiText {
            spec_pos: Vec2::new(cx, 24.0),
            spec_font_size: 4.0,
            z: Z_UI,
        },
        Text2d::new("WORKERS"),
        TextFont {
            font: assets.font.clone(),
            font_size: 4.0,
            font_smoothing: bevy::text::FontSmoothing::None,
            ..default()
        },
        TextColor(colors::FG_DIM),
        Transform::default(),
        RenderLayers::layer(UI_LAYER),
    ));
    commands.spawn((
        WorkersValue,
        ScreenFixedText,
        UiText {
            spec_pos: Vec2::new(cx, 30.0),
            spec_font_size: 6.0,
            z: Z_UI,
        },
        Text2d::new("0"),
        TextFont {
            font: assets.font.clone(),
            font_size: 6.0,
            font_smoothing: bevy::text::FontSmoothing::None,
            ..default()
        },
        TextColor(colors::FG),
        Transform::default(),
        RenderLayers::layer(UI_LAYER),
    ));

    // WOOD — secondary currency, hidden until the Tree Surgeon
    // upgrade is purchased. Sits under WORKERS so the top-center
    // stack reads as `SKIMS / WORKERS / WOOD`.
    commands.spawn((
        WoodLabel,
        ScreenFixedText,
        UiText {
            spec_pos: Vec2::new(cx, 38.0),
            spec_font_size: 4.0,
            z: Z_UI,
        },
        Text2d::new("WOOD"),
        TextFont {
            font: assets.font.clone(),
            font_size: 4.0,
            font_smoothing: bevy::text::FontSmoothing::None,
            ..default()
        },
        TextColor(colors::FG_DIM),
        Transform::default(),
        Visibility::Hidden,
        RenderLayers::layer(UI_LAYER),
    ));
    commands.spawn((
        WoodValue,
        ScreenFixedText,
        UiText {
            spec_pos: Vec2::new(cx, 44.0),
            spec_font_size: 6.0,
            z: Z_UI,
        },
        Text2d::new("0"),
        TextFont {
            font: assets.font.clone(),
            font_size: 6.0,
            font_smoothing: bevy::text::FontSmoothing::None,
            ..default()
        },
        TextColor(colors::TREE_FOLIAGE_LIGHT),
        Transform::default(),
        Visibility::Hidden,
        RenderLayers::layer(UI_LAYER),
    ));
}

fn spawn_rate_row(
    commands: &mut Commands,
    assets: &GameAssets,
    shapes: &Shapes,
    y: f32,
    sign: &str,
    is_pro: bool,
) {
    // "+" / "-" sign on the far left of the row.
    commands.spawn((
        ScreenFixedText,
        UiText {
            spec_pos: Vec2::new(HUD_X + 3.0, y),
            spec_font_size: 4.0,
            z: Z_UI,
        },
        Text2d::new(sign.to_string()),
        TextFont {
            font: assets.font.clone(),
            font_size: 4.0,
            font_smoothing: bevy::text::FontSmoothing::None,
            ..default()
        },
        TextColor(colors::FG_DIM),
        Transform::default(),
        Anchor::CENTER_LEFT,
        RenderLayers::layer(UI_LAYER),
    ));

    // Stone glyph — actual small-rock sprite, scaled down to fit the
    // row. Uses the round-small banded variant so it reads as a
    // pebble even at this size.
    commands.spawn((
        ScreenAnchored {
            spec_x: HUD_X + 8.0,
        },
        Pos(Vec2::new(HUD_X + 8.0, y)),
        Layer(Z_UI - 0.02),
        Sprite {
            image: shapes.small_rock_round_small.clone(),
            custom_size: Some(Vec2::new(4.0, 4.0)),
            ..default()
        },
        Transform::default(),
    ));

    // "/s" suffix.
    commands.spawn((
        ScreenFixedText,
        UiText {
            spec_pos: Vec2::new(HUD_X + 11.0, y),
            spec_font_size: 4.0,
            z: Z_UI,
        },
        Text2d::new("/s"),
        TextFont {
            font: assets.font.clone(),
            font_size: 4.0,
            font_smoothing: bevy::text::FontSmoothing::None,
            ..default()
        },
        TextColor(colors::FG_DIM),
        Transform::default(),
        Anchor::CENTER_LEFT,
        RenderLayers::layer(UI_LAYER),
    ));

    // White bar — width set every refresh from the live rates. Spawn
    // with zero width so it's invisible until the first update.
    let mut bar = commands.spawn((
        ScreenAnchored {
            spec_x: BAR_LEFT_X,
        },
        Pos(Vec2::new(BAR_LEFT_X, y)),
        Layer(Z_UI - 0.02),
        Sprite {
            color: Color::WHITE,
            custom_size: Some(Vec2::new(0.0, BAR_HEIGHT)),
            ..default()
        },
        Transform::default(),
        Anchor::CENTER_LEFT,
    ));
    if is_pro {
        bar.insert(ProBar);
    } else {
        bar.insert(ThrBar);
    }
}

fn update_skims_text(
    skims: Res<Skims>,
    mut q: Query<&mut Text2d, With<SkimsValue>>,
) {
    if !skims.is_changed() {
        return;
    }
    for mut t in &mut q {
        t.0 = format!("{}", skims.total);
    }
}

fn update_wood_text(
    wood: Res<Wood>,
    mut q: Query<&mut Text2d, With<WoodValue>>,
) {
    if !wood.is_changed() {
        return;
    }
    for mut t in &mut q {
        t.0 = format!("{}", wood.total);
    }
}

/// Reveal the WOOD label + value once the research mission's scout
/// finishes its cinematic; keep them hidden until then so the
/// top-center stack stays uncluttered for new players.
fn update_wood_visibility(
    mission: Res<ResearchMission>,
    mut label_q: Query<&mut Visibility, (With<WoodLabel>, Without<WoodValue>)>,
    mut value_q: Query<&mut Visibility, (With<WoodValue>, Without<WoodLabel>)>,
) {
    if !mission.is_changed() {
        return;
    }
    let target = if mission.unlocked {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut v in &mut label_q {
        if *v != target {
            *v = target;
        }
    }
    for mut v in &mut value_q {
        if *v != target {
            *v = target;
        }
    }
}

fn update_workers_text(
    workers: Res<Workers>,
    mut q: Query<&mut Text2d, With<WorkersValue>>,
) {
    if !workers.is_changed() {
        return;
    }
    for mut t in &mut q {
        t.0 = format!("{}", workers.count);
    }
}

fn update_rates(
    time: Res<Time>,
    skims: Res<Skims>,
    mut rates: ResMut<Rates>,
    mut skm_q: Query<&mut Text2d, With<SkimsRateValue>>,
    mut pro_q: Query<&mut Sprite, (With<ProBar>, Without<ThrBar>)>,
    mut thr_q: Query<&mut Sprite, (With<ThrBar>, Without<ProBar>)>,
) {
    // Sample skim deltas — saturating_sub guards against any future
    // path that resets the counter.
    let delta = skims.total.saturating_sub(rates.last_skims_total);
    rates.skims_added_window = rates.skims_added_window.saturating_add(delta);
    rates.last_skims_total = skims.total;

    // Roll the in-flight accumulators into the current bucket every
    // frame. When the bucket fills up (>=1s), advance the ring index
    // and clear the new bucket so the oldest second drops out.
    rates.bucket_time += time.delta_secs();
    let idx = rates.bucket_index;
    let skim_sample = u32::try_from(rates.skims_added_window).unwrap_or(u32::MAX);
    rates.skim_buckets[idx] = rates.skim_buckets[idx].saturating_add(skim_sample);
    rates.produced_buckets[idx] = rates.produced_buckets[idx]
        .saturating_add(rates.stones_produced_window);
    rates.thrown_buckets[idx] = rates.thrown_buckets[idx]
        .saturating_add(rates.stones_thrown_window);
    rates.skims_added_window = 0;
    rates.stones_produced_window = 0;
    rates.stones_thrown_window = 0;

    while rates.bucket_time >= 1.0 {
        rates.bucket_time -= 1.0;
        rates.bucket_index = (rates.bucket_index + 1) % 60;
        let new_idx = rates.bucket_index;
        rates.skim_buckets[new_idx] = 0;
        rates.produced_buckets[new_idx] = 0;
        rates.thrown_buckets[new_idx] = 0;
    }

    // Sliding 60-second averages — the sum updates every frame
    // (the in-flight portion is folded into the current bucket
    // above) so no EMA smoothing is needed.
    let skim_sum: u64 = rates.skim_buckets.iter().map(|&v| v as u64).sum();
    let pro_sum: u64 = rates.produced_buckets.iter().map(|&v| v as u64).sum();
    let thr_sum: u64 = rates.thrown_buckets.iter().map(|&v| v as u64).sum();
    rates.skims_per_sec = skim_sum as f32 / 60.0;
    rates.stones_produced_per_sec = pro_sum as f32 / 60.0;
    rates.stones_thrown_per_sec = thr_sum as f32 / 60.0;

    for mut t in &mut skm_q {
        t.0 = format!("SKM/S {:.1}", rates.skims_per_sec);
    }
    // Bars are normalised against (pro + thr) so they always share
    // the same horizontal track. Equal rates → 50% / 50%; PRO at
    // double THR → 66% / 33%. Sprites are CENTER_LEFT-anchored so
    // their left edges stay pinned to `BAR_LEFT_X` and they grow
    // rightward as width increases.
    let pro = rates.stones_produced_per_sec.max(0.0);
    let thr = rates.stones_thrown_per_sec.max(0.0);
    let total = pro + thr;
    let (pro_w, thr_w) = if total < 0.001 {
        (0.0, 0.0)
    } else {
        (BAR_MAX_W * pro / total, BAR_MAX_W * thr / total)
    };
    for mut s in &mut pro_q {
        s.custom_size = Some(Vec2::new(pro_w, BAR_HEIGHT));
    }
    for mut s in &mut thr_q {
        s.custom_size = Some(Vec2::new(thr_w, BAR_HEIGHT));
    }
}

fn update_fps_text(
    diagnostics: Res<DiagnosticsStore>,
    mut q: Query<&mut Text2d, With<FpsCounter>>,
) {
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed());
    let label = match fps {
        Some(v) => format!("FPS {:>3}", v.round() as u32),
        None => "FPS --".to_string(),
    };
    for mut t in &mut q {
        if t.0 != label {
            t.0 = label.clone();
        }
    }
}
