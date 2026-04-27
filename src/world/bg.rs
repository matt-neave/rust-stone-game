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

/// Number of random light-tinted sand patches. The sprinkle is dense
/// enough that the beach reads as granular, with light dominating so it
/// matches the bright reference look.
const SAND_LIGHT_PATCHES: u32 = 140;
/// Number of random dark-tinted sand patches — fewer and smaller than
/// the light ones so they read as scattered grit rather than shadow.
const SAND_DARK_PATCHES: u32 = 65;

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
        app.add_systems(Startup, spawn_bg);
    }
}

fn spawn_bg(mut commands: Commands) {
    // Sand half — solid base.
    commands.spawn((
        Pos(Vec2::new(SHORELINE_X * 0.5, INTERNAL_HEIGHT * 0.5)),
        Layer(Z_BG),
        Sprite::from_color(colors::SAND, Vec2::new(SHORELINE_X, INTERNAL_HEIGHT)),
        Transform::default(),
    ));

    // Water half — five-band shore→deep gradient with sine-wobbled seams.
    spawn_water_gradient(&mut commands);

    // Wet-sand strip — a soft transition just before the foam line.
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

    // Sand-side texture — a sprinkle of small random patches in the
    // lighter and darker sand tones over the primary fill. Stops short
    // of the wet-sand strip so the shoreline stays clean.
    let mut rng = rand::thread_rng();
    let x_min = 2.0;
    let x_max = SHORELINE_X - 8.0;
    let y_min = 2.0;
    let y_max = INTERNAL_HEIGHT - 2.0;
    for (count, color) in [
        (SAND_LIGHT_PATCHES, colors::SAND_LIGHT),
        (SAND_DARK_PATCHES, colors::SAND_DARK),
    ] {
        for _ in 0..count {
            let cx: f32 = rng.gen_range(x_min..x_max);
            let cy: f32 = rng.gen_range(y_min..y_max);
            // Most patches are 1-2 px specks; occasional 3 px ones give
            // a bit of size variation without the patches reading as
            // chunky tiles.
            let w: f32 = if rng.gen_bool(0.7) {
                rng.gen_range(1.0..2.5)
            } else {
                rng.gen_range(2.5..3.5)
            };
            let h: f32 = if rng.gen_bool(0.7) {
                rng.gen_range(1.0..2.5)
            } else {
                rng.gen_range(2.5..3.5)
            };
            commands.spawn((
                Pos(Vec2::new(cx, cy)),
                Layer(Z_BG_DETAIL),
                Sprite::from_color(color, Vec2::new(w, h)),
                Transform::default(),
            ));
        }
    }

    // Soft vignette — same `srgba(0,0,0,0.18)` overlay as SNKRX. Frames the
    // canvas without making the corners look painted.
    commands.spawn((
        Pos(Vec2::new(INTERNAL_WIDTH * 0.5, INTERNAL_HEIGHT * 0.5)),
        Layer(Z_BG_DETAIL + 0.5),
        Sprite::from_color(
            Color::srgba(0.0, 0.0, 0.0, 0.15),
            Vec2::new(INTERNAL_WIDTH, INTERNAL_HEIGHT),
        ),
        Transform::default(),
    ));
}

/// Render the water as five shore→deep bands. Each row of pixels is a
/// horizontal strip cut into five sprites by four seam x-coordinates;
/// every seam is offset by `WATER_WAVE_AMP * sin(2π * y / WATER_WAVE_LEN +
/// phase)` so the seams read as wandering currents rather than vertical
/// rules. Each seam has its own phase, so the bands don't all wobble in
/// lockstep.
fn spawn_water_gradient(commands: &mut Commands) {
    let water_x_start = SHORELINE_X;
    let water_x_end = INTERNAL_WIDTH;
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
