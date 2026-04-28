//! The port — a small wooden dock east of the pier, on the water.
//! Bought from the pier panel; unlocks Boatman conversions.

use bevy::prelude::*;

use crate::audio::{PlaySoundEvent, SoundKind};
use crate::core::colors;
use crate::core::common::{Layer, Pos};
use crate::core::constants::{PORT_H, PORT_W, PORT_X, PORT_Y, Z_PIER};
use crate::economy::{Port, PurchaseEvent, PurchaseKind};

#[derive(Component)]
pub struct PortPart;

pub struct PortPlugin;

impl Plugin for PortPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, handle_port_purchase);
    }
}

fn handle_port_purchase(
    mut events: MessageReader<PurchaseEvent>,
    mut commands: Commands,
    mut port: ResMut<Port>,
    mut sound: MessageWriter<PlaySoundEvent>,
) {
    for ev in events.read() {
        if ev.kind != PurchaseKind::Port {
            continue;
        }
        if port.owned {
            continue;
        }
        port.owned = true;
        spawn_port_visual(&mut commands);
        sound.write(PlaySoundEvent {
            kind: SoundKind::SmallRockSpawn,
            pitch: 0.7,
            volume: 0.55,
        });
    }
}

fn spawn_port_visual(commands: &mut Commands) {
    // Main deck — wider than the pier, sits flush on the water.
    commands.spawn((
        PortPart,
        Pos(Vec2::new(PORT_X, PORT_Y)),
        Layer(Z_PIER),
        Sprite::from_color(colors::HUT_ROOF, Vec2::new(PORT_W, PORT_H)),
        Transform::default(),
    ));
    // Two stubby pylons either side of the deck.
    for dx in [-PORT_W * 0.4, PORT_W * 0.4] {
        commands.spawn((
            PortPart,
            Pos(Vec2::new(PORT_X + dx, PORT_Y + PORT_H * 0.5 + 1.5)),
            Layer(Z_PIER + 0.05),
            Sprite::from_color(colors::HUT_WALL, Vec2::new(2.0, 4.0)),
            Transform::default(),
        ));
    }
    // Front rail — a darker plank along the seaward edge.
    commands.spawn((
        PortPart,
        Pos(Vec2::new(PORT_X, PORT_Y + PORT_H * 0.5 - 0.5)),
        Layer(Z_PIER + 0.1),
        Sprite::from_color(colors::HUT_WALL, Vec2::new(PORT_W - 2.0, 1.0)),
        Transform::default(),
    ));
}
