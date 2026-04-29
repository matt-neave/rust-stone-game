//! Foragers hut — a single small structure on the sand. Buying a hut
//! unlocks the rest of the economy and grants two starter workers.
//!
//! The hut is built from three primitive sprites layered together: a tan
//! body rectangle, a darker triangular roof, and a near-black door. All
//! three are tagged with `HutPart` so they can be queried as a group.

use bevy::prelude::*;

use crate::audio::{PlaySoundEvent, SoundKind};
use crate::core::colors;
use crate::core::common::{Layer, Pos};
use crate::core::constants::*;
use crate::crew::builder::StructureBuiltEvent;
use crate::economy::{
    BeachcomberHut, FisherHut, Hut, MinerHut, PurchaseEvent, PurchaseKind, SkimmerHut,
    StonemasonHut, Workers,
};
use crate::render::shapes::Shapes;

#[derive(Component)]
pub struct HutPart;

/// The default cave structure on the sand. It's there from the start
/// and acts as the hover target for the BUY HUT button — hovering it
/// surfaces the purchase prompt next to it.
#[derive(Component)]
pub struct Cave;

#[derive(Component)]
pub struct CavePart;

/// Emitted when a worker should be spawned at a given position. `crew.rs`
/// listens for this and creates the entity. Used for both the two starter
/// workers from buying a hut and direct worker purchases.
#[derive(Message)]
pub struct SpawnWorkerEvent {
    pub pos: Vec2,
}

pub struct HutPlugin;

impl Plugin for HutPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SpawnWorkerEvent>()
            .add_systems(Startup, spawn_cave_visual)
            .add_systems(Update, (on_purchase, on_structure_built));
    }
}

fn spawn_cave_visual(mut commands: Commands, shapes: Res<Shapes>) {
    // Body — bumpy dark mound.
    commands.spawn((
        Cave,
        CavePart,
        Pos(Vec2::new(CAVE_X, CAVE_Y)),
        Layer(Z_CAVE),
        Sprite {
            image: shapes.cave_body.clone(),
            color: colors::CAVE_BODY,
            custom_size: Some(Vec2::new(CAVE_W, CAVE_H)),
            ..default()
        },
        Transform::default(),
    ));
    // Opening — small dark ellipse near the lower-centre of the body.
    commands.spawn((
        CavePart,
        Pos(Vec2::new(CAVE_X, CAVE_Y + 2.0)),
        Layer(Z_CAVE + 0.05),
        Sprite {
            image: shapes.cave_opening.clone(),
            color: colors::CAVE_OPENING,
            custom_size: Some(Vec2::new(CAVE_OPENING_W, CAVE_OPENING_H)),
            ..default()
        },
        Transform::default(),
    ));
}

/// Flip the per-hut `owned` flag immediately on purchase so a second
/// click can't queue another build of the same structure. The visual
/// + starter workers wait for the construction to finish; see
/// [`on_structure_built`].
#[allow(clippy::too_many_arguments)]
fn on_purchase(
    mut events: MessageReader<PurchaseEvent>,
    mut hut: ResMut<Hut>,
    mut miner_hut: ResMut<MinerHut>,
    mut skimmer_hut: ResMut<SkimmerHut>,
    mut fisher_hut: ResMut<FisherHut>,
    mut bc_hut: ResMut<BeachcomberHut>,
    mut sm_hut: ResMut<StonemasonHut>,
    mut workers: ResMut<Workers>,
    mut spawn_worker: MessageWriter<SpawnWorkerEvent>,
) {
    for ev in events.read() {
        match ev.kind {
            PurchaseKind::Hut => hut.owned = true,
            PurchaseKind::HutMiner => miner_hut.owned = true,
            PurchaseKind::HutSkimmer => skimmer_hut.owned = true,
            PurchaseKind::HutFisher => fisher_hut.owned = true,
            PurchaseKind::HutBeachcomber => bc_hut.owned = true,
            PurchaseKind::HutStonemason => sm_hut.owned = true,
            PurchaseKind::Worker => {
                if !hut.owned { continue; }
                workers.count += 1;
                // Track cumulative direct-buys for the dynamic
                // pricing curve. Starter workers from huts don't
                // bump this — only paid worker-row purchases do.
                workers.purchased += 1;
                spawn_worker.write(SpawnWorkerEvent {
                    pos: Vec2::new(HUT_X, HUT_Y + HUT_BODY_H * 0.5 + 12.0),
                });
            }
            // Specialist roles are consumed by `crew::*`. Pier/Port +
            // Fish purchases are consumed by `structures::pier` /
            // `structures::port`. Nothing for the hut to do here.
            PurchaseKind::Miner
            | PurchaseKind::MinerDamage
            | PurchaseKind::Skimmer
            | PurchaseKind::SkimUpgrade
            | PurchaseKind::Fisherman
            | PurchaseKind::Beachcomber
            | PurchaseKind::Stonemason
            | PurchaseKind::Boatman
            | PurchaseKind::Pier
            | PurchaseKind::Port
            | PurchaseKind::Fish => {}
        }
    }
}

/// Spawn the hut visual + build sfx once the construction site fires
/// its completion event. Starter workers come from the builder
/// crabs themselves (they convert in `crew::builder::tick_construction`).
fn on_structure_built(
    mut events: MessageReader<StructureBuiltEvent>,
    mut commands: Commands,
    mut sound: MessageWriter<PlaySoundEvent>,
    shapes: Res<Shapes>,
) {
    for ev in events.read() {
        let (x, y, color) = match ev.kind {
            PurchaseKind::Hut => (HUT_X, HUT_Y, colors::HUT_WALL),
            PurchaseKind::HutMiner => (HUT_MINER_X, HUT_MINER_Y, colors::MINER_BODY),
            PurchaseKind::HutSkimmer => (HUT_SKIMMER_X, HUT_SKIMMER_Y, colors::SKIMMER_BODY),
            PurchaseKind::HutFisher => (HUT_FISHER_X, HUT_FISHER_Y, colors::FISHERMAN_BODY),
            PurchaseKind::HutBeachcomber => {
                (HUT_BEACHCOMBER_X, HUT_BEACHCOMBER_Y, colors::BEACHCOMBER_BODY)
            }
            PurchaseKind::HutStonemason => {
                (HUT_STONEMASON_X, HUT_STONEMASON_Y, colors::STONEMASON_BODY)
            }
            _ => continue,
        };
        spawn_hut_visual(&mut commands, &shapes, x, y, color);
        play_build_sound(&mut sound);
    }
}

fn play_build_sound(sound: &mut MessageWriter<PlaySoundEvent>) {
    sound.write(PlaySoundEvent {
        kind: SoundKind::SmallRockSpawn,
        pitch: 0.85,
        volume: 0.6,
    });
}

/// Spawn one hut at `(x, y)` with the given wall color. Roof, door,
/// eaves are shared across all huts — only the wall tints by role.
fn spawn_hut_visual(
    commands: &mut Commands,
    shapes: &Shapes,
    x: f32,
    y: f32,
    wall_color: bevy::color::Color,
) {
    // Body — solid rectangle.
    let body_y = y + HUT_ROOF_H * 0.5;
    commands.spawn((
        HutPart,
        Pos(Vec2::new(x, body_y)),
        Layer(Z_HUT),
        Sprite::from_color(wall_color, Vec2::new(HUT_BODY_W, HUT_BODY_H)),
        Transform::default(),
    ));
    // Roof — triangular silhouette sitting on top of the body.
    let roof_y = y - HUT_BODY_H * 0.5;
    commands.spawn((
        HutPart,
        Pos(Vec2::new(x, roof_y)),
        Layer(Z_HUT + 0.05),
        Sprite {
            image: shapes.hut_roof.clone(),
            color: colors::HUT_ROOF,
            custom_size: Some(Vec2::new(HUT_ROOF_W, HUT_ROOF_H)),
            ..default()
        },
        Transform::default(),
    ));
    // Door — small dark slit in the body, tucked toward the bottom.
    commands.spawn((
        HutPart,
        Pos(Vec2::new(x, body_y + 1.5)),
        Layer(Z_HUT + 0.1),
        Sprite::from_color(colors::HUT_DOOR, Vec2::new(3.0, 5.0)),
        Transform::default(),
    ));
    // Darker eaves under the roof.
    commands.spawn((
        HutPart,
        Pos(Vec2::new(x, body_y - HUT_BODY_H * 0.5 + 0.5)),
        Layer(Z_HUT + 0.08),
        Sprite::from_color(colors::HUT_ROOF, Vec2::new(HUT_BODY_W, 1.0)),
        Transform::default(),
    ));
}
