//! The material that shows a portal's view on its surface.
//!
//! The surface is sampled at the screen position of the fragment, so the image through the portal
//! lines up with the camera that is looking at it. Each view of the portal tree has its own
//! surface entities (on that view's render layer), because the same portal shows different images
//! to different cameras.

use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError};
use bevy::shader::ShaderRef;

/// A portal surface: shows a view camera's image (`shaders/portal_surface.wesl`), or the "end of
/// the recursion" fill. That fill is a dim violet void; it is called pink after the Unreal
/// plugin, which drew flat pink there.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct PortalSurfaceMaterial {
    /// x: 1 = the end-of-recursion fill, y: 1 = flip the sampled image left-right.
    #[uniform(0)]
    pub params: Vec4,
    /// The image of the view seen through this portal.
    #[texture(1)]
    #[sampler(2)]
    pub texture: Option<Handle<Image>>,
}

impl PortalSurfaceMaterial {
    /// A surface showing the end-of-recursion fill.
    pub fn pink() -> Self {
        Self { params: Vec4::new(1.0, 0.0, 0.0, 0.0), texture: None }
    }

    /// Show `texture`, flipped left-right if `mirror_u`.
    pub fn show(&mut self, texture: &Handle<Image>, mirror_u: bool) {
        self.params = Vec4::new(0.0, if mirror_u { 1.0 } else { 0.0 }, 0.0, 0.0);
        if self.texture.as_ref() != Some(texture) {
            self.texture = Some(texture.clone());
        }
    }

    /// Show the end-of-recursion fill (keeps the texture, which is ignored).
    pub fn show_pink(&mut self) {
        self.params = Vec4::new(1.0, 0.0, 0.0, 0.0);
    }
}

impl Material for PortalSurfaceMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/portal_surface.wesl".into()
    }

    /// No prepass: a surface is only an image of what lies beyond it.
    fn enable_prepass() -> bool {
        false
    }

    /// A portal casts no shadow.
    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None; // a portal is seen from both sides
        Ok(())
    }
}
