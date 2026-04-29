//! Research facility + Aqua Center buildings, the AutoFishing tick,
//! and the Research Mission cinematic.
//!
//! The research mission replaces the old tree-surgeon click upgrade
//! with a small narrative beat: buying the row sends a scout crab
//! west to inspect the standalone tree, walks them back to the edge
//! of the visible canvas, then the camera pans across to "show" the
//! tree to the player before snapping back. Once the cinematic ends,
//! `ResearchMission.unlocked` flips true — at that point the wood
//! HUD readout, the tree itself, and the leftward scroll all light up
//! together.

use bevy::prelude::*;

use crate::audio::{PlaySoundEvent, SoundKind};
use crate::core::colors;
use crate::core::common::{Layer, Pos};
use crate::core::constants::{
    AUTO_FISHING_TARGET, AUTO_FISHING_TICK, HUT_AQUA_X, HUT_AQUA_Y, HUT_BODY_H, HUT_RESEARCH_X,
    HUT_RESEARCH_Y, SCOUT_AT_HUT_DURATION, SCOUT_HOLD_DURATION, SCOUT_PAN_DURATION,
    SCOUT_PRESENT_X, SCOUT_PRESENT_Y, SCOUT_SPEAK_DURATION, SCOUT_WALK_SPEED, SCROLL_FOR_SCOUT,
    SCROLL_FOR_TREE, TREE_STORAGE_X, TREE_STORAGE_Y, TREE_X, TREE_Y, Z_CREW, Z_HUT,
};
use crate::crew::builder::StructureBuiltEvent;
use crate::crew::worker::Worker;
use crate::currency::Skims;
use crate::economy::{
    cost_for, AquaHut, AutoFishing, Fishes, Pier, PurchaseEvent, PurchaseKind, ResearchHut,
    ResearchMission, TreeStorage, Workers,
};
use crate::render::shapes::Shapes;
use crate::render::CameraScroll;
use crate::structures::hut::spawn_hut_visual;
use crate::world::bg::{BrokenStorage, WholeStorage};

const RESEARCH_WALL: Color = Color::srgb(0.40, 0.55, 0.60);
const AQUA_WALL: Color = Color::srgb(0.45, 0.70, 0.80);

/// Bright yellow tint so the scout reads as a distinct caste from the
/// regular crew (white) and the orange builder crabs.
const SCOUT_TINT: Color = Color::srgb(0.95, 0.85, 0.30);

/// What the scout says when they reach the present spot. Kept under
/// 50 chars so the bubble fits comfortably on the canvas.
const SCOUT_LINE: &str = "Skipper! Tree out west\nwith a busted store!";

pub struct ResearchPlugin;

impl Plugin for ResearchPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AutoFishTicker>().add_systems(
            Update,
            (
                on_purchase,
                on_structure_built,
                auto_fish_tick,
                tick_scout,
                crate::crew::tick_walk_animation::<ResearchScout>,
                on_storage_purchase,
            ),
        );
    }
}

/// Cadence accumulator for the auto-fish ticker.
#[derive(Resource, Default)]
struct AutoFishTicker {
    accum: f32,
}

// ---------------------------------------------------------------------------
// Scout
// ---------------------------------------------------------------------------

// Both the briefing `!` and the scout's speech line are emitted as
// `SpawnFloatingTextEvent`s — same red-shaking style as the
// fisherman's "miss". Their durations match the cinematic state's
// duration so each one auto-despawns when its phase ends.

#[derive(Component)]
pub struct ResearchScout {
    pub state: ScoutState,
    pub flap_accum: f32,
    pub walk_frame: bool,
}

pub enum ScoutState {
    /// Scout walks from the foragers hut over to the research hut for
    /// a "briefing" beat — they pause beneath the research hut with a
    /// `!` icon above their head before peeling off west.
    WalkingToHut {
        from: Vec2,
        to: Vec2,
        time: f32,
        dur: f32,
    },
    AtResearchHut {
        time: f32,
        dur: f32,
    },
    WalkingTo {
        from: Vec2,
        to: Vec2,
        time: f32,
        dur: f32,
    },
    AtTree {
        time: f32,
        dur: f32,
    },
    WalkingBack {
        from: Vec2,
        to: Vec2,
        time: f32,
        dur: f32,
    },
    Speaking {
        time: f32,
    },
    PanToTree {
        time: f32,
        from: f32,
        to: f32,
    },
    ShowingTree {
        time: f32,
    },
    PanBack {
        time: f32,
        from: f32,
        to: f32,
    },
    Done,
}

impl crate::crew::CrewWalking for ResearchScout {
    fn is_walking(&self) -> bool {
        matches!(
            self.state,
            ScoutState::WalkingToHut { .. }
                | ScoutState::WalkingTo { .. }
                | ScoutState::WalkingBack { .. }
        )
    }
    fn walk_frame(&self) -> bool {
        self.walk_frame
    }
    fn step_walking(&mut self, walking: bool, dt: f32) -> bool {
        crate::crew::step_walk_frame(walking, &mut self.flap_accum, &mut self.walk_frame, dt)
    }
}

/// Flip the per-building flags immediately on purchase so a second
/// click can't queue another build of the same structure. Also kicks
/// off the research-mission cinematic by spawning the scout entity
/// (and removing the worker that becomes them).
#[allow(clippy::too_many_arguments)]
fn on_purchase(
    mut events: MessageReader<PurchaseEvent>,
    mut research: ResMut<ResearchHut>,
    mut aqua: ResMut<AquaHut>,
    mut auto_fish: ResMut<AutoFishing>,
    mut mission: ResMut<ResearchMission>,
    mut workers: ResMut<Workers>,
    worker_q: Query<(Entity, &Pos), With<Worker>>,
    shapes: Res<Shapes>,
    mut commands: Commands,
) {
    for ev in events.read() {
        match ev.kind {
            PurchaseKind::HutResearch => research.owned = true,
            PurchaseKind::HutAqua => aqua.owned = true,
            PurchaseKind::AutoFishing => {
                auto_fish.owned = true;
                auto_fish.enabled = true;
            }
            PurchaseKind::AutoFishingToggle => {
                if auto_fish.owned {
                    auto_fish.enabled = !auto_fish.enabled;
                }
            }
            PurchaseKind::ResearchMission => {
                if mission.started || mission.unlocked {
                    continue;
                }
                let Some((worker_e, pos)) = crate::crew::pick_worker_to_convert(&worker_q) else {
                    continue;
                };
                commands.entity(worker_e).despawn();
                if workers.count > 0 {
                    workers.count -= 1;
                }
                let from = pos;
                // First leg: walk over to the research hut for a
                // briefing beat. The hut's centre is at
                // (HUT_RESEARCH_X, HUT_RESEARCH_Y); aim just below
                // its body so the scout stops on the sand at the
                // hut's doorstep rather than overlapping the wall.
                let hut_door = Vec2::new(HUT_RESEARCH_X, HUT_RESEARCH_Y + HUT_BODY_H * 0.5 + 4.0);
                let to = hut_door;
                let dur = (from.distance(to).max(1.0) / SCOUT_WALK_SPEED).clamp(0.5, 30.0);
                commands.spawn((
                    ResearchScout {
                        state: ScoutState::WalkingToHut {
                            from,
                            to,
                            time: 0.0,
                            dur,
                        },
                        flap_accum: 0.0,
                        walk_frame: false,
                    },
                    Pos(from),
                    Layer(Z_CREW),
                    Sprite {
                        image: shapes.humanoid.clone(),
                        color: SCOUT_TINT,
                        custom_size: Some(Vec2::new(16.0, 9.0)),
                        ..default()
                    },
                    Transform::default(),
                ));
                mission.started = true;
            }
            _ => {}
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn tick_scout(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut ResearchScout, &mut Pos)>,
    mut scroll: ResMut<CameraScroll>,
    mut mission: ResMut<ResearchMission>,
    mut floating: MessageWriter<crate::effects::SpawnFloatingTextEvent>,
) {
    let dt = time.delta_secs();
    for (entity, mut scout, mut pos) in &mut q {
        let next: Option<ScoutState> = match &mut scout.state {
            ScoutState::WalkingToHut { from, to, time: t, dur } => {
                *t += dt;
                let prog = (*t / *dur).clamp(0.0, 1.0);
                pos.0 = from.lerp(*to, prog);
                if *t >= *dur {
                    pos.0 = *to;
                    // Briefing `!` — same red-shaking floating-text
                    // style as the fisherman's "miss". Lasts the full
                    // AtResearchHut beat; auto-despawns when its timer
                    // expires (no manual cleanup needed).
                    floating.write(crate::effects::SpawnFloatingTextEvent {
                        pos: Vec2::new(pos.0.x, pos.0.y - 8.0),
                        text: "!".into(),
                        color: colors::MISS_RED,
                        size: 8.0,
                        duration: SCOUT_AT_HUT_DURATION,
                        vy: 0.0,
                        shake: 1.2,
                    });
                    Some(ScoutState::AtResearchHut { time: 0.0, dur: SCOUT_AT_HUT_DURATION })
                } else {
                    None
                }
            }
            ScoutState::AtResearchHut { time: t, dur } => {
                *t += dt;
                if *t >= *dur {
                    let from = pos.0;
                    let to = Vec2::new(TREE_X + 18.0, TREE_Y);
                    let dist = from.distance(to).max(1.0);
                    let dur = (dist / SCOUT_WALK_SPEED).clamp(0.5, 30.0);
                    Some(ScoutState::WalkingTo {
                        from,
                        to,
                        time: 0.0,
                        dur,
                    })
                } else {
                    None
                }
            }
            ScoutState::WalkingTo { from, to, time: t, dur } => {
                *t += dt;
                let prog = (*t / *dur).clamp(0.0, 1.0);
                pos.0 = from.lerp(*to, prog);
                if *t >= *dur {
                    pos.0 = *to;
                    Some(ScoutState::AtTree { time: 0.0, dur: 0.8 })
                } else {
                    None
                }
            }
            ScoutState::AtTree { time: t, dur } => {
                *t += dt;
                if *t >= *dur {
                    let from = pos.0;
                    let to = Vec2::new(SCOUT_PRESENT_X, SCOUT_PRESENT_Y);
                    let dist = from.distance(to).max(1.0);
                    let dur = (dist / SCOUT_WALK_SPEED).clamp(0.5, 30.0);
                    Some(ScoutState::WalkingBack {
                        from,
                        to,
                        time: 0.0,
                        dur,
                    })
                } else {
                    None
                }
            }
            ScoutState::WalkingBack { from, to, time: t, dur } => {
                *t += dt;
                let prog = (*t / *dur).clamp(0.0, 1.0);
                pos.0 = from.lerp(*to, prog);
                // Camera tracks the scout through the return walk so
                // the player sees them arrive in the present spot.
                scroll.x = pos.0.x - 150.0;
                if *t >= *dur {
                    pos.0 = *to;
                    scroll.x = SCROLL_FOR_SCOUT;
                    // Speech line — same red-shaking style as "miss".
                    // Auto-despawns at the end of the Speaking phase
                    // because the duration matches.
                    floating.write(crate::effects::SpawnFloatingTextEvent {
                        pos: Vec2::new(pos.0.x, pos.0.y - 12.0),
                        text: SCOUT_LINE.into(),
                        color: colors::MISS_RED,
                        size: 5.0,
                        duration: SCOUT_SPEAK_DURATION,
                        vy: 0.0,
                        shake: 0.6,
                    });
                    Some(ScoutState::Speaking { time: 0.0 })
                } else {
                    None
                }
            }
            ScoutState::Speaking { time: t } => {
                *t += dt;
                scroll.x = SCROLL_FOR_SCOUT;
                if *t >= SCOUT_SPEAK_DURATION {
                    Some(ScoutState::PanToTree {
                        time: 0.0,
                        from: SCROLL_FOR_SCOUT,
                        to: SCROLL_FOR_TREE,
                    })
                } else {
                    None
                }
            }
            ScoutState::PanToTree { time: t, from, to } => {
                *t += dt;
                let prog = (*t / SCOUT_PAN_DURATION).clamp(0.0, 1.0);
                scroll.x = *from + (*to - *from) * prog;
                if *t >= SCOUT_PAN_DURATION {
                    scroll.x = *to;
                    Some(ScoutState::ShowingTree { time: 0.0 })
                } else {
                    None
                }
            }
            ScoutState::ShowingTree { time: t } => {
                *t += dt;
                scroll.x = SCROLL_FOR_TREE;
                if *t >= SCOUT_HOLD_DURATION {
                    Some(ScoutState::PanBack {
                        time: 0.0,
                        from: SCROLL_FOR_TREE,
                        to: 0.0,
                    })
                } else {
                    None
                }
            }
            ScoutState::PanBack { time: t, from, to } => {
                *t += dt;
                let prog = (*t / SCOUT_PAN_DURATION).clamp(0.0, 1.0);
                scroll.x = *from + (*to - *from) * prog;
                if *t >= SCOUT_PAN_DURATION {
                    scroll.x = *to;
                    mission.unlocked = true;
                    Some(ScoutState::Done)
                } else {
                    None
                }
            }
            ScoutState::Done => {
                commands.entity(entity).despawn();
                None
            }
        };
        if let Some(s) = next {
            scout.state = s;
        }
    }
}

// ---------------------------------------------------------------------------
// Tree storage purchase
// ---------------------------------------------------------------------------

fn on_storage_purchase(
    mut events: MessageReader<PurchaseEvent>,
    mut storage: ResMut<TreeStorage>,
    mut commands: Commands,
    broken_q: Query<Entity, With<BrokenStorage>>,
    existing_whole: Query<(), With<WholeStorage>>,
) {
    for ev in events.read() {
        if ev.kind != PurchaseKind::TreeStorage {
            continue;
        }
        if storage.owned {
            continue;
        }
        storage.owned = true;
        // Despawn the broken rubble.
        for be in &broken_q {
            commands.entity(be).despawn();
        }
        if !existing_whole.is_empty() {
            continue;
        }
        // Body — 12×10 trunk-coloured crate.
        commands.spawn((
            WholeStorage,
            Pos(Vec2::new(TREE_STORAGE_X, TREE_STORAGE_Y)),
            Layer(Z_HUT - 0.05),
            Sprite::from_color(colors::TREE_TRUNK, Vec2::new(12.0, 10.0)),
            Transform::default(),
        ));
        // Lid stripe across the top — foliage-light green.
        commands.spawn((
            WholeStorage,
            Pos(Vec2::new(TREE_STORAGE_X, TREE_STORAGE_Y - 4.0)),
            Layer(Z_HUT - 0.04),
            Sprite::from_color(colors::TREE_FOLIAGE_LIGHT, Vec2::new(12.0, 2.0)),
            Transform::default(),
        ));
    }
}

fn on_structure_built(
    mut events: MessageReader<StructureBuiltEvent>,
    mut commands: Commands,
    mut sound: MessageWriter<PlaySoundEvent>,
    shapes: Res<Shapes>,
) {
    for ev in events.read() {
        let (x, y, color) = match ev.kind {
            PurchaseKind::HutResearch => (HUT_RESEARCH_X, HUT_RESEARCH_Y, RESEARCH_WALL),
            PurchaseKind::HutAqua => (HUT_AQUA_X, HUT_AQUA_Y, AQUA_WALL),
            _ => continue,
        };
        spawn_hut_visual(&mut commands, &shapes, x, y, color);
        sound.write(PlaySoundEvent {
            kind: SoundKind::SmallRockSpawn,
            pitch: 0.85,
            volume: 0.6,
        });
    }
}

/// Once a second, when AutoFishing is owned + enabled and the fish
/// stockpile is below its target, deduct one bucket's cost from skims
/// and write a `PurchaseEvent { kind: Fish }` so the existing pier
/// fish-spawn handler kicks in.
fn auto_fish_tick(
    time: Res<Time>,
    mut ticker: ResMut<AutoFishTicker>,
    auto_fish: Res<AutoFishing>,
    pier: Res<Pier>,
    fishes: Res<Fishes>,
    mut skims: ResMut<Skims>,
    mut purchases: MessageWriter<PurchaseEvent>,
) {
    if !auto_fish.owned || !auto_fish.enabled || !pier.owned {
        return;
    }
    ticker.accum += time.delta_secs();
    if ticker.accum < AUTO_FISHING_TICK {
        return;
    }
    ticker.accum = 0.0;
    if fishes.count >= AUTO_FISHING_TARGET {
        return;
    }
    let cost = cost_for(PurchaseKind::Fish);
    if skims.total < cost {
        return;
    }
    skims.total = skims.total.saturating_sub(cost);
    purchases.write(PurchaseEvent {
        kind: PurchaseKind::Fish,
    });
}
