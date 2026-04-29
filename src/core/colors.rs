//! The rust-stones palette. Picked to feel adjacent to the SNKRX palette
//! (`shared.lua:3-16`) but tuned for a beach scene — warm sand, deep water,
//! pebble grays.

use bevy::color::Color;

/// Sky / window clear color. Used as letterboxing color when the window
/// aspect doesn't match 480:270.
pub const SKY: Color = Color::srgb(0.07, 0.09, 0.11);

/// Foreground white — matches SNKRX `fg = #dadada`. Reserved for future
/// labels; the only text right now is the dimmed "SKIMS" caption and the
/// yellow counter value.
#[allow(dead_code)]
pub const FG: Color = Color::srgb(0xda as f32 / 255.0, 0xda as f32 / 255.0, 0xda as f32 / 255.0);

/// Dim text (counter sublabel etc).
pub const FG_DIM: Color = Color::srgb(0xb0 as f32 / 255.0, 0xa8 as f32 / 255.0, 0x9f as f32 / 255.0);

/// Yellow — SNKRX `#facf00`. Used for the +1 SKIM floaters.
pub const YELLOW: Color = Color::srgb(0xfa as f32 / 255.0, 0xcf as f32 / 255.0, 0x00 as f32 / 255.0);

/// Sand base — primary fill across the dry beach.
pub const SAND: Color = Color::srgb(0xf5 as f32 / 255.0, 0xd7 as f32 / 255.0, 0xb1 as f32 / 255.0);
/// Lighter sand — small random patches scattered over the base, plus the
/// rare 1-px ambient glints.
pub const SAND_LIGHT: Color = Color::srgb(0xf9 as f32 / 255.0, 0xef as f32 / 255.0, 0xd3 as f32 / 255.0);
/// Darker sand — small random patches, plus rock dust and rock imprints.
pub const SAND_DARK: Color = Color::srgb(0xc9 as f32 / 255.0, 0xa0 as f32 / 255.0, 0x8d as f32 / 255.0);
/// Wet sand near the shoreline.
pub const SAND_WET: Color = Color::srgb(0xa9 as f32 / 255.0, 0x8a as f32 / 255.0, 0x5a as f32 / 255.0);

/// Water gradient — five bands going shore→deep. Drawn as horizontal
/// slices in `world::bg`, with the seams between bands wobbling on a
/// sine so they don't read as straight stripes.
pub const WATER_BAND_1: Color = Color::srgb(0x80 as f32 / 255.0, 0xaf as f32 / 255.0, 0xb7 as f32 / 255.0);
pub const WATER_BAND_2: Color = Color::srgb(0x61 as f32 / 255.0, 0x9b as f32 / 255.0, 0xb2 as f32 / 255.0);
pub const WATER_BAND_3: Color = Color::srgb(0x51 as f32 / 255.0, 0x8e as f32 / 255.0, 0xb5 as f32 / 255.0);
pub const WATER_BAND_4: Color = Color::srgb(0x47 as f32 / 255.0, 0x81 as f32 / 255.0, 0xb2 as f32 / 255.0);
pub const WATER_BAND_5: Color = Color::srgb(0x3d as f32 / 255.0, 0x76 as f32 / 255.0, 0xa8 as f32 / 255.0);
/// Foam at the shoreline.
pub const FOAM: Color = Color::srgb(0xe6 as f32 / 255.0, 0xee as f32 / 255.0, 0xf0 as f32 / 255.0);

/// Rock band — top, lit edge. Drawn as the upper band of every rock with
/// a slightly rounded boundary so it reads as the highlight on a 3D form.
/// Also used for the lighter speckles on the big rock and rock-dust trails.
pub const ROCK_LIGHT: Color = Color::srgb(0x77 as f32 / 255.0, 0x85 as f32 / 255.0, 0x90 as f32 / 255.0);
/// Rock band — main mid-tone. The middle band on every rock body. The
/// rock textures bake this byte-for-byte (see `render::shapes`), so the
/// `Color` form is kept here for palette completeness only.
#[allow(dead_code)]
pub const ROCK: Color = Color::srgb(0x77 as f32 / 255.0, 0x68 as f32 / 255.0, 0x71 as f32 / 255.0);
/// Rock band — bottom, shadowed edge. Also used for shadow specks, rock
/// dust, and small dark silhouettes (flotsam, birds).
pub const ROCK_DARK: Color = Color::srgb(0x56 as f32 / 255.0, 0x4b as f32 / 255.0, 0x5a as f32 / 255.0);

/// Foragers hut — warm tan walls, deep brown roof, near-black door.
pub const HUT_WALL: Color = Color::srgb(0xa0 as f32 / 255.0, 0x7a as f32 / 255.0, 0x55 as f32 / 255.0);
pub const HUT_ROOF: Color = Color::srgb(0x5a as f32 / 255.0, 0x3a as f32 / 255.0, 0x22 as f32 / 255.0);
pub const HUT_DOOR: Color = Color::srgb(0x2a as f32 / 255.0, 0x1a as f32 / 255.0, 0x10 as f32 / 255.0);

/// Foragers cave — slightly cooler than the rocks so it reads as a
/// distinct landmark, with a near-black opening.
pub const CAVE_BODY: Color = Color::srgb(0x3a as f32 / 255.0, 0x3a as f32 / 255.0, 0x42 as f32 / 255.0);
pub const CAVE_OPENING: Color = Color::srgb(0x0e as f32 / 255.0, 0x06 as f32 / 255.0, 0x10 as f32 / 255.0);

/// Worker tunic — muted blue-gray.
pub const WORKER_BODY: Color = Color::srgb(0x4a as f32 / 255.0, 0x5a as f32 / 255.0, 0x70 as f32 / 255.0);
/// Miner outfit — rusty red-brown so miners read distinctly from workers.
pub const MINER_BODY: Color = Color::srgb(0x8a as f32 / 255.0, 0x4a as f32 / 255.0, 0x2a as f32 / 255.0);
/// Skimmer outfit — moss green.
pub const SKIMMER_BODY: Color = Color::srgb(0x4c as f32 / 255.0, 0x80 as f32 / 255.0, 0x40 as f32 / 255.0);
/// Fisherman outfit — sea-teal.
pub const FISHERMAN_BODY: Color = Color::srgb(0x2a as f32 / 255.0, 0x6a as f32 / 255.0, 0x80 as f32 / 255.0);
/// Beachcomber outfit — sandy earth tone. Skeleton color staged
/// for the Beachcomber crew feature; not used yet.
pub const BEACHCOMBER_BODY: Color = Color::srgb(0x8a as f32 / 255.0, 0x6e as f32 / 255.0, 0x46 as f32 / 255.0);
/// Stonemason outfit — stony grey-brown.
pub const STONEMASON_BODY: Color = Color::srgb(0x6c as f32 / 255.0, 0x5a as f32 / 255.0, 0x52 as f32 / 255.0);
/// Boatman outfit — navy blue.
pub const BOATMAN_BODY: Color = Color::srgb(0x24 as f32 / 255.0, 0x40 as f32 / 255.0, 0x6a as f32 / 255.0);
/// Pickaxe handle — dark wood.
pub const PICKAXE: Color = Color::srgb(0x3a as f32 / 255.0, 0x25 as f32 / 255.0, 0x10 as f32 / 255.0);
/// Fishing rod — slightly lighter wood than the pickaxe.
pub const FISHING_ROD: Color = Color::srgb(0x6a as f32 / 255.0, 0x46 as f32 / 255.0, 0x22 as f32 / 255.0);
/// Fishing line — pale, near-white. Semi-transparent so it reads as
/// a thin filament rather than a hard pixel line.
pub const FISHING_LINE: Color = Color::srgba(
    0xee as f32 / 255.0,
    0xee as f32 / 255.0,
    0xe0 as f32 / 255.0,
    0.55,
);

/// Tree trunk — dark brown rectangle.
pub const TREE_TRUNK: Color = Color::srgb(
    0x4a as f32 / 255.0,
    0x2c as f32 / 255.0,
    0x16 as f32 / 255.0,
);
/// Tree foliage main mass — muted forest green.
pub const TREE_FOLIAGE: Color = Color::srgb(
    0x35 as f32 / 255.0,
    0x6a as f32 / 255.0,
    0x32 as f32 / 255.0,
);
/// Tree foliage highlight — top-right lit edge.
pub const TREE_FOLIAGE_LIGHT: Color = Color::srgb(
    0x55 as f32 / 255.0,
    0x8e as f32 / 255.0,
    0x42 as f32 / 255.0,
);

/// Purchase-button background — a dim warm panel that reads against both
/// sand and water without competing with the rocks.
pub const BUTTON_BG: Color = Color::srgba(0.06, 0.07, 0.09, 0.78);
/// Disabled / unaffordable button — even dimmer.
pub const BUTTON_BG_DIM: Color = Color::srgba(0.03, 0.03, 0.04, 0.55);
/// Button background while the row is being hovered — slight cool tint
/// so the active row clearly stands out from siblings.
pub const BUTTON_BG_HOVER: Color = Color::srgba(0.14, 0.18, 0.22, 0.85);
/// Button border + label colour when affordable.
pub const BUTTON_BORDER: Color = Color::srgb(0xb0 as f32 / 255.0, 0xa8 as f32 / 255.0, 0x9f as f32 / 255.0);
/// Button label colour when not affordable.
pub const BUTTON_DIM_TEXT: Color = Color::srgb(0x60 as f32 / 255.0, 0x5d as f32 / 255.0, 0x58 as f32 / 255.0);

/// Detail-panel header when the action is currently affordable —
/// matches the "Buy!" green from gnorp-like panels.
pub const DETAIL_OK: Color = Color::srgb(0x6a as f32 / 255.0, 0xc8 as f32 / 255.0, 0x64 as f32 / 255.0);
/// Detail-panel header when the action is locked / can't afford yet.
pub const DETAIL_LOCKED: Color = Color::srgb(0xc8 as f32 / 255.0, 0x6a as f32 / 255.0, 0x4a as f32 / 255.0);

/// Darker red for shouted in-world labels — the fisherman's "miss",
/// the research scout's "!" briefing, and the scout's speech line.
/// Reads as urgent against sand and water without competing with the
/// brighter `DETAIL_LOCKED` used in the locked-row detail headers.
pub const MISS_RED: Color = Color::srgb(
    0x9a as f32 / 255.0,
    0x2c as f32 / 255.0,
    0x22 as f32 / 255.0,
);
