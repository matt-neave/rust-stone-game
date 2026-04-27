//! High-DPI text component + per-frame layout sync.
//!
//! [`UiText`] holds a position + font size in spec units (480×270
//! top-left, Y-down). Every frame, [`sync_ui_text`] multiplies both by
//! [`DisplayScale`] and rewrites the entity's `Transform` and
//! `TextFont` so the underlying glyph rasteriser samples against the
//! window's native pixel grid — text stays sharp regardless of how the
//! 480×270 RTT is upscaled.
//!
//! `TextFont` mutations force Bevy's text pipeline to re-rasterise the
//! glyph atlas, so the font size is only touched when the value
//! actually changes (effectively only on window resize).

use bevy::prelude::*;

use crate::core::constants::{INTERNAL_HEIGHT, INTERNAL_WIDTH};
use crate::render::pipeline::DisplayScale;

#[derive(Component, Clone, Copy)]
pub struct UiText {
    pub spec_pos: Vec2,
    pub spec_font_size: f32,
    pub z: f32,
}

pub(super) fn sync_ui_text(
    display_scale: Res<DisplayScale>,
    mut q: Query<(&UiText, &mut Transform, &mut TextFont)>,
) {
    let s = display_scale.0.max(1.0);
    for (ui, mut tf, mut font) in &mut q {
        tf.translation.x = (ui.spec_pos.x - INTERNAL_WIDTH * 0.5) * s;
        tf.translation.y = (INTERNAL_HEIGHT * 0.5 - ui.spec_pos.y) * s;
        tf.translation.z = ui.z;
        let new_size = (ui.spec_font_size * s).max(1.0);
        if (font.font_size - new_size).abs() > 0.01 {
            font.font_size = new_size;
        }
    }
}
