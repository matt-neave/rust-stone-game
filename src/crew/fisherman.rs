//! Fishermen — sit at the water edge and cast a line; on a successful
//! catch they pull a small rock onto the beach behind them.
//!
//! Cycle (~7-13 s wait):
//!
//! * `WalkingToSpot` — walk to assigned shoreline spot.
//! * `Casting` — pole snaps back-then-forward; line extends.
//! * `Fishing` — wait for a bite (random 7-13 s).
//! * `Reeling` — tug animation. On finish, 50% chance spawns a rock
//!   that arcs onto a random pocket of the beach behind the
//!   fisherman.
//! * `Held` — rod up, line gone, 3 s pause before recasting.
//!
//! The rig (pole + line) is two child entities tied to the owner via
//! [`FishingPole`] / [`FishingLine`]; [`update_fishing_rigs`] computes
//! their position, rotation, and length each frame from the owner's
//! [`FishermanState`].

use bevy::prelude::*;
use rand::Rng;

use crate::audio::{PlaySoundEvent, SoundKind};
use crate::core::colors;
use crate::core::common::{Layer, Pos};
use crate::core::constants::{
    FISHERMAN_CATCH_CHANCE, FISHERMAN_FISH_TIME_MAX, FISHERMAN_FISH_TIME_MIN, FISHERMAN_PULL_TIME,
    FISHERMAN_WALK_SPEED, INTERNAL_HEIGHT, SAND_LAND_X_MIN, SHORELINE_X, Z_CREW,
};
use crate::economy::{Fishermen, PurchaseKind};
use crate::effects::floating_text::SpawnFloatingTextEvent;
use crate::render::shapes::Shapes;
use crate::rocks::small::{SpawnSmallRockEvent, SMALL_ROCK_FALL_DURATION_SLOW};

use super::{step_walk_frame, tick_walk_animation, CrewWalking, SpawnConversionEvent};

/// Bounding range for fishing spots. Each cycle a fisherman picks a
/// fresh point inside this band — just on the sand side of the
/// shoreline (x) and spread vertically (y) across the beach.
const FISHERMAN_SPOT_X_MIN: f32 = 178.0;
const FISHERMAN_SPOT_X_MAX: f32 = 196.0;
const FISHERMAN_SPOT_Y_MIN: f32 = 40.0;
const FISHERMAN_SPOT_Y_MAX: f32 = 252.0;
/// Walking speed between casting spots (between cycles). Same as the
/// initial walk-to-spot speed but kept named for clarity.
const FISHERMAN_RELOCATE_SPEED: f32 = FISHERMAN_WALK_SPEED;

// Rig geometry / animation tunables.

/// Where the rod's handle sits relative to the fisherman's body
/// centre. Anchor for the rotation pivot.
const POLE_HANDLE_OFFSET: Vec2 = Vec2::new(2.0, -1.0);
const POLE_DRAW_SIZE: f32 = 5.0;
/// Default rod tilt while fishing — points up-right toward the water.
const POLE_FISHING_ANGLE: f32 = -0.55;
/// Tilt while just walking around with the rod over the shoulder.
const POLE_WALK_ANGLE: f32 = 0.7;
/// How far back the cast wind-up rotates the rod.
const POLE_CAST_BACK: f32 = 1.1;
/// Forward end of the cast snap.
const POLE_CAST_FORWARD: f32 = -0.9;
/// Rod-up pose held during the post-reel pause.
const POLE_HELD_ANGLE: f32 = 0.85;
/// Pause duration after a reel-in before the next cast.
const FISHERMAN_HELD_DURATION: f32 = 3.0;

/// Horizontal distance from the pole tip to the bobber while the line
/// is fully cast out into the water.
const LINE_CAST_DISTANCE: f32 = 60.0;
/// Distance at the end of a reel-in — close to the tip so the line
/// shortens as it's reeled.
const LINE_REEL_DISTANCE: f32 = 8.0;

#[derive(Component)]
pub struct Fisherman {
    pub state: FishermanState,
    /// The spot the fisherman is currently fishing from. Updated each
    /// cycle when the fisherman relocates to a fresh random spot.
    pub spot: Vec2,
    pub flap_accum: f32,
    pub walk_frame: bool,
}

pub enum FishermanState {
    WalkingToSpot { from: Vec2, to: Vec2, time: f32, dur: f32 },
    Casting { time: f32, dur: f32 },
    Fishing { time: f32, dur: f32 },
    Reeling { time: f32, dur: f32, success: bool },
    /// Just-reeled-in pause: line is gone, rod up.
    Held { time: f32, dur: f32 },
}

impl CrewWalking for Fisherman {
    fn is_walking(&self) -> bool {
        matches!(self.state, FishermanState::WalkingToSpot { .. })
    }
    fn walk_frame(&self) -> bool {
        self.walk_frame
    }
    fn step_walking(&mut self, walking: bool, dt: f32) -> bool {
        step_walk_frame(walking, &mut self.flap_accum, &mut self.walk_frame, dt)
    }
}

#[derive(Component)]
pub struct FishingPole {
    pub owner: Entity,
}

#[derive(Component)]
pub struct FishingLine {
    pub owner: Entity,
}

pub struct FishermanPlugin;

impl Plugin for FishermanPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                handle_fisherman_spawn,
                tick_fishermen,
                update_fishing_rigs,
                tick_walk_animation::<Fisherman>,
            ),
        );
    }
}

// ---------------------------------------------------------------------------
// Spawn
// ---------------------------------------------------------------------------

fn handle_fisherman_spawn(
    mut events: MessageReader<SpawnConversionEvent>,
    mut commands: Commands,
    mut fishermen: ResMut<Fishermen>,
    shapes: Res<Shapes>,
) {
    for ev in events.read() {
        if ev.kind != PurchaseKind::Fisherman {
            continue;
        }
        spawn_fisherman(&mut commands, &shapes, &mut fishermen, ev.from_pos);
    }
}

fn spawn_fisherman(
    commands: &mut Commands,
    shapes: &Shapes,
    fishermen: &mut Fishermen,
    worker_pos: Vec2,
) {
    fishermen.count += 1;
    let mut rng = rand::thread_rng();
    let spot = pick_fisherman_spot(&mut rng);

    let dist = worker_pos.distance(spot).max(1.0);
    let dur = (dist / FISHERMAN_WALK_SPEED).clamp(0.5, 6.0);

    let fisher_e = commands
        .spawn((
            Fisherman {
                state: FishermanState::WalkingToSpot {
                    from: worker_pos,
                    to: spot,
                    time: 0.0,
                    dur,
                },
                spot,
                flap_accum: 0.0,
                walk_frame: false,
            },
            Pos(worker_pos),
            Layer(Z_CREW),
            Sprite {
                image: shapes.humanoid.clone(),
                color: Color::WHITE,
                custom_size: Some(Vec2::new(16.0, 9.0)),
                ..default()
            },
            Transform::default(),
        ))
        .id();

    commands.spawn((
        FishingPole { owner: fisher_e },
        Pos(worker_pos + POLE_HANDLE_OFFSET),
        Layer(Z_CREW + 0.1),
        Sprite {
            image: shapes.fishing_rod.clone(),
            color: colors::FISHING_ROD,
            custom_size: Some(Vec2::splat(POLE_DRAW_SIZE)),
            ..default()
        },
        Transform::default(),
    ));
    commands.spawn((
        FishingLine { owner: fisher_e },
        Pos(worker_pos),
        Layer(Z_CREW + 0.05),
        Sprite::from_color(colors::FISHING_LINE, Vec2::new(1.0, 1.0)),
        Transform::default(),
        Visibility::Hidden,
    ));
}

// ---------------------------------------------------------------------------
// State machine
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn tick_fishermen(
    time: Res<Time>,
    mut q: Query<(&mut Fisherman, &mut Pos)>,
    mut spawn_rock: MessageWriter<SpawnSmallRockEvent>,
    mut sound: MessageWriter<PlaySoundEvent>,
    mut floating: MessageWriter<SpawnFloatingTextEvent>,
) {
    let dt = time.delta_secs();
    let mut rng = rand::thread_rng();
    for (mut fisher, mut pos) in &mut q {
        let spot = fisher.spot;
        let next: Option<FishermanState> = match &mut fisher.state {
            FishermanState::WalkingToSpot { from, to, time: t, dur } => {
                *t += dt;
                let prog = (*t / *dur).clamp(0.0, 1.0);
                pos.0 = from.lerp(*to, prog);
                if *t >= *dur {
                    pos.0 = *to;
                    fisher.spot = *to;
                    Some(FishermanState::Casting { time: 0.0, dur: 0.6 })
                } else { None }
            }
            FishermanState::Casting { time: t, dur } => {
                *t += dt;
                if *t >= *dur {
                    let fish_dur =
                        rng.gen_range(FISHERMAN_FISH_TIME_MIN..FISHERMAN_FISH_TIME_MAX);
                    Some(FishermanState::Fishing { time: 0.0, dur: fish_dur })
                } else { None }
            }
            FishermanState::Fishing { time: t, dur } => {
                *t += dt;
                if *t >= *dur {
                    let success = rng.gen_bool(FISHERMAN_CATCH_CHANCE);
                    Some(FishermanState::Reeling {
                        time: 0.0,
                        dur: FISHERMAN_PULL_TIME,
                        success,
                    })
                } else { None }
            }
            FishermanState::Reeling { time: t, dur, success } => {
                *t += dt;
                if *t >= *dur {
                    if *success {
                        // Rock arcs out of the water into a tight
                        // pocket behind the fisherman.
                        let from = Vec2::new(SHORELINE_X + 30.0, spot.y);
                        let to = Vec2::new(
                            (spot.x + rng.gen_range(-44.0..-12.0))
                                .clamp(SAND_LAND_X_MIN, SHORELINE_X - 10.0),
                            (spot.y + rng.gen_range(-22.0..22.0))
                                .clamp(20.0, INTERNAL_HEIGHT - 18.0),
                        );
                        spawn_rock.write(SpawnSmallRockEvent {
                            from,
                            to,
                            duration: SMALL_ROCK_FALL_DURATION_SLOW,
                        });
                        sound.write(PlaySoundEvent {
                            kind: SoundKind::SmallRockSpawn,
                            pitch: 0.9,
                            volume: 0.4,
                        });
                    } else {
                        sound.write(PlaySoundEvent {
                            kind: SoundKind::Splash,
                            pitch: 1.1,
                            volume: 0.18,
                        });
                        // Bright red shaking "miss" above the fisherman.
                        floating.write(SpawnFloatingTextEvent {
                            pos: Vec2::new(spot.x, spot.y - 8.0),
                            text: "miss".into(),
                            color: colors::MISS_RED,
                            size: 7.0,
                            duration: 0.9,
                            vy: -10.0,
                            shake: 1.2,
                        });
                    }
                    // Rod up, line gone — short rest before recasting.
                    Some(FishermanState::Held {
                        time: 0.0,
                        dur: FISHERMAN_HELD_DURATION,
                    })
                } else { None }
            }
            FishermanState::Held { time: t, dur } => {
                *t += dt;
                if *t >= *dur {
                    // Pick a fresh spot for the next cast and walk to
                    // it — fishermen no longer stay anchored to one
                    // shoreline tile.
                    let next_spot = pick_fisherman_spot(&mut rng);
                    let dist = pos.0.distance(next_spot).max(1.0);
                    let walk_dur = (dist / FISHERMAN_RELOCATE_SPEED).clamp(0.4, 5.0);
                    Some(FishermanState::WalkingToSpot {
                        from: pos.0,
                        to: next_spot,
                        time: 0.0,
                        dur: walk_dur,
                    })
                } else { None }
            }
        };
        if let Some(s) = next {
            fisher.state = s;
        }
    }
}

// ---------------------------------------------------------------------------
// Rig animation (pole + line)
// ---------------------------------------------------------------------------

fn update_fishing_rigs(
    time: Res<Time>,
    fishers: Query<(&Fisherman, &Pos), Without<FishingPole>>,
    mut poles: Query<
        (&FishingPole, &mut Pos, &mut Transform, &mut Visibility),
        (Without<Fisherman>, Without<FishingLine>),
    >,
    mut lines: Query<
        (&FishingLine, &mut Pos, &mut Sprite, &mut Transform, &mut Visibility),
        (Without<Fisherman>, Without<FishingPole>),
    >,
) {
    let now = time.elapsed_secs();
    for (pole, mut pos, mut tf, mut vis) in &mut poles {
        let Ok((fisher, fisher_pos)) = fishers.get(pole.owner) else {
            *vis = Visibility::Hidden;
            continue;
        };
        let rotation = pole_rotation(&fisher.state, now);
        pos.0 = fisher_pos.0 + POLE_HANDLE_OFFSET;
        tf.rotation = Quat::from_rotation_z(rotation);
        *vis = Visibility::Visible;
    }
    for (line, mut pos, mut sprite, mut tf, mut vis) in &mut lines {
        let Ok((fisher, fisher_pos)) = fishers.get(line.owner) else {
            *vis = Visibility::Hidden;
            continue;
        };
        // Line is gone while the rig is moving (walking), holding the
        // rod up after a reel-in, or in the wind-up half of a cast
        // (where the rod is going *back*, not yet releasing the line).
        let line_visible = match fisher.state {
            FishermanState::WalkingToSpot { .. } | FishermanState::Held { .. } => false,
            FishermanState::Casting { time, dur } => time / dur >= 0.45,
            _ => true,
        };
        if !line_visible {
            *vis = Visibility::Hidden;
            continue;
        }
        *vis = Visibility::Visible;

        let rotation = pole_rotation(&fisher.state, now);
        let pole_center = fisher_pos.0 + POLE_HANDLE_OFFSET;
        let tip = pole_tip(pole_center, rotation);

        // Bobber: cast distance grows over the forward-snap of a
        // cast, holds full while fishing, and tracks back during
        // reel-in. Plus a gentle bob keyed off the fisherman's y so
        // multiple fishermen don't bob in lockstep.
        let cast_distance = match fisher.state {
            FishermanState::Casting { time, dur } => {
                // Visibility check above guarantees we're in the
                // snap-forward half (t >= 0.45) here.
                let snap_progress = ((time / dur - 0.45) / 0.55).clamp(0.0, 1.0);
                LINE_CAST_DISTANCE * snap_progress
            }
            FishermanState::Reeling { time, dur, .. } => {
                let p = (time / dur).clamp(0.0, 1.0);
                lerp_angle(LINE_CAST_DISTANCE, LINE_REEL_DISTANCE, p)
            }
            _ => LINE_CAST_DISTANCE,
        };
        let bob = (now * 1.3 + fisher_pos.0.y * 0.07).sin() * 1.0;
        let bobber = Vec2::new(tip.x + cast_distance, fisher_pos.0.y + 3.0 + bob);

        // Render the line as a 1-px-tall sprite scaled to the
        // tip → bobber distance and rotated to match.
        let spec_delta = bobber - tip;
        let length = spec_delta.length().max(1.0);
        let mid = (tip + bobber) * 0.5;
        let line_angle = (-spec_delta.y).atan2(spec_delta.x);

        pos.0 = mid;
        sprite.custom_size = Some(Vec2::new(length, 1.0));
        tf.rotation = Quat::from_rotation_z(line_angle);
    }
}

/// Rotation in Bevy world (Y-up, CCW positive) for the rod, given
/// the owner's current state and a time accumulator (for idle bob).
fn pole_rotation(state: &FishermanState, now: f32) -> f32 {
    match *state {
        FishermanState::WalkingToSpot { .. } => POLE_WALK_ANGLE,
        FishermanState::Casting { time, dur } => {
            let t = (time / dur).clamp(0.0, 1.0);
            if t < 0.45 {
                let p = t / 0.45;
                lerp_angle(POLE_FISHING_ANGLE, POLE_CAST_BACK, p)
            } else {
                let p = ((t - 0.45) / 0.55).clamp(0.0, 1.0);
                lerp_angle(POLE_CAST_BACK, POLE_CAST_FORWARD, p)
            }
        }
        FishermanState::Fishing { .. } => POLE_FISHING_ANGLE + (now * 1.6).sin() * 0.04,
        FishermanState::Reeling { time, .. } => {
            POLE_FISHING_ANGLE + (time * 18.0).sin() * 0.18
        }
        FishermanState::Held { time, .. } => {
            // Quick lift from the reel-in pose to the held-up pose
            // over the first 0.4 s, then steady.
            let t = (time / 0.4).clamp(0.0, 1.0);
            lerp_angle(POLE_FISHING_ANGLE, POLE_HELD_ANGLE, t)
        }
    }
}

fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Spec-space position of the rod's tip pixel, given the sprite's
/// centre position and current rotation. The rod art has its tip in
/// the top-right of a 5×5 mask, which in unrotated world-local
/// coords sits at `(+2, +2)` (Bevy Y-up). Apply the rotation matrix,
/// then flip the Y component back into spec (Y-down) space.
fn pole_tip(center: Vec2, rotation: f32) -> Vec2 {
    let (s, c) = rotation.sin_cos();
    let off_x = c * 2.0 - s * 2.0;
    let off_y = s * 2.0 + c * 2.0;
    center + Vec2::new(off_x, -off_y)
}

/// Sample a fresh fishing spot. Spread along the shoreline (x just
/// landward of the foam line) and across the full beach height (y).
fn pick_fisherman_spot<R: Rng + ?Sized>(rng: &mut R) -> Vec2 {
    Vec2::new(
        rng.gen_range(FISHERMAN_SPOT_X_MIN..FISHERMAN_SPOT_X_MAX),
        rng.gen_range(FISHERMAN_SPOT_Y_MIN..FISHERMAN_SPOT_Y_MAX),
    )
}
