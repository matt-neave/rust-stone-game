//! Mouse input → spec-space click events. Reads cursor position each frame,
//! converts window pixels to internal-canvas coords using the current
//! `DisplayScale`, and emits `ClickEvent` on left-button presses.
//!
//! Mouse-to-spec math is the same as rust-SNKRX/src/input.rs: the upscale
//! sprite is centered in the window at integer scale, so:
//!   spec = (cursor - letterbox_offset) / display_scale

use bevy::input::ButtonInput;
use bevy::prelude::*;

use crate::core::constants::{INTERNAL_HEIGHT, INTERNAL_WIDTH};
use crate::render::{
    CameraScroll, DisplayMode, DisplayScale, DockButtonHover, MuteButtonHover,
};

#[derive(Message)]
pub struct ClickEvent {
    /// Click position in spec coordinates.
    pub pos: Vec2,
}

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ClickEvent>()
            .add_systems(Update, emit_clicks);
    }
}

fn emit_clicks(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    display_scale: Res<DisplayScale>,
    mode: Res<DisplayMode>,
    dock_hover: Res<DockButtonHover>,
    mute_hover: Res<MuteButtonHover>,
    scroll: Res<CameraScroll>,
    mut writer: MessageWriter<ClickEvent>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    // Docked mode: the game plays out non-interactively — only the
    // dock + mute buttons are interactive (handled in render::dock).
    if *mode == DisplayMode::Docked {
        return;
    }
    // Cursor is over a top-right HUD button — let it own the click.
    if dock_hover.0 || mute_hover.0 {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(spec) = cursor_to_spec(window, display_scale.0, scroll.x) else {
        return;
    };
    writer.write(ClickEvent { pos: spec });
}

/// Convert a window's cursor position to spec (internal-canvas) coordinates,
/// accounting for the integer-upscale letterboxing **and** the current
/// horizontal camera scroll. The returned spec is in *world* spec
/// coordinates — i.e., directly comparable to entity `Pos` values —
/// not screen-relative. Returns `None` if the cursor is outside the
/// visible canvas (or the window has no cursor).
///
/// Public so other systems can read the cursor without re-doing the math.
/// `bigrock`'s autoclick uses it to gate hold-to-autoclick on cursor
/// position.
pub fn cursor_to_spec(window: &Window, display_scale: f32, scroll_x: f32) -> Option<Vec2> {
    let cursor = window.cursor_position()?;
    let win_w = window.width();
    let win_h = window.height();
    let scale = display_scale.max(1.0);
    let display_w = INTERNAL_WIDTH * scale;
    let display_h = INTERNAL_HEIGHT * scale;
    let off_x = (win_w - display_w) * 0.5;
    let off_y = (win_h - display_h) * 0.5;
    let screen_x = (cursor.x - off_x) / scale;
    let screen_y = (cursor.y - off_y) / scale;
    if screen_x < 0.0 || screen_x > INTERNAL_WIDTH || screen_y < 0.0 || screen_y > INTERNAL_HEIGHT {
        return None;
    }
    Some(Vec2::new(screen_x + scroll_x, screen_y))
}
