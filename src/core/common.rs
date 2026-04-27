//! Shared ECS components and the `Pos → Transform` sync system. Same shape
//! as rust-SNKRX's `common.rs` — gameplay lives in spec coordinates, the
//! sync system is the only code that knows about Bevy's center-origin world.

use bevy::prelude::*;

use crate::core::constants::{to_world, Z_ROCK};

/// Logical position in spec coordinates (top-left origin, Y-down).
/// For rocks this is always the *ground* position — air arc lives in
/// [`ZHeight`] so paired shadows can track the ground independently.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Pos(pub Vec2);

/// Velocity in spec coordinates (px/s). Reserved for future use — the
/// current entities encode their own motion in their phase enums, but this
/// is the canonical bucket if anything wants generic linear velocity.
#[allow(dead_code)]
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Vel(pub Vec2);

/// Height above the entity's ground position, in spec px. Positive
/// means "up" in screen space (away from the ground). Rendered by
/// [`sync_transforms`] as a y-offset on top of `Pos`. Decoupling the
/// arc from `Pos` lets paired shadow entities track only the ground
/// position while the rock visually arcs above.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ZHeight(pub f32);

/// Z layer used by the transform-sync system.
#[derive(Component, Debug, Clone, Copy)]
pub struct Layer(pub f32);

impl Default for Layer {
    fn default() -> Self {
        Self(Z_ROCK)
    }
}

/// Public ordering anchors for everything that touches `Transform` per frame.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum SyncSet {
    Transforms,
    Visuals,
}

pub struct CommonPlugin;

impl Plugin for CommonPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(PostUpdate, (SyncSet::Transforms, SyncSet::Visuals).chain())
            .add_systems(PostUpdate, sync_transforms.in_set(SyncSet::Transforms));
    }
}

fn sync_transforms(
    mut q: Query<(&Pos, Option<&Layer>, Option<&ZHeight>, &mut Transform)>,
) {
    for (pos, layer, height, mut tf) in &mut q {
        let z = layer.map(|l| l.0).unwrap_or(Z_ROCK);
        // Spec is Y-down, so positive z_height (= "up") shifts the visual
        // y up by subtracting from the ground spec y before converting.
        let h = height.map(|h| h.0).unwrap_or(0.0);
        tf.translation = to_world(pos.0.x, pos.0.y - h, z);
    }
}
