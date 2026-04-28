// Rock fragment shader — directional sphere-bulge lighting.
//
// Each rock entity passes its current world-space Z rotation as a
// uniform. The fragment shader inverse-rotates the pixel offset back
// into world space, treats the silhouette interior as a unit-radius
// sphere bulging out toward the camera, and shades the resulting
// surface against a fixed top-right light source. The result is
// curved band boundaries that *follow* the rock's rotation: the
// highlight stays anchored to the world top-right no matter how
// fast the rock spins.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct RockLitParams {
    // Z-axis rotation of the rock in radians (Bevy's convention,
    // counter-clockwise positive). The shader inverse-rotates by
    // this to map UV space back into world space.
    rotation: f32,
    // Multiplier on the band color. 1.0 is default; masoned rocks
    // (sharpened by a stonemason) are lighter — driven from the
    // `Masoned` component on the Rust side.
    brightness: f32,
    _pad0: f32,
    _pad1: f32,
};

@group(2) @binding(0) var rock_tex: texture_2d<f32>;
@group(2) @binding(1) var rock_sampler: sampler;
@group(2) @binding(2) var<uniform> params: RockLitParams;

// Three-band rock palette — must match `colors::ROCK_LIGHT/MAIN/DARK`.
// Bytes / 255: 0x77=119/255, 0x85=133/255, 0x90=144/255, 0x68=104/255,
// 0x71=113/255, 0x56=86/255, 0x4b=75/255, 0x5a=90/255.
const BAND_LIGHT: vec3<f32> = vec3<f32>(0.4667, 0.5216, 0.5647);
const BAND_MAIN:  vec3<f32> = vec3<f32>(0.4667, 0.4078, 0.4431);
const BAND_DARK:  vec3<f32> = vec3<f32>(0.3373, 0.2941, 0.3529);

// Threshold tuning — light is the thinnest sliver, dark slightly
// thicker, main occupies the middle.
const LIGHT_THRESHOLD: f32 = 0.62;
const DARK_THRESHOLD:  f32 = 0.22;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Only run the lighting math on opaque silhouette pixels — fully
    // transparent UVs are outside the rock and discarded.
    let mask = textureSample(rock_tex, rock_sampler, in.uv);
    if (mask.a < 0.5) {
        discard;
    }

    // Centered UV in [-1, 1]. UV's y grows downward; we flip it so
    // +y is "up" in screen space, which matches the convention used
    // for the world-up light direction below.
    let centered = vec2<f32>(
        (in.uv.x - 0.5) * 2.0,
        (0.5 - in.uv.y) * 2.0,
    );

    // Inverse-rotate the offset by the rock's world rotation so that
    // the lighting calculation runs in world space — the highlight
    // ends up where the world's top-right is, regardless of how the
    // sprite is currently oriented.
    let cos_r = cos(-params.rotation);
    let sin_r = sin(-params.rotation);
    let world = vec2<f32>(
        centered.x * cos_r - centered.y * sin_r,
        centered.x * sin_r + centered.y * cos_r,
    );

    // Sphere-bulge approximation: treat the silhouette as a hemisphere
    // poking out of the screen. The "z" coord is how far the surface
    // pokes toward the camera, and the resulting surface normal gives
    // curved (not straight) iso-shading bands wrapping the rock.
    let r2 = world.x * world.x + world.y * world.y;
    let z = sqrt(max(0.0, 1.0 - r2));
    // Slight z bias keeps the highlight from clamping to a single
    // bright dot at very small rocks.
    let normal = normalize(vec3<f32>(world.x, world.y, z + 0.15));

    // Light direction — top-right, slightly forward. Normalised.
    let light = normalize(vec3<f32>(0.6, 0.6, 0.5));

    let shade = dot(normal, light);

    var color: vec3<f32>;
    if (shade > LIGHT_THRESHOLD) {
        color = BAND_LIGHT;
    } else if (shade > DARK_THRESHOLD) {
        color = BAND_MAIN;
    } else {
        color = BAND_DARK;
    }

    color = clamp(color * params.brightness, vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(color, mask.a);
}
