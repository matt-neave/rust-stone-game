//! Ambient atmosphere — primitive-only continuous effects that bring the
//! beach scene to life. No shaders, just tinted sprites with fade envelopes.
//! Patterned on `rust-SNKRX/src/stars.rs` and `effects.rs::HitParticleFx` —
//! spawn-on-timer + drift + fade + despawn.
//!
//! Five independent generators run continuously, each on its own random
//! cadence so the eye always catches motion somewhere:
//!
//! * **Sea sparkles** — 1-px foam pixels that twinkle in random water spots.
//! * **Sand glints** — 1-px bright sand pixels that flicker on the beach.
//! * **Flotsam** — 1-px dark specks that drift slowly rightward across the
//!   water, like kelp/debris in a current. Reads as continuous water motion
//!   rather than the in-place twinkle of sparkles.
//! * **Birds** — tiny seagull-V silhouettes that drift across the canvas
//!   diagonally. Direct lift from SNKRX's `stars.rs` cadence.
//! * **Cloud shadows** — large soft dark blobs that pass slowly across the
//!   whole scene at long intervals. Atmospheric punctuation; rare enough not
//!   to feel like weather, frequent enough to register.

use bevy::color::Alpha;
use bevy::prelude::*;
use rand::Rng;

use crate::core::colors;
use crate::core::common::{Layer, Pos};
use crate::core::constants::*;
use crate::render::shapes::Shapes;

/// Layer for ambient effects — above the bg checker, below ripples + rocks.
const Z_AMBIENT: f32 = Z_BG_DETAIL + 0.3;
/// Flotsam sits at the same depth as other ambient water details.
const Z_FLOTSAM: f32 = Z_AMBIENT + 0.05;
/// Cloud shadows fall on the world: above particles + floating text, below UI.
const Z_CLOUD: f32 = Z_FLOATING + 0.5;
/// Birds fly above everything except the UI.
const Z_BIRD: f32 = Z_CLOUD + 0.5;

#[derive(Component)]
pub struct SeaSparkle {
    pub time: f32,
    pub lifetime: f32,
    pub max_alpha: f32,
}

#[derive(Component)]
pub struct SandGlint {
    pub time: f32,
    pub lifetime: f32,
    pub max_alpha: f32,
}

#[derive(Component)]
pub struct Flotsam {
    pub vel: Vec2,
    /// Phase + freq for the sine Y-wobble that gives flotsam a current-like
    /// motion rather than perfectly straight rightward drift.
    pub wobble_phase: f32,
    pub wobble_freq: f32,
    pub wobble_amp: f32,
    pub time: f32,
    pub max_alpha: f32,
}

#[derive(Component)]
pub struct Bird {
    pub vel: Vec2,
    pub max_alpha: f32,
    /// Flap toggle accumulator — flips `wings_up` every `flap_interval` secs.
    pub flap_accum: f32,
    pub flap_interval: f32,
    pub wings_up: bool,
}

#[derive(Component)]
pub struct CloudShadow {
    pub vel: Vec2,
    pub time: f32,
    pub lifetime: f32,
    pub max_alpha: f32,
}

#[derive(Resource, Default)]
pub struct AmbientTimers {
    pub sparkle: f32,
    pub glint: f32,
    pub flotsam: f32,
    pub bird: f32,
    pub cloud: f32,
}

pub struct AmbientPlugin;

impl Plugin for AmbientPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AmbientTimers>().add_systems(
            Update,
            (
                spawn_sea_sparkles,
                spawn_sand_glints,
                spawn_flotsam,
                spawn_birds,
                spawn_cloud_shadows,
                tick_sea_sparkles,
                tick_sand_glints,
                tick_flotsam,
                tick_birds,
                tick_cloud_shadows,
            ),
        );
    }
}

fn spawn_sea_sparkles(
    time: Res<Time>,
    mut timers: ResMut<AmbientTimers>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    timers.sparkle -= dt;
    if timers.sparkle > 0.0 {
        return;
    }
    let mut rng = rand::thread_rng();
    // ~2-3 spawns/s; lifetime ~1.5-2.5s → steady state ~4-8 visible.
    // Was 0.10-0.18 originally, but that pegged ~25 entities on screen at
    // once with constant spawn/despawn churn that was starving the audio
    // thread under debug builds. The lower rate still reads as constant
    // motion but is much cheaper.
    timers.sparkle = rng.gen_range(0.30..0.50);

    // Stay clear of the foam line; let the sparkle volume cover the rest of
    // the water column.
    let x: f32 = rng.gen_range((SHORELINE_X + 6.0)..(INTERNAL_WIDTH - 4.0));
    let y: f32 = rng.gen_range(6.0..(INTERNAL_HEIGHT - 6.0));
    let lifetime: f32 = rng.gen_range(1.5..2.5);
    let max_alpha: f32 = rng.gen_range(0.25..0.55);

    let mut color = colors::FOAM;
    color.set_alpha(0.0);

    commands.spawn((
        SeaSparkle {
            time: 0.0,
            lifetime,
            max_alpha,
        },
        Pos(Vec2::new(x, y)),
        Layer(Z_AMBIENT),
        Sprite::from_color(color, Vec2::splat(1.0)),
        Transform::default(),
    ));
}

fn tick_sea_sparkles(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut SeaSparkle, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (e, mut s, mut sprite) in &mut q {
        s.time += dt;
        let t = (s.time / s.lifetime).clamp(0.0, 1.0);
        // Symmetric ease in/out — fade up to peak at t=0.5, back to zero at 1.
        let env = 1.0 - (2.0 * t - 1.0).abs();
        let mut c = sprite.color;
        c.set_alpha(env * s.max_alpha);
        sprite.color = c;
        if s.time >= s.lifetime {
            commands.entity(e).despawn();
        }
    }
}

fn spawn_sand_glints(
    time: Res<Time>,
    mut timers: ResMut<AmbientTimers>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    timers.glint -= dt;
    if timers.glint > 0.0 {
        return;
    }
    let mut rng = rand::thread_rng();
    // Sparse: one every 0.4-0.9s. Lifetime ~0.6-1.0s → ~1-2 visible.
    timers.glint = rng.gen_range(0.4..0.9);

    // Stay on dry sand — well clear of the wet-sand strip.
    let x: f32 = rng.gen_range(6.0..(SHORELINE_X - 12.0));
    let y: f32 = rng.gen_range(6.0..(INTERNAL_HEIGHT - 6.0));
    let lifetime: f32 = rng.gen_range(0.6..1.0);
    let max_alpha: f32 = rng.gen_range(0.30..0.55);

    let mut color = colors::SAND_LIGHT;
    color.set_alpha(0.0);

    commands.spawn((
        SandGlint {
            time: 0.0,
            lifetime,
            max_alpha,
        },
        Pos(Vec2::new(x, y)),
        Layer(Z_AMBIENT),
        Sprite::from_color(color, Vec2::splat(1.0)),
        Transform::default(),
    ));
}

fn tick_sand_glints(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut SandGlint, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (e, mut g, mut sprite) in &mut q {
        g.time += dt;
        let t = (g.time / g.lifetime).clamp(0.0, 1.0);
        // Sharper twinkle — peak earlier than midpoint, quicker fall.
        let env = if t < 0.3 {
            t / 0.3
        } else {
            ((1.0 - t) / 0.7).max(0.0)
        };
        let mut c = sprite.color;
        c.set_alpha(env * g.max_alpha);
        sprite.color = c;
        if g.time >= g.lifetime {
            commands.entity(e).despawn();
        }
    }
}

// ===========================================================================
// Flotsam — slow-drifting dark specks in the water.
// ===========================================================================

fn spawn_flotsam(
    time: Res<Time>,
    mut timers: ResMut<AmbientTimers>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    timers.flotsam -= dt;
    if timers.flotsam > 0.0 {
        return;
    }
    let mut rng = rand::thread_rng();
    // One every 6-10s; at 4-8 px/s the flotsam takes 30-60s to cross the
    // water, so steady-state ~4-8 visible. Sparse on purpose so it reads
    // as drifting debris, not a school.
    timers.flotsam = rng.gen_range(6.0..10.0);

    // Spawn just past the shoreline so they emerge into the water cleanly,
    // not on top of the foam line.
    let x = SHORELINE_X + rng.gen_range(8.0..40.0);
    let y: f32 = rng.gen_range(8.0..(INTERNAL_HEIGHT - 8.0));
    let vx: f32 = rng.gen_range(4.0..8.0); // slow rightward drift
    let vy: f32 = rng.gen_range(-0.5..0.5);
    let max_alpha: f32 = rng.gen_range(0.45..0.75);

    let mut color = colors::ROCK_DARK;
    color.set_alpha(0.0);

    commands.spawn((
        Flotsam {
            vel: Vec2::new(vx, vy),
            wobble_phase: rng.gen_range(0.0..std::f32::consts::TAU),
            wobble_freq: rng.gen_range(0.6..1.4),
            wobble_amp: rng.gen_range(0.8..1.6),
            time: 0.0,
            max_alpha,
        },
        Pos(Vec2::new(x, y)),
        Layer(Z_FLOTSAM),
        Sprite::from_color(color, Vec2::splat(1.0)),
        Transform::default(),
    ));
}

fn tick_flotsam(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Flotsam, &mut Pos, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (e, mut f, mut pos, mut sprite) in &mut q {
        let dt_vel = f.vel * dt;
        let wobble_y = (f.wobble_phase + f.time * f.wobble_freq).sin() * f.wobble_amp;
        f.time += dt;
        pos.0 += dt_vel + Vec2::new(0.0, wobble_y * dt);

        // Fade in over first 2s, hold, fade out as it leaves the right edge.
        let fade_in = (f.time / 2.0).min(1.0);
        let edge_fade = ((INTERNAL_WIDTH - 6.0 - pos.0.x) / 30.0).clamp(0.0, 1.0);
        let alpha = fade_in * edge_fade * f.max_alpha;
        let mut c = sprite.color;
        c.set_alpha(alpha);
        sprite.color = c;

        if pos.0.x > INTERNAL_WIDTH + 4.0 {
            commands.entity(e).despawn();
        }
    }
}

// ===========================================================================
// Birds — tiny seagull silhouettes drifting across the canvas.
// ===========================================================================

fn spawn_birds(
    time: Res<Time>,
    mut timers: ResMut<AmbientTimers>,
    mut commands: Commands,
    shapes: Res<Shapes>,
) {
    let dt = time.delta_secs();
    timers.bird -= dt;
    if timers.bird > 0.0 {
        return;
    }
    let mut rng = rand::thread_rng();
    // One every 8-15s; ~25s travel time → steady-state ~2 visible.
    timers.bird = rng.gen_range(8.0..15.0);

    // Always travel left-to-right OR right-to-left, with a slight downward
    // drift so a bird passes through the upper half of the canvas. SNKRX's
    // stars do the same edge-spawn trick.
    let from_left = rng.gen_bool(0.5);
    let speed: f32 = rng.gen_range(18.0..30.0);
    let (start_x, vx) = if from_left {
        (-8.0, speed)
    } else {
        (INTERNAL_WIDTH + 8.0, -speed)
    };
    let start_y: f32 = rng.gen_range(6.0..70.0);
    let vy: f32 = rng.gen_range(-1.5..3.0);
    // Birds read better with stronger alpha — they're tiny and dark already.
    let max_alpha: f32 = rng.gen_range(0.55..0.85);

    let mut color = colors::ROCK_DARK;
    color.set_alpha(0.0);

    commands.spawn((
        Bird {
            vel: Vec2::new(vx, vy),
            max_alpha,
            flap_accum: 0.0,
            // ~3 Hz flap — fast enough to read as flying, slow enough to
            // count discrete frames at our internal resolution.
            flap_interval: rng.gen_range(0.14..0.20),
            wings_up: true,
        },
        Pos(Vec2::new(start_x, start_y)),
        Layer(Z_BIRD),
        Sprite {
            image: shapes.bird_up.clone(),
            color,
            // 7×3 native — render at native size for the chunky look.
            custom_size: Some(Vec2::new(7.0, 3.0)),
            ..default()
        },
        Transform::default(),
    ));
}

fn tick_birds(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Bird, &mut Pos, &mut Sprite)>,
    shapes: Res<Shapes>,
) {
    let dt = time.delta_secs();
    for (e, mut b, mut pos, mut sprite) in &mut q {
        pos.0 += b.vel * dt;

        // Wing flap — toggle the sprite image when the accumulator crosses
        // the per-bird flap interval.
        b.flap_accum += dt;
        if b.flap_accum >= b.flap_interval {
            b.flap_accum -= b.flap_interval;
            b.wings_up = !b.wings_up;
            sprite.image = if b.wings_up {
                shapes.bird_up.clone()
            } else {
                shapes.bird_down.clone()
            };
        }

        // Quick fade in (first ~10 px after spawning), hold, quick fade out
        // as we approach the opposite edge.
        let edge_left = (pos.0.x + 12.0).clamp(0.0, 24.0) / 24.0;
        let edge_right = ((INTERNAL_WIDTH + 12.0 - pos.0.x).clamp(0.0, 24.0)) / 24.0;
        let alpha = edge_left.min(edge_right) * b.max_alpha;
        let mut c = sprite.color;
        c.set_alpha(alpha);
        sprite.color = c;

        if pos.0.x < -16.0 || pos.0.x > INTERNAL_WIDTH + 16.0 {
            commands.entity(e).despawn();
        }
    }
}

// ===========================================================================
// Cloud shadows — large dim blobs drifting across the whole canvas.
// ===========================================================================

fn spawn_cloud_shadows(
    time: Res<Time>,
    mut timers: ResMut<AmbientTimers>,
    mut commands: Commands,
    shapes: Res<Shapes>,
) {
    let dt = time.delta_secs();
    timers.cloud -= dt;
    if timers.cloud > 0.0 {
        return;
    }
    let mut rng = rand::thread_rng();
    // One every 25-45s; lifetime ≈ 30s → mostly 0-1 cloud visible at a time.
    // Atmospheric punctuation rather than a constant feature.
    timers.cloud = rng.gen_range(25.0..45.0);

    // Spawn just off the top-left, drift toward bottom-right at a slow
    // diagonal — simulates a high-altitude cloud's shadow tracking the
    // sun's angle across the beach.
    let from_left = rng.gen_bool(0.7);
    let speed: f32 = rng.gen_range(10.0..18.0);
    let (start_x, vx) = if from_left {
        (rng.gen_range(-100.0..0.0), speed)
    } else {
        (rng.gen_range(INTERNAL_WIDTH..(INTERNAL_WIDTH + 100.0)), -speed)
    };
    let start_y: f32 = rng.gen_range(-20.0..(INTERNAL_HEIGHT - 60.0));
    let vy: f32 = rng.gen_range(2.0..6.0);
    // Soft and dim — should darken what it passes over by maybe ~12%.
    let max_alpha: f32 = rng.gen_range(0.10..0.16);
    // Random scale so each cloud feels distinct.
    let scale: f32 = rng.gen_range(1.4..2.2);

    // Black tint at low alpha = subtractive-feel darkening over the bg.
    let color = Color::srgba(0.0, 0.0, 0.0, 0.0);

    let dist = ((INTERNAL_WIDTH + 200.0).powi(2) + (INTERNAL_HEIGHT + 200.0).powi(2)).sqrt();
    let lifetime = dist / speed;

    commands.spawn((
        CloudShadow {
            vel: Vec2::new(vx, vy),
            time: 0.0,
            lifetime,
            max_alpha,
        },
        Pos(Vec2::new(start_x, start_y)),
        Layer(Z_CLOUD),
        Sprite {
            image: shapes.cloud_shadow.clone(),
            color,
            // Native cloud_shadow image is 80×24; scale up for soft drift.
            custom_size: Some(Vec2::new(80.0 * scale, 24.0 * scale)),
            ..default()
        },
        Transform::default(),
    ));
}

fn tick_cloud_shadows(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut CloudShadow, &mut Pos, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (e, mut c, mut pos, mut sprite) in &mut q {
        c.time += dt;
        pos.0 += c.vel * dt;

        // Long fade in (first 25%) and out (last 25%) so a cloud entering /
        // leaving doesn't pop. The mid 50% sits at full alpha.
        let t = (c.time / c.lifetime).clamp(0.0, 1.0);
        let env = if t < 0.25 {
            t / 0.25
        } else if t > 0.75 {
            ((1.0 - t) / 0.25).max(0.0)
        } else {
            1.0
        };
        let mut col = sprite.color;
        col.set_alpha(env * c.max_alpha);
        sprite.color = col;

        // Despawn when fully off-canvas or lifetime spent.
        if c.time >= c.lifetime
            || pos.0.x > INTERNAL_WIDTH + 200.0
            || pos.0.x < -200.0
            || pos.0.y > INTERNAL_HEIGHT + 200.0
        {
            commands.entity(e).despawn();
        }
    }
}
