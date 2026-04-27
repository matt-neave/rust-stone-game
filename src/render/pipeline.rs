//! Render pipeline — render to a 480×270 internal image, upscale it,
//! and overlay UI text at native window DPI. Lifted from
//! rust-SNKRX/src/render.rs; the big-picture comment there explains
//! *why* this matters for the pixel aesthetic.
//!
//! Three cameras:
//!   * [`GameCamera`] (order -1) — gameplay → RTT image.
//!   * [`UpscaleCamera`] (order 0) — draws an upscale sprite of the
//!     RTT image, integer-scaled to fit the window. Layer
//!     [`UPSCALE_LAYER`].
//!   * [`UiCamera`] (order 1) — draws UI text directly to the window
//!     at native DPI, on top of the upscale. Layer [`UI_LAYER`].
//!     Uses [`ClearColorConfig::None`] so it doesn't wipe the
//!     gameplay image before overlaying.

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ClearColorConfig, ImageRenderTarget, RenderTarget};
use bevy::image::{Image, ImageSampler};
use bevy::prelude::*;
use bevy::render::render_resource::{
    Extent3d, TextureDimension, TextureFormat, TextureUsages,
};
use bevy::render::view::Msaa;

use crate::core::constants::{INTERNAL_HEIGHT, INTERNAL_WIDTH};
use crate::render::dock::DisplayMode;
use crate::render::ui_text::sync_ui_text;

#[derive(Component)]
pub struct GameCamera;

#[derive(Component)]
pub struct UpscaleCamera;

#[derive(Component)]
pub struct UpscaleSprite;

#[derive(Component)]
pub struct UiCamera;

pub const UPSCALE_LAYER: usize = 1;
pub const UI_LAYER: usize = 2;

/// Current display upscale factor — updated each frame in
/// [`resize_upscale_sprite`]. Read by `core::input` to convert mouse
/// pixels back to spec coords on a window of any size, and by
/// [`sync_ui_text`] to scale text positions / font sizes.
#[derive(Resource, Default, Clone, Copy)]
pub struct DisplayScale(pub f32);

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DisplayScale>()
            .add_systems(Startup, setup_render_pipeline)
            .add_systems(Update, (resize_upscale_sprite, sync_ui_text).chain());
    }
}

fn setup_render_pipeline(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let size = Extent3d {
        width: INTERNAL_WIDTH as u32,
        height: INTERNAL_HEIGHT as u32,
        depth_or_array_layers: 1,
    };
    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
        | TextureUsages::COPY_DST
        | TextureUsages::RENDER_ATTACHMENT;
    image.sampler = ImageSampler::nearest();

    let image_handle = images.add(image);

    commands.spawn((
        GameCamera,
        Camera2d,
        Camera {
            order: -1,
            ..default()
        },
        RenderTarget::Image(ImageRenderTarget {
            handle: image_handle.clone(),
            scale_factor: 1.0,
        }),
        Msaa::Off,
    ));

    commands.spawn((
        UpscaleCamera,
        Camera2d,
        Camera {
            order: 0,
            ..default()
        },
        RenderLayers::layer(UPSCALE_LAYER),
        Msaa::Off,
    ));

    commands.spawn((
        UpscaleSprite,
        Sprite {
            image: image_handle,
            custom_size: Some(Vec2::new(INTERNAL_WIDTH * 2.0, INTERNAL_HEIGHT * 2.0)),
            ..default()
        },
        Transform::default(),
        RenderLayers::layer(UPSCALE_LAYER),
    ));

    // UI camera — renders UI text directly to the window on top of
    // the upscaled gameplay. `ClearColorConfig::None` is critical:
    // the upscale camera has already filled the window, and clearing
    // here would wipe it before drawing the text.
    commands.spawn((
        UiCamera,
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        RenderLayers::layer(UI_LAYER),
        Msaa::Off,
    ));
}

fn resize_upscale_sprite(
    windows: Query<&Window>,
    mode: Res<DisplayMode>,
    mut display_scale: ResMut<DisplayScale>,
    mut sprites: Query<&mut Sprite, With<UpscaleSprite>>,
) {
    // Docked mode owns the upscale sprite — see `render::dock`. Skip
    // here so we don't fight it.
    if *mode == DisplayMode::Docked {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let win_w = window.physical_width() as f32 / window.scale_factor();
    let win_h = window.physical_height() as f32 / window.scale_factor();
    if win_w <= 0.0 || win_h <= 0.0 {
        return;
    }
    let scale_x = (win_w / INTERNAL_WIDTH).floor();
    let scale_y = (win_h / INTERNAL_HEIGHT).floor();
    let scale = scale_x.min(scale_y).max(1.0);
    if (display_scale.0 - scale).abs() > 0.001 {
        display_scale.0 = scale;
    }
    let display_w = INTERNAL_WIDTH * scale;
    let display_h = INTERNAL_HEIGHT * scale;
    for mut sprite in &mut sprites {
        sprite.custom_size = Some(Vec2::new(display_w, display_h));
    }
}
