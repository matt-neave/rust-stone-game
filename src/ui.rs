//! Top-of-screen HUD — SKIMS currency counter centered on top, with a
//! smaller WORKERS counter pinned beside it so the player can see
//! both at a glance.

use bevy::camera::visibility::RenderLayers;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

use crate::core::assets::GameAssets;
use crate::core::colors;
use crate::core::constants::{INTERNAL_WIDTH, Z_UI};
use crate::currency::Skims;
use crate::economy::Workers;
use crate::render::{UiText, UI_LAYER};

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

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .add_systems(Startup, spawn_ui)
            .add_systems(
                Update,
                (update_skims_text, update_workers_text, update_fps_text),
            );
    }
}

fn spawn_ui(mut commands: Commands, assets: Res<GameAssets>) {
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

    // FPS — top-left corner, dim small text. Anchored left so the
    // value (which can be 1–3 digits) grows rightward without
    // shifting visually.
    commands.spawn((
        FpsCounter,
        UiText {
            spec_pos: Vec2::new(4.0, 6.0),
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
        bevy::sprite::Anchor::CENTER_LEFT,
        RenderLayers::layer(UI_LAYER),
    ));

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
