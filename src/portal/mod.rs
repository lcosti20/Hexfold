#![allow(dead_code, unused_imports)] // the plugin exposes more API than this game uses

//! PortalWorlds for Bevy: seamless, recursive portals for non-Euclidean spaces.
//!
//! A port of the Unreal Engine plugin of the same name (itself a port of the technique in
//! CodeParade's NonEuclidean engine, MIT licence): tunnels longer inside than outside, rooms
//! bigger inside than out, doorways that change your size, and portals into mirror worlds.
//!
//! * [`Portal`]: a rectangle whose two sides lead to other portals.
//! * [`PortalTraveller`] + [`PortalPawn`] / [`PortalObject`]: things that pass through portals.
//! * [`PortalCamera`]: the player's camera; portal views are rendered recursively from it.
//! * [`PortalSpace`]: keeps rooms that were built far apart out of sight of each other.
//! * [`grab`]: pick up, carry and throw physics objects, also through portals.
//! * [`clip`]: objects partway through a portal are cut at its plane and shown on both sides.

pub mod clip;
pub mod geom;
pub mod grab;
pub mod material;
pub mod projection;
pub mod render;
pub mod traversal;

use avian3d::prelude::PhysicsSystems;
use bevy::prelude::*;

pub use clip::{ClipMaterial, PortalClipExt, PortalClipped};
pub use geom::{Portal, PortalGeom, PortalSet, PortalSpace, connect, matrix_scale};
pub use grab::{ObjectGrabbed, ObjectReleased, PortalGrab, ReleaseObject};
pub use material::PortalSurfaceMaterial;
pub use projection::PortalProjection;
pub use render::{PlayerSpace, PortalCamera, PortalSettings, PortalStats, layers_for_space, space_layer};
pub use traversal::{PortalObject, PortalPawn, PortalTraveller, PortalTraversed};

/// Order of the plugin's per-frame work (all in `PostUpdate`, after physics and before transforms
/// propagate, so the frame is rendered from the new position).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum PortalSystems {
    Refresh,
    Traverse,
    Camera,
    Objects,
    Render,
}

pub struct PortalWorldsPlugin;

impl Plugin for PortalWorldsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            MaterialPlugin::<PortalSurfaceMaterial>::default(),
            MaterialPlugin::<clip::ClipMaterial>::default(),
        ))
        .init_resource::<PortalSet>()
        .init_resource::<PortalSettings>()
        .init_resource::<PlayerSpace>()
        .init_resource::<PortalStats>()
        .init_resource::<render::PortalRenderState>()
        .init_resource::<clip::ClippedActors>()
        .add_message::<PortalTraversed>()
        .add_message::<ObjectGrabbed>()
        .add_message::<ObjectReleased>()
        .add_message::<ReleaseObject>()
        .configure_sets(
            PostUpdate,
            (PortalSystems::Refresh, PortalSystems::Traverse, PortalSystems::Camera, PortalSystems::Objects, PortalSystems::Render)
                .chain()
                .after(PhysicsSystems::Writeback)
                .before(TransformSystems::Propagate),
        )
        .add_systems(
            PostUpdate,
            (
                geom::refresh_portal_set.in_set(PortalSystems::Refresh),
                (traversal::update_eyes, traversal::traverse, track_player_space).chain().in_set(PortalSystems::Traverse),
                render::update_pawn_camera.in_set(PortalSystems::Camera),
                (clip::update_crossing_objects, clip::apply_copy_mirror, clip::apply_space_layers).chain().in_set(PortalSystems::Objects),
                render::render_portals.in_set(PortalSystems::Render),
            ),
        )
        .add_systems(Update, grab::update_grab);
    }
}

/// The player's space follows the portals they walk through.
fn track_player_space(
    mut reader: MessageReader<PortalTraversed>,
    set: Res<PortalSet>,
    pawn: Query<Entity, With<PortalPawn>>,
    mut space: ResMut<PlayerSpace>,
) {
    for m in reader.read() {
        if pawn.get(m.entity).is_ok()
            && let Some(g) = set.get(m.to)
        {
            space.0 = g.space;
        }
    }
}
