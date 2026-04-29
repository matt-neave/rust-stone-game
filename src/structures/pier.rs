//! The pier — a wooden walkway that extends from the shoreline into
//! the water once purchased. Hovering it surfaces a panel that lets
//! the player buy fish; each fish then idles in the surrounding
//! water and helps rescue failing skim bounces (see `crate::rocks`
//! for the actual rescue logic).

use bevy::prelude::*;
use rand::Rng;

use crate::core::colors;
use crate::core::common::{Layer, Pos};
use crate::core::constants::{
    FISHES_PER_BUCKET, INTERNAL_HEIGHT, PIER_H, PIER_W, PIER_X, PIER_Y, SHORELINE_X,
    STARTING_FISH_FROM_PIER, WORLD_WIDTH, Z_FISH, Z_PIER,
};

/// Fish wander box — spans the full ocean (shoreline to right edge of
/// the scrollable world, top to bottom of canvas) so fish cover the
/// whole water rather than clustering at the pier.
const FISH_X_MIN: f32 = SHORELINE_X + 4.0;
const FISH_X_MAX: f32 = WORLD_WIDTH - 6.0;
const FISH_Y_MIN: f32 = 6.0;
const FISH_Y_MAX: f32 = INTERNAL_HEIGHT - 6.0;
/// Fraction of fish wander targets that bias toward the shoreline
/// (the inshore band where the rocks splash). The rest spread uniformly
/// across the open water.
const FISH_INSHORE_BIAS: f64 = 0.6;
/// X-range of the inshore band when biased.
const FISH_INSHORE_X_MAX: f32 = SHORELINE_X + 80.0;
use crate::crew::builder::StructureBuiltEvent;
use crate::economy::{Fishes, Pier, PurchaseEvent, PurchaseKind};

#[derive(Component)]
pub struct PierPart;

/// A friendly fish that idles in the water near the pier. When a
/// rock would sink within reach, the fish flicks it back into one
/// extra bounce. See `rocks::small::tick_skimming` for the rescue.
#[derive(Component)]
pub struct Fish {
    pub state: FishState,
    pub flap_accum: f32,
}

pub enum FishState {
    Idle { time: f32, dur: f32 },
    Swimming { from: Vec2, to: Vec2, time: f32, dur: f32 },
}

pub struct PierPlugin;

impl Plugin for PierPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                handle_pier_purchase,
                handle_pier_built,
                handle_fish_purchase,
                tick_fish,
            ),
        );
    }
}

// ---------------------------------------------------------------------------
// Pier purchase + visual
// ---------------------------------------------------------------------------

/// Flag the pier as owned immediately on purchase so the cave panel
/// stops offering it. The visual + starter fish wait for the
/// construction-complete signal in [`handle_pier_built`].
fn handle_pier_purchase(mut events: MessageReader<PurchaseEvent>, mut pier: ResMut<Pier>) {
    for ev in events.read() {
        if ev.kind == PurchaseKind::Pier {
            pier.owned = true;
        }
    }
}

fn handle_pier_built(
    mut events: MessageReader<StructureBuiltEvent>,
    mut commands: Commands,
    mut fishes: ResMut<Fishes>,
) {
    for ev in events.read() {
        if ev.kind != PurchaseKind::Pier {
            continue;
        }
        spawn_pier_visual(&mut commands);
        // Comes with one starter fish so the pier doesn't feel empty
        // until the player gets around to buying more.
        for _ in 0..STARTING_FISH_FROM_PIER {
            spawn_fish_at(&mut commands, fish_spawn_pos());
            fishes.count += 1;
        }
    }
}

fn spawn_pier_visual(commands: &mut Commands) {
    // Main planks — long brown rectangle from the shoreline out into
    // the water.
    commands.spawn((
        PierPart,
        Pos(Vec2::new(PIER_X, PIER_Y)),
        Layer(Z_PIER),
        Sprite::from_color(colors::HUT_ROOF, Vec2::new(PIER_W, PIER_H)),
        Transform::default(),
    ));
    // Plank seams — short darker stripes every ~10 px.
    let seam_x_start = PIER_X - PIER_W * 0.5 + 4.0;
    for i in 0..6 {
        let x = seam_x_start + i as f32 * 10.0;
        commands.spawn((
            PierPart,
            Pos(Vec2::new(x, PIER_Y)),
            Layer(Z_PIER + 0.05),
            Sprite::from_color(colors::HUT_DOOR, Vec2::new(1.0, PIER_H)),
            Transform::default(),
        ));
    }
    // Pillars — thin vertical stubs going into the water.
    for i in 0..3 {
        let x = PIER_X - PIER_W * 0.4 + i as f32 * (PIER_W * 0.4);
        commands.spawn((
            PierPart,
            Pos(Vec2::new(x, PIER_Y + PIER_H * 0.5 + 2.0)),
            Layer(Z_PIER - 0.02),
            Sprite::from_color(colors::HUT_DOOR, Vec2::new(1.0, 4.0)),
            Transform::default(),
        ));
    }
}

// ---------------------------------------------------------------------------
// Fish — repeatable purchase + idle wandering
// ---------------------------------------------------------------------------

fn handle_fish_purchase(
    mut events: MessageReader<PurchaseEvent>,
    mut commands: Commands,
    mut fishes: ResMut<Fishes>,
    pier: Res<Pier>,
) {
    for ev in events.read() {
        if ev.kind != PurchaseKind::Fish || !pier.owned {
            continue;
        }
        // Bucket: one purchase = a school of one-shot fish.
        for _ in 0..FISHES_PER_BUCKET {
            spawn_fish_at(&mut commands, fish_spawn_pos());
            fishes.count += 1;
        }
    }
}

fn fish_spawn_pos() -> Vec2 {
    // Spawn at the pier's seaward end so a freshly-bought bucket of
    // fish visibly streams *out* of the pier into the ocean. Tiny
    // jitter so a 10-fish bucket doesn't stack on a single pixel.
    let mut rng = rand::thread_rng();
    Vec2::new(
        PIER_X + PIER_W * 0.45 + rng.gen_range(-2.0..2.0),
        PIER_Y + rng.gen_range(-2.0..3.0),
    )
}

/// Pick a fish body color — slight blue/teal variations around the
/// base fisherman body shade so a school of fish doesn't look like
/// a row of clones.
fn pick_fish_color<R: Rng + ?Sized>(rng: &mut R) -> Color {
    let palette = [
        Color::srgb(0.165, 0.416, 0.502), // base fisherman teal (#2a6a80)
        Color::srgb(0.110, 0.360, 0.600), // bluer
        Color::srgb(0.220, 0.500, 0.560), // brighter teal
        Color::srgb(0.180, 0.310, 0.560), // deep blue
        Color::srgb(0.260, 0.560, 0.660), // pale aqua
        Color::srgb(0.130, 0.470, 0.520), // muted ocean
    ];
    palette[rng.gen_range(0..palette.len())]
}

/// Sample a fish wander target inside the ocean. With probability
/// `FISH_INSHORE_BIAS` the target is in the inshore band (closer to
/// the shore, where rocks land); otherwise it's uniformly anywhere in
/// the open water.
fn pick_fish_target<R: Rng + ?Sized>(rng: &mut R) -> Vec2 {
    let x_max = if rng.gen_bool(FISH_INSHORE_BIAS) {
        FISH_INSHORE_X_MAX
    } else {
        FISH_X_MAX
    };
    Vec2::new(
        rng.gen_range(FISH_X_MIN..x_max),
        rng.gen_range(FISH_Y_MIN..FISH_Y_MAX),
    )
}

fn spawn_fish_at(commands: &mut Commands, pos: Vec2) {
    let mut rng = rand::thread_rng();
    // Each fish departs the pier IMMEDIATELY toward a random ocean
    // target — visually they "scatter" from the pier rather than
    // sitting next to it for an idle beat first.
    let target = pick_fish_target(&mut rng);
    let dist = pos.distance(target).max(1.0);
    let speed: f32 = rng.gen_range(8.0..16.0);
    let color = pick_fish_color(&mut rng);
    commands.spawn((
        Fish {
            state: FishState::Swimming {
                from: pos,
                to: target,
                time: 0.0,
                dur: dist / speed,
            },
            flap_accum: 0.0,
        },
        Pos(pos),
        Layer(Z_FISH),
        Sprite::from_color(color, Vec2::new(3.0, 2.0)),
        Transform::default(),
    ));
}

fn tick_fish(time: Res<Time>, mut q: Query<(&mut Fish, &mut Pos)>) {
    let dt = time.delta_secs();
    let mut rng = rand::thread_rng();
    for (mut fish, mut pos) in &mut q {
        match &mut fish.state {
            FishState::Idle { time: t, dur } => {
                *t += dt;
                if *t >= *dur {
                    // Pick a new wander target anywhere in the ocean —
                    // most picks bias toward the inshore band.
                    let target = pick_fish_target(&mut rng);
                    let dist = pos.0.distance(target).max(1.0);
                    let speed: f32 = rng.gen_range(8.0..16.0);
                    fish.state = FishState::Swimming {
                        from: pos.0,
                        to: target,
                        time: 0.0,
                        dur: dist / speed,
                    };
                }
            }
            FishState::Swimming { from, to, time: t, dur } => {
                *t += dt;
                let prog = (*t / *dur).clamp(0.0, 1.0);
                pos.0 = from.lerp(*to, prog);
                if *t >= *dur {
                    fish.state = FishState::Idle {
                        time: 0.0,
                        dur: rng.gen_range(0.3..1.2),
                    };
                }
            }
        }
        // Flap accumulator tracked here so a future frame-flip
        // animation can hook in without another query.
        fish.flap_accum += dt;
    }
}
