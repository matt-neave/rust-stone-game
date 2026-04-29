//! Render pipeline + shared visual primitives.
//!
//! * [`pipeline`] — three-camera setup (game RTT → upscale → UI).
//! * [`ui_text`] — high-DPI text component + sync system.
//! * [`shapes`] — pre-baked sprite masks (rocks, humanoids, hut roof, etc.).

pub mod dock;
pub mod pipeline;
pub mod rock_material;
pub mod scroll;
pub mod shapes;
pub mod ui_text;

pub use dock::{DisplayMode, DockButtonHover, DockPlugin, MuteButtonHover};
pub use pipeline::{DisplayScale, RenderPlugin, UI_LAYER};
pub use rock_material::{RockLitMaterial, RockLitParams, RockMaterialPlugin, RockQuad};
pub use scroll::{CameraScroll, ScreenAnchored, ScreenFixedText, ScrollPlugin};
pub use ui_text::UiText;
