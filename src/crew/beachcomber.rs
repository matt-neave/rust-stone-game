//! Beachcombers — wander the sand with a shovel digging up small rocks.
//!
//! Cycle (~8 s): walk to a fresh sand spot → dig (shovel rocks side-to-side)
//! → spawn a small rock that pops up out of the ground → repeat.
//!
//! Unlike the fisherman (which spawns rocks arcing in from off-screen)
//! the beachcomber's rock pops up at the dig site itself — short fall
//! arc with a tiny upward hop.

use bevy::prelude::*;
use rand::Rng;

use crate::audio::{PlaySoundEvent, SoundKind};
use crate::core::colors;
use crate::core::common::{Layer, Pos};
use crate::core::constants::{
    SAND_LAND_X_MAX, SAND_LAND_X_MIN, SAND_LAND_Y_MAX, SAND_LAND_Y_MIN, Z_CREW,
};
use crate::economy::{Beachcombers, PurchaseKind};
use crate::render::shapes::Shapes;
use crate::rocks::small::{SpawnSmallRockEvent, SMALL_ROCK_FALL_DURATION_FAST};

use super::{step_walk_frame, tick_walk_animation, CrewWalking, SpawnConversionEvent};

const BEACHCOMBER_WALK_SPEED: f32 = 18.0;
const DIG_DURATION: f32 = 2.4;
const REST_DURATION: f32 = 0.6;
/// Where the shovel sits relative to the comber's body when held.
const SHOVEL_HELD_OFFSET: Vec2 = Vec2::new(2.5, -3.0);
const SHOVEL_DRAW_SIZE: f32 = 4.0;
/// Apex of the rock-popping-out-of-the-ground arc.
const ROCK_POP_APEX: f32 = 6.0;

#[derive(Component)]
pub struct Beachcomber {
    pub state: BeachcomberState,
    pub flap_accum: f32,
    pub walk_frame: bool,
}

pub enum BeachcomberState {
    WalkingToSpot { from: Vec2, to: Vec2, time: f32, dur: f32 },
    Digging { time: f32, dur: f32 },
    Resting { time: f32, dur: f32 },
}

impl CrewWalking for Beachcomber {
    fn is_walking(&self) -> bool {
        matches!(self.state, BeachcomberState::WalkingToSpot { .. })
    }
    fn walk_frame(&self) -> bool {
        self.walk_frame
    }
    fn step_walking(&mut self, walking: bool, dt: f32) -> bool {
        step_walk_frame(walking, &mut self.flap_accum, &mut self.walk_frame, dt)
    }
}

#[derive(Component)]
pub struct Shovel {
    pub owner: Entity,
}

pub struct BeachcomberPlugin;

impl Plugin for BeachcomberPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                handle_beachcomber_spawn,
                tick_beachcombers,
                update_shovels,
                tick_walk_animation::<Beachcomber>,
            ),
        );
    }
}

fn handle_beachcomber_spawn(
    mut events: MessageReader<SpawnConversionEvent>,
    mut commands: Commands,
    mut combers: ResMut<Beachcombers>,
    shapes: Res<Shapes>,
) {
    for ev in events.read() {
        if ev.kind != PurchaseKind::Beachcomber {
            continue;
        }
        spawn_beachcomber(&mut commands, &shapes, &mut combers, ev.from_pos);
    }
}

fn spawn_beachcomber(
    commands: &mut Commands,
    shapes: &Shapes,
    combers: &mut Beachcombers,
    worker_pos: Vec2,
) {
    combers.count += 1;
    let mut rng = rand::thread_rng();
    let spot = pick_dig_spot(&mut rng);
    let dist = worker_pos.distance(spot).max(1.0);
    let dur = (dist / BEACHCOMBER_WALK_SPEED).clamp(0.5, 6.0);

    let comber_e = commands
        .spawn((
            Beachcomber {
                state: BeachcomberState::WalkingToSpot {
                    from: worker_pos,
                    to: spot,
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
                color: Color::WHITE,
                custom_size: Some(Vec2::new(16.0, 9.0)),
                ..default()
            },
            Transform::default(),
        ))
        .id();

    commands.spawn((
        Shovel { owner: comber_e },
        Pos(worker_pos + SHOVEL_HELD_OFFSET),
        Layer(Z_CREW + 0.1),
        Sprite {
            image: shapes.pickaxe.clone(),
            color: colors::PICKAXE,
            custom_size: Some(Vec2::splat(SHOVEL_DRAW_SIZE)),
            ..default()
        },
        Transform::default(),
    ));
}

fn tick_beachcombers(
    time: Res<Time>,
    mut q: Query<(&mut Beachcomber, &mut Pos)>,
    mut spawn_rock: MessageWriter<SpawnSmallRockEvent>,
    mut sound: MessageWriter<PlaySoundEvent>,
) {
    let dt = time.delta_secs();
    let mut rng = rand::thread_rng();
    for (mut comber, mut pos) in &mut q {
        let next: Option<BeachcomberState> = match &mut comber.state {
            BeachcomberState::WalkingToSpot { from, to, time: t, dur } => {
                *t += dt;
                let prog = (*t / *dur).clamp(0.0, 1.0);
                pos.0 = from.lerp(*to, prog);
                if *t >= *dur {
                    pos.0 = *to;
                    Some(BeachcomberState::Digging { time: 0.0, dur: DIG_DURATION })
                } else {
                    None
                }
            }
            BeachcomberState::Digging { time: t, dur } => {
                *t += dt;
                if *t >= *dur {
                    // Rock pops up just beside the dig site.
                    let dx = if rng.gen_bool(0.5) { -3.0 } else { 3.0 };
                    let from = Vec2::new(pos.0.x + dx, pos.0.y - ROCK_POP_APEX);
                    let to = Vec2::new(pos.0.x + dx, pos.0.y + 1.0);
                    spawn_rock.write(SpawnSmallRockEvent {
                        from,
                        to,
                        duration: SMALL_ROCK_FALL_DURATION_FAST,
                    });
                    sound.write(PlaySoundEvent {
                        kind: SoundKind::SmallRockSpawn,
                        pitch: 0.95,
                        volume: 0.32,
                    });
                    Some(BeachcomberState::Resting { time: 0.0, dur: REST_DURATION })
                } else {
                    None
                }
            }
            BeachcomberState::Resting { time: t, dur } => {
                *t += dt;
                if *t >= *dur {
                    let next_spot = pick_dig_spot(&mut rng);
                    let dist = pos.0.distance(next_spot).max(1.0);
                    let walk_dur = (dist / BEACHCOMBER_WALK_SPEED).clamp(0.4, 5.0);
                    Some(BeachcomberState::WalkingToSpot {
                        from: pos.0,
                        to: next_spot,
                        time: 0.0,
                        dur: walk_dur,
                    })
                } else {
                    None
                }
            }
        };
        if let Some(s) = next {
            comber.state = s;
        }
    }
}

/// Anchors the shovel to the comber's hand and rotates it during the
/// dig animation: tilts up-back, then thrusts down to scoop.
fn update_shovels(
    combers: Query<(&Beachcomber, &Pos), Without<Shovel>>,
    mut shovels: Query<(&Shovel, &mut Pos, &mut Transform), Without<Beachcomber>>,
) {
    for (sh, mut pos, mut tf) in &mut shovels {
        let Ok((comber, comber_pos)) = combers.get(sh.owner) else {
            continue;
        };
        match &comber.state {
            BeachcomberState::WalkingToSpot { .. } | BeachcomberState::Resting { .. } => {
                pos.0 = comber_pos.0 + SHOVEL_HELD_OFFSET;
                tf.rotation = Quat::from_rotation_z(0.6);
            }
            BeachcomberState::Digging { time, dur } => {
                // 4 thrusts across the dig duration: up-back, then
                // stab down.
                let phase = (time / dur * 4.0).fract();
                let angle = if phase < 0.5 {
                    0.6 + phase * 1.2
                } else {
                    1.2 - (phase - 0.5) * 2.0
                };
                pos.0 = comber_pos.0 + SHOVEL_HELD_OFFSET;
                tf.rotation = Quat::from_rotation_z(angle);
            }
        }
    }
}

fn pick_dig_spot<R: Rng + ?Sized>(rng: &mut R) -> Vec2 {
    Vec2::new(
        rng.gen_range(SAND_LAND_X_MIN..SAND_LAND_X_MAX),
        rng.gen_range(SAND_LAND_Y_MIN..SAND_LAND_Y_MAX),
    )
}
