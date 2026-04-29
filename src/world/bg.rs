//! Beach + water background — minimal and SNKRX-style.
//!
//! Sand keeps the SNKRX-style two-tone checker (`rust-SNKRX/src/bg.rs`):
//! a base color plus alternating "off" tiles for very subtle variation.
//! Water is a five-band shore→deep gradient, rendered as 1-px horizontal
//! slices so the seams between bands can wobble on a sine and not read
//! as straight stripes.
//!
//! The shoreline is two thin vertical strips: a wet-sand band and a foam
//! line, sitting between the sand checker and the water gradient.

use std::f32::consts::TAU;

use bevy::prelude::*;
use rand::Rng;

use crate::core::colors;
use crate::core::common::{Layer, Pos};
use crate::core::constants::*;
use crate::render::shapes::Shapes;

/// Number of light-tinted sand grains scattered over the base sand.
/// Each grain is a single 1×1 pixel placed on the integer grid so the
/// sprinkle reads as natural granular noise rather than sub-pixel
/// blocks. Bumped from the old fractional-rect count since each entry
/// now covers far less area.
const SAND_LIGHT_GRAINS: u32 = 900;
/// Number of dark-tinted sand grains — sparser so they read as
/// scattered grit rather than shadow.
const SAND_DARK_GRAINS: u32 = 380;
/// Per-grain probability of an extra 1-px neighbour stuck on a random
/// side. Adds the occasional 2-px cluster for size variation without
/// the regularity of fixed-size rectangles.
const SAND_GRAIN_CLUSTER_CHANCE: f64 = 0.18;

/// Shore→deep band palette. Index 0 is closest to the sand, index 4 is the
/// deepest water at the right edge.
const WATER_BANDS: [Color; 5] = [
    colors::WATER_BAND_1,
    colors::WATER_BAND_2,
    colors::WATER_BAND_3,
    colors::WATER_BAND_4,
    colors::WATER_BAND_5,
];

/// Height of one water slice. 1 px keeps the sine seams smooth at internal
/// resolution; the whole field is ~270 slices × 5 bands ≈ 1350 sprites,
/// all spawned once at startup.
const WATER_SLICE_H: f32 = 1.0;
/// Vertical wavelength of the seam wobble, in spec px.
const WATER_WAVE_LEN: f32 = 72.0;
/// Peak horizontal displacement of a seam from its nominal x.
const WATER_WAVE_AMP: f32 = 4.5;

pub struct BgPlugin;

impl Plugin for BgPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (spawn_bg, spawn_tree))
            .add_systems(Update, spawn_broken_storage_on_unlock);
    }
}

/// Marker for the three sprites that make up the standalone tree.
/// Always visible from startup — the player can scroll west and see
/// the tree at any time, even before the research mission is bought.
#[derive(Component)]
pub struct Tree;

/// Marker for the main foliage sprite specifically — distinct from the
/// trunk and the highlight stamp. The wood plugin's per-click wiggle
/// system filters on this so it can wobble the foliage's `Transform`
/// without disturbing the cached `Pos`.
#[derive(Component)]
pub struct TreeFoliage;

/// Marker for the scattered "wreck" sprites that appear at the
/// `TREE_STORAGE_*` location once the research mission completes.
/// They stand in for the broken storage; despawned wholesale when the
/// player buys the `TreeStorage` upgrade.
#[derive(Component)]
pub struct BrokenStorage;

/// Marker for the upgraded, intact storage crate. Spawned by the
/// purchase handler in [`crate::structures::research`].
#[derive(Component)]
pub struct WholeStorage;

/// Trunk dimensions, in spec px.
const TREE_TRUNK_W: f32 = 5.0;
const TREE_TRUNK_H: f32 = 18.0;
/// Foliage size — must match the texture aspect baked in `Shapes`.
const TREE_FOLIAGE_W: f32 = 32.0;
const TREE_FOLIAGE_H: f32 = 28.0;

/// Spawn the standalone tree well to the left of the big rock. Three
/// sprites: trunk, foliage main mass, foliage highlight on top-right.
fn spawn_tree(mut commands: Commands, shapes: Res<Shapes>) {
    // Trunk — small brown rectangle anchored at the tree's base.
    let trunk_cy = TREE_Y + TREE_TRUNK_H * 0.5;
    commands.spawn((
        Tree,
        Pos(Vec2::new(TREE_X, trunk_cy)),
        Layer(Z_HUT),
        Sprite::from_color(colors::TREE_TRUNK, Vec2::new(TREE_TRUNK_W, TREE_TRUNK_H)),
        Transform::default(),
    ));
    // Foliage main mass — irregular green blob sitting above the trunk.
    let foliage_cy = TREE_Y - TREE_FOLIAGE_H * 0.5 + 4.0;
    commands.spawn((
        Tree,
        TreeFoliage,
        Pos(Vec2::new(TREE_X, foliage_cy)),
        Layer(Z_HUT + 0.05),
        Sprite {
            image: shapes.tree_foliage.clone(),
            color: colors::TREE_FOLIAGE,
            custom_size: Some(Vec2::new(TREE_FOLIAGE_W, TREE_FOLIAGE_H)),
            ..default()
        },
        Transform::default(),
    ));
    // Highlight stamp — same dimensions, smaller blobs lighting the
    // top-right of the canopy.
    commands.spawn((
        Tree,
        Pos(Vec2::new(TREE_X, foliage_cy)),
        Layer(Z_HUT + 0.07),
        Sprite {
            image: shapes.tree_foliage_light.clone(),
            color: colors::TREE_FOLIAGE_LIGHT,
            custom_size: Some(Vec2::new(TREE_FOLIAGE_W, TREE_FOLIAGE_H)),
            ..default()
        },
        Transform::default(),
    ));
}

/// Once the cinematic finishes, sprinkle a few small dark-brown
/// rectangles where the storage will eventually go — visual cue that
/// there's something here for the player to invest in.
fn spawn_broken_storage_on_unlock(
    mission: Res<crate::economy::ResearchMission>,
    existing: Query<(), With<BrokenStorage>>,
    storage: Res<crate::economy::TreeStorage>,
    mut commands: Commands,
) {
    // Only fire the first time we observe the unlock; never re-spawn
    // (and never spawn after the player buys the upgrade).
    if !mission.unlocked || storage.owned {
        return;
    }
    if !existing.is_empty() {
        return;
    }
    // Four small chunks scattered in roughly a 12×8 footprint.
    let chunks: [(f32, f32, f32, f32); 4] = [
        (-4.0, -2.0, 4.0, 2.0),
        (1.0, -3.0, 3.0, 2.0),
        (-1.0, 1.0, 5.0, 2.0),
        (4.0, 0.0, 2.0, 2.0),
    ];
    for (dx, dy, w, h) in chunks {
        commands.spawn((
            BrokenStorage,
            Pos(Vec2::new(
                crate::core::constants::TREE_STORAGE_X + dx,
                crate::core::constants::TREE_STORAGE_Y + dy,
            )),
            Layer(Z_HUT - 0.05),
            Sprite::from_color(colors::TREE_TRUNK, Vec2::new(w, h)),
            Transform::default(),
        ));
    }
}

fn spawn_bg(mut commands: Commands) {
    // Sand — spans the world's left edge to the shoreline. With
    // scrolling enabled, sand reaches into negative spec x so the
    // tree (and any future leftward content) sits on the same beach.
    let sand_w = SHORELINE_X - WORLD_LEFT;
    let sand_cx = (WORLD_LEFT + SHORELINE_X) * 0.5;
    commands.spawn((
        Pos(Vec2::new(sand_cx, INTERNAL_HEIGHT * 0.5)),
        Layer(Z_BG),
        Sprite::from_color(colors::SAND, Vec2::new(sand_w, INTERNAL_HEIGHT)),
        Transform::default(),
    ));

    // Water half — five-band shore→deep gradient with sine-wobbled seams.
    spawn_water_gradient(&mut commands);

    // Wet-sand strip — a soft transition just before the foam line.
    // Vertical strip; spans full canvas height.
    commands.spawn((
        Pos(Vec2::new(SHORELINE_X - 5.0, INTERNAL_HEIGHT * 0.5)),
        Layer(Z_BG_DETAIL),
        Sprite::from_color(colors::SAND_WET, Vec2::new(6.0, INTERNAL_HEIGHT)),
        Transform::default(),
    ));
    // Foam line — single 1-px stripe, like SNKRX wall lines.
    commands.spawn((
        Pos(Vec2::new(SHORELINE_X, INTERNAL_HEIGHT * 0.5)),
        Layer(Z_BG_DETAIL + 0.1),
        Sprite::from_color(colors::FOAM, Vec2::new(1.0, INTERNAL_HEIGHT)),
        Transform::default(),
    ));

    // Sand-side texture — individual 1-px grains scattered on the
    // integer grid. Each grain is placed at an integer (x, y) so it
    // lands cleanly on a single output pixel after upscaling, giving
    // the beach a true granular look instead of the soft-edged fractional
    // rectangles the old version produced. A small fraction of grains
    // get an extra 1-px neighbour for occasional 2-px clusters.
    let mut rng = rand::thread_rng();
    let x_min = (WORLD_LEFT as i32) + 2;
    let x_max = (SHORELINE_X as i32) - 8;
    let y_min = 2i32;
    let y_max = (INTERNAL_HEIGHT as i32) - 2;
    for (count, color) in [
        (SAND_LIGHT_GRAINS, colors::SAND_LIGHT),
        (SAND_DARK_GRAINS, colors::SAND_DARK),
    ] {
        for _ in 0..count {
            let gx = rng.gen_range(x_min..x_max);
            let gy = rng.gen_range(y_min..y_max);
            spawn_grain(&mut commands, gx, gy, color);
            if rng.gen_bool(SAND_GRAIN_CLUSTER_CHANCE) {
                // Stick one neighbour on a random side. Using small
                // 2-cluster fragments rather than fixed shapes keeps
                // the noise irregular at every scale.
                let (ox, oy) = match rng.gen_range(0..4) {
                    0 => (1, 0),
                    1 => (-1, 0),
                    2 => (0, 1),
                    _ => (0, -1),
                };
                let nx = (gx + ox).clamp(x_min, x_max - 1);
                let ny = (gy + oy).clamp(y_min, y_max - 1);
                spawn_grain(&mut commands, nx, ny, color);
            }
        }
    }

    // Soft vignette — same `srgba(0,0,0,0.18)` overlay as SNKRX. Frames the
    // canvas without making the corners look painted. Spans the full
    // scrollable world (including the leftward sand strip) so the
    // tint is uniform whatever's on screen.
    let world_full_w = WORLD_WIDTH - WORLD_LEFT;
    commands.spawn((
        Pos(Vec2::new(
            (WORLD_LEFT + WORLD_WIDTH) * 0.5,
            INTERNAL_HEIGHT * 0.5,
        )),
        Layer(Z_BG_DETAIL + 0.5),
        Sprite::from_color(
            Color::srgba(0.0, 0.0, 0.0, 0.15),
            Vec2::new(world_full_w, INTERNAL_HEIGHT),
        ),
        Transform::default(),
    ));
}

/// Spawn a single 1×1 sand grain at integer spec coordinates. Centred
/// on `(x + 0.5, y + 0.5)` so the sprite snaps to a single output
/// pixel after the upscale step.
fn spawn_grain(commands: &mut Commands, x: i32, y: i32, color: Color) {
    commands.spawn((
        Pos(Vec2::new(x as f32 + 0.5, y as f32 + 0.5)),
        Layer(Z_BG_DETAIL),
        Sprite::from_color(color, Vec2::new(1.0, 1.0)),
        Transform::default(),
    ));
}

/// Render the water as five shore→deep bands. Each row of pixels is a
/// horizontal strip cut into five sprites by four seam x-coordinates;
/// every seam is offset by `WATER_WAVE_AMP * sin(2π * y / WATER_WAVE_LEN +
/// phase)` so the seams read as wandering currents rather than vertical
/// rules. Each seam has its own phase, so the bands don't all wobble in
/// lockstep.
///
/// The water now spans the full scrollable world (shoreline →
/// `WORLD_WIDTH`), so the bands stretch correspondingly wider; the
/// deep bands take up most of the off-screen ocean.
fn spawn_water_gradient(commands: &mut Commands) {
    let water_x_start = SHORELINE_X;
    let water_x_end = WORLD_WIDTH;
    let water_w = water_x_end - water_x_start;
    let band_w = water_w / WATER_BANDS.len() as f32;

    // Per-seam phase offsets — chosen so adjacent seams visibly disagree.
    let seam_phases: [f32; 4] = [0.0, 1.4, 2.7, 4.1];

    let n_slices = (INTERNAL_HEIGHT / WATER_SLICE_H).ceil() as i32;
    for s in 0..n_slices {
        let cy = s as f32 * WATER_SLICE_H + WATER_SLICE_H * 0.5;

        // Compute the four wobbled seam positions for this row.
        let mut seams = [0.0f32; 4];
        for (k, phase) in seam_phases.iter().enumerate() {
            let nominal = water_x_start + band_w * (k as f32 + 1.0);
            let offset = WATER_WAVE_AMP * (TAU * cy / WATER_WAVE_LEN + phase).sin();
            seams[k] = nominal + offset;
        }

        // Five segments, one per band: [start, seam0, seam1, seam2, seam3, end].
        let xs: [f32; 6] = [
            water_x_start,
            seams[0],
            seams[1],
            seams[2],
            seams[3],
            water_x_end,
        ];
        for b in 0..WATER_BANDS.len() {
            let x_left = xs[b].clamp(water_x_start, water_x_end);
            let x_right = xs[b + 1].clamp(water_x_start, water_x_end);
            if x_right <= x_left {
                continue;
            }
            let cx = (x_left + x_right) * 0.5;
            let w = x_right - x_left;
            commands.spawn((
                Pos(Vec2::new(cx, cy)),
                Layer(Z_BG),
                Sprite::from_color(WATER_BANDS[b], Vec2::new(w, WATER_SLICE_H)),
                Transform::default(),
            ));
        }
    }
}
