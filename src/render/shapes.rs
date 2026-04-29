//! Pre-baked shape texture masks. Bevy's `Sprite::from_color` is a hard
//! rectangle, so we bake a few rounded-rect and circle masks at startup and
//! tint them via the sprite's color field. This is the same pattern as
//! `rust-SNKRX/src/shapes.rs`.

use bevy::asset::RenderAssetUsages;
use bevy::image::{Image, ImageSampler};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Three-band rock palette baked into the rock textures. Light at the
/// top-right corner of every rock, main across the bulk of the body,
/// dark wrapping the bottom-left where the implicit top-right light
/// source can't reach.
/// Bytes match `colors::ROCK_LIGHT`, `colors::ROCK`, `colors::ROCK_DARK`.
const ROCK_BAND_LIGHT: [u8; 4] = [0x77, 0x85, 0x90, 0xff];
const ROCK_BAND_MAIN: [u8; 4] = [0x77, 0x68, 0x71, 0xff];
const ROCK_BAND_DARK: [u8; 4] = [0x56, 0x4b, 0x5a, 0xff];

/// Light-band threshold against the `rx · ry²` field. Tuned so the
/// highlight reads as a thin curve hugging the rock's top shoulder
/// after the sun-angle tilt — peak field on a circular rim is ~0.65,
/// so this picks roughly the upper third of the lit side.
const ROCK_LIGHT_THRESHOLD: f32 = 0.28;
/// Dark-band threshold on the same field — pixels below this fall in
/// the shadow band. With the sun closer to overhead the shadow leans
/// along the bottom edge, anchoring the bottom-left corner.
const ROCK_DARK_THRESHOLD: f32 = 0.05;

/// Small rock shape variants. Every variant is a true circle or ellipse so
/// the silhouettes all read as smooth pebbles — no rounded squares, no
/// pills with flat sides. Variety comes from size + aspect ratio rather
/// than corner styling. SNKRX-style smooth-pebble feel at every angle.
#[derive(Clone, Copy, Debug)]
pub enum SmallRockShape {
    /// Circle 8.
    RoundSmall,
    /// Circle 9.
    Round,
    /// Circle 10.
    RoundLarge,
    /// 11×8 horizontal ellipse — slightly wider than tall.
    OvalH,
    /// 8×11 vertical ellipse — slightly taller than wide.
    OvalV,
}

impl SmallRockShape {
    pub const ALL: [SmallRockShape; 5] = [
        SmallRockShape::RoundSmall,
        SmallRockShape::Round,
        SmallRockShape::RoundLarge,
        SmallRockShape::OvalH,
        SmallRockShape::OvalV,
    ];

    /// Native size in spec px — the sprite is rendered at this size.
    pub fn size(self) -> Vec2 {
        match self {
            SmallRockShape::RoundSmall => Vec2::new(8.0, 8.0),
            SmallRockShape::Round => Vec2::new(9.0, 9.0),
            SmallRockShape::RoundLarge => Vec2::new(10.0, 10.0),
            SmallRockShape::OvalH => Vec2::new(11.0, 8.0),
            SmallRockShape::OvalV => Vec2::new(8.0, 11.0),
        }
    }
}

#[derive(Resource, Default, Clone)]
pub struct Shapes {
    /// Small-rock variants — all true circles or ellipses. Bands are
    /// baked in directionally (top-right light), so a single image per
    /// shape suffices — variety comes from the silhouette and how the
    /// fixed light hits each shape.
    pub small_rock_round_small: Handle<Image>,
    pub small_rock_round: Handle<Image>,
    pub small_rock_round_large: Handle<Image>,
    pub small_rock_oval_h: Handle<Image>,
    pub small_rock_oval_v: Handle<Image>,
    /// Masoned (polished) variants — same silhouettes baked with a
    /// brightened band palette. Stonemasons swap a rock's sprite image
    /// to one of these on completion.
    pub small_rock_round_small_lit: Handle<Image>,
    pub small_rock_round_lit: Handle<Image>,
    pub small_rock_round_large_lit: Handle<Image>,
    pub small_rock_oval_h_lit: Handle<Image>,
    pub small_rock_oval_v_lit: Handle<Image>,
    /// Large rounded boulder body — irregular blob mask, same banding.
    pub big_rock: Handle<Image>,
    /// Soft elliptical shadow used beneath every rock. Single image,
    /// resized per-rock at spawn via `Sprite::custom_size`.
    pub shadow: Handle<Image>,
    /// 32-px diameter circle — used for ripples.
    pub circle: Handle<Image>,
    /// 16-px diameter circle.
    #[allow(dead_code)]
    pub circle_small: Handle<Image>,
    /// 4×4 unrounded — generic pixel block, reserved.
    #[allow(dead_code)]
    pub speck: Handle<Image>,
    /// 7×3 seagull silhouette — wings up frame.
    pub bird_up: Handle<Image>,
    /// 7×3 seagull silhouette — wings down frame. Alternated with `bird_up`
    /// at ~3 Hz so birds visibly flap as they cross the canvas.
    pub bird_down: Handle<Image>,
    /// ~80×24 fluffy cloud blob — used as a soft tinted shadow that drifts
    /// across the scene. Built from overlapping circles via `boulder_image`.
    pub cloud_shadow: Handle<Image>,
    /// Per-shape imprint variants — each one pixel smaller in radius
    /// than the matching small-rock silhouette so the dent reads as the
    /// rock's footprint sunk slightly into the sand.
    pub imprint_round_small: Handle<Image>,
    pub imprint_round: Handle<Image>,
    pub imprint_round_large: Handle<Image>,
    pub imprint_oval_h: Handle<Image>,
    pub imprint_oval_v: Handle<Image>,
    /// 22×8 triangular roof for the foragers hut.
    pub hut_roof: Handle<Image>,
    /// 16×9 crab silhouette used for every crew role. Body, claws and legs
    /// are baked white so the sprite's `color` field tints them per role;
    /// the two eyes are baked black so they read as eyes against any tint.
    pub humanoid: Handle<Image>,
    /// Walk-frame crab — legs shifted one column right of the stand pose.
    pub humanoid_walk: Handle<Image>,
    /// 4×4 pickaxe silhouette — diagonal handle with a head at the top.
    pub pickaxe: Handle<Image>,
    /// 5×5 fishing rod — diagonal stick from lower-left handle up to
    /// upper-right tip.
    pub fishing_rod: Handle<Image>,
    /// Cave silhouette — irregular dark mound, ~20×16.
    pub cave_body: Handle<Image>,
    /// Cave opening — small dark ellipse drawn over the lower body.
    pub cave_opening: Handle<Image>,
    /// Big tree foliage — irregular green blob, ~32×28.
    pub tree_foliage: Handle<Image>,
    /// Tree foliage highlight — slightly smaller blob baked off-center
    /// so the lit side reads brighter.
    pub tree_foliage_light: Handle<Image>,
    /// 6×3 horizontally-elongated rounded log silhouette — used by
    /// freshly-spawned wood pieces so they read as little tree chunks
    /// rather than featureless brown squares.
    pub log: Handle<Image>,
}

impl Shapes {
    /// Pick the right pre-baked image for a given shape variant.
    pub fn small_rock_image(&self, shape: SmallRockShape) -> Handle<Image> {
        match shape {
            SmallRockShape::RoundSmall => self.small_rock_round_small.clone(),
            SmallRockShape::Round => self.small_rock_round.clone(),
            SmallRockShape::RoundLarge => self.small_rock_round_large.clone(),
            SmallRockShape::OvalH => self.small_rock_oval_h.clone(),
            SmallRockShape::OvalV => self.small_rock_oval_v.clone(),
        }
    }

    /// Masoned variant of [`small_rock_image`] — the same silhouette
    /// with a brighter baked palette, used after a stonemason polishes
    /// the rock.
    pub fn small_rock_image_lit(&self, shape: SmallRockShape) -> Handle<Image> {
        match shape {
            SmallRockShape::RoundSmall => self.small_rock_round_small_lit.clone(),
            SmallRockShape::Round => self.small_rock_round_lit.clone(),
            SmallRockShape::RoundLarge => self.small_rock_round_large_lit.clone(),
            SmallRockShape::OvalH => self.small_rock_oval_h_lit.clone(),
            SmallRockShape::OvalV => self.small_rock_oval_v_lit.clone(),
        }
    }

    /// Imprint silhouette for a given small-rock shape — same outline,
    /// one pixel smaller in radius along each axis.
    pub fn imprint_image(&self, shape: SmallRockShape) -> Handle<Image> {
        match shape {
            SmallRockShape::RoundSmall => self.imprint_round_small.clone(),
            SmallRockShape::Round => self.imprint_round.clone(),
            SmallRockShape::RoundLarge => self.imprint_round_large.clone(),
            SmallRockShape::OvalH => self.imprint_oval_h.clone(),
            SmallRockShape::OvalV => self.imprint_oval_v.clone(),
        }
    }

    /// Render size for an imprint — matches the imprint mask, two pixels
    /// smaller than the source rock along each axis (one px smaller in
    /// radius), with a sane lower bound so tiny shapes still draw.
    pub fn imprint_size(shape: SmallRockShape) -> Vec2 {
        let s = shape.size();
        Vec2::new((s.x - 2.0).max(2.0), (s.y - 2.0).max(2.0))
    }
}

pub struct ShapesPlugin;

impl Plugin for ShapesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Shapes>()
            .add_systems(PreStartup, build_shapes);
    }
}

fn build_shapes(mut shapes: ResMut<Shapes>, mut images: ResMut<Assets<Image>>) {
    // All small-rock variants are pure circles or ellipses — no flat
    // sides. Bands are baked in with directional lighting (top-right
    // source) so every rock shares the same coherent shading.
    shapes.small_rock_round_small = images.add(circle_image_banded(8, 1.0));
    shapes.small_rock_round = images.add(circle_image_banded(9, 1.0));
    shapes.small_rock_round_large = images.add(circle_image_banded(10, 1.0));
    shapes.small_rock_oval_h = images.add(ellipse_image_banded(11, 8, 1.0));
    shapes.small_rock_oval_v = images.add(ellipse_image_banded(8, 11, 1.0));

    // Masoned (polished) variants — same silhouettes, brighter palette.
    shapes.small_rock_round_small_lit = images.add(circle_image_banded(8, MASONED_BRIGHTNESS));
    shapes.small_rock_round_lit = images.add(circle_image_banded(9, MASONED_BRIGHTNESS));
    shapes.small_rock_round_large_lit = images.add(circle_image_banded(10, MASONED_BRIGHTNESS));
    shapes.small_rock_oval_h_lit = images.add(ellipse_image_banded(11, 8, MASONED_BRIGHTNESS));
    shapes.small_rock_oval_v_lit = images.add(ellipse_image_banded(8, 11, MASONED_BRIGHTNESS));

    // (Big rock still uses the live `RockLitMaterial` shader so its
    // highlight stays anchored to world top-right while it spins; the
    // banded helper below takes the same brightness=1.0 default.)

    // Big rock — irregular boulder silhouette built from a few
    // overlapping circles. Wider than tall, with an asymmetric top
    // profile so it reads as a real rock rather than a rounded square.
    shapes.big_rock = images.add(boulder_image_banded(
        60,
        48,
        &[
            (30.0, 28.0, 22.0), // main mass
            (40.0, 13.0, 13.0), // upper-right hump
            (47.0, 28.0, 13.0), // right shoulder
            (15.0, 32.0, 14.0), // lower-left lobe
            (22.0, 16.0, 9.0),  // small upper-left bump for character
        ],
    ));

    // Soft drop-shadow ellipse used by every rock. Stored once and resized
    // via `custom_size` per rock in `rocks::shadow`.
    shapes.shadow = images.add(ellipse_image(20, 6));

    shapes.circle = images.add(circle_image(32));
    shapes.circle_small = images.add(circle_image(16));
    shapes.speck = images.add(rounded_rect_image(2, 2, 0));

    // Bird silhouettes — two frames so birds visibly flap. 7×3 native size
    // (a 5×3 V was too small and read as a chunk). Wings up = mid-upstroke,
    // wings down = mid-downstroke; alternated by `tick_birds`.
    shapes.bird_up = images.add(pattern_image(&[
        "X.....X",
        ".X...X.",
        "..XXX..",
    ]));
    shapes.bird_down = images.add(pattern_image(&[
        "..XXX..",
        ".X...X.",
        "X.....X",
    ]));

    // Sand imprints — one variant per rock shape, each one pixel smaller
    // in radius than the source so the dent reads as the rock's footprint.
    // Lower bound of 2 keeps tiny shapes from collapsing to nothing.
    shapes.imprint_round_small = images.add(circle_image(6));
    shapes.imprint_round = images.add(circle_image(7));
    shapes.imprint_round_large = images.add(circle_image(8));
    shapes.imprint_oval_h = images.add(ellipse_image(9, 6));
    shapes.imprint_oval_v = images.add(ellipse_image(6, 9));

    // Hut roof — triangular silhouette, 22 wide × 8 tall. Drawn over the
    // hut body rectangle.
    shapes.hut_roof = images.add(pattern_image(&[
        "..........XX..........",
        ".........XXXX.........",
        "........XXXXXX........",
        ".......XXXXXXXX.......",
        "......XXXXXXXXXX......",
        ".....XXXXXXXXXXXX.....",
        "....XXXXXXXXXXXXXX....",
        "...XXXXXXXXXXXXXXXX...",
    ]));

    // Crab — fully-coloured pixel art. See `CRAB_STAND` / `CRAB_WALK` for
    // the per-pixel matrices and `crab_image` for the colour mapping.
    shapes.humanoid = images.add(crab_image(&CRAB_STAND));
    shapes.humanoid_walk = images.add(crab_image(&CRAB_WALK));

    // Pickaxe — 4×4 with the head in the upper-right and a handle running
    // diagonally toward the lower-left.
    shapes.pickaxe = images.add(pattern_image(&[
        "..XX",
        ".XX.",
        "XX..",
        "X...",
    ]));

    // Fishing rod — 5×5 diagonal from lower-left handle to upper-right
    // tip. Tinted in `crew.rs` and rotated based on the fisherman's
    // current state for cast/reel animation.
    shapes.fishing_rod = images.add(pattern_image(&[
        "....X",
        "...X.",
        "..X..",
        ".X...",
        "X....",
    ]));

    // Cave — bumpy mound with three lobes for an irregular silhouette.
    // Same multi-circle approach as the big rock, just smaller and
    // wider than tall so it reads as a low entrance carved into a rock.
    shapes.cave_body = images.add(boulder_image(
        20,
        16,
        &[
            (10.0, 10.0, 7.0),
            (4.0, 10.0, 4.5),
            (16.0, 9.0, 5.0),
            (10.0, 5.0, 4.0),
        ],
    ));
    // Opening — a 7×5 dark ellipse drawn over the lower-centre of the
    // body. Reads as the mouth of the cave against the lighter rock.
    shapes.cave_opening = images.add(ellipse_image(7, 5));

    // Tree foliage — overlapping circles for an organic blob. The
    // larger blob is the main canopy; the smaller "light" image
    // stamps the upper-right with a brighter green.
    shapes.tree_foliage = images.add(boulder_image(
        32,
        28,
        &[
            (16.0, 14.0, 11.0),
            (10.0, 10.0, 8.0),
            (22.0, 11.0, 8.0),
            (16.0, 6.0, 7.0),
            (12.0, 18.0, 7.0),
            (22.0, 18.0, 6.5),
        ],
    ));
    // Log silhouette — short rounded bar so wood pieces read as
    // little fallen logs rather than 3×3 squares. Tinted with
    // TREE_TRUNK at the sprite-color step.
    shapes.log = images.add(pattern_image(&[
        ".XXXX.",
        "XXXXXX",
        ".XXXX.",
    ]));

    shapes.tree_foliage_light = images.add(boulder_image(
        32,
        28,
        &[
            (20.0, 9.0, 6.0),
            (24.0, 12.0, 5.0),
            (16.0, 7.0, 4.5),
        ],
    ));

    // Cloud shadow — irregular fluffy blob. Same multi-circle approach as the
    // big rock, just stretched into a horizontally-elongated cloud shape.
    shapes.cloud_shadow = images.add(boulder_image(
        80,
        24,
        &[
            (40.0, 12.0, 12.0),
            (25.0, 14.0, 9.0),
            (55.0, 11.0, 11.0),
            (15.0, 12.0, 7.0),
            (65.0, 13.0, 8.0),
            (45.0, 8.0, 8.0),
        ],
    ));
}

// ---------------------------------------------------------------------------
// Crab sprite (replaces the old humanoid). Two 16×9 frames — `CRAB_STAND` is
// the resting pose; `CRAB_WALK` shifts the legs one column to the right so the
// crab visibly steps when alternated. Pixel codes:
//   0 = transparent
//   1 = body / claw-top  (CRAB_BODY)
//   2 = claw-shadow / leg (CRAB_SHADOW)
//   3 = eye               (CRAB_EYE)
// Colour-coding lives in `crab_image`.
pub const CRAB_W: usize = 16;
pub const CRAB_H: usize = 9;
pub type CrabPattern = [[u8; CRAB_W]; CRAB_H];

const CRAB_BODY: [u8; 4] = [0xb0, 0x38, 0x38, 0xff];
const CRAB_SHADOW: [u8; 4] = [0x6b, 0x24, 0x24, 0xff];
const CRAB_EYE: [u8; 4] = [0x00, 0x00, 0x00, 0xff];

#[rustfmt::skip]
const CRAB_STAND: CrabPattern = [
    [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
    [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
    [0,1,0,0,0,0,0,0,0,0,0,1,0,0,0,0],
    [0,1,0,0,0,0,0,0,0,0,0,1,0,0,0,0],
    [0,0,2,0,3,0,0,0,3,0,2,0,0,0,0,0],
    [0,0,2,0,3,1,1,1,3,0,2,0,0,0,0,0],
    [0,0,0,1,1,1,1,1,1,1,0,0,0,0,0,0],
    [0,0,0,1,1,1,1,1,1,1,0,0,0,0,0,0],
    [0,0,2,0,2,0,0,0,2,0,2,0,0,0,0,0],
];

#[rustfmt::skip]
const CRAB_WALK: CrabPattern = [
    [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
    [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
    [0,1,0,0,0,0,0,0,0,0,0,1,0,0,0,0],
    [0,1,0,0,0,0,0,0,0,0,0,1,0,0,0,0],
    [0,0,2,0,3,0,0,0,3,0,2,0,0,0,0,0],
    [0,0,2,0,3,1,1,1,3,0,2,0,0,0,0,0],
    [0,0,0,1,1,1,1,1,1,1,0,0,0,0,0,0],
    [0,0,0,1,1,1,1,1,1,1,0,0,0,0,0,0],
    [0,0,0,2,0,2,0,0,0,2,0,2,0,0,0,0],
];

/// Bake a `CRAB_W × CRAB_H` colour image from a pixel-code matrix.
pub fn crab_image(pattern: &CrabPattern) -> Image {
    let mut data = vec![0u8; CRAB_W * CRAB_H * 4];
    for (y, row) in pattern.iter().enumerate() {
        for (x, &code) in row.iter().enumerate() {
            let rgba = match code {
                1 => CRAB_BODY,
                2 => CRAB_SHADOW,
                3 => CRAB_EYE,
                _ => continue,
            };
            let idx = (y * CRAB_W + x) * 4;
            data[idx..idx + 4].copy_from_slice(&rgba);
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: CRAB_W as u32,
            height: CRAB_H as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    image
}

/// Bake a white mask from a 2D character grid: `'.'` → transparent, anything
/// else → opaque white. Lets us declare tiny pixel sprites inline (birds,
/// glyphs, etc) without hand-rolling pixel arrays each time.
pub fn pattern_image(pattern: &[&str]) -> Image {
    let h = pattern.len() as u32;
    let w = pattern[0].chars().count() as u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for (y, row) in pattern.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            if ch != '.' {
                let idx = ((y as u32 * w + x as u32) * 4) as usize;
                data[idx..idx + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    image
}

pub fn rounded_rect_image(w: u32, h: u32, radius: u32) -> Image {
    let pixel_count = (w * h) as usize;
    let mut data = vec![0u8; pixel_count * 4];
    for y in 0..h {
        for x in 0..w {
            if inside_rounded_rect(x, y, w, h, radius) {
                let idx = ((y * w + x) * 4) as usize;
                data[idx] = 255;
                data[idx + 1] = 255;
                data[idx + 2] = 255;
                data[idx + 3] = 255;
            }
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    image
}

/// Build a white mask by unioning a list of `(cx, cy, radius)` circles in
/// the bounding box. Pixels inside any circle are opaque white; the rest
/// are transparent. Used to bake irregular blob silhouettes (the big rock).
pub fn boulder_image(w: u32, h: u32, blobs: &[(f32, f32, f32)]) -> Image {
    let pixel_count = (w * h) as usize;
    let mut data = vec![0u8; pixel_count * 4];
    for y in 0..h {
        for x in 0..w {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let inside = blobs.iter().any(|&(cx, cy, r)| {
                let dx = px - cx;
                let dy = py - cy;
                dx * dx + dy * dy <= r * r
            });
            if inside {
                let idx = ((y * w + x) * 4) as usize;
                data[idx] = 255;
                data[idx + 1] = 255;
                data[idx + 2] = 255;
                data[idx + 3] = 255;
            }
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    image
}

/// Build a white mask of a filled axis-aligned ellipse fitting `w × h`.
/// Same nearest-pixel sampling as the other mask builders. For square
/// dimensions this matches `circle_image`; for non-square it produces a
/// smooth oval rather than a max-radius rounded rect (which would still
/// have flat top/bottom or left/right edges on the long axis).
pub fn ellipse_image(w: u32, h: u32) -> Image {
    let pixel_count = (w * h) as usize;
    let mut data = vec![0u8; pixel_count * 4];
    let cx = w as f32 * 0.5 - 0.5;
    let cy = h as f32 * 0.5 - 0.5;
    let rx = w as f32 * 0.5;
    let ry = h as f32 * 0.5;
    for y in 0..h {
        for x in 0..w {
            let dx = (x as f32 - cx) / rx;
            let dy = (y as f32 - cy) / ry;
            if dx * dx + dy * dy <= 1.0 {
                let idx = ((y * w + x) * 4) as usize;
                data[idx] = 255;
                data[idx + 1] = 255;
                data[idx + 2] = 255;
                data[idx + 3] = 255;
            }
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    image
}

pub fn circle_image(diameter: u32) -> Image {
    let pixel_count = (diameter * diameter) as usize;
    let mut data = vec![0u8; pixel_count * 4];
    let center = diameter as f32 * 0.5 - 0.5;
    let r = diameter as f32 * 0.5;
    let r_sq = r * r;
    for y in 0..diameter {
        for x in 0..diameter {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            if dx * dx + dy * dy <= r_sq {
                let idx = ((y * diameter + x) * 4) as usize;
                data[idx] = 255;
                data[idx + 1] = 255;
                data[idx + 2] = 255;
                data[idx + 3] = 255;
            }
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: diameter,
            height: diameter,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    image
}

/// Brightness multiplier baked into the masoned (polished) rock
/// textures. Matches the old shader-uniform value `MASONED_BRIGHTNESS`
/// in `crew::stonemason` so the visible "polish" lift is identical.
pub const MASONED_BRIGHTNESS: f32 = 1.45;

/// Pick which of the three rock bands a pixel falls into using a fixed
/// top-right directional light. For each pixel we project its offset
/// from the rock's center onto the light direction L = (1, -1)/√2
/// (right is +x, "up" is -y in the spec's Y-down system) and normalise
/// to `[0, 1]`. High projection → top-right of the rock → highlight
/// band; low projection → bottom-left → dark band.
///
/// Bands are sized asymmetrically per the design: the highlight is the
/// thinnest sliver, the dark band slightly thicker (the shadow wraps
/// further around the bottom-left), and the main mid-tone takes the
/// rest of the body.
///
/// `brightness` multiplies each band's RGB channels (clamped at 255)
/// so the banded helpers can bake a polished/lit variant without
/// duplicating the geometry pass.
fn rock_band_rgba(x: u32, y: u32, w: u32, h: u32, brightness: f32) -> [u8; 4] {
    // Hyperbolic lighting field: brightness ∝ rx · ry², where (rx, ry)
    // are normalised pixel coordinates measured from the rock's
    // bottom-left corner (rx grows rightward, ry grows upward).
    //
    // The squared `ry` term tilts the sun upward: at the top-right
    // corner the gradient is `(1, 2)`, so the lit direction sits at
    // `atan(2) ≈ 63.4°` above horizontal — a high-angle sun closer to
    // overhead than the 45° corner direction of plain `rx · ry`.
    //
    // Iso-bands belong to the `xy² = c` family — the band edges run
    // nearly flat along the rock's top edge and steepen toward the
    // bottom, keeping the "1/x parabola" feel rotated to match the
    // higher sun.
    let rx = (x as f32 + 0.5) / w as f32;
    let ry = (h as f32 - 0.5 - y as f32) / h as f32;
    let norm = (rx * ry * ry).clamp(0.0, 1.0);
    let band = if norm > ROCK_LIGHT_THRESHOLD {
        ROCK_BAND_LIGHT
    } else if norm > ROCK_DARK_THRESHOLD {
        ROCK_BAND_MAIN
    } else {
        ROCK_BAND_DARK
    };
    if (brightness - 1.0).abs() < f32::EPSILON {
        band
    } else {
        let scale = |c: u8| ((c as f32 * brightness).round().clamp(0.0, 255.0)) as u8;
        [scale(band[0]), scale(band[1]), scale(band[2]), band[3]]
    }
}

/// Banded variant of [`circle_image`] — same silhouette, but each inside
/// pixel is filled with one of the three rock-band colors via the
/// directional lighting model.
pub fn circle_image_banded(diameter: u32, brightness: f32) -> Image {
    let pixel_count = (diameter * diameter) as usize;
    let mut data = vec![0u8; pixel_count * 4];
    let center = diameter as f32 * 0.5 - 0.5;
    let r = diameter as f32 * 0.5;
    let r_sq = r * r;
    for y in 0..diameter {
        for x in 0..diameter {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            if dx * dx + dy * dy <= r_sq {
                let rgba = rock_band_rgba(x, y, diameter, diameter, brightness);
                let idx = ((y * diameter + x) * 4) as usize;
                data[idx..idx + 4].copy_from_slice(&rgba);
            }
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: diameter,
            height: diameter,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    image
}

/// Banded variant of [`ellipse_image`].
pub fn ellipse_image_banded(w: u32, h: u32, brightness: f32) -> Image {
    let pixel_count = (w * h) as usize;
    let mut data = vec![0u8; pixel_count * 4];
    let cx = w as f32 * 0.5 - 0.5;
    let cy = h as f32 * 0.5 - 0.5;
    let rx = w as f32 * 0.5;
    let ry = h as f32 * 0.5;
    for y in 0..h {
        for x in 0..w {
            let dx = (x as f32 - cx) / rx;
            let dy = (y as f32 - cy) / ry;
            if dx * dx + dy * dy <= 1.0 {
                let rgba = rock_band_rgba(x, y, w, h, brightness);
                let idx = ((y * w + x) * 4) as usize;
                data[idx..idx + 4].copy_from_slice(&rgba);
            }
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    image
}

/// Banded variant of [`boulder_image`].
pub fn boulder_image_banded(w: u32, h: u32, blobs: &[(f32, f32, f32)]) -> Image {
    let pixel_count = (w * h) as usize;
    let mut data = vec![0u8; pixel_count * 4];
    for y in 0..h {
        for x in 0..w {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let inside = blobs.iter().any(|&(cx, cy, r)| {
                let dx = px - cx;
                let dy = py - cy;
                dx * dx + dy * dy <= r * r
            });
            if inside {
                let rgba = rock_band_rgba(x, y, w, h, 1.0);
                let idx = ((y * w + x) * 4) as usize;
                data[idx..idx + 4].copy_from_slice(&rgba);
            }
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    image
}

fn inside_rounded_rect(x: u32, y: u32, w: u32, h: u32, radius: u32) -> bool {
    if radius == 0 {
        return true;
    }
    let px = x as f32 + 0.5;
    let py = y as f32 + 0.5;
    let r = radius as f32;
    let in_arc = |cx: f32, cy: f32| -> bool {
        let dx = px - cx;
        let dy = py - cy;
        dx * dx + dy * dy <= r * r
    };
    if x < radius && y < radius {
        return in_arc(r, r);
    }
    if x >= w - radius && y < radius {
        return in_arc((w - radius) as f32, r);
    }
    if x < radius && y >= h - radius {
        return in_arc(r, (h - radius) as f32);
    }
    if x >= w - radius && y >= h - radius {
        return in_arc((w - radius) as f32, (h - radius) as f32);
    }
    true
}
