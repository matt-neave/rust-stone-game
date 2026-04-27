//! Short-lived in-world text labels that drift, optionally shake,
//! and fade out. Used for "+1 SKIM" reward markers and the "miss"
//! indicator on a failed fisherman cast.

use bevy::prelude::*;
use rand::Rng;

use crate::core::assets::GameAssets;
use crate::core::common::{Layer, Pos};
use crate::core::constants::Z_FLOATING;

#[derive(Component)]
pub struct FloatingText {
    pub time: f32,
    pub duration: f32,
    pub vy: f32,
    /// Settled position (no shake). Per-frame jitter is added on top
    /// when writing to `Pos`, so it never accumulates into the path.
    pub base_pos: Vec2,
    pub shake_amp: f32,
}

#[derive(Message)]
pub struct SpawnFloatingTextEvent {
    pub pos: Vec2,
    pub text: String,
    pub color: Color,
    pub size: f32,
    pub duration: f32,
    /// Upward speed in spec px/s. Spec is Y-down so up = negative.
    pub vy: f32,
    /// Per-frame random jitter amplitude in spec px. Use 0.0 for
    /// settled labels (e.g. the +1 reward); a small value (1-2 px)
    /// gives the label a "shouted" feel.
    pub shake: f32,
}

impl SpawnFloatingTextEvent {
    /// Default reward-style label: drifts up, no shake.
    pub fn reward(pos: Vec2, text: impl Into<String>, color: Color) -> Self {
        Self {
            pos,
            text: text.into(),
            color,
            size: 8.0,
            duration: 0.7,
            vy: -22.0,
            shake: 0.0,
        }
    }
}

pub struct FloatingTextPlugin;

impl Plugin for FloatingTextPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SpawnFloatingTextEvent>()
            .add_systems(Update, (spawn_floating, tick_floating).chain());
    }
}

fn spawn_floating(
    mut commands: Commands,
    mut events: MessageReader<SpawnFloatingTextEvent>,
    assets: Res<GameAssets>,
) {
    for ev in events.read() {
        commands.spawn((
            FloatingText {
                time: 0.0,
                duration: ev.duration,
                vy: ev.vy,
                base_pos: ev.pos,
                shake_amp: ev.shake,
            },
            Pos(ev.pos),
            Layer(Z_FLOATING),
            Text2d::new(ev.text.clone()),
            TextFont {
                font: assets.font.clone(),
                font_size: ev.size,
                font_smoothing: bevy::text::FontSmoothing::None,
                ..default()
            },
            TextColor(ev.color),
            Transform::default(),
        ));
    }
}

fn tick_floating(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut FloatingText, &mut Pos, &mut TextColor)>,
) {
    let dt = time.delta_secs();
    let mut rng = rand::thread_rng();
    for (e, mut ft, mut pos, mut color) in &mut q {
        ft.time += dt;
        ft.base_pos.y += ft.vy * dt;

        let shake = if ft.shake_amp > 0.0 {
            Vec2::new(
                rng.gen_range(-ft.shake_amp..=ft.shake_amp),
                rng.gen_range(-ft.shake_amp..=ft.shake_amp),
            )
        } else {
            Vec2::ZERO
        };
        pos.0 = ft.base_pos + shake;

        let t = (ft.time / ft.duration).clamp(0.0, 1.0);
        // Pop in for the first 10%, hold, fade out the last 30%.
        let alpha = if t < 0.1 {
            t / 0.1
        } else if t > 0.7 {
            (1.0 - (t - 0.7) / 0.3).max(0.0)
        } else {
            1.0
        };
        let mut c = color.0;
        c.set_alpha(alpha);
        color.0 = c;
        if ft.time >= ft.duration {
            commands.entity(e).despawn();
        }
    }
}
