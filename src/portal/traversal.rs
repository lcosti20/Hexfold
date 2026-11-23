//! Walking (or being thrown) through portals.
//!
//! Each frame the path of a traveller's eye since the last frame is tested against every portal
//! rectangle, with the plane moved 2 mm (times the traveller's size) towards the side the eye
//! came from. On a crossing, the traveller is warped to the linked portal, shifted twice the
//! bump past the exit plane. Because the eye never reaches the surface before teleporting, the
//! near plane never clips through it.
//!
//! * A **pawn** (the player) gets its location, view, velocity and size warped. A mirroring warp
//!   flips its handedness (`mirrored`), which the controller and the camera compensate for.
//! * An **object** (anything else with a [`PortalObject`]) has its whole transform warped, so a
//!   scaling portal resizes it, and a mirroring portal mirrors it (the visual child gets a
//!   negative X scale). Its linear and angular velocity go through the portal with it.

use avian3d::prelude::*;
use bevy::prelude::*;

use super::geom::{PortalGeom, PortalSet, PortalSpace};

/// Anything that can pass through portals.
#[derive(Component, Debug)]
pub struct PortalTraveller {
    /// True while mirrored relative to the world (after an odd number of mirroring portals).
    pub mirrored: bool,
    /// The point that has to cross a portal (the camera for a pawn, the origin for an object).
    /// Kept up to date by the plugin.
    pub eye: Vec3,
    prev_eye: Option<Vec3>,
    /// Portals passed through since the game began.
    pub traversal_count: u32,
}

impl Default for PortalTraveller {
    fn default() -> Self {
        Self { mirrored: false, eye: Vec3::ZERO, prev_eye: None, traversal_count: 0 }
    }
}

impl PortalTraveller {
    pub fn new(mirrored: bool) -> Self {
        Self { mirrored, ..default() }
    }

    /// Forget the eye's previous position. Call after moving the owner by other means (respawn,
    /// scripted teleport), or the jump itself may count as crossing a portal.
    pub fn reset_tracking(&mut self) {
        self.prev_eye = None;
    }
}

/// A first-person pawn: the player. The plugin owns its view angles and size, because crossing a
/// portal changes both.
#[derive(Component, Debug)]
pub struct PortalPawn {
    /// Yaw in radians about +Y (positive turns left).
    pub yaw: f32,
    /// Pitch in radians (positive looks up).
    pub pitch: f32,
    /// Size relative to normal (portals of different sizes change it).
    pub scale: f32,
    /// Height of the eye above the entity origin at normal size, in metres.
    pub eye_height: f32,
}

impl Default for PortalPawn {
    fn default() -> Self {
        Self { yaw: 0.0, pitch: 0.0, scale: 1.0, eye_height: 0.7 }
    }
}

impl PortalPawn {
    pub fn view_rotation(&self) -> Quat {
        Quat::from_euler(EulerRot::YXZ, self.yaw, self.pitch, 0.0)
    }

    pub fn forward(&self) -> Vec3 {
        self.view_rotation() * Vec3::NEG_Z
    }

    /// Eye position for a pawn standing at `origin`.
    pub fn eye_at(&self, origin: Vec3) -> Vec3 {
        origin + Vec3::Y * (self.eye_height * self.scale)
    }

    pub fn set_view_from_forward(&mut self, forward: Vec3) {
        let f = forward.normalize_or_zero();
        if f == Vec3::ZERO {
            return;
        }
        self.yaw = (-f.x).atan2(-f.z);
        self.pitch = f.y.clamp(-1.0, 1.0).asin();
    }
}

/// A physics object that passes through portals. The root entity has a positive uniform scale
/// (the size) and the rigid body; `visual` is a child that holds the meshes and carries the
/// mirror flip as a negative X scale.
#[derive(Component, Debug)]
pub struct PortalObject {
    pub visual: Entity,
}

/// Sent after a traveller has gone through a portal.
#[derive(Message, Debug, Clone)]
pub struct PortalTraversed {
    pub entity: Entity,
    pub from: Entity,
    pub to: Entity,
}

/// Minimum distance (m) the camera's near plane may be pulled to, and the bump distance.
const BUMP: f32 = 0.002;

pub fn update_eyes(mut q: Query<(&mut PortalTraveller, &Transform, Option<&PortalPawn>)>) {
    for (mut tr, t, pawn) in &mut q {
        tr.eye = match pawn {
            Some(p) => p.eye_at(t.translation),
            None => t.translation,
        };
    }
}

struct Crossing {
    entity: Entity,
    from: usize,
    to: usize,
    bump: Vec3,
}

/// Splits a (possibly mirroring) matrix into a proper rotation, a uniform scale and a mirror flag
/// (the object's local X is the mirrored axis).
pub fn decompose_object(m: Mat4) -> (Vec3, Quat, f32, bool) {
    let mirrored = m.determinant() < 0.0;
    let mut x = m.x_axis.truncate();
    let y = m.y_axis.truncate();
    let z = m.z_axis.truncate();
    if mirrored {
        x = -x;
    }
    let s = y.length().max(1e-6);
    let rot = Quat::from_mat3(&Mat3::from_cols(x / s, y / s, z / s)).normalize();
    (m.w_axis.truncate(), rot, s, mirrored)
}

#[allow(clippy::type_complexity)]
pub fn traverse(
    set: Res<PortalSet>,
    mut messages: MessageWriter<PortalTraversed>,
    mut travellers: Query<(
        Entity,
        &mut PortalTraveller,
        &mut Transform,
        Option<&mut PortalPawn>,
        Option<&PortalObject>,
        Option<&mut LinearVelocity>,
        Option<&mut AngularVelocity>,
        Option<&mut PortalSpace>,
        Option<&mut Position>,
        Option<&mut Rotation>,
    )>,
    mut commands: Commands,
) {
    let mut crossings: Vec<Crossing> = Vec::new();
    for (entity, mut tr, t, pawn, ..) in &mut travellers {
        let size = pawn.map_or(t.scale.y.abs(), |p| p.scale);
        let current = tr.eye;
        if let Some(prev) = tr.prev_eye {
            let bump = BUMP * size;
            for (i, g) in set.list.iter().enumerate() {
                let side = g.signed_distance(prev);
                let bump_vec = g.normal * if side >= 0.0 { bump } else { -bump };
                if !g.intersects(prev, current, bump_vec) {
                    continue;
                }
                let target = if side >= 0.0 { g.front } else { g.back };
                if let Some(target) = target
                    && let Some(to) = set.list.iter().position(|p| p.entity == target)
                {
                    crossings.push(Crossing { entity, from: i, to, bump: bump_vec });
                    break; // at most one portal per frame
                }
            }
        }
        tr.prev_eye = Some(current);
    }

    for c in crossings {
        let Ok((entity, mut tr, mut t, pawn, object, vel, ang, space, position, rotation)) = travellers.get_mut(c.entity) else {
            continue;
        };
        let from: &PortalGeom = &set.list[c.from];
        let to: &PortalGeom = &set.list[c.to];
        let warp = from.warp_to(to);
        let scale_factor = warp.transform_vector3(Vec3::X).length();
        let mirror_warp = warp.determinant() < 0.0;

        if let Some(mut pawn) = pawn {
            t.translation = warp.transform_point3(t.translation - c.bump * 2.0);
            let rot = pawn.view_rotation();
            let forward = warp.transform_vector3(rot * Vec3::NEG_Z);
            pawn.set_view_from_forward(forward);
            t.rotation = Quat::from_rotation_y(pawn.yaw);
            if let Some(mut v) = vel {
                v.0 = warp.transform_vector3(v.0);
            }
            pawn.scale *= scale_factor;
            if mirror_warp {
                tr.mirrored = !tr.mirrored;
            }
            tr.eye = pawn.eye_at(t.translation);
        } else if let Some(object) = object {
            let old = t.to_matrix();
            let flip = if tr.mirrored { Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0)) } else { Mat4::IDENTITY };
            let mut new = warp * old * flip;
            new.w_axis = warp.transform_point3(t.translation - c.bump * 2.0).extend(1.0);
            let (loc, rot, s, mirrored) = decompose_object(new);
            t.translation = loc;
            t.rotation = rot;
            t.scale = Vec3::splat(s);
            if let Some(mut v) = vel {
                v.0 = warp.transform_vector3(v.0);
            }
            if let Some(mut w) = ang {
                // Angular velocity is an axial vector: it turns with the warp's rotation, isn't
                // scaled, and reverses under a reflection.
                let spin = warp.transform_vector3(w.0) / scale_factor;
                w.0 = if mirror_warp { -spin } else { spin };
            }
            if mirror_warp || mirrored != tr.mirrored {
                tr.mirrored = mirrored;
                commands.entity(object.visual).insert(Transform::from_scale(Vec3::new(if mirrored { -1.0 } else { 1.0 }, 1.0, 1.0)));
            }
            tr.eye = t.translation;
            if let (Some(mut sp), Some(to_space)) = (space, to.space)
                && sp.0 != to_space
            {
                sp.0 = to_space;
            }
        }
        // Physics owns position and rotation: teleport them too, or the next physics step would
        // write the old place back over the transform.
        if let Some(mut p) = position {
            p.0 = t.translation;
        }
        if let Some(mut r) = rotation {
            r.0 = t.rotation;
        }
        tr.prev_eye = Some(tr.eye); // the next frame's path starts on this side of the exit
        tr.traversal_count += 1;
        messages.write(PortalTraversed { entity, from: from.entity, to: to.entity });
    }
}
