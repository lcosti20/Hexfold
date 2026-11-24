//! Runes: physics objects that can be carried, thrown and sent through portals. The player walks
//! through them (they don't collide), so a rune can't be shoved into a wall or out of the level.
//!
//! A rune's gameplay state lives entirely in the engine: its world scale (the root's uniform
//! scale) and its mirrored flag (`PortalTraveller::mirrored`). Portals change both; nothing else
//! needs to track them. The root has a positive uniform scale and the rigid body; the meshes
//! hang off a `visual` child that carries the mirror flip.

use avian3d::prelude::*;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

use crate::level::{Layer, data::RUNES};
use crate::portal::*;
use crate::ue;

use super::mats::Mats;

/// Bounding radius of a rune at normal size (m): the hex plate's corner radius plus its depth.
pub const RUNE_RADIUS: f32 = 0.43;

/// A rune and where it started.
#[derive(Component, Debug)]
pub struct Rune {
    /// Which hex socket it belongs to (0-5).
    pub index: u8,
    /// Where it started (restored by `ReturnToPedestal`).
    pub spawn: Transform,
    /// Whether it started mirrored.
    pub spawn_mirrored: bool,
    /// The space it started in.
    pub spawn_space: Option<i32>,
}

/// Send to put a rune back on its pedestal (it fell somewhere out of reach).
#[derive(Message, Debug, Clone)]
pub struct ReturnToPedestal(pub Entity);

/// The hex plate's outline, for the collider.
fn hex_points(radius: f32, half_thickness: f32) -> Vec<Vec3> {
    let mut pts = Vec::new();
    for k in 0..6 {
        let a = std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::FRAC_PI_3;
        for z in [-half_thickness, half_thickness] {
            pts.push(Vec3::new(a.cos() * radius, a.sin() * radius, z));
        }
    }
    pts
}

/// Spawns the six runes from the level data: a dynamic body with a hexagonal hull (the backing
/// plate's outline, with swept CCD so it can't fall through a floor), a portal traveller and
/// object, and clip materials so it is cut at portals. A negative X scale in the data means the
/// rune starts mirrored.
pub fn spawn_runes(mut commands: Commands, assets: Res<AssetServer>, mats: Res<Mats>, mut clip_materials: ResMut<Assets<ClipMaterial>>) {
    for r in RUNES {
        let mut t = ue::transform(r.loc, r.rot, r.scale);
        let mirrored = t.scale.x < 0.0;
        t.scale = t.scale.abs();
        let space = (r.space >= 0).then_some(r.space);

        let gold = clip_materials.add(ClipMaterial { base: mats.rune_gold.clone(), extension: PortalClipExt::default() });
        let plate = clip_materials.add(ClipMaterial { base: mats.rune_plate.clone(), extension: PortalClipExt::default() });
        let path = format!("models/Runes/SM_Rune_{}.glb", r.index);
        let strokes: Handle<Mesh> = assets.load(format!("{path}#Mesh0/Primitive0"));
        let back: Handle<Mesh> = assets.load(format!("{path}#Mesh0/Primitive1"));
        let parts = vec![(strokes.clone(), gold.clone()), (back.clone(), plate.clone())];

        let visual = commands
            .spawn((Name::new("RuneVisual"), Transform::from_scale(Vec3::new(if mirrored { -1.0 } else { 1.0 }, 1.0, 1.0)), Visibility::Inherited))
            .with_children(|c| {
                c.spawn((Mesh3d(strokes), MeshMaterial3d(gold.clone())));
                c.spawn((Mesh3d(back), MeshMaterial3d(plate.clone())));
            })
            .id();

        let mut e = commands.spawn((
            Name::new(r.name),
            Rune { index: r.index, spawn: t, spawn_mirrored: mirrored, spawn_space: space },
            t,
            Visibility::Inherited,
            RigidBody::Dynamic,
            Collider::convex_hull(&hex_points(0.28, 0.05)).expect("rune hull"),
            ColliderDensity(600.0),
            SweptCcd::default(),
            Friction::new(0.6),
            Restitution::new(0.05),
            // Runes and the player pass through each other, so the player can't shove a rune
            // into a wall or out of the level.
            CollisionLayers::new(Layer::Prop, [Layer::World, Layer::Prop, Layer::Trigger]),
            PortalTraveller::new(mirrored),
            PortalObject { visual },
            PortalClipped::new(parts, RUNE_RADIUS),
        ));
        if let Some(s) = space {
            e.insert((PortalSpace(s), RenderLayers::layer(space_layer(s))));
        }
        e.add_child(visual);
    }
}

/// `ReturnToPedestal`: zero the velocities, teleport back (scale included), restore the mirror
/// state and the space.
#[allow(clippy::type_complexity)]
pub fn return_to_pedestal(
    mut messages: MessageReader<ReturnToPedestal>,
    mut runes: Query<(&Rune, &mut Transform, &mut LinearVelocity, &mut AngularVelocity, &mut PortalTraveller, &PortalObject, Option<&mut Position>, Option<&mut Rotation>)>,
    mut commands: Commands,
    mut release: MessageWriter<ReleaseObject>,
) {
    for m in messages.read() {
        let Ok((rune, mut t, mut lv, mut av, mut tr, object, pos, rot)) = runes.get_mut(m.0) else { continue };
        lv.0 = Vec3::ZERO;
        av.0 = Vec3::ZERO;
        *t = rune.spawn;
        if let Some(mut p) = pos {
            p.0 = rune.spawn.translation;
        }
        if let Some(mut r) = rot {
            r.0 = rune.spawn.rotation;
        }
        tr.mirrored = rune.spawn_mirrored;
        tr.reset_tracking();
        commands.entity(object.visual).insert(Transform::from_scale(Vec3::new(if rune.spawn_mirrored { -1.0 } else { 1.0 }, 1.0, 1.0)));
        match rune.spawn_space {
            Some(s) => {
                commands.entity(m.0).insert(PortalSpace(s));
            }
            None => {
                commands.entity(m.0).remove::<PortalSpace>();
            }
        }
        release.write(ReleaseObject(m.0));
    }
}
