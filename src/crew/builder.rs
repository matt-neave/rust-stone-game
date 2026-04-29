//! Builder crabs + construction sites — when a structure is purchased
//! a pair of crabs scuttles out of the cave and a "construction site"
//! at the build location begins to fill in pixel-by-pixel from the
//! ground up. Once the pixels finish stacking the real structure
//! visual + side effects spawn (via [`StructureBuiltEvent`]) and the
//! crabs head home.
//!
//! Decoupling note: the building's `owned` flag is still flipped
//! immediately on `PurchaseEvent` (so the cave panel can't be
//! double-clicked into spawning two huts). What's deferred is the
//! visual + worker/fish bonuses — those wait for `StructureBuiltEvent`.

use bevy::prelude::*;
use rand::Rng;

use crate::core::common::{Layer, Pos};
use crate::core::constants::{
    CAVE_X, CAVE_Y, HUT_BEACHCOMBER_X, HUT_BEACHCOMBER_Y, HUT_BODY_H, HUT_FISHER_X, HUT_FISHER_Y,
    HUT_MINER_X, HUT_MINER_Y, HUT_ROOF_H, HUT_ROOF_W, HUT_SKIMMER_X, HUT_SKIMMER_Y,
    HUT_STONEMASON_X, HUT_STONEMASON_Y, HUT_X, HUT_Y, PIER_H, PIER_W, PIER_X, PIER_Y, PORT_H,
    PORT_W, PORT_X, PORT_Y, Z_CREW, Z_HUT,
};
use crate::core::colors;
use crate::economy::{PurchaseEvent, PurchaseKind, Workers};
use crate::render::shapes::Shapes;
use crate::structures::hut::SpawnWorkerEvent;

use super::{step_walk_frame, tick_walk_animation, CrewWalking};

const BUILDER_SPEED: f32 = 38.0;
/// Total construction time, measured from the moment both builder
/// crabs reach the site. The site is inert until then so the player
/// can see the crabs scuttle out of the cave first.
const CONSTRUCTION_DURATION: f32 = 3.0;
/// Number of builder crabs spawned per construction. They become the
/// hut's two starter workers when the build finishes.
const BUILDERS_PER_SITE: usize = 2;
/// Sandy/orange tint so the builder crabs read as a different caste
/// from the regular crew.
const BUILDER_TINT: Color = Color::srgb(0.85, 0.55, 0.25);

#[derive(Component)]
pub struct BuilderCrab {
    pub state: BuilderState,
    pub flap_accum: f32,
    pub walk_frame: bool,
    /// The construction site this crab is bound to. Once that entity
    /// despawns (build complete) the crab transitions to walking home.
    pub site: Entity,
}

pub enum BuilderState {
    WalkingTo { from: Vec2, to: Vec2, time: f32, dur: f32 },
    /// Idle pose at the build site, waiting for the construction
    /// timer to finish. No internal duration — the crab leaves when
    /// its `site` entity is despawned.
    Building,
    WalkingHome { from: Vec2, to: Vec2, time: f32, dur: f32 },
}

impl CrewWalking for BuilderCrab {
    fn is_walking(&self) -> bool {
        matches!(
            self.state,
            BuilderState::WalkingTo { .. } | BuilderState::WalkingHome { .. }
        )
    }
    fn walk_frame(&self) -> bool {
        self.walk_frame
    }
    fn step_walking(&mut self, walking: bool, dt: f32) -> bool {
        step_walk_frame(walking, &mut self.flap_accum, &mut self.walk_frame, dt)
    }
}

/// A pixel-stacking construction in progress. One per active build.
#[derive(Component)]
pub struct ConstructionSite {
    pub kind: PurchaseKind,
    /// Top-left corner of the footprint (spec coords, Y-down).
    pub top_left: Vec2,
    pub color: Color,
    pub time: f32,
    pub dur: f32,
    /// Pre-shuffled list of pixel offsets within the footprint, sorted
    /// roughly bottom-to-top so the building visually fills from the
    /// ground up. Index `[0..spawned]` are already on screen.
    pub pixels: Vec<Vec2>,
    pub spawned: usize,
}

/// Marker for a single placed construction pixel — child of a
/// `ConstructionSite`. Despawned together with the site when the
/// build finishes.
#[derive(Component)]
pub struct ConstructionPixel {
    pub site: Entity,
}

/// Fired when a construction site finishes filling. Hut/Pier/Port
/// plugins listen to this (instead of `PurchaseEvent`) to spawn the
/// real visual + side effects.
#[derive(Message)]
pub struct StructureBuiltEvent {
    pub kind: PurchaseKind,
}

pub struct BuilderPlugin;

impl Plugin for BuilderPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<StructureBuiltEvent>().add_systems(
            Update,
            (
                spawn_builders_on_purchase,
                tick_construction,
                tick_builders,
                tick_walk_animation::<BuilderCrab>,
            ),
        );
    }
}

struct BuildSpec {
    site: Vec2,
    footprint: Vec2,
    color: Color,
}

fn build_spec_for(kind: PurchaseKind) -> Option<BuildSpec> {
    let hut_w = HUT_ROOF_W;
    let hut_h = HUT_BODY_H + HUT_ROOF_H;
    Some(match kind {
        PurchaseKind::Hut => BuildSpec {
            site: Vec2::new(HUT_X, HUT_Y),
            footprint: Vec2::new(hut_w, hut_h),
            color: colors::HUT_WALL,
        },
        PurchaseKind::HutMiner => BuildSpec {
            site: Vec2::new(HUT_MINER_X, HUT_MINER_Y),
            footprint: Vec2::new(hut_w, hut_h),
            color: colors::MINER_BODY,
        },
        PurchaseKind::HutSkimmer => BuildSpec {
            site: Vec2::new(HUT_SKIMMER_X, HUT_SKIMMER_Y),
            footprint: Vec2::new(hut_w, hut_h),
            color: colors::SKIMMER_BODY,
        },
        PurchaseKind::HutFisher => BuildSpec {
            site: Vec2::new(HUT_FISHER_X, HUT_FISHER_Y),
            footprint: Vec2::new(hut_w, hut_h),
            color: colors::FISHERMAN_BODY,
        },
        PurchaseKind::HutBeachcomber => BuildSpec {
            site: Vec2::new(HUT_BEACHCOMBER_X, HUT_BEACHCOMBER_Y),
            footprint: Vec2::new(hut_w, hut_h),
            color: colors::BEACHCOMBER_BODY,
        },
        PurchaseKind::HutStonemason => BuildSpec {
            site: Vec2::new(HUT_STONEMASON_X, HUT_STONEMASON_Y),
            footprint: Vec2::new(hut_w, hut_h),
            color: colors::STONEMASON_BODY,
        },
        PurchaseKind::Pier => BuildSpec {
            site: Vec2::new(PIER_X, PIER_Y),
            footprint: Vec2::new(PIER_W, PIER_H),
            color: colors::HUT_ROOF,
        },
        PurchaseKind::Port => BuildSpec {
            site: Vec2::new(PORT_X, PORT_Y),
            footprint: Vec2::new(PORT_W, PORT_H),
            color: colors::HUT_ROOF,
        },
        _ => return None,
    })
}

fn cave_door() -> Vec2 {
    Vec2::new(CAVE_X, CAVE_Y + 2.0)
}

/// Build the pixel-fill order for a footprint: every integer cell,
/// sorted bottom-row first (Y-down system → larger y = bottom), with
/// each row internally shuffled so the fill looks organic instead of
/// a sweeping line.
fn build_pixel_order(footprint: Vec2) -> Vec<Vec2> {
    let w = footprint.x.round() as i32;
    let h = footprint.y.round() as i32;
    let mut rng = rand::thread_rng();
    let mut out: Vec<Vec2> = Vec::with_capacity((w * h) as usize);
    // Y-down: iterate from bottom row up so earlier indices = lower pixels.
    for y in (0..h).rev() {
        let row_start = out.len();
        for x in 0..w {
            out.push(Vec2::new(x as f32 + 0.5, y as f32 + 0.5));
        }
        // Shuffle just this row.
        let row_end = out.len();
        for i in (row_start + 1..row_end).rev() {
            let j = rng.gen_range(row_start..=i);
            out.swap(i, j);
        }
    }
    out
}

fn spawn_builders_on_purchase(
    mut commands: Commands,
    mut events: MessageReader<PurchaseEvent>,
    shapes: Res<Shapes>,
) {
    let mut rng = rand::thread_rng();
    for ev in events.read() {
        let Some(spec) = build_spec_for(ev.kind) else { continue; };
        let top_left = spec.site - spec.footprint * 0.5;
        let pixels = build_pixel_order(spec.footprint);
        let site_entity = commands
            .spawn(ConstructionSite {
                kind: ev.kind,
                top_left,
                color: spec.color,
                time: 0.0,
                dur: CONSTRUCTION_DURATION,
                pixels,
                spawned: 0,
            })
            .id();

        let from = cave_door();
        for i in 0..BUILDERS_PER_SITE {
            let dx = if i == 0 { -4.0 } else { 4.0 };
            let dy = rng.gen_range(-2.0..2.0);
            let to = Vec2::new(spec.site.x + dx, spec.site.y + dy);
            let dist = from.distance(to).max(1.0);
            let dur = (dist / BUILDER_SPEED).clamp(0.5, 4.0);
            commands.spawn((
                BuilderCrab {
                    state: BuilderState::WalkingTo {
                        from,
                        to,
                        time: 0.0,
                        dur,
                    },
                    flap_accum: rng.gen_range(0.0..0.18),
                    walk_frame: false,
                    site: site_entity,
                },
                Pos(from),
                Layer(Z_CREW),
                Sprite {
                    image: shapes.humanoid.clone(),
                    color: BUILDER_TINT,
                    custom_size: Some(Vec2::new(16.0, 9.0)),
                    ..default()
                },
                Transform::default(),
            ));
        }
    }
}

fn tick_construction(
    time: Res<Time>,
    mut commands: Commands,
    mut sites: Query<(Entity, &mut ConstructionSite)>,
    pixels: Query<(Entity, &ConstructionPixel)>,
    crabs: Query<(Entity, &BuilderCrab, &Pos)>,
    mut workers: ResMut<Workers>,
    mut spawn_worker: MessageWriter<SpawnWorkerEvent>,
    mut built: MessageWriter<StructureBuiltEvent>,
) {
    let dt = time.delta_secs();
    for (site_e, mut site) in &mut sites {
        // Wait until both crabs have reached the site before any
        // pixels stack — the player should see the crabs arrive
        // first.
        let arrived = crabs
            .iter()
            .filter(|(_, c, _)| c.site == site_e && matches!(c.state, BuilderState::Building))
            .count();
        if arrived < BUILDERS_PER_SITE {
            continue;
        }
        site.time += dt;
        let progress = (site.time / site.dur).clamp(0.0, 1.0);
        let target = (progress * site.pixels.len() as f32).round() as usize;
        // Spawn any newly revealed pixels this frame.
        while site.spawned < target.min(site.pixels.len()) {
            let p = site.pixels[site.spawned];
            let world = site.top_left + p;
            let color = site.color;
            commands.spawn((
                ConstructionPixel { site: site_e },
                Pos(world),
                Layer(Z_HUT - 0.1),
                Sprite::from_color(color, Vec2::new(1.0, 1.0)),
                Transform::default(),
            ));
            site.spawned += 1;
        }
        if site.time >= site.dur {
            built.write(StructureBuiltEvent { kind: site.kind });
            // For huts: the two builder crabs become the hut's two
            // starter workers — despawn each crab and emit a
            // SpawnWorkerEvent at its current position. For pier /
            // port (no starter workers), leave the crabs alone;
            // tick_builders will notice the site vanish and walk
            // them home.
            if is_hut_kind(site.kind) {
                for (crab_e, crab, pos) in &crabs {
                    if crab.site != site_e {
                        continue;
                    }
                    spawn_worker.write(SpawnWorkerEvent { pos: pos.0 });
                    workers.count += 1;
                    commands.entity(crab_e).despawn();
                }
            }
            for (px_e, px) in &pixels {
                if px.site == site_e {
                    commands.entity(px_e).despawn();
                }
            }
            commands.entity(site_e).despawn();
        }
    }
}

fn is_hut_kind(k: PurchaseKind) -> bool {
    matches!(
        k,
        PurchaseKind::Hut
            | PurchaseKind::HutMiner
            | PurchaseKind::HutSkimmer
            | PurchaseKind::HutFisher
            | PurchaseKind::HutBeachcomber
            | PurchaseKind::HutStonemason
    )
}

fn tick_builders(
    time: Res<Time>,
    mut commands: Commands,
    sites: Query<(), With<ConstructionSite>>,
    mut q: Query<(Entity, &mut BuilderCrab, &mut Pos)>,
) {
    let dt = time.delta_secs();
    for (e, mut crab, mut pos) in &mut q {
        match &mut crab.state {
            BuilderState::WalkingTo { from, to, time: t, dur } => {
                *t += dt;
                let prog = (*t / *dur).clamp(0.0, 1.0);
                pos.0 = from.lerp(*to, prog);
                if *t >= *dur {
                    pos.0 = *to;
                    crab.state = BuilderState::Building;
                }
            }
            BuilderState::Building => {
                // The site entity is despawned by `tick_construction`
                // when the build finishes — that's the crab's "go
                // home" signal. Until then, just stand at the site.
                if sites.get(crab.site).is_err() {
                    let from = pos.0;
                    let to = cave_door();
                    let dist = from.distance(to).max(1.0);
                    let walk_dur = (dist / BUILDER_SPEED).clamp(0.5, 4.0);
                    crab.state = BuilderState::WalkingHome {
                        from,
                        to,
                        time: 0.0,
                        dur: walk_dur,
                    };
                }
            }
            BuilderState::WalkingHome { from, to, time: t, dur } => {
                *t += dt;
                let prog = (*t / *dur).clamp(0.0, 1.0);
                pos.0 = from.lerp(*to, prog);
                if *t >= *dur {
                    commands.entity(e).despawn();
                }
            }
        }
    }
}

