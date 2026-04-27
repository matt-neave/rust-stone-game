//! Custom 2D material for rock lighting (approach 3).
//!
//! Each rock is rendered as a 1×1 quad scaled to its sprite size,
//! with [`RockLitMaterial`] sampling the silhouette texture and
//! computing per-pixel directional shading in WGSL. The rock's
//! current Z rotation is fed in via the `params.rotation` uniform so
//! the shader can inverse-rotate UV coordinates back into world
//! space — that's what keeps the highlight anchored to the world
//! top-right while the rock spins.

use bevy::asset::Asset;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};

#[derive(Asset, AsBindGroup, Clone, TypePath)]
pub struct RockLitMaterial {
    /// Silhouette texture — only the alpha channel is read by the
    /// shader; the RGB content is ignored, since the shader supplies
    /// its own banded color from the lighting calculation.
    #[texture(0)]
    #[sampler(1)]
    pub silhouette: Handle<Image>,
    #[uniform(2)]
    pub params: RockLitParams,
}

#[derive(ShaderType, Clone, Copy, Default)]
pub struct RockLitParams {
    /// World-space Z rotation of the sprite, in radians. Updated each
    /// frame from the rock entity's `Transform`.
    pub rotation: f32,
    pub _pad0: f32,
    pub _pad1: f32,
    pub _pad2: f32,
}

impl Material2d for RockLitMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/rock_lit.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// Shared 1×1 quad mesh handle — every rock entity uses it and the
/// rock's render size is set via `Transform.scale`. One mesh asset
/// across the whole rock pile keeps the GPU upload count to one.
#[derive(Resource, Clone)]
pub struct RockQuad(pub Handle<Mesh>);

pub struct RockMaterialPlugin;

impl Plugin for RockMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<RockLitMaterial>::default())
            .add_systems(PreStartup, init_rock_quad)
            .add_systems(Update, sync_rock_rotation);
    }
}

fn init_rock_quad(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    let mesh = meshes.add(Rectangle::new(1.0, 1.0));
    commands.insert_resource(RockQuad(mesh));
}

/// Mirror each rock's `Transform.rotation` into its
/// `RockLitMaterial`'s `rotation` uniform so the shader-side world
/// inverse-rotation is always one frame fresh.
fn sync_rock_rotation(
    rocks: Query<(&Transform, &MeshMaterial2d<RockLitMaterial>)>,
    mut materials: ResMut<Assets<RockLitMaterial>>,
) {
    for (tf, mat_handle) in &rocks {
        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            // Bevy's quat → euler returns Z first when `EulerRot::ZYX`.
            // Rocks only ever rotate around Z, so this is exact.
            let (z, _, _) = tf.rotation.to_euler(EulerRot::ZYX);
            if (mat.params.rotation - z).abs() > 1e-4 {
                mat.params.rotation = z;
            }
        }
    }
}
