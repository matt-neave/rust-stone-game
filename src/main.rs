//! rust-stones — a small Bevy clicker where the player chips small rocks off
//! a big rock on a beach and skips them across the water for currency.
//!
//! Style is lifted from rust-SNKRX: 480×270 internal canvas, integer-upscale
//! RTT pipeline, PixulBrush font, SNKRX sound effects + Kubbi music. See
//! `game-mvp.md` for the gameplay spec.
//!
//! ## Module layout
//!
//! Top-level modules group by concern, not by entity:
//!
//! * [`core`] — engine primitives (constants, palette, components, input).
//! * [`render`] — three-camera pipeline, sprite masks, high-DPI text.
//! * [`audio`] — SFX bank + music player.
//! * [`world`] — beach/water background and ambient effects.
//! * [`effects`] — particles, ripples, floating text.
//! * [`rocks`] — the big boulder, small rocks, sand imprints.
//! * [`structures`] — buildings on the beach (cave, hut).
//! * [`crew`] — autonomous workers, miners, skimmers, fishermen.
//! * [`economy`] — purchase events, resources, hover panels.
//! * [`currency`] — the skim resource.
//! * [`ui`] — top-level SKIMS counter.

mod audio;
mod core;
mod crew;
mod currency;
mod economy;
mod effects;
mod render;
mod rocks;
mod structures;
mod ui;
mod world;

use bevy::image::ImagePlugin;
use bevy::prelude::*;
use bevy::window::{MonitorSelection, WindowMode, WindowPosition, WindowResolution};

use crate::audio::AudioPlugin;
use crate::core::assets::AssetsPlugin;
use crate::core::common::CommonPlugin;
use crate::core::input::InputPlugin;
use crate::core::state::StatePlugin;
use crate::crew::CrewPlugin;
use crate::currency::CurrencyPlugin;
use crate::economy::EconomyPlugin;
use crate::effects::{FloatingTextPlugin, ParticlesPlugin, RipplePlugin};
use crate::render::shapes::ShapesPlugin;
use crate::render::{DockPlugin, RenderPlugin, RockMaterialPlugin, ScrollPlugin};
use crate::rocks::{BigRockPlugin, SandDentPlugin, ShadowPlugin, SmallRockPlugin};
use crate::structures::{HutPlugin, PierPlugin, PortPlugin};
use crate::ui::UiPlugin;
use crate::world::{AmbientPlugin, BgPlugin};

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "rust-stones".into(),
                        // Default to a centered windowed mode at 1280×720 —
                        // the dock-button cycle (see `render::dock`) can
                        // promote this to docked or fullscreen at runtime.
                        mode: WindowMode::Windowed,
                        resolution: WindowResolution::new(1280, 720)
                            .with_scale_factor_override(1.0),
                        position: WindowPosition::Centered(MonitorSelection::Current),
                        resizable: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .insert_resource(ClearColor(crate::core::colors::SKY))
        // Bevy's `Plugins` impl is implemented per tuple-arity up to 15
        // — split the list across two `add_plugins` calls so we don't hit
        // the trait-bound limit.
        .add_plugins((
            CommonPlugin,
            StatePlugin,
            AssetsPlugin,
            ShapesPlugin,
            RenderPlugin,
            RockMaterialPlugin,
            DockPlugin,
            AudioPlugin,
            InputPlugin,
            BgPlugin,
            AmbientPlugin,
        ))
        .add_plugins((
            ParticlesPlugin,
            RipplePlugin,
            FloatingTextPlugin,
            CurrencyPlugin,
            UiPlugin,
            BigRockPlugin,
            SmallRockPlugin,
            ShadowPlugin,
            SandDentPlugin,
            EconomyPlugin,
            HutPlugin,
            PierPlugin,
            PortPlugin,
            CrewPlugin,
            ScrollPlugin,
        ))
        .run();
}
