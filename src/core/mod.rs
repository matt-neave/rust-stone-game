//! Engine-level primitives shared across every gameplay module.
//!
//! * [`constants`] — game-wide tunables (canvas size, costs, timings).
//! * [`colors`] — palette.
//! * [`state`] — top-level [`bevy::state::state::States`] enum.
//! * [`common`] — `Pos` / `Layer` / `Vel` and the spec→world transform sync.
//! * [`assets`] — font handles.
//! * [`input`] — mouse → spec-coordinate click events.

pub mod assets;
pub mod colors;
pub mod common;
pub mod constants;
pub mod input;
pub mod state;
