//! Autonomous crew — the people that gather around the foragers hut
//! and do the work of mining, skimming, and fishing.
//!
//! ## Design
//!
//! Each crew role lives in its own submodule and ships as its own
//! [`Plugin`]. Adding a new role doesn't require editing the
//! dispatcher, the animation system, or any other role's file —
//! everything role-specific is colocated.
//!
//! Roles share three contracts:
//!
//! 1. **The [`CrewWalking`] trait** — implemented by each role's
//!    primary state component, lets [`tick_walk_animation`] flip the
//!    humanoid sprite between standing/walking frames generically.
//! 2. **[`SpawnConversionEvent`]** — fired by [`purchase`] when the
//!    player buys a specialist role. Each role's spawn system filters
//!    the event by [`PurchaseKind`].
//! 3. **[`pick_worker_to_convert`]** — picks the worker entity nearest
//!    the hut door so conversions visibly remove someone from the
//!    crowd around the hut.
//!
//! ## Adding a new role
//!
//! 1. Create `crew/<name>.rs` with:
//!    * A component holding the role's state machine.
//!    * `impl CrewWalking for <Name> { ... }`.
//!    * A `<Name>Plugin` that registers the tick + walk-anim
//!      (`tick_walk_animation::<Name>`) + a spawn handler that
//!      filters [`SpawnConversionEvent`] by the matching
//!      [`PurchaseKind`].
//! 2. Add `pub mod <name>;` here and append `<name>::<Name>Plugin`
//!    to [`CrewPlugin::build`].
//! 3. Extend [`PurchaseKind`] (`crate::economy`) and the panel data
//!    (`HUT_PANEL_KINDS`, [`crate::economy::detail_for`], etc.).
//!
//! No edits to the dispatcher or animation are required.

use bevy::ecs::component::Mutable;
use bevy::prelude::*;

use crate::core::common::Pos;
use crate::core::constants::{HUT_BODY_H, HUT_X, HUT_Y};
use crate::render::shapes::Shapes;

pub mod beachcomber;
pub mod boatman;
pub mod fisherman;
pub mod miner;
pub mod purchase;
pub mod skimmer;
pub mod stonemason;
pub mod worker;

pub use purchase::SpawnConversionEvent;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct CrewPlugin;

impl Plugin for CrewPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SpawnConversionEvent>()
            .add_systems(Update, purchase::handle_role_purchase)
            .add_plugins((
                worker::WorkerPlugin,
                miner::MinerPlugin,
                skimmer::SkimmerPlugin,
                fisherman::FishermanPlugin,
                beachcomber::BeachcomberPlugin,
                stonemason::StonemasonPlugin,
                boatman::BoatmanPlugin,
            ));
    }
}

// ---------------------------------------------------------------------------
// Walking animation — generic over the role component
// ---------------------------------------------------------------------------

/// Implemented by each role's primary component so a single generic
/// system can run the walk-frame swap for every crew type.
///
/// The `Mutability = Mutable` bound is required by Bevy's `Query<&mut T>`
/// — components inherit immutability via the derive macro by default and
/// must opt in to mutable access through this associated type.
///
/// `step_walking` is delegated to each impl rather than provided as a
/// default body so that each type can split-borrow its own private
/// timer and frame-toggle fields directly (which trait methods can't
/// do across separate accessors).
pub trait CrewWalking: Component<Mutability = Mutable> {
    /// Whether the entity is in a state that should animate walking.
    fn is_walking(&self) -> bool;
    /// Current pose toggle: true means the "walk" sprite, false the
    /// "stand" sprite.
    fn walk_frame(&self) -> bool;
    /// Advance the walk-frame timer. Returns true if the sprite
    /// should be swapped this frame; the caller reads
    /// [`walk_frame`](Self::walk_frame) afterward for the new pose.
    fn step_walking(&mut self, walking: bool, dt: f32) -> bool;
}

/// Generic walk-frame system. Each role's plugin registers its own
/// typed instance via `app.add_systems(Update, tick_walk_animation::<Self>)`.
pub fn tick_walk_animation<T: CrewWalking>(
    time: Res<Time>,
    mut q: Query<(&mut T, &mut Sprite)>,
    shapes: Res<Shapes>,
) {
    let dt = time.delta_secs();
    for (mut x, mut sprite) in &mut q {
        let walking = x.is_walking();
        if x.step_walking(walking, dt) {
            sprite.image = humanoid_image(&shapes, x.walk_frame());
        }
    }
}

/// Advance a walk-frame timer. Returns true if the sprite should be
/// swapped this frame (either to a new walk pose or back to standing).
pub fn step_walk_frame(walking: bool, accum: &mut f32, frame: &mut bool, dt: f32) -> bool {
    const FRAME_INTERVAL: f32 = 0.18;
    if walking {
        *accum += dt;
        if *accum >= FRAME_INTERVAL {
            *accum -= FRAME_INTERVAL;
            *frame = !*frame;
            return true;
        }
        false
    } else if *frame {
        *frame = false;
        *accum = 0.0;
        true
    } else {
        false
    }
}

pub fn humanoid_image(shapes: &Shapes, walk_frame: bool) -> Handle<Image> {
    if walk_frame {
        shapes.humanoid_walk.clone()
    } else {
        shapes.humanoid.clone()
    }
}

// ---------------------------------------------------------------------------
// Shared worker picking
// ---------------------------------------------------------------------------

/// Pick the worker closest to the hut door. Conversions despawn the
/// most-prominent worker so the player visibly loses someone from the
/// crowd around the hut.
pub fn pick_worker_to_convert(
    worker_q: &Query<(Entity, &Pos), With<worker::Worker>>,
) -> Option<(Entity, Vec2)> {
    let door = Vec2::new(HUT_X, HUT_Y + HUT_BODY_H * 0.5 + 4.0);
    let mut best: Option<(f32, Entity, Vec2)> = None;
    for (e, pos) in worker_q {
        let d = pos.0.distance(door);
        if best.map_or(true, |(bd, _, _)| d < bd) {
            best = Some((d, e, pos.0));
        }
    }
    best.map(|(_, e, p)| (e, p))
}

// Re-exports used by neighbours.
pub use worker::Worker;
