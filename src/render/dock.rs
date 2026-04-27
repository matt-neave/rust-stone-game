//! Display-mode toggle: fullscreen view vs. docked strip-on-bottom view.
//!
//! The game starts in `DisplayMode::Fullscreen` (BorderlessFullscreen).
//! Toggling to `Docked` resizes the window to a thin strip along the
//! bottom of the monitor, drops the window decorations, pushes it
//! always-on-top and gates game input — the dock button is the only
//! interactive element in docked mode. The RTT view in docked mode is
//! cropped (via the upscale sprite's `rect`) to a horizontal slice
//! centered on the big rock so the docked window shows the rocky
//! action strip rather than the whole canvas.

use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy::window::{MonitorSelection, PrimaryWindow, WindowLevel, WindowMode, WindowPosition};

use crate::audio::Muted;
use crate::core::common::SyncSet;
use crate::core::constants::{BIG_ROCK_Y, INTERNAL_HEIGHT, INTERNAL_WIDTH};
use crate::render::pipeline::{DisplayScale, UpscaleSprite, UI_LAYER};

/// Fraction of monitor height the docked window occupies.
const DOCKED_HEIGHT_FRAC: f32 = 0.20;

/// Resolution of the regular Windowed mode.
const WINDOWED_W: f32 = 1280.0;
const WINDOWED_H: f32 = 720.0;

/// Visible button hitbox in window-pixel coords.
const BUTTON_W: f32 = 90.0;
const BUTTON_H: f32 = 26.0;
const BUTTON_INSET: f32 = 8.0;

/// Display mode cycles Windowed → Docked → Fullscreen → Windowed via
/// the dock button at the top-right of the window. Game opens in
/// Windowed mode by default — fullscreen is reachable but not forced.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum DisplayMode {
    #[default]
    Windowed,
    Docked,
    Fullscreen,
}

impl DisplayMode {
    /// What the dock button shows in this mode — i.e. the *next* mode
    /// in the cycle that the click will switch to.
    fn label(self) -> &'static str {
        match self {
            DisplayMode::Windowed => "DOCK",
            DisplayMode::Docked => "FULL",
            DisplayMode::Fullscreen => "WIND",
        }
    }

    fn next(self) -> Self {
        match self {
            DisplayMode::Windowed => DisplayMode::Docked,
            DisplayMode::Docked => DisplayMode::Fullscreen,
            DisplayMode::Fullscreen => DisplayMode::Windowed,
        }
    }
}

/// Resource: cached monitor size used to compute docked geometry.
/// Filled in once we have a window with a resolved physical size.
#[derive(Resource, Clone, Copy, Default)]
struct MonitorPx {
    w: f32,
    h: f32,
}

#[derive(Component)]
pub struct DockButton;

#[derive(Component)]
pub struct DockButtonLabel;

#[derive(Component)]
pub struct MuteButton;

#[derive(Component)]
pub struct MuteButtonLabel;

/// Marker — true when the cursor is currently over the dock OR mute
/// button. Used by the input plugin to suppress click propagation to
/// game so the buttons don't double-fire as game clicks.
#[derive(Resource, Default, Clone, Copy)]
pub struct DockButtonHover(pub bool);

pub struct DockPlugin;

impl Plugin for DockPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DisplayMode>()
            .init_resource::<MonitorPx>()
            .init_resource::<DockButtonHover>()
            .init_resource::<MuteButtonHover>()
            .add_systems(Startup, spawn_dock_button)
            .add_systems(
                Update,
                (
                    track_monitor_size,
                    handle_dock_input,
                    apply_display_mode.after(track_monitor_size),
                    layout_dock_button.after(apply_display_mode),
                    update_dock_button_visuals.after(layout_dock_button),
                    apply_docked_view_crop.after(apply_display_mode).before(SyncSet::Transforms),
                ),
            );
    }
}

fn spawn_dock_button(
    mut commands: Commands,
    assets: Res<crate::core::assets::GameAssets>,
) {
    commands.spawn((
        DockButton,
        Sprite::from_color(Color::srgba(0.05, 0.06, 0.08, 0.85), Vec2::new(BUTTON_W, BUTTON_H)),
        Transform::default(),
        Visibility::Visible,
        bevy::camera::visibility::RenderLayers::layer(UI_LAYER),
    ));
    commands.spawn((
        DockButtonLabel,
        Text2d::new("DOCK"),
        TextFont {
            font: assets.font.clone(),
            font_size: 12.0,
            font_smoothing: bevy::text::FontSmoothing::None,
            ..default()
        },
        TextColor(crate::core::colors::FG),
        Transform::default(),
        Visibility::Visible,
        bevy::camera::visibility::RenderLayers::layer(UI_LAYER),
    ));
    commands.spawn((
        MuteButton,
        Sprite::from_color(Color::srgba(0.05, 0.06, 0.08, 0.85), Vec2::new(BUTTON_W, BUTTON_H)),
        Transform::default(),
        Visibility::Visible,
        bevy::camera::visibility::RenderLayers::layer(UI_LAYER),
    ));
    commands.spawn((
        MuteButtonLabel,
        Text2d::new("MUTE"),
        TextFont {
            font: assets.font.clone(),
            font_size: 12.0,
            font_smoothing: bevy::text::FontSmoothing::None,
            ..default()
        },
        TextColor(crate::core::colors::FG),
        Transform::default(),
        Visibility::Visible,
        bevy::camera::visibility::RenderLayers::layer(UI_LAYER),
    ));
}

/// Cache the monitor's physical size so docked geometry can be computed
/// from it. We can't reach `Monitors` directly from a regular system in
/// every Bevy version, so we use the primary window's current physical
/// size while it's borderless-fullscreen as a stand-in. The first
/// frame after entering fullscreen the value is correct, and we don't
/// need it before that.
fn track_monitor_size(
    windows: Query<&Window, With<PrimaryWindow>>,
    mode: Res<DisplayMode>,
    mut mon: ResMut<MonitorPx>,
) {
    let Ok(window) = windows.single() else { return };
    if *mode == DisplayMode::Fullscreen {
        let w = window.physical_width() as f32 / window.scale_factor();
        let h = window.physical_height() as f32 / window.scale_factor();
        if (mon.w - w).abs() > 0.5 || (mon.h - h).abs() > 0.5 {
            mon.w = w;
            mon.h = h;
        }
    }
}

/// React to changes in `DisplayMode` by reshaping the primary window.
/// Idempotent — re-applies whenever the resource changes.
fn apply_display_mode(
    mode: Res<DisplayMode>,
    mon: Res<MonitorPx>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if !mode.is_changed() {
        return;
    }
    let Ok(mut window) = windows.single_mut() else { return };
    match *mode {
        DisplayMode::Fullscreen => {
            window.mode = WindowMode::BorderlessFullscreen(MonitorSelection::Current);
            window.decorations = true;
            window.window_level = WindowLevel::Normal;
            window.position = WindowPosition::Automatic;
        }
        DisplayMode::Windowed => {
            window.mode = WindowMode::Windowed;
            window.decorations = true;
            window.window_level = WindowLevel::Normal;
            window.resolution.set(WINDOWED_W, WINDOWED_H);
            window.position = WindowPosition::Centered(MonitorSelection::Current);
        }
        DisplayMode::Docked => {
            let mw = if mon.w > 0.0 { mon.w } else { 1920.0 };
            let mh = if mon.h > 0.0 { mon.h } else { 1080.0 };
            let dock_h = (mh * DOCKED_HEIGHT_FRAC).round().max(120.0);
            window.mode = WindowMode::Windowed;
            window.decorations = false;
            window.window_level = WindowLevel::AlwaysOnTop;
            window.resolution.set(mw, dock_h);
            window.position = WindowPosition::At(IVec2::new(0, (mh - dock_h) as i32));
        }
    }
}

/// Position the dock + mute buttons at the top-right corner of the
/// window, using window-pixel coordinates (UI camera renders at
/// native DPI). Mute sits just to the left of dock with the same
/// inset gap.
#[allow(clippy::too_many_arguments)]
fn layout_dock_button(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut dock_btn_q: Query<
        &mut Transform,
        (With<DockButton>, Without<DockButtonLabel>, Without<MuteButton>, Without<MuteButtonLabel>),
    >,
    mut dock_label_q: Query<
        &mut Transform,
        (With<DockButtonLabel>, Without<DockButton>, Without<MuteButton>, Without<MuteButtonLabel>),
    >,
    mut mute_btn_q: Query<
        &mut Transform,
        (With<MuteButton>, Without<MuteButtonLabel>, Without<DockButton>, Without<DockButtonLabel>),
    >,
    mut mute_label_q: Query<
        &mut Transform,
        (With<MuteButtonLabel>, Without<MuteButton>, Without<DockButton>, Without<DockButtonLabel>),
    >,
) {
    let Ok(window) = windows.single() else { return };
    let win_w = window.physical_width() as f32 / window.scale_factor();
    let win_h = window.physical_height() as f32 / window.scale_factor();
    if win_w <= 0.0 || win_h <= 0.0 {
        return;
    }
    // UI camera: world origin at window center, +y up.
    let dock_cx = win_w * 0.5 - BUTTON_W * 0.5 - BUTTON_INSET;
    let mute_cx = dock_cx - BUTTON_W - BUTTON_INSET;
    let cy = win_h * 0.5 - BUTTON_H * 0.5 - BUTTON_INSET;
    for mut tf in &mut dock_btn_q {
        tf.translation = Vec3::new(dock_cx, cy, 100.0);
    }
    for mut tf in &mut dock_label_q {
        tf.translation = Vec3::new(dock_cx, cy, 101.0);
    }
    for mut tf in &mut mute_btn_q {
        tf.translation = Vec3::new(mute_cx, cy, 100.0);
    }
    for mut tf in &mut mute_label_q {
        tf.translation = Vec3::new(mute_cx, cy, 101.0);
    }
}

/// Update label text + button BG color depending on hover state,
/// current display mode, and current mute state.
#[allow(clippy::too_many_arguments)]
fn update_dock_button_visuals(
    mode: Res<DisplayMode>,
    hover: Res<DockButtonHover>,
    mute_hover: Res<MuteButtonHover>,
    muted: Res<Muted>,
    mut dock_bg_q: Query<&mut Sprite, (With<DockButton>, Without<MuteButton>)>,
    mut dock_label_q: Query<&mut Text2d, (With<DockButtonLabel>, Without<MuteButtonLabel>)>,
    mut mute_bg_q: Query<&mut Sprite, (With<MuteButton>, Without<DockButton>)>,
    mut mute_label_q: Query<&mut Text2d, (With<MuteButtonLabel>, Without<DockButtonLabel>)>,
) {
    if !mode.is_changed()
        && !hover.is_changed()
        && !mute_hover.is_changed()
        && !muted.is_changed()
    {
        return;
    }
    let dock_bg = if hover.0 {
        Color::srgba(0.14, 0.18, 0.22, 0.95)
    } else {
        Color::srgba(0.05, 0.06, 0.08, 0.85)
    };
    for mut sprite in &mut dock_bg_q {
        sprite.color = dock_bg;
    }
    for mut text in &mut dock_label_q {
        text.0 = mode.label().to_string();
    }
    let mute_bg = if mute_hover.0 {
        Color::srgba(0.14, 0.18, 0.22, 0.95)
    } else {
        Color::srgba(0.05, 0.06, 0.08, 0.85)
    };
    for mut sprite in &mut mute_bg_q {
        sprite.color = mute_bg;
    }
    for mut text in &mut mute_label_q {
        text.0 = if muted.0 { "UNMUTE" } else { "MUTE" }.to_string();
    }
}

/// Marker — true when the cursor is currently over the mute button.
/// Sibling of `DockButtonHover`. Both flags are OR'd by the input
/// plugin so neither button leaks clicks through to game.
#[derive(Resource, Default, Clone, Copy)]
pub struct MuteButtonHover(pub bool);

/// Detect cursor-over-button + click-toggle for both top-right
/// buttons. Updates `DockButtonHover` / `MuteButtonHover` every
/// frame; cycles `DisplayMode` or toggles `Muted` on left-click
/// within the respective bounds.
#[allow(clippy::too_many_arguments)]
fn handle_dock_input(
    windows: Query<&Window, With<PrimaryWindow>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut mode: ResMut<DisplayMode>,
    mut muted: ResMut<Muted>,
    mut hover: ResMut<DockButtonHover>,
    mut mute_hover: ResMut<MuteButtonHover>,
) {
    let Ok(window) = windows.single() else { return };
    let win_w = window.physical_width() as f32 / window.scale_factor();
    let _win_h = window.physical_height() as f32 / window.scale_factor();
    let Some(cursor) = window.cursor_position() else {
        if hover.0 { hover.0 = false; }
        if mute_hover.0 { mute_hover.0 = false; }
        return;
    };
    // Dock button bounds (top-right corner).
    let dock_max_x = win_w - BUTTON_INSET;
    let dock_min_x = dock_max_x - BUTTON_W;
    let min_y = BUTTON_INSET;
    let max_y = BUTTON_INSET + BUTTON_H;
    let on_dock =
        cursor.x >= dock_min_x && cursor.x <= dock_max_x && cursor.y >= min_y && cursor.y <= max_y;
    if hover.0 != on_dock {
        hover.0 = on_dock;
    }
    // Mute button bounds — one button-plus-inset to the left of dock.
    let mute_max_x = dock_min_x - BUTTON_INSET;
    let mute_min_x = mute_max_x - BUTTON_W;
    let on_mute =
        cursor.x >= mute_min_x && cursor.x <= mute_max_x && cursor.y >= min_y && cursor.y <= max_y;
    if mute_hover.0 != on_mute {
        mute_hover.0 = on_mute;
    }
    if mouse.just_pressed(MouseButton::Left) {
        if on_dock {
            *mode = mode.next();
        } else if on_mute {
            muted.0 = !muted.0;
        }
    }
}

/// In docked mode we keep pixel-perfect resolution and just show
/// **less** of the canvas vertically — the displayed view is a
/// horizontal strip of the RTT centered on the big rock. The strip
/// fills the full window width at integer scale; the strip's
/// vertical extent is whatever fits in the dock window's height at
/// that same scale. Fullscreen and Windowed clear the override so
/// the standard pipeline upscale handles them.
fn apply_docked_view_crop(
    mode: Res<DisplayMode>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut display_scale: ResMut<DisplayScale>,
    mut sprites: Query<&mut Sprite, With<UpscaleSprite>>,
) {
    let Ok(window) = windows.single() else { return };
    let win_w = window.physical_width() as f32 / window.scale_factor();
    let win_h = window.physical_height() as f32 / window.scale_factor();
    if win_w <= 0.0 || win_h <= 0.0 {
        return;
    }
    match *mode {
        DisplayMode::Fullscreen | DisplayMode::Windowed => {
            for mut sprite in &mut sprites {
                if sprite.rect.is_some() {
                    sprite.rect = None;
                }
            }
        }
        DisplayMode::Docked => {
            // Integer scale that fills the full window width with the
            // canvas's full width — pixel-perfect, no resolution
            // change. (`floor` rounds down so we never overflow.)
            let scale = (win_w / INTERNAL_WIDTH).floor().max(1.0);
            // How many canvas rows fit in the dock window's height
            // at this scale. Less than `INTERNAL_HEIGHT` means we
            // crop vertically — that's the whole point.
            let strip_h = (win_h / scale).floor().clamp(8.0, INTERNAL_HEIGHT);
            // Centre the strip vertically on the big rock so the
            // tossing/skim action lands in frame.
            let strip_top = (BIG_ROCK_Y - strip_h * 0.5)
                .clamp(0.0, INTERNAL_HEIGHT - strip_h)
                .round();
            let strip_bottom = strip_top + strip_h;
            display_scale.0 = scale;
            for mut sprite in &mut sprites {
                sprite.rect = Some(Rect::new(0.0, strip_top, INTERNAL_WIDTH, strip_bottom));
                sprite.custom_size =
                    Some(Vec2::new(INTERNAL_WIDTH * scale, strip_h * scale));
            }
        }
    }
}
