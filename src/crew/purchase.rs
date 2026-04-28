//! Conversion dispatcher: turns a worker into a specialist role.
//!
//! Reads [`PurchaseEvent`]s, picks the worker entity nearest the hut
//! door, despawns it, and emits a [`SpawnConversionEvent`] carrying
//! the kind + last-known worker position. Each role's plugin
//! subscribes to the conversion event and filters by its own kind —
//! adding a new role does not require editing this file.
//!
//! The `Hut` and `Worker` purchase kinds are handled elsewhere:
//! [`structures::hut`] catches them directly because they're about
//! the hut spawning workers, not consuming them.

use bevy::prelude::*;

use crate::audio::{PlaySoundEvent, SoundKind};
use crate::core::common::Pos;
use crate::economy::{PurchaseEvent, PurchaseKind, Workers};

use super::{pick_worker_to_convert, Worker};

/// Fired when a worker has been despawned and a specialist of the
/// given kind should spawn at `from_pos`.
#[derive(Message)]
pub struct SpawnConversionEvent {
    pub kind: PurchaseKind,
    pub from_pos: Vec2,
}

pub(super) fn handle_role_purchase(
    mut events: MessageReader<PurchaseEvent>,
    mut convert: MessageWriter<SpawnConversionEvent>,
    mut workers: ResMut<Workers>,
    worker_q: Query<(Entity, &Pos), With<Worker>>,
    mut commands: Commands,
    mut sound: MessageWriter<PlaySoundEvent>,
) {
    for ev in events.read() {
        // Only true worker→specialist conversions consume a worker.
        // Hut purchases, plain Worker buys, repeatable upgrades, fish
        // buckets, etc. don't go through this dispatcher.
        if !matches!(
            ev.kind,
            PurchaseKind::Miner
                | PurchaseKind::Skimmer
                | PurchaseKind::Fisherman
                | PurchaseKind::Beachcomber
                | PurchaseKind::Stonemason
                | PurchaseKind::Boatman
        ) {
            continue;
        }
        if workers.count == 0 {
            continue;
        }
        let Some((worker_e, worker_pos)) = pick_worker_to_convert(&worker_q) else {
            continue;
        };
        commands.entity(worker_e).despawn();
        workers.count = workers.count.saturating_sub(1);

        convert.write(SpawnConversionEvent {
            kind: ev.kind,
            from_pos: worker_pos,
        });
        sound.write(PlaySoundEvent {
            kind: SoundKind::Reward,
            pitch: 0.85,
            volume: 0.4,
        });
    }
}
