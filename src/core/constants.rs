//! Game-wide constants. Same internal-resolution / RTT-upscale strategy as
//! rust-SNKRX: 480×270 spec coords, top-left origin, Y-down. All gameplay
//! systems work in spec coords; `to_world` converts to Bevy's center-origin
//! Y-up world for transform sync.

use bevy::math::Vec3;

pub const INTERNAL_WIDTH: f32 = 480.0;
pub const INTERNAL_HEIGHT: f32 = 270.0;


/// X coordinate of the sand/water boundary. Sand is left of this line, water
/// is right. The transition is rendered as a thin foam strip in `bg.rs`.
pub const SHORELINE_X: f32 = 200.0;

/// The sand strip a small rock can land on after falling from the big rock.
/// Slightly inset from the shoreline so freshly-spawned rocks don't drop
/// into the water.
pub const SAND_LAND_X_MIN: f32 = 95.0;
pub const SAND_LAND_X_MAX: f32 = SHORELINE_X - 12.0;
pub const SAND_LAND_Y_MIN: f32 = 150.0;
pub const SAND_LAND_Y_MAX: f32 = 240.0;

/// Big rock position + size. Anchored to the left edge per the spec. The
/// boulder silhouette is wider than tall (`60×48` native), with an
/// asymmetric profile baked in `shapes::boulder_image`.
pub const BIG_ROCK_X: f32 = 56.0;
pub const BIG_ROCK_Y: f32 = 184.0;
pub const BIG_ROCK_W: f32 = 60.0;
pub const BIG_ROCK_H: f32 = 48.0;
/// Click radius — covers the full silhouette plus a few pixels of slop.
pub const BIG_ROCK_CLICK_R: f32 = 32.0;
/// Clicks per small-rock spawn (per the spec).
pub const CLICKS_PER_SMALL_ROCK: u32 = 10;

/// Click radius for small rocks. Native sprite size varies by shape (see
/// `shapes::SmallRockShape::size`); the click hitbox stays a uniform ~9 px
/// circle so picking feels consistent regardless of variant.
pub const SMALL_ROCK_CLICK_R: f32 = 9.0;

/// Skim physics tuning.
pub const SKIM_SPEED: f32 = 165.0;
/// Time between bounce checks while a rock is skimming. With SKIM_SPEED
/// above, this lands roughly one bounce every ~55 px of horizontal travel.
pub const SKIM_BOUNCE_INTERVAL: f32 = 0.34;
/// Initial arc height between bounces — peak Y rise above water.
pub const SKIM_ARC_HEIGHT: f32 = 9.0;
/// Each successive bounce loses this fraction of arc height (skim decay).
pub const SKIM_ARC_DECAY: f32 = 0.85;

/// Particle tuning.
pub const ROCK_DUST_PER_CLICK: u32 = 7;

/// Z layers — lower draws first.
pub const Z_BG: f32 = 0.0;
pub const Z_BG_DETAIL: f32 = 0.5;
pub const Z_RIPPLE: f32 = 2.0;
pub const Z_ROCK: f32 = 3.0;
pub const Z_CAVE: f32 = 3.2;
pub const Z_PIER: f32 = 3.25;
pub const Z_FISH: f32 = 3.4;
pub const Z_HUT: f32 = 3.3;
pub const Z_BIGROCK: f32 = 3.5;
pub const Z_PARTICLE: f32 = 4.0;
pub const Z_CREW: f32 = 4.2;
pub const Z_PICKAXE: f32 = 4.3;
pub const Z_FLOATING: f32 = 5.0;
pub const Z_BUTTON: f32 = 9.0;
pub const Z_UI: f32 = 10.0;

/// Foragers-hut economy. All worker conversions are uniformly priced
/// — 1 worker + 10 skims — so the cost is a pair (workers, skims).
pub const HUT_COST: u64 = 10;
pub const WORKER_COST: u64 = 10;
/// Pier — second one-time structure, unlocks fish purchases.
pub const PIER_COST: u64 = 30;
/// Fish — repeatable, bought from the pier panel. Each purchase is
/// a *bucket* — one purchase spawns `FISHES_PER_BUCKET` one-shot
/// fish that despawn after their first rescue.
pub const FISH_COST: u64 = 5;
pub const FISHES_PER_BUCKET: u32 = 10;
pub const STARTING_WORKERS_FROM_HUT: u32 = 2;

/// Foragers hut placement — top-left of the sand, well clear of the
/// big rock (centred at 56,184) and the small-rock landing zone
/// (x 95-188, y 150-240). Three sibling huts (miner, skimmer, fisher)
/// sit at the other corners of the sand and unlock alongside it.
pub const HUT_X: f32 = 50.0;
pub const HUT_Y: f32 = 100.0;
pub const HUT_MINER_X: f32 = 50.0;
pub const HUT_MINER_Y: f32 = 145.0;
pub const HUT_SKIMMER_X: f32 = 50.0;
pub const HUT_SKIMMER_Y: f32 = 220.0;
pub const HUT_FISHER_X: f32 = 50.0;
pub const HUT_FISHER_Y: f32 = 250.0;
pub const HUT_BODY_W: f32 = 16.0;
pub const HUT_BODY_H: f32 = 10.0;
pub const HUT_ROOF_W: f32 = 22.0;
pub const HUT_ROOF_H: f32 = 8.0;
/// Radius around the hut where idle workers wander.
pub const WORKER_WANDER_RADIUS: f32 = 14.0;

/// Foragers cave — small dark mound on the sand, sits to the west of
/// where the hut will appear. Always present; hovering it surfaces the
/// BUY HUT button.
pub const CAVE_X: f32 = 16.0;
pub const CAVE_Y: f32 = 110.0;
pub const CAVE_W: f32 = 20.0;
pub const CAVE_H: f32 = 16.0;
pub const CAVE_OPENING_W: f32 = 7.0;

/// Pier — wooden walkway that extends from the shoreline into the
/// water once purchased. Centre y is in the upper third of the
/// canvas so the pier sits clear of the small-rock landing zone.
pub const PIER_X: f32 = 240.0;
pub const PIER_Y: f32 = 90.0;
pub const PIER_W: f32 = 64.0;
pub const PIER_H: f32 = 5.0;
pub const STARTING_FISH_FROM_PIER: u32 = 1;
/// How close (spec px) a fish needs to be to a sinking rock to flick
/// it back into a bounce.
pub const FISH_ASSIST_RADIUS: f32 = 22.0;
pub const CAVE_OPENING_H: f32 = 5.0;

/// Miner timing — one full throw/fetch cycle is roughly 10 seconds.
pub const MINER_THROW_WIND_UP: f32 = 0.9;
pub const MINER_PICKAXE_FLIGHT: f32 = 1.1;
pub const MINER_REST: f32 = 0.6;
pub const MINER_WALK_SPEED: f32 = 14.0;
pub const MINER_HUT_TO_SPOT_SPEED: f32 = 26.0;

/// Skimmer timing — full pick-up-and-throw cycle is roughly 15 seconds.
pub const SKIMMER_WALK_SPEED: f32 = 18.0;
pub const SKIMMER_PICKUP_TIME: f32 = 0.6;
pub const SKIMMER_CHARGE_TIME: f32 = 1.2;
pub const SKIMMER_REST: f32 = 0.5;
pub const SKIMMER_SEARCH_RETRY: f32 = 0.8;
/// Bounce chance per arc when a skimmer-thrown stone lands on water.
/// (Player-thrown stones bounce 50%; skimmers are bad throwers.)
pub const SKIMMER_BOUNCE_CHANCE: f32 = 0.25;
pub const PLAYER_BOUNCE_CHANCE: f32 = 0.5;

/// Bounce chance added per `Skim Up` upgrade (one row in the skimmer
/// panel). Stacks linearly; the effective chance is clamped before the
/// dice roll so we never exceed 95%.
pub const SKIM_UPGRADE_DELTA: f32 = 0.05;
/// Maximum effective bounce chance after upgrades — never quite 100%.
pub const BOUNCE_CHANCE_MAX: f32 = 0.95;
/// Cost in skims of one Skim Up upgrade.
pub const SKIM_UPGRADE_COST: u64 = 25;

/// Fisherman timing — sits at the water edge and pulls something up
/// every 7-13 seconds. 50/50 whether the catch is a rock.
pub const FISHERMAN_FISH_TIME_MIN: f32 = 7.0;
pub const FISHERMAN_FISH_TIME_MAX: f32 = 13.0;
pub const FISHERMAN_CATCH_CHANCE: f64 = 0.5;
pub const FISHERMAN_PULL_TIME: f32 = 0.6;
pub const FISHERMAN_WALK_SPEED: f32 = 24.0;

/// Spec → Bevy world conversion. Spec is top-left origin Y-down; Bevy world
/// is center-origin Y-up. The game camera's orthographic projection at
/// scale 1.0 means 1 spec px == 1 world unit, which lands the 480×270 spec
/// area exactly on the 480×270 RTT image.
#[inline]
pub fn to_world(spec_x: f32, spec_y: f32, z: f32) -> Vec3 {
    Vec3::new(
        spec_x - INTERNAL_WIDTH * 0.5,
        INTERNAL_HEIGHT * 0.5 - spec_y,
        z,
    )
}
