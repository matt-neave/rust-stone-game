//! Generic particle system — short-lived sprites with velocity + damping +
//! shrink-out. Patterned on `rust-SNKRX/src/effects.rs::HitParticleFx`.
//! Used for rock dust on big-rock clicks and splash droplets on water sinks.

use bevy::prelude::*;
use rand::Rng;

use crate::core::common::{Layer, Pos};
use crate::core::constants::Z_PARTICLE;

#[derive(Component)]
pub struct Particle {
    pub vel: Vec2,
    pub lifetime: f32,
    pub initial_lifetime: f32,
    pub initial_size: Vec2,
    /// Linear velocity damping (0 = none, 1 = stop instantly).
    pub damping: f32,
    /// Optional gravity pull (px/s² along +Y, since spec is Y-down).
    pub gravity: f32,
}

#[derive(Message)]
pub struct SpawnParticleBurstEvent {
    pub pos: Vec2,
    pub color: Color,
    pub count: u32,
    /// Min/max angle of the cone (radians, 0 = +X). For an omnidirectional
    /// burst, set min=0, max=TAU.
    pub angle_min: f32,
    pub angle_max: f32,
    pub speed_min: f32,
    pub speed_max: f32,
    pub size_min: f32,
    pub size_max: f32,
    pub lifetime_min: f32,
    pub lifetime_max: f32,
    pub damping: f32,
    pub gravity: f32,
}

pub struct ParticlesPlugin;

impl Plugin for ParticlesPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SpawnParticleBurstEvent>()
            .add_systems(Update, (spawn_bursts, tick_particles).chain());
    }
}

fn spawn_bursts(
    mut commands: Commands,
    mut events: MessageReader<SpawnParticleBurstEvent>,
) {
    let mut rng = rand::thread_rng();
    for ev in events.read() {
        for _ in 0..ev.count {
            let angle: f32 = if ev.angle_min == ev.angle_max {
                ev.angle_min
            } else {
                rng.gen_range(ev.angle_min..ev.angle_max)
            };
            let speed: f32 = rng.gen_range(ev.speed_min..=ev.speed_max);
            let life: f32 = rng.gen_range(ev.lifetime_min..=ev.lifetime_max);
            let size: f32 = rng.gen_range(ev.size_min..=ev.size_max);
            let vel = Vec2::new(angle.cos(), angle.sin()) * speed;
            commands.spawn((
                Particle {
                    vel,
                    lifetime: life,
                    initial_lifetime: life,
                    initial_size: Vec2::splat(size),
                    damping: ev.damping,
                    gravity: ev.gravity,
                },
                Pos(ev.pos),
                Layer(Z_PARTICLE),
                Sprite::from_color(ev.color, Vec2::splat(size)),
                Transform::default(),
            ));
        }
    }
}

fn tick_particles(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Particle, &mut Pos, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (e, mut p, mut pos, mut sprite) in &mut q {
        p.vel.y += p.gravity * dt;
        pos.0 += p.vel * dt;
        let damp = (1.0 - p.damping * dt).clamp(0.0, 1.0);
        p.vel *= damp;

        p.lifetime -= dt;
        let t = (p.lifetime / p.initial_lifetime).clamp(0.0, 1.0);
        // Shrink to ~15% of starting size before despawn for a chunky
        // poof-out feel.
        let size = p.initial_size * t.max(0.15);
        sprite.custom_size = Some(size);

        if p.lifetime <= 0.0 {
            commands.entity(e).despawn();
        }
    }
}
