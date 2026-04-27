//! Small rocks — the player's actual unit of currency-generation. A small
//! rock has four lifecycle phases:
//!
//! 1. **Falling** — just spawned by the big rock; arcs from the big rock's
//!    position to a target sand spot. On landing transitions to Idle.
//! 2. **Idle** — sitting on sand, clickable. On click transitions to Tossing.
//! 3. **Tossing** — a single big parabolic throw from the rock's sand
//!    position to a landing point in the water. The toss flies cleanly over
//!    any sand between the rock and the shoreline (so a far-away rock
//!    doesn't appear to skip across sand). On landing, the first dice roll
//!    happens — bounce or sink.
//! 4. **Skimming** — only entered after a successful first bounce off the
//!    water. The rock continues rightward with periodic bounce checks,
//!    each 50/50 to continue (award +1 skim) or sink (despawn).
//!
//! The arc between skim bounces is cosmetic: a parabolic Y oscillation
//! between the water "skim line" and a peak height that decays each bounce.

use bevy::prelude::*;
use rand::Rng;

use crate::audio::{PlaySoundEvent, SoundKind};
use crate::core::colors;
use crate::core::common::{Layer, Pos, ZHeight};
use crate::core::constants::*;
use crate::crew::skimmer::effective_bounce_chance;
use crate::currency::Skims;
use crate::economy::SkimUpgrades;
use crate::effects::floating_text::SpawnFloatingTextEvent;
use crate::core::input::ClickEvent;
use crate::effects::particles::SpawnParticleBurstEvent;
use crate::effects::ripple::SpawnRippleEvent;
use crate::rocks::imprint::SpawnImprintEvent;
use crate::rocks::shadow::spawn_rock_shadow;
use crate::render::shapes::{Shapes, SmallRockShape};
use crate::render::{RockLitMaterial, RockLitParams, RockQuad};
use crate::structures::pier::Fish;

#[derive(Component)]
pub struct SmallRock;

/// The shape variant a small rock was spawned with — tracked so its
/// sand imprint, when tossed, can match the rock's silhouette.
#[derive(Component, Clone, Copy)]
pub struct RockShape(pub SmallRockShape);

#[derive(Component)]
pub enum SmallRockPhase {
    Falling {
        from: Vec2,
        to: Vec2,
        time: f32,
        duration: f32,
    },
    Idle,
    /// Reserved by a crew member who's walking over to pick it up.
    /// Visually identical to `Idle` (no movement), but other systems
    /// can't re-claim it. The `by` field is a future-proofing hook
    /// for returning abandoned claims to `Idle`.
    #[allow(dead_code)]
    Claimed {
        by: Entity,
    },
    /// Held by another entity (skimmer carrying it overhead). The
    /// rock's `Pos` follows `carrier.Pos + offset` each frame.
    Carried {
        carrier: Entity,
        offset: Vec2,
    },
    Tossing {
        from: Vec2,
        to: Vec2,
        time: f32,
        duration: f32,
        skim_speed: f32,
    },
    Skimming {
        speed: f32,
        base_y: f32,
        bounce_index: u32,
        time_since_bounce: f32,
        bounce_interval: f32,
        arc_height: f32,
    },
}

/// Per-bounce probability that a skimming rock bounces (vs sinks)
/// when it lands on water. Player-thrown stones default to
/// `PLAYER_BOUNCE_CHANCE`; skimmers attach the lower
/// `SKIMMER_BOUNCE_CHANCE`. Stored on the rock entity so the toss/
/// skim systems don't need to know who threw it.
#[derive(Component, Clone, Copy)]
pub struct BounceChance(pub f32);

/// Marker — present on a rock that has already been rescued by a
/// fish at least once. Each rock can only be fish-skimmed once.
#[derive(Component, Clone, Copy)]
pub struct FishSkimmed;

/// Build a fresh `Tossing` phase from a launch position, picking the
/// water-edge target + flight duration + skim speed the same way
/// player clicks do. Public so other crew systems (skimmer) can
/// trigger a toss without duplicating the physics tuning.
pub fn make_toss_phase(from: Vec2, rng: &mut impl Rng) -> SmallRockPhase {
    let toss_to = Vec2::new(
        SHORELINE_X + rng.gen_range(20.0..55.0),
        (from.y + rng.gen_range(-20.0..20.0)).clamp(80.0, 230.0),
    );
    let dist = (toss_to - from).length();
    let duration = (dist / 220.0).clamp(0.35, 0.9);
    let skim_speed = SKIM_SPEED * rng.gen_range(0.95..1.10);
    SmallRockPhase::Tossing {
        from,
        to: toss_to,
        time: 0.0,
        duration,
        skim_speed,
    }
}

/// Default falling-arc duration for rocks chipped off the big rock —
/// short and snappy.
pub const SMALL_ROCK_FALL_DURATION_FAST: f32 = 0.55;
/// Slower arc used when something pulls a rock out of the water (e.g.
/// a fisherman) — reads as a heavier, weightier catch.
pub const SMALL_ROCK_FALL_DURATION_SLOW: f32 = 1.6;

#[derive(Message)]
pub struct SpawnSmallRockEvent {
    pub from: Vec2,
    pub to: Vec2,
    /// How long the falling arc takes, in seconds. Use the
    /// `SMALL_ROCK_FALL_DURATION_*` constants unless you need a
    /// custom feel.
    pub duration: f32,
}

pub struct SmallRockPlugin;

impl Plugin for SmallRockPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SpawnSmallRockEvent>().add_systems(
            Update,
            (
                spawn_small_rocks,
                tick_falling,
                handle_clicks,
                tick_carried,
                tick_tossing,
                tick_skimming,
            )
                .chain(),
        );
    }
}

fn spawn_small_rocks(
    mut commands: Commands,
    mut events: MessageReader<SpawnSmallRockEvent>,
    shapes: Res<Shapes>,
    rock_quad: Res<RockQuad>,
    mut materials: ResMut<Assets<RockLitMaterial>>,
) {
    let mut rng = rand::thread_rng();
    for ev in events.read() {
        // Shape is the only per-rock variation now — banding is
        // computed live by the rock shader (`shaders/rock_lit.wgsl`)
        // from a fixed top-right light direction.
        let shape = SmallRockShape::ALL[rng.gen_range(0..SmallRockShape::ALL.len())];
        let size = shape.size();
        let material = materials.add(RockLitMaterial {
            silhouette: shapes.small_rock_image(shape),
            params: RockLitParams::default(),
        });
        let rock_e = commands
            .spawn((
                SmallRock,
                RockShape(shape),
                SmallRockPhase::Falling {
                    from: ev.from,
                    to: ev.to,
                    time: 0.0,
                    duration: ev.duration.max(0.1),
                },
                Pos(ev.from),
                ZHeight(0.0),
                Layer(Z_ROCK),
                Mesh2d(rock_quad.0.clone()),
                MeshMaterial2d(material),
                Transform::from_scale(Vec3::new(size.x, size.y, 1.0)),
            ))
            .id();
        spawn_rock_shadow(&mut commands, &shapes, rock_e, ev.from, size);
    }
}

/// While a rock is in the `Carried` phase, slave its position to the
/// carrier's. `Without<SmallRockPhase>` keeps the carrier query
/// disjoint from the mutable rock query so Bevy's borrow checker is
/// happy — only crew entities (Skimmer, etc.) have `Pos` without a
/// rock phase. The carry offset is split: x flows into the rock's
/// ground `Pos`, the upward y component goes into `ZHeight` so the
/// shadow stays at the carrier's feet.
fn tick_carried(
    mut rocks: Query<(&SmallRockPhase, &mut Pos, &mut ZHeight)>,
    carriers: Query<&Pos, Without<SmallRockPhase>>,
) {
    for (phase, mut rock_pos, mut zh) in &mut rocks {
        let SmallRockPhase::Carried { carrier, offset } = *phase else {
            continue;
        };
        if let Ok(carrier_pos) = carriers.get(carrier) {
            rock_pos.0 = Vec2::new(carrier_pos.0.x + offset.x, carrier_pos.0.y);
            // offset.y is in spec coords (negative = up), convert to
            // positive-up z-height.
            zh.0 = -offset.y;
        }
    }
}

fn tick_falling(
    time: Res<Time>,
    mut q: Query<(&mut SmallRockPhase, &mut Pos, &mut ZHeight, &mut Transform)>,
    mut burst: MessageWriter<SpawnParticleBurstEvent>,
    mut sound: MessageWriter<PlaySoundEvent>,
) {
    let dt = time.delta_secs();
    for (mut phase, mut pos, mut zh, mut tf) in &mut q {
        let SmallRockPhase::Falling { from, to, time: cur_time, duration } = *phase else {
            continue;
        };
        let new_time = cur_time + dt;
        let t = (new_time / duration).clamp(0.0, 1.0);
        // Linear x/y lerp gives the *ground* position; the air arc lives
        // in `ZHeight` so the paired shadow can track the ground while
        // the rock visually flies above it. The apex scales with the
        // fall duration so a slow arc (a fisherman pulling a rock out
        // of the water) reads visibly higher than a fast pop off the
        // big rock.
        let lerp_pos = from.lerp(to, t);
        let apex = (8.0 * (duration / 0.55)).clamp(8.0, 36.0);
        let height = apex * 4.0 * t * (1.0 - t); // positive = up
        pos.0 = lerp_pos;
        zh.0 = height;
        tf.rotation = Quat::from_rotation_z(new_time * 6.0);

        if t >= 1.0 {
            pos.0 = to;
            zh.0 = 0.0;
            tf.rotation = Quat::IDENTITY;
            *phase = SmallRockPhase::Idle;
            burst.write(SpawnParticleBurstEvent {
                pos: to,
                color: colors::SAND_DARK,
                count: 6,
                angle_min: -std::f32::consts::PI,
                angle_max: 0.0,
                speed_min: 30.0,
                speed_max: 80.0,
                size_min: 1.0,
                size_max: 2.0,
                lifetime_min: 0.22,
                lifetime_max: 0.45,
                damping: 1.8,
                gravity: 200.0,
            });
            sound.write(PlaySoundEvent {
                kind: SoundKind::Land,
                pitch: 1.0,
                volume: 0.32,
            });
        } else {
            *phase = SmallRockPhase::Falling {
                from,
                to,
                time: new_time,
                duration,
            };
        }
    }
}

fn handle_clicks(
    mut commands: Commands,
    mut events: MessageReader<ClickEvent>,
    mut q: Query<(Entity, &mut SmallRockPhase, &Pos, &RockShape)>,
    upgrades: Res<SkimUpgrades>,
    mut sound: MessageWriter<PlaySoundEvent>,
    mut imprint: MessageWriter<SpawnImprintEvent>,
) {
    let mut rng = rand::thread_rng();
    for ev in events.read() {
        // Find the closest sand-bound rock within click radius. Players
        // can yank rocks that a skimmer has already reserved (Claimed)
        // — the skimmer's state machine notices on its next tick and
        // falls back to Searching. Carried rocks (already in a
        // skimmer's hands) are off-limits.
        let mut best: Option<(f32, Entity)> = None;
        for (e, phase, pos, _) in &q {
            if !matches!(*phase, SmallRockPhase::Idle | SmallRockPhase::Claimed { .. }) {
                continue;
            }
            let d = pos.0.distance(ev.pos);
            if d <= SMALL_ROCK_CLICK_R && best.map_or(true, |(bd, _)| d < bd) {
                best = Some((d, e));
            }
        }
        let Some((_, target)) = best else { continue };
        let Ok((_, mut phase, pos, shape)) = q.get_mut(target) else { continue };

        // Toss target — a point in the water just past the shoreline. Y is
        // similar to the rock's current Y so the throw reads as a sideways
        // toss, not a wild lob. The far-from-shore case (rock spawned near
        // the big rock) gets a longer flight, the near-shore case a shorter
        // one — duration is distance-based so apparent toss speed is
        // roughly constant regardless of starting position.
        let toss_to = Vec2::new(
            SHORELINE_X + rng.gen_range(20.0..55.0),
            (pos.0.y + rng.gen_range(-20.0..20.0)).clamp(80.0, 230.0),
        );
        let dist = (toss_to - pos.0).length();
        // 220 px/s feels right — fast enough to be snappy, slow enough that
        // the arc is readable.
        let duration = (dist / 220.0).clamp(0.35, 0.9);
        let skim_speed = SKIM_SPEED * rng.gen_range(0.95..1.10);

        // Leave a dark imprint where the rock used to sit. Persists ~15s.
        imprint.write(SpawnImprintEvent {
            pos: pos.0,
            shape: shape.0,
        });

        *phase = SmallRockPhase::Tossing {
            from: pos.0,
            to: toss_to,
            time: 0.0,
            duration,
            skim_speed,
        };
        let chance = effective_bounce_chance(PLAYER_BOUNCE_CHANCE, &upgrades);
        commands.entity(target).insert(BounceChance(chance));
        sound.write(PlaySoundEvent {
            kind: SoundKind::Click,
            pitch: 1.15,
            volume: 0.30,
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn tick_tossing(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<
        (
            Entity,
            &mut SmallRockPhase,
            &mut Pos,
            &mut ZHeight,
            &mut Transform,
            Option<&BounceChance>,
            Option<&FishSkimmed>,
        ),
        With<SmallRock>,
    >,
    fish_q: Query<(Entity, &Pos), (With<Fish>, Without<SmallRock>)>,
    mut fishes: ResMut<crate::economy::Fishes>,
    mut skims: ResMut<Skims>,
    mut burst: MessageWriter<SpawnParticleBurstEvent>,
    mut ripple: MessageWriter<SpawnRippleEvent>,
    mut floating: MessageWriter<SpawnFloatingTextEvent>,
    mut sound: MessageWriter<PlaySoundEvent>,
) {
    let dt = time.delta_secs();
    let mut rng = rand::thread_rng();
    for (e, mut phase, mut pos, mut zh, mut tf, bounce, fish_skimmed) in &mut q {
        let bounce_chance = bounce.map(|b| b.0).unwrap_or(PLAYER_BOUNCE_CHANCE);
        let SmallRockPhase::Tossing {
            from,
            to,
            time: cur_time,
            duration,
            skim_speed,
        } = *phase
        else {
            continue;
        };
        let new_time = cur_time + dt;
        let t = (new_time / duration).clamp(0.0, 1.0);

        // Ground position lerps from launch to landing; the tall toss
        // arc lives in `ZHeight` so the rock's shadow tracks the ground
        // while the rock visibly flies high above it. Apex height
        // scales with horizontal distance so short tosses don't lob
        // ridiculously high.
        let lerp_pos = from.lerp(to, t);
        let dx = (to.x - from.x).abs().max(20.0);
        let apex = (dx * 0.35).clamp(28.0, 60.0);
        let height = apex * 4.0 * t * (1.0 - t);
        pos.0 = lerp_pos;
        zh.0 = height;
        tf.rotation = Quat::from_rotation_z(new_time * 8.0);

        if t < 1.0 {
            *phase = SmallRockPhase::Tossing {
                from,
                to,
                time: new_time,
                duration,
                skim_speed,
            };
            continue;
        }

        // Landed at the toss target. First dice roll happens here.
        pos.0 = to;
        zh.0 = 0.0;
        tf.rotation = Quat::IDENTITY;
        let mut sink: bool = rng.gen_bool((1.0 - bounce_chance).clamp(0.0, 1.0) as f64);
        // Fish rescue — only on the first sink, never repeats. The
        // rescuing fish is consumed (despawned) since fish are now
        // one-shot bucket purchases.
        if sink && fish_skimmed.is_none() {
            if let Some(fish_e) = nearest_fish_within(&fish_q, to, FISH_ASSIST_RADIUS) {
                sink = false;
                commands.entity(e).insert(FishSkimmed);
                commands.entity(fish_e).despawn();
                fishes.count = fishes.count.saturating_sub(1);
                spawn_fish_assist_fx(
                    to,
                    &mut burst,
                    &mut floating,
                    &mut sound,
                );
            }
        }
        if sink {
            ripple.write(SpawnRippleEvent { pos: to, big: true });
            burst.write(SpawnParticleBurstEvent {
                pos: to,
                color: colors::FOAM,
                count: 8,
                angle_min: -std::f32::consts::PI,
                angle_max: 0.0,
                speed_min: 60.0,
                speed_max: 130.0,
                size_min: 1.0,
                size_max: 2.0,
                lifetime_min: 0.3,
                lifetime_max: 0.55,
                damping: 1.0,
                gravity: 240.0,
            });
            sound.write(PlaySoundEvent {
                kind: SoundKind::Splash,
                pitch: 1.0,
                volume: 0.45,
            });
            commands.entity(e).despawn();
            continue;
        }

        // First bounce — award +1, ripple, transition to Skimming. From
        // here, normal mid-water skim physics take over.
        skims.total = skims.total.saturating_add(1);
        ripple.write(SpawnRippleEvent { pos: to, big: false });
        burst.write(SpawnParticleBurstEvent {
            pos: to,
            color: colors::FOAM,
            count: 4,
            angle_min: -std::f32::consts::PI * 0.8,
            angle_max: -std::f32::consts::PI * 0.2,
            speed_min: 40.0,
            speed_max: 90.0,
            size_min: 1.0,
            size_max: 1.5,
            lifetime_min: 0.18,
            lifetime_max: 0.32,
            damping: 1.6,
            gravity: 220.0,
        });
        floating.write(SpawnFloatingTextEvent::reward(
            Vec2::new(to.x, to.y - 6.0),
            "+1",
            colors::YELLOW,
        ));
        sound.write(PlaySoundEvent {
            kind: SoundKind::SkimBounce,
            pitch: 1.0,
            volume: 0.32,
        });
        sound.write(PlaySoundEvent {
            kind: SoundKind::Reward,
            pitch: 1.0,
            volume: 0.25,
        });
        *phase = SmallRockPhase::Skimming {
            speed: skim_speed,
            base_y: to.y,
            bounce_index: 1,
            time_since_bounce: 0.0,
            bounce_interval: SKIM_BOUNCE_INTERVAL,
            arc_height: SKIM_ARC_HEIGHT * SKIM_ARC_DECAY,
        };
    }
}

#[allow(clippy::too_many_arguments)]
fn tick_skimming(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<
        (
            Entity,
            &mut SmallRockPhase,
            &mut Pos,
            &mut ZHeight,
            &mut Transform,
            Option<&BounceChance>,
            Option<&FishSkimmed>,
        ),
        With<SmallRock>,
    >,
    fish_q: Query<(Entity, &Pos), (With<Fish>, Without<SmallRock>)>,
    mut fishes: ResMut<crate::economy::Fishes>,
    mut skims: ResMut<Skims>,
    mut burst: MessageWriter<SpawnParticleBurstEvent>,
    mut ripple: MessageWriter<SpawnRippleEvent>,
    mut floating: MessageWriter<SpawnFloatingTextEvent>,
    mut sound: MessageWriter<PlaySoundEvent>,
) {
    let dt = time.delta_secs();
    let mut rng = rand::thread_rng();
    for (e, mut phase, mut pos, mut zh, mut tf, bounce, fish_skimmed) in &mut q {
        let bounce_chance = bounce.map(|b| b.0).unwrap_or(PLAYER_BOUNCE_CHANCE);
        let SmallRockPhase::Skimming {
            speed,
            base_y,
            bounce_index,
            ref mut time_since_bounce,
            bounce_interval,
            arc_height,
        } = *phase
        else {
            continue;
        };

        // Advance horizontally on the water surface; the bounce arc
        // lives in `ZHeight` so `Pos` always sits on `base_y`. (The
        // skim shadow is hidden anyway — the rock is on water — but
        // keeping ground/arc separate matches the rest of the rocks.)
        pos.0.x += speed * dt;
        pos.0.y = base_y;
        // Arc shape: parabola peaking at bounce_interval/2, returning to
        // base_y at the next bounce point. Height oscillates as
        //   z = arc_height * 4 * t * (1 - t)
        // where t goes 0 → 1 across one bounce_interval.
        *time_since_bounce += dt;
        let t = (*time_since_bounce / bounce_interval).clamp(0.0, 1.0);
        zh.0 = arc_height * 4.0 * t * (1.0 - t);
        tf.rotation = Quat::from_rotation_z(*time_since_bounce * 12.0 + bounce_index as f32);

        // If the rock leaves the right edge of the screen without sinking,
        // count it as a final bounce and despawn.
        if pos.0.x > INTERNAL_WIDTH - 6.0 {
            commands.entity(e).despawn();
            continue;
        }

        // If we've completed a full arc, resolve a bounce.
        if *time_since_bounce >= bounce_interval {
            *time_since_bounce = 0.0;
            let bounce_pos = Vec2::new(pos.0.x, base_y);
            // Bounce vs sink — chance is per-thrower (player vs skimmer).
            let mut sink: bool = rng.gen_bool((1.0 - bounce_chance).clamp(0.0, 1.0) as f64);
            // Fish rescue — first failed bounce only, then never again.
            // The rescuing fish is consumed.
            if sink && fish_skimmed.is_none() {
                if let Some(fish_e) = nearest_fish_within(&fish_q, bounce_pos, FISH_ASSIST_RADIUS) {
                    sink = false;
                    commands.entity(e).insert(FishSkimmed);
                    commands.entity(fish_e).despawn();
                    fishes.count = fishes.count.saturating_sub(1);
                    spawn_fish_assist_fx(
                        bounce_pos,
                        &mut burst,
                        &mut floating,
                        &mut sound,
                    );
                }
            }
            if sink {
                ripple.write(SpawnRippleEvent {
                    pos: bounce_pos,
                    big: true,
                });
                burst.write(SpawnParticleBurstEvent {
                    pos: bounce_pos,
                    color: colors::FOAM,
                    count: 8,
                    angle_min: -std::f32::consts::PI,
                    angle_max: 0.0, // upward fan
                    speed_min: 60.0,
                    speed_max: 130.0,
                    size_min: 1.0,
                    size_max: 2.0,
                    lifetime_min: 0.3,
                    lifetime_max: 0.55,
                    damping: 1.0,
                    gravity: 240.0,
                });
                sound.write(PlaySoundEvent {
                    kind: SoundKind::Splash,
                    pitch: 1.0,
                    volume: 0.45,
                });
                commands.entity(e).despawn();
                continue;
            }

            // Bounce — award +1, ripple, sound.
            skims.total = skims.total.saturating_add(1);
            ripple.write(SpawnRippleEvent {
                pos: bounce_pos,
                big: false,
            });
            burst.write(SpawnParticleBurstEvent {
                pos: bounce_pos,
                color: colors::FOAM,
                count: 4,
                angle_min: -std::f32::consts::PI * 0.8,
                angle_max: -std::f32::consts::PI * 0.2,
                speed_min: 40.0,
                speed_max: 90.0,
                size_min: 1.0,
                size_max: 1.5,
                lifetime_min: 0.18,
                lifetime_max: 0.32,
                damping: 1.6,
                gravity: 220.0,
            });
            floating.write(SpawnFloatingTextEvent::reward(
                Vec2::new(bounce_pos.x, bounce_pos.y - 6.0),
                "+1",
                colors::YELLOW,
            ));
            sound.write(PlaySoundEvent {
                kind: SoundKind::SkimBounce,
                pitch: 1.0 + bounce_index as f32 * 0.04,
                volume: 0.32,
            });
            sound.write(PlaySoundEvent {
                kind: SoundKind::Reward,
                pitch: 1.0 + bounce_index as f32 * 0.05,
                volume: 0.25,
            });

            // Continue skimming with decayed arc height.
            *phase = SmallRockPhase::Skimming {
                speed,
                base_y,
                bounce_index: bounce_index + 1,
                time_since_bounce: 0.0,
                bounce_interval,
                arc_height: arc_height * SKIM_ARC_DECAY,
            };
        }
    }
}


// ---------------------------------------------------------------------------
// Fish assist
// ---------------------------------------------------------------------------

/// Find the closest fish within `radius` of `point`. Returns the
/// fish entity so the caller can despawn it on a rescue (fish are
/// one-shot — the bucket-of-fish purchase stocks ten of them).
fn nearest_fish_within(
    fish_q: &Query<(Entity, &Pos), (With<Fish>, Without<SmallRock>)>,
    point: Vec2,
    radius: f32,
) -> Option<Entity> {
    let r2 = radius * radius;
    let mut best: Option<(f32, Entity)> = None;
    for (e, p) in fish_q {
        let d2 = (p.0 - point).length_squared();
        if d2 <= r2 && best.map_or(true, |(bd, _)| d2 < bd) {
            best = Some((d2, e));
        }
    }
    best.map(|(_, e)| e)
}

/// Visual + audio for a fish save: a foam burst, a labelled "+fish!"
/// floater in the worker palette, and a soft chime.
fn spawn_fish_assist_fx(
    pos: Vec2,
    burst: &mut MessageWriter<SpawnParticleBurstEvent>,
    floating: &mut MessageWriter<SpawnFloatingTextEvent>,
    sound: &mut MessageWriter<PlaySoundEvent>,
) {
    burst.write(SpawnParticleBurstEvent {
        pos,
        color: colors::FISHERMAN_BODY,
        count: 6,
        angle_min: -std::f32::consts::PI,
        angle_max: 0.0,
        speed_min: 50.0,
        speed_max: 110.0,
        size_min: 1.0,
        size_max: 1.5,
        lifetime_min: 0.18,
        lifetime_max: 0.32,
        damping: 1.4,
        gravity: 200.0,
    });
    floating.write(SpawnFloatingTextEvent {
        pos: Vec2::new(pos.x, pos.y - 12.0),
        text: "fish!".into(),
        color: colors::FISHERMAN_BODY,
        size: 7.0,
        duration: 1.0,
        vy: -18.0,
        shake: 0.6,
    });
    sound.write(PlaySoundEvent {
        kind: SoundKind::SkimBounce,
        pitch: 1.25,
        volume: 0.32,
    });
}
