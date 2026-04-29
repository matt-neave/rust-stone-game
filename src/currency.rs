//! Skim currency — the primary currency. Updated by `smallrock.rs`
//! when a rock bounces; rendered by `ui.rs` at the top of the screen.
//!
//! Wood — secondary currency unlocked by the Tree Surgeon research
//! upgrade. Each click on the standalone tree (see `world::bg`) adds
//! one to `Wood.total`. Hidden from the HUD until the upgrade is
//! purchased.

use bevy::prelude::*;

#[derive(Resource, Default, Debug)]
pub struct Skims {
    pub total: u64,
}

#[derive(Resource, Default, Debug)]
pub struct Wood {
    pub total: u64,
}

pub struct CurrencyPlugin;

impl Plugin for CurrencyPlugin {
    fn build(&self, app: &mut App) {
        // Test/dev default — start with enough skims to buy the hut +
        // a couple of conversions without grinding clicks first.
        app.insert_resource(Skims { total: 10000 })
            .init_resource::<Wood>();
    }
}
