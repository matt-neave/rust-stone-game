//! Sand imprints — dark depressions left in the sand where small rocks used
//! to sit.
//!
//! When the player clicks an idle small rock to toss it, the rock emits a
//! `SpawnImprintEvent` at its sand position. We spawn a small ellipse,
//! tinted darker than the sand, that holds for ~13 seconds and fades over
//! the last ~2 — total lifetime 15s — then despawns. The imprint is on a
//! Z layer just above the bg checker so it sits *on* the sand rather than
//! under it, but below all rocks/ripples/UI.

use bevy::color::Alpha;
use bevy::prelude::*;

use crate::core::colors;
use crate::core::common::{Layer, Pos};
use crate::core::constants::*;
use crate::render::shapes::{Shapes, SmallRockShape};

/// Layer for imprints — just above the bg checker, well below rocks/ripples.
const Z_IMPRINT: f32 = Z_BG_DETAIL + 0.4;
/// Total imprint lifetime in seconds — per the spec.
const IMPRINT_LIFETIME: f32 = 15.0;
/// Alpha of the imprint at full visibility. Low value = "slightly darker"
/// than the sand rather than a hard black mark.
const IMPRINT_MAX_ALPHA: f32 = 0.55;

#[derive(Message)]
pub struct SpawnImprintEvent {
    pub pos: Vec2,
    /// Silhouette to use for the imprint — matches the rock's shape, one
    /// pixel smaller in radius.
    pub shape: SmallRockShape,
}

#[derive(Component)]
pub struct SandImprint {
    pub time: f32,
}

pub struct SandDentPlugin;

impl Plugin for SandDentPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SpawnImprintEvent>()
            .add_systems(Update, (spawn_imprints, tick_imprints));
    }
}

fn spawn_imprints(
    mut commands: Commands,
    mut events: MessageReader<SpawnImprintEvent>,
    shapes: Res<Shapes>,
) {
    for ev in events.read() {
        // Safety: don't draw imprints over the wet-sand strip or water.
        // Rocks always idle on dry sand so this should never trigger, but
        // catches any stray events.
        if ev.pos.x >= SHORELINE_X - 6.0 {
            continue;
        }

        let mut color = colors::SAND_DARK;
        color.set_alpha(IMPRINT_MAX_ALPHA);

        commands.spawn((
            SandImprint { time: 0.0 },
            Pos(ev.pos),
            Layer(Z_IMPRINT),
            Sprite {
                image: shapes.imprint_image(ev.shape),
                color,
                custom_size: Some(Shapes::imprint_size(ev.shape)),
                ..default()
            },
            Transform::default(),
        ));
    }
}

fn tick_imprints(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut SandImprint, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (e, mut imp, mut sprite) in &mut q {
        imp.time += dt;
        let t = (imp.time / IMPRINT_LIFETIME).clamp(0.0, 1.0);
        // Fade linearly across the full lifetime — imprint pops in at
        // peak alpha when the rock leaves and steadily dissolves into
        // the sand from there.
        let env = (1.0 - t).clamp(0.0, 1.0);
        let mut c = sprite.color;
        c.set_alpha(env * IMPRINT_MAX_ALPHA);
        sprite.color = c;

        if imp.time >= IMPRINT_LIFETIME {
            commands.entity(e).despawn();
        }
    }
}
