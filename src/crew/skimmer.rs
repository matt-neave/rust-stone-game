//! Stone-skimming specialists.
//!
//! Cycle (~15 seconds):
//!
//! 1. `WalkingFromHut` — initial walk to a spawn-spread spot along
//!    the water edge.
//! 2. `Searching` — scan for the closest idle small rock within
//!    [`SKIMMER_SEARCH_RADIUS`]. On find, claim it (rock phase →
//!    [`SmallRockPhase::Claimed`]) so other skimmers don't race.
//! 3. `WalkingToRock` — walk to the claim. If the player or another
//!    crew member yanks the rock mid-walk, fall back to `Searching`.
//! 4. `PickingUp` — grab animation. On completion, double-check the
//!    claim and switch the rock to `Carried`. Otherwise fall back.
//! 5. `WalkingToWater` — walk back to the *nearest* water edge at the
//!    skimmer's current Y (no fixed throw spot).
//! 6. `ChargingUp` — wind-up. Then toss with a 25% bounce chance.
//! 7. `Resting` → loop to step 2.

use bevy::prelude::*;

use crate::audio::{PlaySoundEvent, SoundKind};
use crate::core::common::{Layer, Pos};
use crate::core::constants::{
    BOUNCE_CHANCE_MAX, INTERNAL_HEIGHT, SHORELINE_X, SKIMMER_BOUNCE_CHANCE, SKIMMER_CHARGE_TIME,
    SKIMMER_PICKUP_TIME, SKIMMER_REST, SKIMMER_SEARCH_RETRY, SKIMMER_WALK_SPEED, SKIM_UPGRADE_DELTA,
    Z_CREW,
};
use crate::economy::{PurchaseEvent, PurchaseKind, SkimUpgrades, Skimmers};
use crate::render::shapes::Shapes;
use crate::rocks::small::{make_toss_phase, BounceChance, SmallRock, SmallRockPhase};

use super::{step_walk_frame, tick_walk_animation, CrewWalking, SpawnConversionEvent};

/// Initial spawn destinations spread along the water edge so a
/// crew of skimmers doesn't all converge on one point. After the
/// first walk, throw positions are computed dynamically per cycle
/// (see [`nearest_water_edge`]).
const SKIMMER_SPAWN_SPOTS: &[(f32, f32)] = &[
    (188.0, 90.0),
    (188.0, 130.0),
    (188.0, 230.0),
    (188.0, 250.0),
    (170.0, 110.0),
    (170.0, 250.0),
];

/// Where the carried rock sits relative to the skimmer's `Pos` —
/// just above the sprite's head.
const SKIMMER_CARRY_OFFSET: Vec2 = Vec2::new(0.0, -8.0);

/// Maximum distance a skimmer is willing to walk to pick up a rock.
const SKIMMER_SEARCH_RADIUS: f32 = 220.0;

#[derive(Component)]
pub struct Skimmer {
    pub state: SkimmerState,
    /// Currently held / claimed rock.
    pub rock: Option<Entity>,
    pub flap_accum: f32,
    pub walk_frame: bool,
}

pub enum SkimmerState {
    WalkingFromHut { from: Vec2, to: Vec2, time: f32, dur: f32 },
    Searching { retry: f32 },
    WalkingToRock { from: Vec2, to: Vec2, time: f32, dur: f32 },
    PickingUp { time: f32, dur: f32 },
    WalkingToWater { from: Vec2, to: Vec2, time: f32, dur: f32 },
    ChargingUp { time: f32, dur: f32 },
    Resting { time: f32, dur: f32 },
}

impl CrewWalking for Skimmer {
    fn is_walking(&self) -> bool {
        matches!(
            self.state,
            SkimmerState::WalkingFromHut { .. }
                | SkimmerState::WalkingToRock { .. }
                | SkimmerState::WalkingToWater { .. }
        )
    }
    fn walk_frame(&self) -> bool {
        self.walk_frame
    }
    fn step_walking(&mut self, walking: bool, dt: f32) -> bool {
        step_walk_frame(walking, &mut self.flap_accum, &mut self.walk_frame, dt)
    }
}

pub struct SkimmerPlugin;

impl Plugin for SkimmerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                handle_skimmer_spawn,
                handle_skim_upgrade,
                tick_skimmers,
                tick_walk_animation::<Skimmer>,
            ),
        );
    }
}

/// Effective bounce chance for a thrown rock, accounting for `Skim Up`
/// upgrades. Used by both the player click path and skimmer toss.
pub fn effective_bounce_chance(base: f32, upgrades: &SkimUpgrades) -> f32 {
    (base + SKIM_UPGRADE_DELTA * upgrades.level as f32).clamp(0.0, BOUNCE_CHANCE_MAX)
}

/// Bump `SkimUpgrades.level` whenever a `SkimUpgrade` purchase fires.
/// In-flight rocks keep whatever bounce chance was stamped on them at
/// throw time; new throws read the resource fresh.
fn handle_skim_upgrade(
    mut events: MessageReader<PurchaseEvent>,
    mut upgrades: ResMut<SkimUpgrades>,
) {
    for ev in events.read() {
        if ev.kind == PurchaseKind::SkimUpgrade {
            upgrades.level = upgrades.level.saturating_add(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Spawn
// ---------------------------------------------------------------------------

fn handle_skimmer_spawn(
    mut events: MessageReader<SpawnConversionEvent>,
    mut commands: Commands,
    mut skimmers: ResMut<Skimmers>,
    shapes: Res<Shapes>,
) {
    for ev in events.read() {
        if ev.kind != PurchaseKind::Skimmer {
            continue;
        }
        spawn_skimmer(&mut commands, &shapes, &mut skimmers, ev.from_pos);
    }
}

fn spawn_skimmer(
    commands: &mut Commands,
    shapes: &Shapes,
    skimmers: &mut Skimmers,
    worker_pos: Vec2,
) {
    let spot_idx = skimmers.count as usize;
    let (sx, sy) = SKIMMER_SPAWN_SPOTS[spot_idx % SKIMMER_SPAWN_SPOTS.len()];
    let initial_spot = Vec2::new(sx, sy);
    skimmers.count += 1;

    let dist = worker_pos.distance(initial_spot).max(1.0);
    let dur = (dist / SKIMMER_WALK_SPEED).clamp(0.5, 6.0);

    commands.spawn((
        Skimmer {
            state: SkimmerState::WalkingFromHut {
                from: worker_pos,
                to: initial_spot,
                time: 0.0,
                dur,
            },
            rock: None,
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
    ));
}

// ---------------------------------------------------------------------------
// State machine
// ---------------------------------------------------------------------------

/// Nearest water-edge throw point at a given Y. Skimmers walk to the
/// rock, pick it up, then return here at whatever Y they're at — so
/// they don't backtrack across the beach to a fixed throw spot.
fn nearest_water_edge(y: f32) -> Vec2 {
    Vec2::new(SHORELINE_X - 5.0, y.clamp(8.0, INTERNAL_HEIGHT - 8.0))
}

/// True if the given rock is still in `Claimed { by: claimer }`.
/// Used by skimmers to abandon a trip when the player or another
/// crew member yanks the rock out from under them.
fn rock_still_ours(
    rocks: &Query<(Entity, &mut SmallRockPhase, &Pos), (With<SmallRock>, Without<Skimmer>)>,
    rock: Option<Entity>,
    claimer: Entity,
) -> bool {
    let Some(rock_e) = rock else { return false };
    match rocks.get(rock_e) {
        Ok((_, phase, _)) => {
            matches!(*phase, SmallRockPhase::Claimed { by } if by == claimer)
        }
        Err(_) => false,
    }
}

#[allow(clippy::too_many_arguments)]
fn tick_skimmers(
    time: Res<Time>,
    mut commands: Commands,
    mut skimmers: Query<(Entity, &mut Skimmer, &mut Pos), Without<SmallRock>>,
    mut rocks: Query<(Entity, &mut SmallRockPhase, &Pos), (With<SmallRock>, Without<Skimmer>)>,
    upgrades: Res<SkimUpgrades>,
    mut sound: MessageWriter<PlaySoundEvent>,
) {
    let dt = time.delta_secs();
    let mut rng = rand::thread_rng();
    for (skimmer_e, mut skimmer, mut pos) in &mut skimmers {
        let claimed_rock = skimmer.rock;
        let next: Option<SkimmerState> = match &mut skimmer.state {
            SkimmerState::WalkingFromHut { from, to, time: t, dur } => {
                *t += dt;
                let prog = (*t / *dur).clamp(0.0, 1.0);
                pos.0 = from.lerp(*to, prog);
                if *t >= *dur {
                    pos.0 = *to;
                    Some(SkimmerState::Searching { retry: 0.0 })
                } else { None }
            }
            SkimmerState::Searching { retry } => {
                *retry += dt;
                if *retry < SKIMMER_SEARCH_RETRY {
                    None
                } else {
                    *retry = 0.0;
                    let here = pos.0;
                    // Closest idle rock within reach.
                    let mut best: Option<(f32, Entity, Vec2)> = None;
                    for (rock_e, phase, rock_pos) in rocks.iter() {
                        if !matches!(*phase, SmallRockPhase::Idle) { continue; }
                        let d = rock_pos.0.distance(here);
                        if d > SKIMMER_SEARCH_RADIUS { continue; }
                        if best.map_or(true, |(bd, _, _)| d < bd) {
                            best = Some((d, rock_e, rock_pos.0));
                        }
                    }
                    match best {
                        Some((_, rock_e, rock_pos)) => {
                            // Claim the rock so other skimmers don't race for it.
                            if let Ok((_, mut phase, _)) = rocks.get_mut(rock_e) {
                                *phase = SmallRockPhase::Claimed { by: skimmer_e };
                            }
                            skimmer.rock = Some(rock_e);
                            let dist = here.distance(rock_pos).max(1.0);
                            let dur = dist / SKIMMER_WALK_SPEED;
                            Some(SkimmerState::WalkingToRock {
                                from: here,
                                to: rock_pos,
                                time: 0.0,
                                dur,
                            })
                        }
                        None => None,
                    }
                }
            }
            SkimmerState::WalkingToRock { from, to, time: t, dur } => {
                if !rock_still_ours(&rocks, claimed_rock, skimmer_e) {
                    skimmer.rock = None;
                    Some(SkimmerState::Searching { retry: 0.0 })
                } else {
                    *t += dt;
                    let prog = (*t / *dur).clamp(0.0, 1.0);
                    pos.0 = from.lerp(*to, prog);
                    if *t >= *dur {
                        pos.0 = *to;
                        Some(SkimmerState::PickingUp {
                            time: 0.0,
                            dur: SKIMMER_PICKUP_TIME,
                        })
                    } else { None }
                }
            }
            SkimmerState::PickingUp { time: t, dur } => {
                *t += dt;
                if *t >= *dur {
                    // Final claim check + transition to Carried. If the
                    // rock got pulled out from under us, reset.
                    let mut took = false;
                    if let Some(rock_e) = skimmer.rock {
                        if let Ok((_, mut phase, _)) = rocks.get_mut(rock_e) {
                            let ours = matches!(
                                *phase,
                                SmallRockPhase::Claimed { by } if by == skimmer_e,
                            );
                            if ours {
                                *phase = SmallRockPhase::Carried {
                                    carrier: skimmer_e,
                                    offset: SKIMMER_CARRY_OFFSET,
                                };
                                took = true;
                            }
                        }
                    }
                    if !took {
                        skimmer.rock = None;
                        Some(SkimmerState::Searching { retry: 0.0 })
                    } else {
                        // Walk to the nearest water edge from here.
                        let target = nearest_water_edge(pos.0.y);
                        let dist = pos.0.distance(target).max(1.0);
                        let dur = dist / SKIMMER_WALK_SPEED;
                        Some(SkimmerState::WalkingToWater {
                            from: pos.0,
                            to: target,
                            time: 0.0,
                            dur,
                        })
                    }
                } else { None }
            }
            SkimmerState::WalkingToWater { from, to, time: t, dur } => {
                *t += dt;
                let prog = (*t / *dur).clamp(0.0, 1.0);
                pos.0 = from.lerp(*to, prog);
                if *t >= *dur {
                    pos.0 = *to;
                    Some(SkimmerState::ChargingUp {
                        time: 0.0,
                        dur: SKIMMER_CHARGE_TIME,
                    })
                } else { None }
            }
            SkimmerState::ChargingUp { time: t, dur } => {
                *t += dt;
                if *t >= *dur {
                    if let Some(rock_e) = skimmer.rock {
                        if let Ok((_, mut phase, _)) = rocks.get_mut(rock_e) {
                            *phase = make_toss_phase(pos.0, &mut rng);
                        }
                        let chance = effective_bounce_chance(SKIMMER_BOUNCE_CHANCE, &upgrades);
                        commands.entity(rock_e).insert(BounceChance(chance));
                    }
                    skimmer.rock = None;
                    sound.write(PlaySoundEvent {
                        kind: SoundKind::Click,
                        pitch: 0.95,
                        volume: 0.25,
                    });
                    Some(SkimmerState::Resting {
                        time: 0.0,
                        dur: SKIMMER_REST,
                    })
                } else { None }
            }
            SkimmerState::Resting { time: t, dur } => {
                *t += dt;
                if *t >= *dur {
                    Some(SkimmerState::Searching { retry: 0.0 })
                } else { None }
            }
        };
        if let Some(s) = next {
            skimmer.state = s;
        }
    }
}
