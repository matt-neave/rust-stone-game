//! Idle workers — the base crew that wanders around the hut and
//! gets converted into specialist roles (miner, skimmer, fisherman).

use bevy::prelude::*;
use rand::Rng;
use std::f32::consts::PI;

use crate::core::colors;
use crate::core::common::{Layer, Pos};
use crate::core::constants::{HUT_BODY_H, HUT_X, HUT_Y, WORKER_WANDER_RADIUS, Z_CREW};
use crate::render::shapes::Shapes;
use crate::structures::hut::SpawnWorkerEvent;

use super::{step_walk_frame, tick_walk_animation, CrewWalking};

/// A worker that loiters around the hut. Doesn't produce skims
/// directly — must be converted into a specialist role first.
#[derive(Component)]
pub struct Worker {
    pub state: WorkerState,
    pub home: Vec2,
    pub flap_accum: f32,
    pub walk_frame: bool,
}

pub enum WorkerState {
    Idle { time: f32, dur: f32 },
    Wandering { from: Vec2, to: Vec2, time: f32, dur: f32 },
}

impl CrewWalking for Worker {
    fn is_walking(&self) -> bool {
        matches!(self.state, WorkerState::Wandering { .. })
    }
    fn walk_frame(&self) -> bool {
        self.walk_frame
    }
    fn step_walking(&mut self, walking: bool, dt: f32) -> bool {
        step_walk_frame(walking, &mut self.flap_accum, &mut self.walk_frame, dt)
    }
}

pub struct WorkerPlugin;

impl Plugin for WorkerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (spawn_workers, tick_workers, tick_walk_animation::<Worker>),
        );
    }
}

fn spawn_workers(
    mut commands: Commands,
    mut events: MessageReader<SpawnWorkerEvent>,
    shapes: Res<Shapes>,
) {
    for ev in events.read() {
        spawn_worker_entity(&mut commands, &shapes, ev.pos);
    }
}

fn spawn_worker_entity(commands: &mut Commands, shapes: &Shapes, pos: Vec2) -> Entity {
    let mut rng = rand::thread_rng();
    commands
        .spawn((
            Worker {
                state: WorkerState::Idle {
                    time: 0.0,
                    dur: rng.gen_range(0.6..1.6),
                },
                home: Vec2::new(HUT_X, HUT_Y + HUT_BODY_H * 0.5 + 10.0),
                flap_accum: 0.0,
                walk_frame: false,
            },
            Pos(pos),
            Layer(Z_CREW),
            Sprite {
                image: shapes.humanoid.clone(),
                color: colors::WORKER_BODY,
                custom_size: Some(Vec2::new(4.0, 6.0)),
                ..default()
            },
            Transform::default(),
        ))
        .id()
}

fn tick_workers(time: Res<Time>, mut q: Query<(&mut Worker, &mut Pos)>) {
    let dt = time.delta_secs();
    let mut rng = rand::thread_rng();
    for (mut worker, mut pos) in &mut q {
        match &mut worker.state {
            WorkerState::Idle { time: t, dur } => {
                *t += dt;
                if *t >= *dur {
                    // Pick a new wander target inside the hut's apron.
                    let angle: f32 = rng.gen_range(0.0..(2.0 * PI));
                    let radius: f32 = rng.gen_range(0.0..WORKER_WANDER_RADIUS);
                    let target = worker.home + Vec2::new(angle.cos(), angle.sin() * 0.5) * radius;
                    let dist = pos.0.distance(target).max(1.0);
                    let speed: f32 = rng.gen_range(8.0..14.0);
                    let dur = dist / speed;
                    worker.state = WorkerState::Wandering {
                        from: pos.0,
                        to: target,
                        time: 0.0,
                        dur,
                    };
                }
            }
            WorkerState::Wandering {
                from,
                to,
                time: t,
                dur,
            } => {
                *t += dt;
                let prog = (*t / *dur).clamp(0.0, 1.0);
                pos.0 = from.lerp(*to, prog);
                if *t >= *dur {
                    worker.state = WorkerState::Idle {
                        time: 0.0,
                        dur: rng.gen_range(0.5..1.8),
                    };
                }
            }
        }
    }
}
