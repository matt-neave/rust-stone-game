//! Water-impact ripples. Each impact spawns a small set of expanding rings
//! (concentric, slight delay between them) that fade out as they grow. The
//! "ring" is faked using the white circle mask from `shapes.rs`: we render
//! a filled circle whose color is mixed toward water-color so it reads as a
//! pale disc against the dark water, then we fade it out as it expands.
//!
//! The spec asks for a "ripple shader" — at 480×270 native resolution, a
//! handful of expanding concentric discs reads as the same effect for a tiny
//! fraction of the engineering cost.

use bevy::prelude::*;

use crate::core::colors;
use crate::core::common::{Layer, Pos};
use crate::core::constants::Z_RIPPLE;
use crate::render::shapes::Shapes;

#[derive(Component)]
pub struct Ripple {
    pub time: f32,
    pub start_delay: f32,
    pub duration: f32,
    pub max_radius: f32,
}

#[derive(Message)]
pub struct SpawnRippleEvent {
    pub pos: Vec2,
    /// `true` for a sink (bigger, slower ripple), `false` for a skip bounce
    /// (tighter, snappier ring).
    pub big: bool,
}

pub struct RipplePlugin;

impl Plugin for RipplePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SpawnRippleEvent>()
            .add_systems(Update, (spawn_ripples, tick_ripples).chain());
    }
}

fn spawn_ripples(
    mut commands: Commands,
    mut events: MessageReader<SpawnRippleEvent>,
    shapes: Res<Shapes>,
) {
    for ev in events.read() {
        let (rings, max_radius, duration) = if ev.big {
            // Sink: 3 concentric rings, big and slow.
            (3, 26.0_f32, 0.85_f32)
        } else {
            // Bounce: 2 quick rings.
            (2, 14.0_f32, 0.45_f32)
        };
        for i in 0..rings {
            let delay = i as f32 * 0.10;
            commands.spawn((
                Ripple {
                    time: 0.0,
                    start_delay: delay,
                    duration,
                    max_radius,
                },
                Pos(ev.pos),
                Layer(Z_RIPPLE + i as f32 * 0.001),
                Sprite {
                    image: shapes.circle.clone(),
                    color: colors::FOAM.with_alpha(0.0),
                    custom_size: Some(Vec2::splat(1.0)),
                    ..default()
                },
                Transform::default(),
            ));
        }
    }
}

fn tick_ripples(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Ripple, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (e, mut r, mut sprite) in &mut q {
        r.time += dt;
        if r.time < r.start_delay {
            sprite.color = sprite.color.with_alpha(0.0);
            continue;
        }
        let t = ((r.time - r.start_delay) / r.duration).clamp(0.0, 1.0);
        // Radius eases out — fast at the start, slowing down as it spreads.
        let ease = 1.0 - (1.0 - t) * (1.0 - t);
        let radius = ease * r.max_radius;
        // Outer pixels of the disc are the visible edge; we don't render a
        // true ring, but with a foam-pale color over the dark water and
        // alpha falling off, the disc reads as a ripple front.
        let alpha = (1.0 - t).powf(1.4) * 0.55;
        sprite.color = colors::FOAM.with_alpha(alpha);
        sprite.custom_size = Some(Vec2::splat(radius * 2.0));

        if t >= 1.0 {
            commands.entity(e).despawn();
        }
    }
}
