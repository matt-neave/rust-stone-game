//! Skim currency — the only currency in the MVP. Updated by `smallrock.rs`
//! when a rock bounces; rendered by `ui.rs` at the top of the screen.

use bevy::prelude::*;

#[derive(Resource, Default, Debug)]
pub struct Skims {
    pub total: u64,
}

pub struct CurrencyPlugin;

impl Plugin for CurrencyPlugin {
    fn build(&self, app: &mut App) {
        // Test/dev default — start with enough skims to buy the hut +
        // a couple of conversions without grinding clicks first.
        app.insert_resource(Skims { total: 10000 });
    }
}
