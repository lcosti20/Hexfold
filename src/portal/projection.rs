//! The player camera's projection: a perspective with a horizontal field of view, an optional
//! left-right flip (while the player is mirrored) and the near plane the portals need.

use bevy::camera::{CameraProjection, PerspectiveProjection, SubCameraView};
use bevy::math::Vec3A;
use bevy::prelude::*;

#[derive(Debug, Clone)]
pub struct PortalProjection {
    pub inner: PerspectiveProjection,
    /// Horizontal field of view in radians (kept as the window's aspect changes).
    pub fov_x: f32,
    /// Flip the image left-right. The camera must also have `Camera::invert_culling` set, because
    /// the flip reverses every triangle's winding.
    pub flip_x: bool,
}

impl Default for PortalProjection {
    fn default() -> Self {
        let fov_x = 90f32.to_radians();
        Self {
            inner: PerspectiveProjection { fov: 60f32.to_radians(), near: 0.01, far: 2000.0, ..default() },
            fov_x,
            flip_x: false,
        }
    }
}

impl PortalProjection {
    pub fn tan_half_y(&self) -> f32 {
        (self.inner.fov * 0.5).tan()
    }

    pub fn tan_half_x(&self) -> f32 {
        self.tan_half_y() * self.inner.aspect_ratio
    }
}

impl CameraProjection for PortalProjection {
    fn get_clip_from_view(&self) -> Mat4 {
        let mut m = self.inner.get_clip_from_view();
        if self.flip_x {
            m.x_axis.x = -m.x_axis.x;
        }
        m
    }

    fn get_clip_from_view_for_sub(&self, sub_view: &SubCameraView) -> Mat4 {
        let mut m = self.inner.get_clip_from_view_for_sub(sub_view);
        if self.flip_x {
            m.x_axis.x = -m.x_axis.x;
        }
        m
    }

    fn update(&mut self, width: f32, height: f32) {
        self.inner.update(width, height);
        // Keep the horizontal field of view fixed, like Unreal's default.
        let aspect = width / height;
        self.inner.fov = 2.0 * ((self.fov_x * 0.5).tan() / aspect).atan();
    }

    fn far(&self) -> f32 {
        self.inner.far
    }

    fn get_frustum_corners(&self, z_near: f32, z_far: f32) -> [Vec3A; 8] {
        self.inner.get_frustum_corners(z_near, z_far)
    }
}
