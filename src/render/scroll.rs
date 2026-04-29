//! Horizontal camera scrolling for the wider-than-screen world.
//!
//! The world is `WORLD_WIDTH` spec px wide; the game camera's RTT is
//! 480 px wide, so it shows a sliding window into the world. Pressing
//! Left/Right (or A/D) advances [`CameraScroll`], which is mirrored
//! into [`GameCamera`]'s transform each frame.
//!
//! Screen-anchored UI sprites (the top-left HUD container, its bars
//! and icons) carry a [`ScreenAnchored`] marker. Their `Pos.x` is
//! rewritten each frame to `anchor_spec_x + scroll.x`, keeping them
//! visually pinned to the screen even as the rest of the world scrolls
//! beneath them.

use bevy::input::ButtonInput;
use bevy::prelude::*;

use crate::core::common::Pos;
use crate::core::constants::{INTERNAL_WIDTH, WORLD_WIDTH};

use super::pipeline::GameCamera;

/// Horizontal scroll offset of the game camera, in spec px. 0.0
/// corresponds to the world's left edge being at the left of the
/// visible window; max value lines up the world's right edge with
/// the right of the visible window.
#[derive(Resource, Default, Clone, Copy)]
pub struct CameraScroll {
    pub x: f32,
}

/// Pixels per second the player can scroll with the keyboard.
const SCROLL_SPEED: f32 = 220.0;

/// A sprite that should stay glued to the screen even as the camera
/// scrolls. `spec_x` is the desired offset from the screen's left
/// edge; the sync system rewrites the entity's `Pos.x` each frame to
/// `spec_x + scroll.x` so the sprite renders at the same on-screen
/// location regardless of the camera position.
#[derive(Component)]
pub struct ScreenAnchored {
    pub spec_x: f32,
}

pub struct ScrollPlugin;

impl Plugin for ScrollPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraScroll>().add_systems(
            Update,
            (tick_scroll_input, sync_camera, sync_screen_anchored).chain(),
        );
    }
}

fn tick_scroll_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut scroll: ResMut<CameraScroll>,
) {
    let mut dx = 0.0;
    if keys.pressed(KeyCode::ArrowLeft) || keys.pressed(KeyCode::KeyA) {
        dx -= 1.0;
    }
    if keys.pressed(KeyCode::ArrowRight) || keys.pressed(KeyCode::KeyD) {
        dx += 1.0;
    }
    if dx == 0.0 {
        return;
    }
    let max_scroll = (WORLD_WIDTH - INTERNAL_WIDTH).max(0.0);
    let new_x = (scroll.x + dx * SCROLL_SPEED * time.delta_secs()).clamp(0.0, max_scroll);
    if (new_x - scroll.x).abs() > f32::EPSILON {
        scroll.x = new_x;
    }
}

fn sync_camera(
    scroll: Res<CameraScroll>,
    mut q: Query<&mut Transform, With<GameCamera>>,
) {
    if !scroll.is_changed() {
        return;
    }
    for mut t in &mut q {
        // The game camera lives in world coords (Y-up), but the X
        // axis matches spec 1:1 — moving the camera right by N world
        // units shifts the visible spec window right by N px.
        t.translation.x = scroll.x;
    }
}

fn sync_screen_anchored(
    scroll: Res<CameraScroll>,
    mut q: Query<(&ScreenAnchored, &mut Pos)>,
) {
    if !scroll.is_changed() {
        return;
    }
    for (anchor, mut pos) in &mut q {
        pos.0.x = anchor.spec_x + scroll.x;
    }
}
