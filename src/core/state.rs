//! Top-level game state. The MVP only has a single state, but the
//! scaffolding is ready for menu / pause additions.

use bevy::prelude::*;

#[derive(States, Clone, Copy, Eq, PartialEq, Hash, Debug, Default)]
pub enum GameState {
    #[default]
    Playing,
}

pub struct StatePlugin;

impl Plugin for StatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>();
    }
}
