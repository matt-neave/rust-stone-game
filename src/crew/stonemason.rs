//! Stonemasons — pick idle rocks off the sand and sharpen them.
//!
//! Cycle: walk to a fresh idle rock → mason in place (chiselling
//! animation) → mark it [`Masoned`] (lighter-coloured material,
//! guarantees the next 2 bounces) → wander off and find another.
//!
//! Stonemasons claim rocks via [`SmallRockPhase::Claimed`] so two
//! masons can't fight over the same rock. They never carry the rock —
//! the work happens in place.

use bevy::prelude::*;
use rand::Rng;

use crate::audio::{PlaySoundEvent, SoundKind};
use crate::core::colors;
use crate::core::common::{Layer, Pos};
use crate::core::constants::Z_CREW;
use crate::economy::{PurchaseKind, Stonemasons};
use crate::render::shapes::Shapes;
use crate::rocks::small::{Masoned, RockShape, SmallRock, SmallRockPhase};

use super::{step_walk_frame, tick_walk_animation, CrewWalking, SpawnConversionEvent};

const STONEMASON_WALK_SPEED: f32 = 20.0;
const MASON_DURATION: f32 = 3.0;
const REST_DURATION: f32 = 0.4;
const SEARCH_RETRY: f32 = 0.7;
/// Number of guaranteed-success bounce checks a masoned rock gets.
const MASONED_BOUNCE_CHARGES: u8 = 2;

const CHISEL_HELD_OFFSET: Vec2 = Vec2::new(2.5, -3.0);
const CHISEL_DRAW_SIZE: f32 = 4.0;

#[derive(Component)]
pub struct Stonemason {
    pub state: StonemasonState,
    pub flap_accum: f32,
    pub walk_frame: bool,
}

pub enum StonemasonState {
    /// Looking for a free idle rock to claim.
    Searching { time: f32, dur: f32 },
    /// Claimed a rock, walking to it.
    WalkingToRock { rock: Entity, from: Vec2, to: Vec2, time: f32, dur: f32 },
    /// Standing over the rock, chiselling away.
    Masoning { rock: Entity, time: f32, dur: f32 },
    Resting { time: f32, dur: f32 },
}

impl CrewWalking for Stonemason {
    fn is_walking(&self) -> bool {
        matches!(self.state, StonemasonState::WalkingToRock { .. })
    }
    fn walk_frame(&self) -> bool {
        self.walk_frame
    }
    fn step_walking(&mut self, walking: bool, dt: f32) -> bool {
        step_walk_frame(walking, &mut self.flap_accum, &mut self.walk_frame, dt)
    }
}

#[derive(Component)]
pub struct Chisel {
    pub owner: Entity,
}

pub struct StonemasonPlugin;

impl Plugin for StonemasonPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                handle_stonemason_spawn,
                tick_stonemasons,
                update_chisels,
                tick_walk_animation::<Stonemason>,
            ),
        );
    }
}

fn handle_stonemason_spawn(
    mut events: MessageReader<SpawnConversionEvent>,
    mut commands: Commands,
    mut masons: ResMut<Stonemasons>,
    shapes: Res<Shapes>,
) {
    for ev in events.read() {
        if ev.kind != PurchaseKind::Stonemason {
            continue;
        }
        spawn_stonemason(&mut commands, &shapes, &mut masons, ev.from_pos);
    }
}

fn spawn_stonemason(
    commands: &mut Commands,
    shapes: &Shapes,
    masons: &mut Stonemasons,
    worker_pos: Vec2,
) {
    masons.count += 1;
    let mason_e = commands
        .spawn((
            Stonemason {
                state: StonemasonState::Searching { time: 0.0, dur: 0.0 },
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
        Chisel { owner: mason_e },
        Pos(worker_pos + CHISEL_HELD_OFFSET),
        Layer(Z_CREW + 0.1),
        Sprite {
            image: shapes.pickaxe.clone(),
            color: colors::PICKAXE,
            custom_size: Some(Vec2::splat(CHISEL_DRAW_SIZE)),
            ..default()
        },
        Transform::default(),
    ));
}

#[allow(clippy::too_many_arguments)]
fn tick_stonemasons(
    time: Res<Time>,
    mut commands: Commands,
    shapes: Res<Shapes>,
    mut masons: Query<(Entity, &mut Stonemason, &mut Pos), Without<SmallRock>>,
    mut rocks: Query<
        (
            Entity,
            &Pos,
            &mut SmallRockPhase,
            &mut Sprite,
            &RockShape,
            Option<&Masoned>,
        ),
        (With<SmallRock>, Without<Stonemason>),
    >,
    mut sound: MessageWriter<PlaySoundEvent>,
) {
    let dt = time.delta_secs();
    let mut rng = rand::thread_rng();
    for (mason_e, mut mason, mut pos) in &mut masons {
        let next: Option<StonemasonState> = match &mut mason.state {
            StonemasonState::Searching { time: t, dur } => {
                *t += dt;
                if *t < *dur {
                    None
                } else {
                    // Look for the closest idle non-masoned rock.
                    let mut best: Option<(f32, Entity, Vec2)> = None;
                    for (rock_e, rock_pos, phase, _, _, masoned) in &rocks {
                        if !matches!(*phase, SmallRockPhase::Idle) {
                            continue;
                        }
                        if masoned.is_some() {
                            continue;
                        }
                        let d = pos.0.distance(rock_pos.0);
                        if best.map_or(true, |(bd, _, _)| d < bd) {
                            best = Some((d, rock_e, rock_pos.0));
                        }
                    }
                    if let Some((_, rock_e, rock_pos)) = best {
                        // Claim the rock so other masons / skimmers
                        // don't grab it. Walk over.
                        if let Ok((_, _, mut phase, _, _, _)) = rocks.get_mut(rock_e) {
                            *phase = SmallRockPhase::Claimed { by: mason_e };
                        }
                        let dist = pos.0.distance(rock_pos).max(1.0);
                        let dur = (dist / STONEMASON_WALK_SPEED).clamp(0.3, 6.0);
                        Some(StonemasonState::WalkingToRock {
                            rock: rock_e,
                            from: pos.0,
                            to: rock_pos,
                            time: 0.0,
                            dur,
                        })
                    } else {
                        Some(StonemasonState::Searching { time: 0.0, dur: SEARCH_RETRY })
                    }
                }
            }
            StonemasonState::WalkingToRock { rock, from, to, time: t, dur } => {
                *t += dt;
                let prog = (*t / *dur).clamp(0.0, 1.0);
                pos.0 = from.lerp(*to, prog);
                if *t >= *dur {
                    pos.0 = *to;
                    // Verify the rock still exists and is still ours.
                    let still_ours = rocks
                        .get(*rock)
                        .ok()
                        .map(|(_, _, p, _, _, _)| {
                            matches!(*p, SmallRockPhase::Claimed { by } if by == mason_e)
                        })
                        .unwrap_or(false);
                    if still_ours {
                        Some(StonemasonState::Masoning {
                            rock: *rock,
                            time: 0.0,
                            dur: MASON_DURATION,
                        })
                    } else {
                        Some(StonemasonState::Searching {
                            time: 0.0,
                            dur: SEARCH_RETRY,
                        })
                    }
                } else {
                    None
                }
            }
            StonemasonState::Masoning { rock, time: t, dur } => {
                *t += dt;
                if *t >= *dur {
                    // Finalise: insert Masoned, lighten material, set
                    // the rock back to Idle so it can be picked up /
                    // tossed by the player or a skimmer.
                    if let Ok((_, _, mut phase, mut sprite, shape, _)) = rocks.get_mut(*rock) {
                        *phase = SmallRockPhase::Idle;
                        commands.entity(*rock).insert(Masoned {
                            remaining: MASONED_BOUNCE_CHARGES,
                        });
                        sprite.image = shapes.small_rock_image_lit(shape.0);
                        sound.write(PlaySoundEvent {
                            kind: SoundKind::Click,
                            pitch: 1.4,
                            volume: 0.25,
                        });
                    }
                    Some(StonemasonState::Resting { time: 0.0, dur: REST_DURATION })
                } else {
                    // Faint chiselling sound roughly twice a second.
                    if (*t * 2.0).fract() < dt * 2.0 {
                        sound.write(PlaySoundEvent {
                            kind: SoundKind::Click,
                            pitch: 0.95 + rng.gen::<f32>() * 0.1,
                            volume: 0.12,
                        });
                    }
                    None
                }
            }
            StonemasonState::Resting { time: t, dur } => {
                *t += dt;
                if *t >= *dur {
                    Some(StonemasonState::Searching { time: 0.0, dur: 0.0 })
                } else {
                    None
                }
            }
        };
        if let Some(s) = next {
            mason.state = s;
        }
    }
}

fn update_chisels(
    masons: Query<(&Stonemason, &Pos), Without<Chisel>>,
    mut chisels: Query<(&Chisel, &mut Pos, &mut Transform), Without<Stonemason>>,
) {
    for (ch, mut pos, mut tf) in &mut chisels {
        let Ok((mason, mason_pos)) = masons.get(ch.owner) else {
            continue;
        };
        match &mason.state {
            StonemasonState::Searching { .. }
            | StonemasonState::WalkingToRock { .. }
            | StonemasonState::Resting { .. } => {
                pos.0 = mason_pos.0 + CHISEL_HELD_OFFSET;
                tf.rotation = Quat::from_rotation_z(0.6);
            }
            StonemasonState::Masoning { time, dur, .. } => {
                // Quick chip-chip rotation cycling between two angles.
                let phase = (time / dur * 8.0).fract();
                let angle = if phase < 0.5 { 0.4 } else { 1.1 };
                pos.0 = mason_pos.0 + CHISEL_HELD_OFFSET;
                tf.rotation = Quat::from_rotation_z(angle);
            }
        }
    }
}
