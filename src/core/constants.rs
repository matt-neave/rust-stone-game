//! Game-wide constants. Same internal-resolution / RTT-upscale strategy as
//! rust-SNKRX: 480×270 spec coords, top-left origin, Y-down. All gameplay
//! systems work in spec coords; `to_world` converts to Bevy's center-origin
//! Y-up world for transform sync.

use bevy::math::Vec3;

pub const INTERNAL_WIDTH: f32 = 480.0;
pub const INTERNAL_HEIGHT: f32 = 270.0;

/// Total spec width of the scrollable world. The camera shows a
/// 480-wide window into this strip; everything in between scrolls
/// horizontally as the player presses left / right (or A / D). Sand
/// stays anchored to the left side of the world; the rest is open
/// water.
pub const WORLD_WIDTH: f32 = 1440.0;
/// Spec x of the world's left edge. Negative so the playable area
/// extends *left* of the original 0-anchored canvas; this is where
/// the standalone tree lives, accessible only by scrolling.
pub const WORLD_LEFT: f32 = -240.0;

/// Standalone tree placement — sits well to the left of the big
/// rock so the player has to scroll west to see it. Z_TREE matches
/// the hut layer so it depth-sorts correctly with crew sprites.
pub const TREE_X: f32 = -120.0;
pub const TREE_Y: f32 = 195.0;

/// Tree storage — small wreck/box just east of the tree. Spawned
/// initially as scattered "broken" rubble after the research mission
/// cinematic completes; replaced by a whole crate on TreeStorage
/// purchase.
pub const TREE_STORAGE_X: f32 = TREE_X + 35.0;
pub const TREE_STORAGE_Y: f32 = TREE_Y + 10.0;

/// Research-mission scout cinematic timings + destination.
/// The scout walks at a normal worker-style pace and stops at the
/// research hut first (briefing beat with a `!` above their head)
/// before continuing west to the tree.
pub const SCOUT_WALK_SPEED: f32 = 28.0;
pub const SCOUT_AT_HUT_DURATION: f32 = 5.0;
pub const SCOUT_PRESENT_X: f32 = -30.0;
pub const SCOUT_PRESENT_Y: f32 = 200.0;
pub const SCOUT_SPEAK_DURATION: f32 = 3.0;
pub const SCOUT_PAN_DURATION: f32 = 1.5;
pub const SCOUT_HOLD_DURATION: f32 = 1.5;
pub const SCROLL_FOR_SCOUT: f32 = -180.0;
pub const SCROLL_FOR_TREE: f32 = -240.0;

/// Wood-piece interaction radii + landing-spot bounds (so wood always
/// stays on sand, never on the water side).
pub const WOOD_CLICK_R: f32 = 5.0;
pub const WOOD_LAND_X_MIN: f32 = WORLD_LEFT + 5.0;
pub const WOOD_LAND_X_MAX: f32 = -50.0;
pub const WOOD_LAND_Y_MIN: f32 = 160.0;
pub const WOOD_LAND_Y_MAX: f32 = 250.0;


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
/// Clicks on the tree per wood-piece spawn — mirror of the big-rock
/// click counter, scaled up so wood feels like a chunkier resource.
pub const CLICKS_PER_WOOD: u32 = 25;

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
/// Fish — repeatable, bought from the pier panel. Each purchase is
/// a *bucket* — one purchase spawns `FISHES_PER_BUCKET` one-shot
/// fish that despawn after their first rescue.
pub const FISH_COST: u64 = 5;
pub const FISHES_PER_BUCKET: u32 = 10;

/// Village layout — every hut now lives in the top half of the canvas
/// (y < 135) so the bottom half stays clear for the small-rock
/// landing zone (y 150-240) and crew animations on the open sand.
/// Two columns: foragers stack at x=50, beachcomber stack at x=140.
/// Y values are staggered between columns by ~15 px so each panel
/// sits in a clean horizontal strip when its hut is hovered.
pub const HUT_X: f32 = 50.0;
pub const HUT_Y: f32 = 30.0;
pub const HUT_MINER_X: f32 = 50.0;
pub const HUT_MINER_Y: f32 = 60.0;
pub const HUT_SKIMMER_X: f32 = 50.0;
pub const HUT_SKIMMER_Y: f32 = 90.0;
pub const HUT_FISHER_X: f32 = 50.0;
pub const HUT_FISHER_Y: f32 = 120.0;
pub const HUT_BEACHCOMBER_X: f32 = 140.0;
pub const HUT_BEACHCOMBER_Y: f32 = 45.0;
pub const HUT_STONEMASON_X: f32 = 140.0;
pub const HUT_STONEMASON_Y: f32 = 75.0;
/// Research facility + Aqua Center — bottom of the right-hand column.
pub const HUT_RESEARCH_X: f32 = 140.0;
pub const HUT_RESEARCH_Y: f32 = 105.0;
pub const HUT_AQUA_X: f32 = 140.0;
pub const HUT_AQUA_Y: f32 = 130.0;
/// AutoFishing target stockpile — the auto-fisher buys buckets while
/// `Fishes.count` is below this threshold.
pub const AUTO_FISHING_TARGET: u32 = 100;
/// Seconds between auto-fishing checks.
pub const AUTO_FISHING_TICK: f32 = 1.0;
/// Port — water-side structure, gated behind the pier. Boatmen
/// launch from here and patrol the ocean.
pub const PORT_X: f32 = 240.0;
pub const PORT_Y: f32 = 230.0;
pub const PORT_W: f32 = 36.0;
pub const PORT_H: f32 = 12.0;
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
/// Hard cap on repeatable upgrade levels (Skim Up, Pickaxe). Once
/// purchased to this many levels the row darkens and stops accepting
/// further buys.
pub const UPGRADE_LEVEL_CAP: u32 = 4;

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
