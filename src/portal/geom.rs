//! Portal geometry: planes, rectangles and the warp between two linked portals.
//!
//! A portal's surface lies in its local XY plane (Unreal's XZ), centred on the entity, and
//! spans +/-`half_extent` before the entity's scale. Its front normal is local -Z (`forward()`).

use bevy::prelude::*;
use std::collections::HashMap;

/// A seamless portal surface. Link both sides with [`connect`] (or set the links yourself).
///
/// Looking or walking through either side warps space by `target.matrix * self.matrix.inverse()`:
/// a point keeps its coordinates relative to the portal, so entering one portal's front leaves
/// through the target's back. A size difference between the two (uniform scale) resizes whatever
/// passes through, and a negative X scale on one of them mirrors it.
///
/// Portals must be root entities (no parent): their `Transform` is their world transform.
#[derive(Component, Debug, Clone)]
pub struct Portal {
    /// Where you go entering from the front (the side the normal points to).
    pub front_link: Option<Entity>,
    /// Where you go entering from the back.
    pub back_link: Option<Entity>,
    /// Half size of the rectangle in local X and Y, before scale.
    pub half_extent: Vec2,
}

impl Default for Portal {
    fn default() -> Self {
        Self { front_link: None, back_link: None, half_extent: Vec2::ONE }
    }
}

/// Space an entity belongs to (an actor tagged `PortalSpace.<n>` in the Unreal plugin).
/// Entities without one appear in every space.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PortalSpace(pub i32);

/// Links both sides of `a` to `b` and both sides of `b` to `a`.
pub fn connect(portals: &mut Query<&mut Portal>, a: Entity, b: Entity) {
    if let Ok(mut p) = portals.get_mut(a) {
        p.front_link = Some(b);
        p.back_link = Some(b);
    }
    if let Ok(mut p) = portals.get_mut(b) {
        p.front_link = Some(a);
        p.back_link = Some(a);
    }
}

/// A portal's world-space data for one frame.
#[derive(Debug, Clone)]
pub struct PortalGeom {
    pub entity: Entity,
    pub transform: Transform,
    pub half_extent: Vec2,
    pub mat: Mat4,
    pub inv: Mat4,
    pub pos: Vec3,
    /// Unit normal of the front side.
    pub normal: Vec3,
    /// Half-extent axes in world space (include scale).
    pub ax: Vec3,
    pub ay: Vec3,
    /// Uniform size of the portal (|scale.y|).
    pub scale: f32,
    pub front: Option<Entity>,
    pub back: Option<Entity>,
    pub space: Option<i32>,
}

impl PortalGeom {
    pub fn new(entity: Entity, portal: &Portal, t: &Transform, space: Option<i32>) -> Self {
        let mat = t.to_matrix();
        let rot = t.rotation;
        Self {
            entity,
            transform: *t,
            half_extent: portal.half_extent,
            mat,
            inv: mat.inverse(),
            pos: t.translation,
            normal: rot * Vec3::NEG_Z,
            ax: rot * Vec3::X * (t.scale.x * portal.half_extent.x),
            ay: rot * Vec3::Y * (t.scale.y * portal.half_extent.y),
            scale: t.scale.y.abs(),
            front: portal.front_link,
            back: portal.back_link,
            space,
        }
    }

    /// Signed distance from the portal plane (positive on the front side).
    pub fn signed_distance(&self, p: Vec3) -> f32 {
        (p - self.pos).dot(self.normal)
    }

    /// The portal a viewer at `p` looks or walks into.
    pub fn link_for(&self, p: Vec3) -> Option<Entity> {
        let d = self.signed_distance(p);
        if d > 0.0 {
            self.front
        } else if d < 0.0 {
            self.back
        } else {
            None
        }
    }

    /// True if the segment from -> to crosses the rectangle, with the plane moved by `bump`.
    pub fn intersects(&self, from: Vec3, to: Vec3, bump: Vec3) -> bool {
        let p = self.pos + bump;
        let da = (from - p).dot(self.normal);
        let db = (to - p).dot(self.normal);
        if da * db > 0.0 || da == db {
            return false;
        }
        let hit = from + (to - from) * (da / (da - db)) - p;
        hit.dot(self.ax).abs() < self.ax.length_squared() && hit.dot(self.ay).abs() < self.ay.length_squared()
    }

    /// Distance from `p` to the portal rectangle.
    pub fn distance_to(&self, p: Vec3) -> f32 {
        let v = p - self.pos;
        let px = (v.dot(self.ax) / self.ax.length_squared()).clamp(-1.0, 1.0);
        let py = (v.dot(self.ay) / self.ay.length_squared()).clamp(-1.0, 1.0);
        (v - (self.ax * px + self.ay * py)).length()
    }

    pub fn corners(&self) -> [Vec3; 4] {
        [
            self.pos - self.ax - self.ay,
            self.pos + self.ax - self.ay,
            self.pos + self.ax + self.ay,
            self.pos - self.ax + self.ay,
        ]
    }

    /// Matrix taking points seen through this portal into `target`'s space.
    pub fn warp_to(&self, target: &PortalGeom) -> Mat4 {
        target.mat * self.inv
    }

    pub fn is_mirrored_link_to(&self, target: &PortalGeom) -> bool {
        self.warp_to(target).determinant() < 0.0
    }
}

/// Every portal in play, refreshed once a frame (before traversal and rendering).
#[derive(Resource, Default)]
pub struct PortalSet {
    pub list: Vec<PortalGeom>,
    index: HashMap<Entity, usize>,
}

impl PortalSet {
    pub fn get(&self, e: Entity) -> Option<&PortalGeom> {
        self.index.get(&e).map(|&i| &self.list[i])
    }

    pub fn iter(&self) -> impl Iterator<Item = &PortalGeom> {
        self.list.iter()
    }

    /// Distance from `p` to the nearest portal rectangle, or a large number if there are none.
    pub fn nearest_distance(&self, p: Vec3) -> f32 {
        self.list.iter().map(|g| g.distance_to(p)).fold(f32::MAX, f32::min)
    }
}

pub fn refresh_portal_set(mut set: ResMut<PortalSet>, portals: Query<(Entity, &Portal, &Transform, Option<&PortalSpace>)>) {
    set.list.clear();
    set.index.clear();
    for (e, p, t, s) in &portals {
        let i = set.list.len();
        set.index.insert(e, i);
        set.list.push(PortalGeom::new(e, p, t, s.map(|s| s.0)));
    }
}

/// Uniform scale of a warp or frame matrix.
pub fn matrix_scale(m: &Mat4) -> f32 {
    m.transform_vector3(Vec3::X).length()
}
