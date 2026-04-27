//! Font handles. Pixel art renders best at the canvas's native resolution,
//! so font_size values throughout the project should be small integers
//! (~6–14). PixulBrush rasterizes nearly 1:1 at those sizes; FatPixelFont
//! is kept for any large-display use later.

use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct GameAssets {
    pub font: Handle<Font>,
    #[allow(dead_code)]
    pub fat_font: Handle<Font>,
}

pub struct AssetsPlugin;

impl Plugin for AssetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameAssets>()
            .add_systems(PreStartup, load_assets);
    }
}

fn load_assets(mut handles: ResMut<GameAssets>, asset_server: Res<AssetServer>) {
    handles.font = asset_server.load("fonts/PixulBrush.ttf");
    handles.fat_font = asset_server.load("fonts/FatPixelFont.ttf");
}
