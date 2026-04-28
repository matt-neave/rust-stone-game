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

use super::{
    BeachcomberHut, FisherHut, HoverState, Hut, MinerHut, Pier, Port, PurchaseButton, SkimmerHut,
    StonemasonHut, Workers,
};

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
    /// One-time beachcomber hut — gated by the foragers hut.
    HutBeachcomber,
    /// One-time stonemason hut — gated by the miner hut.
    HutStonemason,
    /// Worker — sold by the foragers hut at 10 skims.
    Worker,
    /// Miner conversion — sold by the miner hut for 1 worker.
    Miner,
    /// Repeatable miner upgrade — +1 damage per pickaxe throw.
    MinerDamage,
    /// Skimmer conversion — sold by the skimmer hut for 1 worker.
    Skimmer,
    /// Repeatable upgrade — adds `SKIM_UPGRADE_DELTA` to the bounce
    /// chance for skimmer-thrown rocks (cursor stays unbuffed).
    SkimUpgrade,
    /// Fisherman conversion — sold by the anglers hut for 1 worker.
    Fisherman,
    /// Beachcomber conversion — sold by the foragers hut. Walks the
    /// sand digging up small rocks.
    Beachcomber,
    /// Stonemason conversion — sold by the miner hut. Sharpens
    /// idle rocks into "masoned" rocks that guarantee 2 skims.
    Stonemason,
    /// Boatman conversion — sold by the pier (after Port is built).
    /// Sails the ocean and ferries sunken stones back to shore.
    Boatman,
    /// One-time pier. Bought from the cave panel after the hut.
    Pier,
    /// One-time port. Bought from the pier panel after the pier.
    /// Unlocks Boatman conversions.
    Port,
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
            PurchaseKind::HutBeachcomber => "ComberHut",
            PurchaseKind::HutStonemason => "MasonHut",
            PurchaseKind::Worker => "Worker",
            PurchaseKind::Miner => "Miner",
            PurchaseKind::MinerDamage => "Pickaxe",
            PurchaseKind::Skimmer => "Skimmer",
            PurchaseKind::SkimUpgrade => "Skim Up",
            PurchaseKind::Fisherman => "Fisherman",
            PurchaseKind::Beachcomber => "Comber",
            PurchaseKind::Stonemason => "Mason",
            PurchaseKind::Boatman => "Boatman",
            PurchaseKind::Pier => "Pier",
            PurchaseKind::Port => "Port",
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
            PurchaseKind::HutBeachcomber => "15",
            PurchaseKind::HutStonemason => "15",
            PurchaseKind::Worker => "10",
            PurchaseKind::Miner => "1W",
            PurchaseKind::MinerDamage => "30",
            PurchaseKind::Skimmer => "1W",
            PurchaseKind::SkimUpgrade => "25",
            PurchaseKind::Fisherman => "1W",
            PurchaseKind::Beachcomber => "1W",
            PurchaseKind::Stonemason => "1W",
            PurchaseKind::Boatman => "1W",
            PurchaseKind::Pier => "30",
            PurchaseKind::Port => "50",
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
    PurchaseKind::HutBeachcomber,
    PurchaseKind::HutStonemason,
    PurchaseKind::Pier,
];

/// Foragers hut — sells workers only.
pub const HUT_PANEL_KINDS: &[PurchaseKind] = &[PurchaseKind::Worker];

/// Miner hut — pickaxe-thrower conversion + damage upgrade.
pub const HUT_MINER_KINDS: &[PurchaseKind] =
    &[PurchaseKind::Miner, PurchaseKind::MinerDamage];

/// Skimmer hut — skimmer conversion plus the bounce-chance upgrade.
pub const HUT_SKIMMER_KINDS: &[PurchaseKind] =
    &[PurchaseKind::Skimmer, PurchaseKind::SkimUpgrade];

/// Fisherman hut — angler conversion only.
pub const HUT_FISHER_KINDS: &[PurchaseKind] = &[PurchaseKind::Fisherman];

/// Beachcomber hut — sells beachcomber conversions.
pub const HUT_BEACHCOMBER_KINDS: &[PurchaseKind] = &[PurchaseKind::Beachcomber];

/// Stonemason hut — sells stonemason conversions.
pub const HUT_STONEMASON_KINDS: &[PurchaseKind] = &[PurchaseKind::Stonemason];

/// Pier panel — bucket of fish + the Port unlock.
pub const PIER_PANEL_KINDS: &[PurchaseKind] = &[PurchaseKind::Fish, PurchaseKind::Port];

/// Port panel — Boatman conversion only (Port itself is bought from pier).
pub const PORT_PANEL_KINDS: &[PurchaseKind] = &[PurchaseKind::Boatman];

#[derive(Message)]
pub struct PurchaseEvent {
    pub kind: PurchaseKind,
}

/// Static / starting cost for every purchase. Worker is the special
/// case — its real cost scales with how many workers have already
/// been bought (see [`current_worker_cost`]). Other systems should
/// prefer [`current_cost_for`] when they need the live price.
pub fn cost_for(kind: PurchaseKind) -> u64 {
    match kind {
        PurchaseKind::Hut
        | PurchaseKind::HutMiner
        | PurchaseKind::HutSkimmer
        | PurchaseKind::HutFisher => HUT_COST,
        PurchaseKind::HutBeachcomber | PurchaseKind::HutStonemason => 15,
        PurchaseKind::Worker => WORKER_COST,
        // Specialists cost no skims — only one worker each.
        PurchaseKind::Miner
        | PurchaseKind::Skimmer
        | PurchaseKind::Fisherman
        | PurchaseKind::Beachcomber
        | PurchaseKind::Stonemason
        | PurchaseKind::Boatman => 0,
        PurchaseKind::MinerDamage => 30,
        PurchaseKind::SkimUpgrade => SKIM_UPGRADE_COST,
        PurchaseKind::Pier => PIER_COST,
        PurchaseKind::Port => 50,
        PurchaseKind::Fish => FISH_COST,
    }
}

/// Live cost for a row given the current resource state. Routes
/// Worker to `current_worker_cost`; falls back to `cost_for` for
/// kinds with static prices.
pub fn current_cost_for(kind: PurchaseKind, workers: &Workers) -> u64 {
    match kind {
        PurchaseKind::Worker => current_worker_cost(workers),
        _ => cost_for(kind),
    }
}

/// Multiplier applied to the worker price for every previous worker
/// purchase. `1.15 ^ workers.purchased` — the canonical Cookie
/// Clicker–style ramp. Slightly below the typical income compounder
/// in this game, so the player stays just ahead of the price curve.
const WORKER_COST_MULTIPLIER: f64 = 1.15;

/// Current cost in skims of one worker, given total cumulative
/// purchases. `floor(WORKER_COST * 1.2 ^ purchased)`.
pub fn current_worker_cost(workers: &Workers) -> u64 {
    let scale = WORKER_COST_MULTIPLIER.powi(workers.purchased as i32);
    ((WORKER_COST as f64) * scale).floor() as u64
}

#[allow(clippy::too_many_arguments)]
pub fn can_afford(
    kind: PurchaseKind,
    skims: &Skims,
    hut: &Hut,
    miner_hut: &MinerHut,
    skimmer_hut: &SkimmerHut,
    fisher_hut: &FisherHut,
    bc_hut: &BeachcomberHut,
    sm_hut: &StonemasonHut,
    pier: &Pier,
    port: &Port,
    workers: &Workers,
) -> bool {
    match kind {
        // Cave-panel structure unlocks.
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
        PurchaseKind::HutBeachcomber => {
            hut.owned && !bc_hut.owned && skims.total >= 15
        }
        PurchaseKind::HutStonemason => {
            miner_hut.owned && !sm_hut.owned && skims.total >= 15
        }
        PurchaseKind::Pier => {
            (skimmer_hut.owned || fisher_hut.owned)
                && !pier.owned
                && skims.total >= PIER_COST
        }
        // Pier-panel rows.
        PurchaseKind::Fish => pier.owned && skims.total >= FISH_COST,
        PurchaseKind::Port => pier.owned && !port.owned && skims.total >= 50,
        // Port-panel row.
        PurchaseKind::Boatman => port.owned && workers.count >= 1,
        // Hut-panel rows.
        PurchaseKind::Worker => hut.owned && skims.total >= current_worker_cost(workers),
        PurchaseKind::Beachcomber => bc_hut.owned && workers.count >= 1,
        PurchaseKind::Miner => miner_hut.owned && workers.count >= 1,
        PurchaseKind::MinerDamage => miner_hut.owned && skims.total >= 30,
        PurchaseKind::Stonemason => sm_hut.owned && workers.count >= 1,
        PurchaseKind::Skimmer => skimmer_hut.owned && workers.count >= 1,
        PurchaseKind::Fisherman => fisher_hut.owned && workers.count >= 1,
        PurchaseKind::SkimUpgrade => {
            skimmer_hut.owned && skims.total >= SKIM_UPGRADE_COST
        }
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
    skimmer_hut: &SkimmerHut,
    fisher_hut: &FisherHut,
    _pier: &Pier,
    _port: &Port,
) -> bool {
    match kind {
        PurchaseKind::Hut => true,
        PurchaseKind::HutMiner => hut.owned,
        PurchaseKind::HutSkimmer => miner_hut.owned,
        PurchaseKind::HutFisher => miner_hut.owned,
        PurchaseKind::HutBeachcomber => hut.owned,
        PurchaseKind::HutStonemason => miner_hut.owned,
        // Pier reveals once the player has water-relevant crew.
        PurchaseKind::Pier => skimmer_hut.owned || fisher_hut.owned,
        // Pier-panel rows.
        PurchaseKind::Fish => true,
        PurchaseKind::Port => true,
        // Port-panel row.
        PurchaseKind::Boatman => true,
        // Other panel rows are always relevant once their hut is up.
        PurchaseKind::Worker
        | PurchaseKind::Beachcomber
        | PurchaseKind::Miner
        | PurchaseKind::MinerDamage
        | PurchaseKind::Stonemason
        | PurchaseKind::Skimmer
        | PurchaseKind::SkimUpgrade
        | PurchaseKind::Fisherman => true,
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
    bc_hut: &BeachcomberHut,
    sm_hut: &StonemasonHut,
    pier: &Pier,
    port: &Port,
    hover: &HoverState,
) -> bool {
    match kind {
        PurchaseKind::Hut => !hut.owned && hover.cave,
        PurchaseKind::HutMiner => hut.owned && !miner_hut.owned && hover.cave,
        PurchaseKind::HutSkimmer => miner_hut.owned && !skimmer_hut.owned && hover.cave,
        PurchaseKind::HutFisher => miner_hut.owned && !fisher_hut.owned && hover.cave,
        PurchaseKind::HutBeachcomber => hut.owned && !bc_hut.owned && hover.cave,
        PurchaseKind::HutStonemason => miner_hut.owned && !sm_hut.owned && hover.cave,
        PurchaseKind::Pier => {
            (skimmer_hut.owned || fisher_hut.owned) && !pier.owned && hover.cave
        }
        PurchaseKind::Worker => hut.owned && hover.hut,
        PurchaseKind::Beachcomber => bc_hut.owned && hover.hut_beachcomber,
        PurchaseKind::Miner | PurchaseKind::MinerDamage => {
            miner_hut.owned && hover.hut_miner
        }
        PurchaseKind::Stonemason => sm_hut.owned && hover.hut_stonemason,
        PurchaseKind::Skimmer | PurchaseKind::SkimUpgrade => {
            skimmer_hut.owned && hover.hut_skimmer
        }
        PurchaseKind::Fisherman => fisher_hut.owned && hover.hut_fisher,
        PurchaseKind::Fish => pier.owned && hover.pier,
        PurchaseKind::Port => pier.owned && !port.owned && hover.pier,
        PurchaseKind::Boatman => port.owned && hover.port,
    }
}

/// Whether the given one-time purchase has already been bought. Used
/// to render the row in a "sold out" darker state.
#[allow(clippy::too_many_arguments)]
pub fn is_sold_out(
    kind: PurchaseKind,
    hut: &Hut,
    miner_hut: &MinerHut,
    skimmer_hut: &SkimmerHut,
    fisher_hut: &FisherHut,
    bc_hut: &BeachcomberHut,
    sm_hut: &StonemasonHut,
    pier: &Pier,
    port: &Port,
) -> bool {
    match kind {
        PurchaseKind::Hut => hut.owned,
        PurchaseKind::HutMiner => miner_hut.owned,
        PurchaseKind::HutSkimmer => skimmer_hut.owned,
        PurchaseKind::HutFisher => fisher_hut.owned,
        PurchaseKind::HutBeachcomber => bc_hut.owned,
        PurchaseKind::HutStonemason => sm_hut.owned,
        PurchaseKind::Pier => pier.owned,
        PurchaseKind::Port => port.owned,
        // Repeatable purchases never sold out.
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
    bc_hut: Res<BeachcomberHut>,
    sm_hut: Res<StonemasonHut>,
    pier: Res<Pier>,
    port: Res<Port>,
    workers: Res<Workers>,
    hover: Res<HoverState>,
) {
    for ev in clicks.read() {
        for (btn, pos) in &buttons {
            if !button_active(
                btn.kind, &hut, &miner_hut, &skimmer_hut, &fisher_hut, &bc_hut, &sm_hut, &pier,
                &port, &hover,
            ) {
                continue;
            }
            let half = btn.size * 0.5;
            let dx = (ev.pos.x - pos.0.x).abs();
            let dy = (ev.pos.y - pos.0.y).abs();
            if dx > half.x || dy > half.y {
                continue;
            }
            if !can_afford(
                btn.kind, &skims, &hut, &miner_hut, &skimmer_hut, &fisher_hut, &bc_hut, &sm_hut,
                &pier, &port, &workers,
            ) {
                sound.write(PlaySoundEvent {
                    kind: SoundKind::Click,
                    pitch: 0.6,
                    volume: 0.18,
                });
                continue;
            }
            skims.total = skims.total.saturating_sub(current_cost_for(btn.kind, &workers));
            purchases.write(PurchaseEvent { kind: btn.kind });
            sound.write(PlaySoundEvent {
                kind: SoundKind::Reward,
                pitch: 1.05,
                volume: 0.35,
            });
        }
    }
}
