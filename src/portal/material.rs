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

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct PortalSurfaceMaterial {
    /// x: 1 = flat pink (end of the recursion), y: 1 = flip the sampled image left-right.
    #[uniform(0)]
    pub params: Vec4,
    #[texture(1)]
    #[sampler(2)]
    pub texture: Option<Handle<Image>>,
}

impl PortalSurfaceMaterial {
    pub fn pink() -> Self {
        Self { params: Vec4::new(1.0, 0.0, 0.0, 0.0), texture: None }
    }

    pub fn show(&mut self, texture: &Handle<Image>, mirror_u: bool) {
        self.params = Vec4::new(0.0, if mirror_u { 1.0 } else { 0.0 }, 0.0, 0.0);
        if self.texture.as_ref() != Some(texture) {
            self.texture = Some(texture.clone());
        }
    }

    pub fn show_pink(&mut self) {
        self.params = Vec4::new(1.0, 0.0, 0.0, 0.0);
    }
}

impl Material for PortalSurfaceMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/portal_surface.wesl".into()
    }

    fn enable_prepass() -> bool {
        false
    }

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
