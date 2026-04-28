//! Boatmen — sail the ocean from the port pulling sunken rocks back
//! onto the sand. Gated behind the [`Port`] structure.
//!
//! Cycle: idle at the port → sail to nearest [`SmallRockPhase::Sunken`]
//! rock → haul → if cargo isn't full and another sunken rock exists,
//! sail to it; otherwise sail back to the port and fling each cargo
//! rock onto the sand using the standard tossing physics.
//!
//! Cargo capacity is `BOATMAN_CARGO_CAPACITY` (5). Picked-up rocks
//! ride on the boat as a small visible stack until the boatman gets
//! back to the dock and unloads them one by one.

use bevy::prelude::*;
use rand::Rng;

use crate::audio::{PlaySoundEvent, SoundKind};
use crate::core::colors;
use crate::core::common::{Layer, Pos};
use crate::core::constants::{
    BOUNCE_CHANCE_MAX, INTERNAL_HEIGHT, PLAYER_BOUNCE_CHANCE, PORT_X, PORT_Y, SHORELINE_X,
    SKIM_SPEED, SKIM_UPGRADE_DELTA, Z_CREW,
};
use crate::economy::{Boatmen, PurchaseKind, SkimUpgrades};
use crate::render::shapes::Shapes;
use crate::rocks::small::{BounceChance, SmallRock, SmallRockPhase};

use super::{step_walk_frame, tick_walk_animation, CrewWalking, SpawnConversionEvent};

const BOAT_SPEED: f32 = 32.0;
const PICK_UP_DURATION: f32 = 0.4;
const THROW_DURATION: f32 = 0.45;
const REST_DURATION: f32 = 0.5;
const SEARCH_RETRY: f32 = 1.0;
const BOAT_W: f32 = 8.0;
const BOAT_H: f32 = 4.0;
/// How many sunken rocks a single boatman holds before steaming home.
pub const BOATMAN_CARGO_CAPACITY: usize = 5;
/// Per-rock vertical offset when stacked on the boat (rear cargo
/// stacks higher visually so the pile is readable).
const CARGO_STACK_DY: f32 = -1.5;

#[inline]
fn port_dock_pos() -> Vec2 {
    Vec2::new(PORT_X, PORT_Y - 1.0)
}

#[derive(Component)]
pub struct Boatman {
    pub state: BoatmanState,
    /// Sunken rocks already loaded — drained one at a time during the
    /// `Throwing` state once the boatman is back at the dock.
    pub cargo: Vec<Entity>,
    pub flap_accum: f32,
    pub walk_frame: bool,
}

pub enum BoatmanState {
    Searching { time: f32, dur: f32 },
    SailingToRock { rock: Entity, from: Vec2, to: Vec2, time: f32, dur: f32 },
    PickingUp { rock: Entity, time: f32, dur: f32 },
    ReturningToPort { from: Vec2, to: Vec2, time: f32, dur: f32 },
    /// Standing at the port flinging one rock toward the sand. The
    /// state holds a single rock entity — the boatman re-enters
    /// `Throwing` once per cargo item.
    Throwing { rock: Entity, time: f32, dur: f32 },
    Resting { time: f32, dur: f32 },
}

impl CrewWalking for Boatman {
    fn is_walking(&self) -> bool {
        matches!(
            self.state,
            BoatmanState::SailingToRock { .. } | BoatmanState::ReturningToPort { .. }
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
pub struct Boat {
    pub owner: Entity,
}

pub struct BoatmanPlugin;

impl Plugin for BoatmanPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                handle_boatman_spawn,
                tick_boatmen,
                update_boats,
                update_cargo_positions,
                tick_walk_animation::<Boatman>,
            ),
        );
    }
}

fn handle_boatman_spawn(
    mut events: MessageReader<SpawnConversionEvent>,
    mut commands: Commands,
    mut boatmen: ResMut<Boatmen>,
    shapes: Res<Shapes>,
) {
    for ev in events.read() {
        if ev.kind != PurchaseKind::Boatman {
            continue;
        }
        spawn_boatman(&mut commands, &shapes, &mut boatmen);
    }
}

fn spawn_boatman(commands: &mut Commands, shapes: &Shapes, boatmen: &mut Boatmen) {
    boatmen.count += 1;
    let dock = port_dock_pos();

    let boatman_e = commands
        .spawn((
            Boatman {
                state: BoatmanState::Searching { time: 0.0, dur: 0.0 },
                cargo: Vec::with_capacity(BOATMAN_CARGO_CAPACITY),
                flap_accum: 0.0,
                walk_frame: false,
            },
            Pos(dock),
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
        Boat { owner: boatman_e },
        Pos(dock + Vec2::new(0.0, 4.0)),
        Layer(Z_CREW - 0.05),
        Sprite::from_color(colors::HUT_ROOF, Vec2::new(BOAT_W, BOAT_H)),
        Transform::default(),
    ));
}

#[allow(clippy::too_many_arguments)]
fn tick_boatmen(
    time: Res<Time>,
    mut commands: Commands,
    mut boatmen: Query<(Entity, &mut Boatman, &mut Pos), Without<SmallRock>>,
    mut rocks: Query<
        (Entity, &mut SmallRockPhase, &mut Pos, &mut Visibility),
        (With<SmallRock>, Without<Boatman>),
    >,
    upgrades: Res<SkimUpgrades>,
    mut sound: MessageWriter<PlaySoundEvent>,
) {
    let dt = time.delta_secs();
    let mut rng = rand::thread_rng();
    for (boatman_e, mut boatman, mut pos) in &mut boatmen {
        // Split-borrow the two fields we mutate inside the match —
        // the borrow checker can't see that `state` and `cargo` are
        // disjoint when accessed through `&mut Boatman`.
        let boatman_ref = boatman.as_mut();
        let state = &mut boatman_ref.state;
        let cargo = &mut boatman_ref.cargo;
        let next: Option<BoatmanState> = match state {
            BoatmanState::Searching { time: t, dur } => {
                *t += dt;
                if *t < *dur {
                    None
                } else if let Some((rock_e, target)) =
                    claim_nearest_sunken(&mut rocks, pos.0, boatman_e)
                {
                    let dist = pos.0.distance(target).max(1.0);
                    let dur = (dist / BOAT_SPEED).clamp(0.4, 8.0);
                    Some(BoatmanState::SailingToRock {
                        rock: rock_e,
                        from: pos.0,
                        to: target,
                        time: 0.0,
                        dur,
                    })
                } else if !cargo.is_empty() {
                    // No sunken rocks left to grab — head home and
                    // unload whatever's on board.
                    let dock = port_dock_pos();
                    let dist = pos.0.distance(dock).max(1.0);
                    Some(BoatmanState::ReturningToPort {
                        from: pos.0,
                        to: dock,
                        time: 0.0,
                        dur: (dist / BOAT_SPEED).clamp(0.4, 8.0),
                    })
                } else {
                    Some(BoatmanState::Searching { time: 0.0, dur: SEARCH_RETRY })
                }
            }
            BoatmanState::SailingToRock { rock, from, to, time: t, dur } => {
                *t += dt;
                let prog = (*t / *dur).clamp(0.0, 1.0);
                pos.0 = from.lerp(*to, prog);
                if *t >= *dur {
                    pos.0 = *to;
                    Some(BoatmanState::PickingUp {
                        rock: *rock,
                        time: 0.0,
                        dur: PICK_UP_DURATION,
                    })
                } else {
                    None
                }
            }
            BoatmanState::PickingUp { rock, time: t, dur } => {
                *t += dt;
                if *t >= *dur {
                    // Claim the rock and load it. Visibility stays
                    // hidden until `update_cargo_positions` shows it
                    // on top of the boat.
                    if let Ok((_, mut phase, _, _)) = rocks.get_mut(*rock) {
                        *phase = SmallRockPhase::Carried {
                            // The carrier link is unused for boatmen
                            // since cargo positions are computed by
                            // `update_cargo_positions` rather than
                            // the generic `tick_carried` system; we
                            // still need *some* phase that won't be
                            // re-claimed by other crew.
                            carrier: Entity::PLACEHOLDER,
                            offset: Vec2::ZERO,
                        };
                    }
                    cargo.push(*rock);
                    sound.write(PlaySoundEvent {
                        kind: SoundKind::Click,
                        pitch: 1.1,
                        volume: 0.18,
                    });

                    // Decide what's next: more cargo room and another
                    // sunken rock available → continue collecting;
                    // otherwise head home.
                    let full = cargo.len() >= BOATMAN_CARGO_CAPACITY;
                    let next_target = if full {
                        None
                    } else {
                        claim_nearest_sunken(&mut rocks, pos.0, boatman_e)
                    };
                    if let Some((next_rock, target)) = next_target {
                        let dist = pos.0.distance(target).max(1.0);
                        let dur = (dist / BOAT_SPEED).clamp(0.4, 8.0);
                        Some(BoatmanState::SailingToRock {
                            rock: next_rock,
                            from: pos.0,
                            to: target,
                            time: 0.0,
                            dur,
                        })
                    } else {
                        let dock = port_dock_pos();
                        let dist = pos.0.distance(dock).max(1.0);
                        Some(BoatmanState::ReturningToPort {
                            from: pos.0,
                            to: dock,
                            time: 0.0,
                            dur: (dist / BOAT_SPEED).clamp(0.4, 8.0),
                        })
                    }
                } else {
                    None
                }
            }
            BoatmanState::ReturningToPort { from, to, time: t, dur } => {
                *t += dt;
                let prog = (*t / *dur).clamp(0.0, 1.0);
                pos.0 = from.lerp(*to, prog);
                if *t >= *dur {
                    pos.0 = *to;
                    // Pop the first rock to throw; if none, just rest.
                    if let Some(rock_e) = cargo.first().copied() {
                        Some(BoatmanState::Throwing {
                            rock: rock_e,
                            time: 0.0,
                            dur: THROW_DURATION,
                        })
                    } else {
                        Some(BoatmanState::Resting { time: 0.0, dur: REST_DURATION })
                    }
                } else {
                    None
                }
            }
            BoatmanState::Throwing { rock, time: t, dur } => {
                *t += dt;
                if *t >= *dur {
                    if let Ok((rock_e, mut phase, mut rock_pos, mut vis)) =
                        rocks.get_mut(*rock)
                    {
                        let from = pos.0;
                        let to = Vec2::new(
                            (SHORELINE_X - 25.0 + rng.gen_range(-10.0..10.0))
                                .clamp(20.0, SHORELINE_X - 6.0),
                            (from.y + rng.gen_range(-30.0..30.0))
                                .clamp(20.0, INTERNAL_HEIGHT - 18.0),
                        );
                        let dist = (to - from).length();
                        let duration = (dist / 220.0).clamp(0.35, 0.9);
                        rock_pos.0 = from;
                        *vis = Visibility::Visible;
                        *phase = SmallRockPhase::Tossing {
                            from,
                            to,
                            time: 0.0,
                            duration,
                            skim_speed: SKIM_SPEED,
                        };
                        let bonus = upgrades.level as f32 * SKIM_UPGRADE_DELTA;
                        let chance =
                            (PLAYER_BOUNCE_CHANCE + bonus).min(BOUNCE_CHANCE_MAX);
                        commands.entity(rock_e).insert(BounceChance(chance));
                    }
                    // Drop the just-thrown rock from the cargo list.
                    if let Some(idx) = cargo.iter().position(|e| *e == *rock) {
                        cargo.remove(idx);
                    }
                    sound.write(PlaySoundEvent {
                        kind: SoundKind::Click,
                        pitch: 0.7,
                        volume: 0.32,
                    });
                    // Throw the next stone, or rest if cargo's empty.
                    if let Some(next_rock) = cargo.first().copied() {
                        Some(BoatmanState::Throwing {
                            rock: next_rock,
                            time: 0.0,
                            dur: THROW_DURATION,
                        })
                    } else {
                        Some(BoatmanState::Resting { time: 0.0, dur: REST_DURATION })
                    }
                } else {
                    None
                }
            }
            BoatmanState::Resting { time: t, dur } => {
                *t += dt;
                if *t >= *dur {
                    Some(BoatmanState::Searching { time: 0.0, dur: 0.0 })
                } else {
                    None
                }
            }
        };
        if let Some(s) = next {
            boatman.state = s;
        }
    }
}

/// Find the closest sunken rock to `from`, mark it `Claimed` by the
/// caller, and return its entity + last-known underwater position.
/// Claiming up-front prevents two boatmen from chasing the same rock.
/// Already-claimed and already-carried rocks are skipped automatically
/// (only `Sunken` rocks are eligible).
fn claim_nearest_sunken(
    rocks: &mut Query<
        (Entity, &mut SmallRockPhase, &mut Pos, &mut Visibility),
        (With<SmallRock>, Without<Boatman>),
    >,
    from: Vec2,
    by: Entity,
) -> Option<(Entity, Vec2)> {
    let mut best: Option<(f32, Entity, Vec2)> = None;
    for (rock_e, phase, _, _) in rocks.iter() {
        if let SmallRockPhase::Sunken { pos: sunk } = *phase {
            let d = from.distance(sunk);
            if best.map_or(true, |(bd, _, _)| d < bd) {
                best = Some((d, rock_e, sunk));
            }
        }
    }
    let (_, rock_e, sunk) = best?;
    if let Ok((_, mut phase, _, _)) = rocks.get_mut(rock_e) {
        *phase = SmallRockPhase::Claimed { by };
    }
    Some((rock_e, sunk))
}

fn update_boats(
    boatmen: Query<(&Boatman, &Pos), Without<Boat>>,
    mut boats: Query<(&Boat, &mut Pos, &mut Visibility), Without<Boatman>>,
) {
    for (b, mut pos, mut vis) in &mut boats {
        let Ok((_, owner_pos)) = boatmen.get(b.owner) else {
            *vis = Visibility::Hidden;
            continue;
        };
        pos.0 = owner_pos.0 + Vec2::new(0.0, 4.0);
        *vis = Visibility::Visible;
    }
}

/// Slave each cargo rock's position to its carrier boatman so the
/// pile rides along on the boat. Stacks rocks vertically so a partial
/// haul reads at a glance.
fn update_cargo_positions(
    boatmen: Query<(&Boatman, &Pos), Without<SmallRock>>,
    mut rocks: Query<(&mut Pos, &mut Visibility, &SmallRockPhase), With<SmallRock>>,
) {
    for (boatman, boatman_pos) in &boatmen {
        // While the boatman is *throwing* the head rock, that rock has
        // already transitioned to `Tossing` — leave it alone and
        // restack the rest behind it.
        for (i, &rock_e) in boatman.cargo.iter().enumerate() {
            let Ok((mut rock_pos, mut vis, phase)) = rocks.get_mut(rock_e) else {
                continue;
            };
            // Only restack rocks still in `Carried` — once a rock has
            // been launched it manages its own position again.
            if !matches!(*phase, SmallRockPhase::Carried { .. }) {
                continue;
            }
            rock_pos.0 = boatman_pos.0
                + Vec2::new(0.0, 2.0 + i as f32 * CARGO_STACK_DY);
            *vis = Visibility::Visible;
        }
    }
}
