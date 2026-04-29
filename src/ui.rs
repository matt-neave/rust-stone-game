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
use crate::currency::Skims;
use crate::economy::Workers;
use crate::render::shapes::Shapes;
use crate::render::{ScreenAnchored, UiText, UI_LAYER};

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

/// Rolling per-second counters for the top-left HUD readouts. Other
/// modules push events into the `*_window` accumulators; the UI
/// system flushes them into `*_per_sec` once per `WINDOW` and resets
/// the buckets.
///
/// Skims are sampled by diffing `Skims.total` so we don't have to
/// patch every site that grants skims; produced and thrown stones
/// are bumped at their (single) spawn / toss chokepoints.
#[derive(Resource, Default)]
pub struct Rates {
    pub stones_produced_window: u32,
    pub stones_thrown_window: u32,
    pub skims_added_window: u64,
    pub last_skims_total: u64,
    pub window_time: f32,
    pub skims_per_sec: f32,
    pub stones_produced_per_sec: f32,
    pub stones_thrown_per_sec: f32,
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
/// X position where each PRO/THR row's bar is centered. Picked so
/// the bar track sits in the right half of the container with a few
/// pixels of margin from the right edge.
const BAR_CENTER_X: f32 = HUD_X + 50.0;
/// Maximum bar width — reached when one of the two rates carries
/// 100% of the (PRO+THR) total. Equal rates → both bars at 50%.
const BAR_MAX_W: f32 = 50.0;
const BAR_HEIGHT: f32 = 2.0;

fn spawn_ui(mut commands: Commands, assets: Res<GameAssets>, shapes: Res<Shapes>) {
    let cx = INTERNAL_WIDTH * 0.5;

    // SKIMS — primary currency, big yellow value at top center.
    commands.spawn((
        SkimsLabel,
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
            spec_x: BAR_CENTER_X,
        },
        Pos(Vec2::new(BAR_CENTER_X, y)),
        Layer(Z_UI - 0.02),
        Sprite::from_color(Color::WHITE, Vec2::new(0.0, BAR_HEIGHT)),
        Transform::default(),
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

/// Refresh interval for the per-second readouts. Long enough to
/// smooth out short bursts (a player click flurry, a fisherman cast
/// completing) but short enough to feel responsive.
const RATE_WINDOW: f32 = 1.0;
/// EMA smoothing factor — each window the displayed rate moves
/// `RATE_BLEND` of the way toward the new measurement. 0.5 is a
/// good balance of stability and responsiveness.
const RATE_BLEND: f32 = 0.5;

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

    rates.window_time += time.delta_secs();
    if rates.window_time < RATE_WINDOW {
        return;
    }
    let elapsed = rates.window_time;
    let skm_now = rates.skims_added_window as f32 / elapsed;
    let pro_now = rates.stones_produced_window as f32 / elapsed;
    let thr_now = rates.stones_thrown_window as f32 / elapsed;
    rates.skims_per_sec = rates.skims_per_sec * (1.0 - RATE_BLEND) + skm_now * RATE_BLEND;
    rates.stones_produced_per_sec =
        rates.stones_produced_per_sec * (1.0 - RATE_BLEND) + pro_now * RATE_BLEND;
    rates.stones_thrown_per_sec =
        rates.stones_thrown_per_sec * (1.0 - RATE_BLEND) + thr_now * RATE_BLEND;
    rates.skims_added_window = 0;
    rates.stones_produced_window = 0;
    rates.stones_thrown_window = 0;
    rates.window_time = 0.0;

    for mut t in &mut skm_q {
        t.0 = format!("SKM/S {:.1}", rates.skims_per_sec);
    }
    // Bars are normalised against (pro + thr) so they always share
    // the same horizontal track. Equal rates → 50% / 50%; PRO at
    // double THR → 66% / 33%. Sprites are center-anchored so the
    // bars sit centered on `BAR_CENTER_X` regardless of width.
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
