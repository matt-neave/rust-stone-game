//! [`PurchaseKind`] enum, costs, affordability checks, and the click
//! handler that turns a click on a button into a [`PurchaseEvent`].

use bevy::prelude::*;

use crate::audio::{PlaySoundEvent, SoundKind};
use crate::core::common::Pos;
use crate::core::constants::{
    FISH_COST, HUT_COST, PIER_COST, SKIM_UPGRADE_COST, WORKER_COST,
};
use crate::core::input::ClickEvent;
use crate::currency::Skims;

use super::{FisherHut, HoverState, Hut, MinerHut, Pier, PurchaseButton, SkimmerHut, Workers};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PurchaseKind {
    /// One-time foragers hut. Bought from the cave panel.
    Hut,
    /// One-time miner hut — gates the skimmer + angler huts.
    HutMiner,
    /// One-time skimmer hut.
    HutSkimmer,
    /// One-time anglers hut.
    HutFisher,
    /// Worker — sold by the foragers hut at 10 skims.
    Worker,
    /// Miner conversion — sold by the miner hut for 1 worker.
    Miner,
    /// Skimmer conversion — sold by the skimmer hut for 1 worker.
    Skimmer,
    /// Repeatable upgrade — adds `SKIM_UPGRADE_DELTA` to the bounce
    /// chance for both player- and skimmer-thrown rocks.
    SkimUpgrade,
    /// Fisherman conversion — sold by the anglers hut for 1 worker.
    Fisherman,
    /// One-time pier. Bought from the cave panel after the hut.
    Pier,
    /// Repeatable bucket of fish — sold by the pier.
    Fish,
}

impl PurchaseKind {
    pub fn label(self) -> &'static str {
        match self {
            PurchaseKind::Hut => "Hut",
            PurchaseKind::HutMiner => "MinerHut",
            PurchaseKind::HutSkimmer => "SkimrHut",
            PurchaseKind::HutFisher => "FishrHut",
            PurchaseKind::Worker => "Worker",
            PurchaseKind::Miner => "Miner",
            PurchaseKind::Skimmer => "Skimmer",
            PurchaseKind::SkimUpgrade => "Skim Up",
            PurchaseKind::Fisherman => "Fisherman",
            PurchaseKind::Pier => "Pier",
            PurchaseKind::Fish => "Bucket",
        }
    }

    /// Buy-row cost label. Specialist conversions cost a worker only,
    /// no skims; hut buildings each cost 10 skims.
    pub fn cost_label(self) -> &'static str {
        match self {
            PurchaseKind::Hut => "10",
            PurchaseKind::HutMiner => "10",
            PurchaseKind::HutSkimmer => "10",
            PurchaseKind::HutFisher => "10",
            PurchaseKind::Worker => "10",
            PurchaseKind::Miner => "1W",
            PurchaseKind::Skimmer => "1W",
            PurchaseKind::SkimUpgrade => "25",
            PurchaseKind::Fisherman => "1W",
            PurchaseKind::Pier => "30",
            PurchaseKind::Fish => "5",
        }
    }
}

/// All purchase options offered by the cave panel, in display order.
/// Each is a one-time structure unlock; rows for owned structures
/// hide so the next unbought option steps into the slot.
pub const CAVE_PANEL_KINDS: &[PurchaseKind] = &[
    PurchaseKind::Hut,
    PurchaseKind::HutMiner,
    PurchaseKind::HutSkimmer,
    PurchaseKind::HutFisher,
    PurchaseKind::Pier,
];

/// Foragers hut — sells worker conversions. The specialist roles
/// each have their own dedicated building (`HutMiner`, `HutSkimmer`,
/// `HutFisher`), so this panel is just the one row.
pub const HUT_PANEL_KINDS: &[PurchaseKind] = &[PurchaseKind::Worker];

/// Miner hut — pickaxe-thrower conversion only.
pub const HUT_MINER_KINDS: &[PurchaseKind] = &[PurchaseKind::Miner];

/// Skimmer hut — skimmer conversion plus the bounce-chance upgrade.
pub const HUT_SKIMMER_KINDS: &[PurchaseKind] =
    &[PurchaseKind::Skimmer, PurchaseKind::SkimUpgrade];

/// Fisherman hut — angler conversion only.
pub const HUT_FISHER_KINDS: &[PurchaseKind] = &[PurchaseKind::Fisherman];

/// All purchase options offered by the pier panel.
pub const PIER_PANEL_KINDS: &[PurchaseKind] = &[PurchaseKind::Fish];

#[derive(Message)]
pub struct PurchaseEvent {
    pub kind: PurchaseKind,
}

pub fn cost_for(kind: PurchaseKind) -> u64 {
    match kind {
        PurchaseKind::Hut
        | PurchaseKind::HutMiner
        | PurchaseKind::HutSkimmer
        | PurchaseKind::HutFisher => HUT_COST,
        PurchaseKind::Worker => WORKER_COST,
        // Specialists cost no skims — only one worker each.
        PurchaseKind::Miner | PurchaseKind::Skimmer | PurchaseKind::Fisherman => 0,
        PurchaseKind::SkimUpgrade => SKIM_UPGRADE_COST,
        PurchaseKind::Pier => PIER_COST,
        PurchaseKind::Fish => FISH_COST,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn can_afford(
    kind: PurchaseKind,
    skims: &Skims,
    hut: &Hut,
    miner_hut: &MinerHut,
    skimmer_hut: &SkimmerHut,
    fisher_hut: &FisherHut,
    pier: &Pier,
    workers: &Workers,
) -> bool {
    match kind {
        // Cave-panel structure unlocks. Each is gated behind the
        // previous: foragers first, then the miner hut, then the
        // skimmer + angler huts (both gated behind the miner hut).
        PurchaseKind::Hut => !hut.owned && skims.total >= HUT_COST,
        PurchaseKind::HutMiner => {
            hut.owned && !miner_hut.owned && skims.total >= HUT_COST
        }
        PurchaseKind::HutSkimmer => {
            miner_hut.owned && !skimmer_hut.owned && skims.total >= HUT_COST
        }
        PurchaseKind::HutFisher => {
            miner_hut.owned && !fisher_hut.owned && skims.total >= HUT_COST
        }
        PurchaseKind::Pier => !pier.owned && skims.total >= PIER_COST,
        // Hut-panel rows. Specialists no longer cost skims — just a
        // worker. Each conversion gated behind its dedicated hut.
        PurchaseKind::Worker => hut.owned && skims.total >= WORKER_COST,
        PurchaseKind::Miner => miner_hut.owned && workers.count >= 1,
        PurchaseKind::Skimmer => skimmer_hut.owned && workers.count >= 1,
        PurchaseKind::Fisherman => fisher_hut.owned && workers.count >= 1,
        PurchaseKind::SkimUpgrade => {
            skimmer_hut.owned && skims.total >= SKIM_UPGRADE_COST
        }
        PurchaseKind::Fish => pier.owned && skims.total >= FISH_COST,
    }
}

/// Whether the row should be **shown** in its panel — independent of
/// affordability or hover. A hidden row is collapsed out of the
/// layout entirely; a visible row may still be inactive (locked /
/// sold out), in which case it renders darkened. Cave-panel rows are
/// gated by progression; every other panel's rows are always visible
/// (the panel itself is only shown when its hut exists).
#[allow(clippy::too_many_arguments)]
pub fn row_visible(
    kind: PurchaseKind,
    hut: &Hut,
    miner_hut: &MinerHut,
    _skimmer_hut: &SkimmerHut,
    _fisher_hut: &FisherHut,
    _pier: &Pier,
) -> bool {
    match kind {
        PurchaseKind::Hut => true,
        PurchaseKind::HutMiner => hut.owned,
        PurchaseKind::HutSkimmer => miner_hut.owned,
        PurchaseKind::HutFisher => miner_hut.owned,
        PurchaseKind::Pier => true,
        PurchaseKind::Worker
        | PurchaseKind::Miner
        | PurchaseKind::Skimmer
        | PurchaseKind::SkimUpgrade
        | PurchaseKind::Fisherman
        | PurchaseKind::Fish => true,
    }
}

/// Whether the button currently registers clicks. A row that is
/// `row_visible` but not `button_active` is rendered in the panel
/// darkened (locked or sold out).
#[allow(clippy::too_many_arguments)]
pub fn button_active(
    kind: PurchaseKind,
    hut: &Hut,
    miner_hut: &MinerHut,
    skimmer_hut: &SkimmerHut,
    fisher_hut: &FisherHut,
    pier: &Pier,
    hover: &HoverState,
) -> bool {
    match kind {
        PurchaseKind::Hut => !hut.owned && hover.cave,
        PurchaseKind::HutMiner => hut.owned && !miner_hut.owned && hover.cave,
        PurchaseKind::HutSkimmer => miner_hut.owned && !skimmer_hut.owned && hover.cave,
        PurchaseKind::HutFisher => miner_hut.owned && !fisher_hut.owned && hover.cave,
        PurchaseKind::Pier => !pier.owned && hover.cave,
        PurchaseKind::Worker => hut.owned && hover.hut,
        PurchaseKind::Miner => miner_hut.owned && hover.hut_miner,
        PurchaseKind::Skimmer | PurchaseKind::SkimUpgrade => {
            skimmer_hut.owned && hover.hut_skimmer
        }
        PurchaseKind::Fisherman => fisher_hut.owned && hover.hut_fisher,
        PurchaseKind::Fish => pier.owned && hover.pier,
    }
}

/// Whether the given one-time purchase has already been bought. Used
/// to render the row in a "sold out" darker state.
pub fn is_sold_out(
    kind: PurchaseKind,
    hut: &Hut,
    miner_hut: &MinerHut,
    skimmer_hut: &SkimmerHut,
    fisher_hut: &FisherHut,
    pier: &Pier,
) -> bool {
    match kind {
        PurchaseKind::Hut => hut.owned,
        PurchaseKind::HutMiner => miner_hut.owned,
        PurchaseKind::HutSkimmer => skimmer_hut.owned,
        PurchaseKind::HutFisher => fisher_hut.owned,
        PurchaseKind::Pier => pier.owned,
        // Repeatable purchases are never "sold out".
        _ => false,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_button_clicks(
    mut clicks: MessageReader<ClickEvent>,
    buttons: Query<(&PurchaseButton, &Pos)>,
    mut purchases: MessageWriter<PurchaseEvent>,
    mut sound: MessageWriter<PlaySoundEvent>,
    mut skims: ResMut<Skims>,
    hut: Res<Hut>,
    miner_hut: Res<MinerHut>,
    skimmer_hut: Res<SkimmerHut>,
    fisher_hut: Res<FisherHut>,
    pier: Res<Pier>,
    workers: Res<Workers>,
    hover: Res<HoverState>,
) {
    for ev in clicks.read() {
        for (btn, pos) in &buttons {
            if !button_active(btn.kind, &hut, &miner_hut, &skimmer_hut, &fisher_hut, &pier, &hover) {
                continue;
            }
            let half = btn.size * 0.5;
            let dx = (ev.pos.x - pos.0.x).abs();
            let dy = (ev.pos.y - pos.0.y).abs();
            if dx > half.x || dy > half.y {
                continue;
            }
            if !can_afford(
                btn.kind,
                &skims,
                &hut,
                &miner_hut,
                &skimmer_hut,
                &fisher_hut,
                &pier,
                &workers,
            ) {
                sound.write(PlaySoundEvent {
                    kind: SoundKind::Click,
                    pitch: 0.6,
                    volume: 0.18,
                });
                continue;
            }
            skims.total = skims.total.saturating_sub(cost_for(btn.kind));
            purchases.write(PurchaseEvent { kind: btn.kind });
            sound.write(PlaySoundEvent {
                kind: SoundKind::Reward,
                pitch: 1.05,
                volume: 0.35,
            });
        }
    }
}
