//! Pickaxe-throwing miners.
//!
//! Cycle (~10 seconds): walk from the hut to an assigned throw spot →
//! rest → wind up → fly the pickaxe at the rock (3 damage) → walk to
//! the impact site → walk back → rest → repeat.
//!
//! The pickaxe is a separate entity tied to its owner via [`Pickaxe`];
//! [`update_pickaxes`] computes its position + rotation each frame
//! from the owner's [`MinerState`].

use bevy::prelude::*;
use rand::Rng;

use crate::audio::{PlaySoundEvent, SoundKind};
use crate::core::colors;
use crate::core::common::{Layer, Pos};
use crate::core::constants::{
    BIG_ROCK_X, BIG_ROCK_Y, MINER_HUT_TO_SPOT_SPEED, MINER_PICKAXE_FLIGHT, MINER_REST,
    MINER_THROW_WIND_UP, MINER_WALK_SPEED, Z_CREW, Z_PICKAXE,
};
use crate::economy::{Miners, PurchaseKind};
use crate::render::shapes::Shapes;
use crate::rocks::big::RockHitEvent;

use super::{step_walk_frame, tick_walk_animation, CrewWalking, SpawnConversionEvent};

/// How much damage a single miner pickaxe throw does to the big rock.
pub const MINER_PICKAXE_DAMAGE: u32 = 3;

/// Bounding box around the boulder that miners may pick a throw spot
/// from. Each cycle a fresh random spot inside this box is sampled and
/// rejected if it falls inside the rock's no-stand radius.
const MINER_SPOT_X_MIN: f32 = 10.0;
const MINER_SPOT_X_MAX: f32 = 96.0;
const MINER_SPOT_Y_MIN: f32 = 138.0;
const MINER_SPOT_Y_MAX: f32 = 240.0;
/// Closest a miner can stand to the rock's center. Keeps them out of
/// the silhouette and gives the pickaxe-arc room to read.
const MINER_SPOT_MIN_DIST: f32 = 28.0;

/// Apex height of the pickaxe arc when in flight (px above the chord).
const PICKAXE_FLIGHT_APEX: f32 = 28.0;
/// Pickaxe rendered size — square so rotation looks symmetric.
const PICKAXE_DRAW_SIZE: f32 = 4.0;
/// Where the pickaxe head sits relative to the miner's body when held.
const PICKAXE_HELD_OFFSET: Vec2 = Vec2::new(2.5, -3.5);

#[inline]
fn rock_center() -> Vec2 {
    Vec2::new(BIG_ROCK_X, BIG_ROCK_Y)
}

#[derive(Component)]
pub struct Miner {
    pub state: MinerState,
    pub flap_accum: f32,
    pub walk_frame: bool,
}

pub enum MinerState {
    /// Walking from the hut to the first throw spot.
    WalkingFromHut { from: Vec2, to: Vec2, time: f32, dur: f32 },
    Resting { time: f32, dur: f32 },
    Throwing { time: f32, dur: f32 },
    PickaxeFlight { from: Vec2, to: Vec2, time: f32, dur: f32 },
    WalkingToRock { from: Vec2, to: Vec2, time: f32, dur: f32 },
    /// Walking from the impact site to the next freshly-picked throw
    /// spot. Replaces the old "walk back home" state — miners no
    /// longer hold a fixed home position.
    WalkingToNextSpot { from: Vec2, to: Vec2, time: f32, dur: f32 },
}

impl CrewWalking for Miner {
    fn is_walking(&self) -> bool {
        matches!(
            self.state,
            MinerState::WalkingFromHut { .. }
                | MinerState::WalkingToRock { .. }
                | MinerState::WalkingToNextSpot { .. }
        )
    }
    fn walk_frame(&self) -> bool {
        self.walk_frame
    }
    fn step_walking(&mut self, walking: bool, dt: f32) -> bool {
        step_walk_frame(walking, &mut self.flap_accum, &mut self.walk_frame, dt)
    }
}

#[derive(Component)]
pub struct Pickaxe {
    pub owner: Entity,
}

pub struct MinerPlugin;

impl Plugin for MinerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                handle_miner_spawn,
                tick_miners,
                update_pickaxes,
                tick_walk_animation::<Miner>,
            ),
        );
    }
}

fn handle_miner_spawn(
    mut events: MessageReader<SpawnConversionEvent>,
    mut commands: Commands,
    mut miners: ResMut<Miners>,
    shapes: Res<Shapes>,
) {
    for ev in events.read() {
        if ev.kind != PurchaseKind::Miner {
            continue;
        }
        spawn_miner(&mut commands, &shapes, &mut miners, ev.from_pos);
    }
}

fn spawn_miner(
    commands: &mut Commands,
    shapes: &Shapes,
    miners: &mut Miners,
    worker_pos: Vec2,
) {
    miners.count += 1;
    let mut rng = rand::thread_rng();
    let throw_spot = pick_miner_spot(&mut rng);

    let dist = worker_pos.distance(throw_spot).max(1.0);
    let dur = (dist / MINER_HUT_TO_SPOT_SPEED).clamp(0.5, 6.0);

    let miner_e = commands
        .spawn((
            Miner {
                state: MinerState::WalkingFromHut {
                    from: worker_pos,
                    to: throw_spot,
                    time: 0.0,
                    dur,
                },
                flap_accum: 0.0,
                walk_frame: false,
            },
            Pos(worker_pos),
            Layer(Z_CREW),
            Sprite {
                image: shapes.humanoid.clone(),
                color: colors::MINER_BODY,
                custom_size: Some(Vec2::new(4.0, 6.0)),
                ..default()
            },
            Transform::default(),
        ))
        .id();

    commands.spawn((
        Pickaxe { owner: miner_e },
        Pos(worker_pos + PICKAXE_HELD_OFFSET),
        Layer(Z_PICKAXE),
        Sprite {
            image: shapes.pickaxe.clone(),
            color: colors::PICKAXE,
            custom_size: Some(Vec2::splat(PICKAXE_DRAW_SIZE)),
            ..default()
        },
        Transform::default(),
    ));
}

fn tick_miners(
    time: Res<Time>,
    mut q: Query<(&mut Miner, &mut Pos)>,
    mut hits: MessageWriter<RockHitEvent>,
    mut sound: MessageWriter<PlaySoundEvent>,
) {
    let dt = time.delta_secs();
    let mut rng = rand::thread_rng();
    for (mut miner, mut pos) in &mut q {
        let next: Option<MinerState> = match &mut miner.state {
            MinerState::WalkingFromHut { from, to, time: t, dur } => {
                *t += dt;
                let prog = (*t / *dur).clamp(0.0, 1.0);
                pos.0 = from.lerp(*to, prog);
                if *t >= *dur {
                    pos.0 = *to;
                    Some(MinerState::Resting { time: 0.0, dur: MINER_REST })
                } else { None }
            }
            MinerState::Resting { time: t, dur } => {
                *t += dt;
                if *t >= *dur {
                    Some(MinerState::Throwing { time: 0.0, dur: MINER_THROW_WIND_UP })
                } else { None }
            }
            MinerState::Throwing { time: t, dur } => {
                *t += dt;
                if *t >= *dur {
                    // Pickaxe leaves the miner's hand and flies toward
                    // a point on the rock's perimeter facing the miner —
                    // 16 px from the centre, on the line between them.
                    let from_pos = pos.0 + PICKAXE_HELD_OFFSET;
                    let landing = rock_center()
                        + (from_pos - rock_center()).normalize_or_zero() * 16.0;
                    sound.write(PlaySoundEvent {
                        kind: SoundKind::Click,
                        pitch: 0.85,
                        volume: 0.18,
                    });
                    Some(MinerState::PickaxeFlight {
                        from: from_pos,
                        to: landing,
                        time: 0.0,
                        dur: MINER_PICKAXE_FLIGHT,
                    })
                } else { None }
            }
            MinerState::PickaxeFlight { from: _, to, time: t, dur } => {
                *t += dt;
                if *t >= *dur {
                    // Impact — register a hit on the big rock. The shared
                    // RockHitEvent path handles dust, sound, click counter,
                    // and any small-rock spawning that follows.
                    hits.write(RockHitEvent {
                        pos: *to,
                        damage: MINER_PICKAXE_DAMAGE,
                    });
                    let from_pos = pos.0;
                    let to_pos = *to;
                    let dist = from_pos.distance(to_pos).max(1.0);
                    let walk_dur = dist / MINER_WALK_SPEED;
                    Some(MinerState::WalkingToRock {
                        from: from_pos,
                        to: to_pos,
                        time: 0.0,
                        dur: walk_dur,
                    })
                } else { None }
            }
            MinerState::WalkingToRock { from, to, time: t, dur } => {
                *t += dt;
                let prog = (*t / *dur).clamp(0.0, 1.0);
                pos.0 = from.lerp(*to, prog);
                if *t >= *dur {
                    pos.0 = *to;
                    // Pick a fresh throw spot for the next cycle so
                    // miners drift around the boulder rather than
                    // returning to the same fixed home each time.
                    let next_spot = pick_miner_spot(&mut rng);
                    let dist = to.distance(next_spot).max(1.0);
                    let walk_dur = dist / MINER_WALK_SPEED;
                    Some(MinerState::WalkingToNextSpot {
                        from: *to,
                        to: next_spot,
                        time: 0.0,
                        dur: walk_dur,
                    })
                } else { None }
            }
            MinerState::WalkingToNextSpot { from, to, time: t, dur } => {
                *t += dt;
                let prog = (*t / *dur).clamp(0.0, 1.0);
                pos.0 = from.lerp(*to, prog);
                if *t >= *dur {
                    pos.0 = *to;
                    Some(MinerState::Resting { time: 0.0, dur: MINER_REST })
                } else { None }
            }
        };
        if let Some(next) = next {
            miner.state = next;
        }
    }
}

fn update_pickaxes(
    miner_q: Query<(&Miner, &Pos)>,
    mut pickaxe_q: Query<(&Pickaxe, &mut Pos, &mut Transform), Without<Miner>>,
) {
    for (pa, mut pos, mut tf) in &mut pickaxe_q {
        let Ok((miner, miner_pos)) = miner_q.get(pa.owner) else {
            // Miner gone — leave the pickaxe wherever it last was.
            continue;
        };
        match &miner.state {
            MinerState::WalkingFromHut { .. }
            | MinerState::Resting { .. }
            | MinerState::WalkingToNextSpot { .. } => {
                pos.0 = miner_pos.0 + PICKAXE_HELD_OFFSET;
                tf.rotation = Quat::IDENTITY;
            }
            MinerState::Throwing { time, dur } => {
                // Wind-up — pickaxe rotates back over the shoulder.
                let prog = (time / dur).clamp(0.0, 1.0);
                pos.0 = miner_pos.0 + PICKAXE_HELD_OFFSET;
                tf.rotation = Quat::from_rotation_z(-prog * 1.2);
            }
            MinerState::PickaxeFlight { from, to, time, dur } => {
                let t = (time / dur).clamp(0.0, 1.0);
                let lerp = from.lerp(*to, t);
                let arc = -PICKAXE_FLIGHT_APEX * 4.0 * t * (1.0 - t);
                pos.0 = Vec2::new(lerp.x, lerp.y + arc);
                tf.rotation = Quat::from_rotation_z(time * 18.0);
            }
            MinerState::WalkingToRock { to, .. } => {
                // Pickaxe sticks at the rock until the miner gets there.
                pos.0 = *to;
                tf.rotation = Quat::from_rotation_z(0.6);
            }
        }
    }
}

/// Sample a random throw spot in the miner's working box that isn't
/// inside the boulder. Rejection-sampling — bounded to a few tries
/// and fallback to the box corner so we never spin.
fn pick_miner_spot<R: Rng + ?Sized>(rng: &mut R) -> Vec2 {
    let center = rock_center();
    let min_dist_sq = MINER_SPOT_MIN_DIST * MINER_SPOT_MIN_DIST;
    for _ in 0..16 {
        let x = rng.gen_range(MINER_SPOT_X_MIN..MINER_SPOT_X_MAX);
        let y = rng.gen_range(MINER_SPOT_Y_MIN..MINER_SPOT_Y_MAX);
        let p = Vec2::new(x, y);
        if (p - center).length_squared() >= min_dist_sq {
            return p;
        }
    }
    Vec2::new(MINER_SPOT_X_MIN, MINER_SPOT_Y_MAX)
}
