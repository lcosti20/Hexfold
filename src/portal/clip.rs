//! Objects partway through a portal.
//!
//! While an object straddles a portal, a copy of its meshes is shown at the linked portal, and
//! both are cut at the portal plane: the object keeps the side its centre is on, and the copy
//! keeps only the part that's already through. So from every side each part appears once, in the
//! right place. The cut is done in the material (`PortalClipExt`), which discards fragments on
//! the wrong side of a world-space plane.

use bevy::camera::visibility::RenderLayers;
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

use super::geom::{PortalSet, PortalSpace};
use super::traversal::{PortalObject, PortalTraveller, decompose_object};

/// A plane keeping the points `p` with `dot(p, plane.xyz) >= plane.w`. `NO_CLIP` keeps everything.
pub const NO_CLIP: Vec4 = Vec4::new(0.0, 0.0, 0.0, -1.0);

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct PortalClipExt {
    #[uniform(100)]
    pub plane: Vec4,
}

impl Default for PortalClipExt {
    fn default() -> Self {
        Self { plane: NO_CLIP }
    }
}

impl MaterialExtension for PortalClipExt {
    fn fragment_shader() -> ShaderRef {
        "shaders/portal_clip.wesl".into()
    }

    fn enable_prepass() -> bool {
        false
    }
}

pub type ClipMaterial = ExtendedMaterial<StandardMaterial, PortalClipExt>;

/// Objects that can be cut at a portal: their materials, and the copy's materials.
#[derive(Component, Debug)]
pub struct PortalClipped {
    pub materials: Vec<Handle<ClipMaterial>>,
    /// Meshes and materials of the visual, used to build the copy.
    pub parts: Vec<(Handle<Mesh>, Handle<ClipMaterial>)>,
    /// Bounding radius at normal size (m).
    pub radius: f32,
    /// Show a copy at the linked portal while partway through.
    pub show_copy: bool,
    pub(crate) copy: Option<Entity>,
    pub(crate) copy_materials: Vec<Handle<ClipMaterial>>,
    pub(crate) owner_plane: Vec4,
    pub(crate) copy_plane: Vec4,
}

impl PortalClipped {
    pub fn new(parts: Vec<(Handle<Mesh>, Handle<ClipMaterial>)>, radius: f32) -> Self {
        let materials = parts.iter().map(|p| p.1.clone()).collect();
        Self { materials, parts, radius, show_copy: true, copy: None, copy_materials: Vec::new(), owner_plane: NO_CLIP, copy_plane: NO_CLIP }
    }
}

/// Marks a crossing copy.
#[derive(Component)]
pub struct PortalCopy;

fn write_plane(materials: &mut Assets<ClipMaterial>, handles: &[Handle<ClipMaterial>], plane: Vec4) {
    for h in handles {
        if let Some(m) = materials.get(h)
            && m.extension.plane != plane
            && let Some(mut m) = materials.get_mut(h)
        {
            m.extension.plane = plane;
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn update_crossing_objects(
    mut commands: Commands,
    set: Res<PortalSet>,
    mut materials: ResMut<Assets<ClipMaterial>>,
    mut objects: Query<(&mut PortalClipped, &PortalTraveller, &Transform), (With<PortalObject>, Without<PortalCopy>)>,
    mut copies: Query<(&mut Transform, &mut Visibility, Option<&PortalSpace>), With<PortalCopy>>,
) {
    for (mut clip, traveller, t) in &mut objects {
        let centre = traveller.eye;
        let reach = clip.radius * t.scale.y.abs();
        let mut through = None;
        let mut nearest = reach;
        for (i, g) in set.list.iter().enumerate() {
            let d = g.distance_to(centre);
            if d < nearest {
                nearest = d;
                through = Some(i);
            }
        }
        let link = through.and_then(|i| set.list[i].link_for(centre)).and_then(|l| set.list.iter().position(|g| g.entity == l));

        let (Some(ti), Some(li)) = (through, link) else {
            // Not near a portal: no cut, no copy.
            if clip.owner_plane != NO_CLIP {
                clip.owner_plane = NO_CLIP;
                write_plane(&mut materials, &clip.materials, NO_CLIP);
            }
            if let Some(copy) = clip.copy
                && let Ok((_, mut vis, _)) = copies.get_mut(copy)
                && *vis != Visibility::Hidden
            {
                *vis = Visibility::Hidden;
                clip.copy_plane = NO_CLIP;
            }
            continue;
        };
        let tg = &set.list[ti];
        let lg = &set.list[li];

        // The owner keeps the side of the portal its centre is on: the rest is already through.
        let keep = tg.normal * if tg.signed_distance(centre) >= 0.0 { 1.0 } else { -1.0 };
        let plane = keep.extend(keep.dot(tg.pos));
        if clip.owner_plane != plane {
            clip.owner_plane = plane;
            write_plane(&mut materials, &clip.materials, plane);
        }

        if !clip.show_copy {
            continue;
        }
        // The copy: the owner's transform through the portal.
        let warp = tg.warp_to(lg);
        let copy_matrix = warp * t.to_matrix() * if traveller.mirrored { Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0)) } else { Mat4::IDENTITY };
        let (loc, rot, s, mirrored) = decompose_object(copy_matrix);
        let copy = match clip.copy {
            Some(c) => c,
            None => {
                let c = spawn_copy(&mut commands, &mut materials, &mut clip);
                clip.copy = Some(c);
                c
            }
        };
        // The copy keeps only what's already through: the warped far side of the entry portal.
        let far_side = warp.transform_point3(tg.pos - keep * 0.1);
        let copy_keep = lg.normal * if lg.signed_distance(far_side) >= 0.0 { 1.0 } else { -1.0 };
        let copy_plane = copy_keep.extend(copy_keep.dot(lg.pos));
        if clip.copy_plane != copy_plane {
            clip.copy_plane = copy_plane;
            let handles = clip.copy_materials.clone();
            write_plane(&mut materials, &handles, copy_plane);
        }
        let new_transform = Transform { translation: loc, rotation: rot, scale: Vec3::splat(s) };
        if let Ok((mut ct, mut vis, cspace)) = copies.get_mut(copy) {
            *ct = new_transform;
            *vis = Visibility::Inherited;
            if cspace.map(|c| c.0) != lg.space {
                match lg.space {
                    Some(sp) => {
                        commands.entity(copy).insert(PortalSpace(sp));
                    }
                    None => {
                        commands.entity(copy).remove::<PortalSpace>();
                    }
                }
            }
        }
        // Flip the copy's visual when its handedness differs from the owner's.
        commands.entity(copy).insert(CopyMirror(mirrored));
    }
}

/// Whether a copy is mirrored (its visual child has a negative X scale).
#[derive(Component, Debug, PartialEq)]
pub struct CopyMirror(pub bool);

#[derive(Component)]
pub struct CopyVisual;

fn spawn_copy(commands: &mut Commands, materials: &mut Assets<ClipMaterial>, clip: &mut PortalClipped) -> Entity {
    let mut copy_mats = Vec::new();
    let mut parts = Vec::new();
    for (mesh, mat) in &clip.parts {
        let base = materials.get(mat).cloned();
        let handle = match base {
            Some(m) => materials.add(m),
            None => mat.clone(),
        };
        copy_mats.push(handle.clone());
        parts.push((mesh.clone(), handle));
    }
    clip.copy_materials = copy_mats;
    let visual = commands
        .spawn((CopyVisual, Transform::IDENTITY, Visibility::Inherited))
        .with_children(|c| {
            for (mesh, mat) in parts {
                c.spawn((Mesh3d(mesh), MeshMaterial3d(mat), Transform::IDENTITY, Visibility::Inherited));
            }
        })
        .id();
    commands
        .spawn((PortalCopy, Name::new("PortalCopy"), Transform::IDENTITY, Visibility::Hidden, RenderLayers::layer(0)))
        .add_child(visual)
        .id()
}

/// Applies each copy's mirror flag to its visual child.
pub fn apply_copy_mirror(copies: Query<(&CopyMirror, &Children), Changed<CopyMirror>>, mut visuals: Query<&mut Transform, With<CopyVisual>>) {
    for (m, children) in &copies {
        for c in children.iter() {
            if let Ok(mut t) = visuals.get_mut(c) {
                t.scale.x = if m.0 { -1.0 } else { 1.0 };
            }
        }
    }
}

/// Puts every entity under a `PortalSpace` root on that space's render layer.
pub fn apply_space_layers(
    mut commands: Commands,
    roots: Query<(Entity, &PortalSpace)>,
    children: Query<&Children>,
    layers: Query<Option<&RenderLayers>>,
    mut tick: Local<u32>,
) {
    *tick += 1;
    for (root, space) in &roots {
        let want = super::render::layers_for_space(Some(space.0));
        for e in std::iter::once(root).chain(children.iter_descendants(root)) {
            let Ok(current) = layers.get(e) else { continue };
            if current != Some(&want) {
                commands.entity(e).insert(want.clone());
            }
        }
    }
}
