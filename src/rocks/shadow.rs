//! Drop shadows for rocks. A shadow is a paired entity that lives on
//! the ground and follows the linked rock's `Pos` (the *ground*
//! position; the rock's air arc lives in `ZHeight` per
//! [`crate::core::common`]). The shadow softly shrinks and fades with
//! the rock's height, reads as the rock's footprint when idle on sand,
//! and hides while the rock is bouncing on the water surface.
//!
//! This is what gives the toss/skim arcs their pseudo-3D feel: the
//! rock visibly leaves the ground while a flat shadow slides along
//! beneath it.

use bevy::color::Alpha;
use bevy::prelude::*;

use crate::core::colors;
use crate::core::common::{Layer, Pos, ZHeight};
use crate::core::constants::*;
use crate::render::shapes::Shapes;
use crate::rocks::small::SmallRock;

/// Z-layer for shadows — above the bg detail/sand patches but below
/// imprints, ripples and rocks. Sits on the ground plane visually.
const Z_SHADOW: f32 = Z_BG_DETAIL + 0.45;

/// Base alpha when the rock is at ground level. Tuned soft so shadows
/// read as gentle smudges, not hard ink blots.
const SHADOW_BASE_ALPHA: f32 = 0.42;

/// Per-axis offset of the shadow from the rock's ground position. The
/// scene is lit from the top-right, so shadows fall to the bottom-left.
/// In spec coords (y-down), bottom = +y and left = -x.
const SHADOW_OFFSET_X: f32 = -1.5;
const SHADOW_OFFSET_Y: f32 = 1.5;

/// Arc height (spec px) at which the shadow has fully shrunk and faded
/// to its minimum. Above this, the shadow holds steady at minimum.
const SHADOW_MAX_FADE_HEIGHT: f32 = 36.0;

/// Minimum alpha multiplier — even the highest-arcing rock keeps a
/// faint smudge below it so the player can read where it'll land.
const SHADOW_MIN_ALPHA_MULT: f32 = 0.45;

/// Minimum scale multiplier — likewise, the shadow shrinks as the rock
/// rises but never disappears.
const SHADOW_MIN_SCALE: f32 = 0.55;

/// Shadow sprite — present alone or as the paired drop-shadow of a
/// rock. Use [`ShadowOf`] to link a tracking shadow to its rock.
#[derive(Component)]
pub struct Shadow;

/// Links a shadow to its rock. The shadow's `Pos` is updated each
/// frame to match the rock's ground position; when the rock entity is
/// gone the shadow auto-despawns.
#[derive(Component)]
pub struct ShadowOf(pub Entity);

/// Original-size cache so the per-frame scale system can shrink the
/// shadow proportionally without needing to know the rock's geometry.
#[derive(Component, Clone, Copy)]
pub struct ShadowBaseSize(pub Vec2);

pub struct ShadowPlugin;

impl Plugin for ShadowPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sync_rock_shadows);
    }
}

/// Spawn a shadow paired to a small rock. `ground_pos` is the rock's
/// initial ground position; `rock_size` is the rock's render size in
/// spec px (the shadow is sized as a flatter, slightly wider ellipse).
pub fn spawn_rock_shadow(
    commands: &mut Commands,
    shapes: &Shapes,
    rock: Entity,
    ground_pos: Vec2,
    rock_size: Vec2,
) {
    let base_size = shadow_size_for(rock_size);
    let mut color = colors::ROCK_DARK;
    color.set_alpha(SHADOW_BASE_ALPHA);
    commands.spawn((
        Shadow,
        ShadowOf(rock),
        ShadowBaseSize(base_size),
        // Offset to the bottom-left so the shadow sits where the
        // top-right light source would cast it.
        Pos(ground_pos + Vec2::new(SHADOW_OFFSET_X, SHADOW_OFFSET_Y)),
        Layer(Z_SHADOW),
        Sprite {
            image: shapes.shadow.clone(),
            color,
            custom_size: Some(base_size),
            ..default()
        },
        Transform::default(),
    ));
}

/// Spawn a static shadow under a stationary entity (the big rock).
/// No tracking — the shadow stays at its initial position and at full
/// alpha forever.
pub fn spawn_static_shadow(
    commands: &mut Commands,
    shapes: &Shapes,
    pos: Vec2,
    size: Vec2,
    alpha: f32,
) {
    let mut color = colors::ROCK_DARK;
    color.set_alpha(alpha);
    commands.spawn((
        Shadow,
        Pos(pos),
        Layer(Z_SHADOW),
        Sprite {
            image: shapes.shadow.clone(),
            color,
            custom_size: Some(size),
            ..default()
        },
        Transform::default(),
    ));
}

/// Default shadow size for a rock of `rock_size`. Slightly wider than
/// the rock and substantially flatter so it reads as a ground-cast
/// drop shadow rather than a duplicate body.
fn shadow_size_for(rock_size: Vec2) -> Vec2 {
    let w = rock_size.x + 1.0;
    let h = (rock_size.y * 0.36).max(2.0);
    Vec2::new(w, h)
}

/// Per-frame: copy each rock's ground `Pos` into its shadow, fade and
/// shrink the shadow with the rock's `ZHeight`, and despawn shadows
/// whose rock has vanished. Shadows track the rock onto the water too
/// — during a toss arc, while the rock skims between bounces, the
/// shadow on the water surface gives the bounces their depth.
fn sync_rock_shadows(
    mut commands: Commands,
    mut shadows: Query<
        (Entity, &ShadowOf, &ShadowBaseSize, &mut Pos, &mut Sprite),
        (With<Shadow>, Without<SmallRock>),
    >,
    rocks: Query<(&Pos, &ZHeight), With<SmallRock>>,
) {
    for (e, link, base_size, mut shadow_pos, mut sprite) in &mut shadows {
        let Ok((rock_pos, zh)) = rocks.get(link.0) else {
            commands.entity(e).despawn();
            continue;
        };
        // Anchor the shadow at the rock's ground position offset to the
        // bottom-left, matching the top-right light source.
        shadow_pos.0 = rock_pos.0 + Vec2::new(SHADOW_OFFSET_X, SHADOW_OFFSET_Y);

        // Height fade — eases as the rock rises, bottoms out at the
        // configured minimum so a tossed or skimming rock always shows
        // a hint of where it'll land.
        let fade_t = (zh.0 / SHADOW_MAX_FADE_HEIGHT).clamp(0.0, 1.0);
        let alpha_mult = 1.0 - (1.0 - SHADOW_MIN_ALPHA_MULT) * fade_t;
        let scale = 1.0 - (1.0 - SHADOW_MIN_SCALE) * fade_t;

        let alpha = SHADOW_BASE_ALPHA * alpha_mult;
        let mut c = sprite.color;
        c.set_alpha(alpha);
        sprite.color = c;
        sprite.custom_size = Some(base_size.0 * scale);
    }
}
