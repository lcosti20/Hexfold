//! Recursive portal rendering (the technique of CodeParade's NonEuclidean engine, with render
//! targets in place of OpenGL framebuffers).
//!
//! Every frame, after the player camera has its final place:
//!
//! 1. A tree of views is built. The root is the player's view. For every portal visible in a view,
//!    a child view is made by warping that view's camera through the portal. Visibility is tested
//!    on the CPU (facing, clip plane, screen rectangle inside the parent portal's rectangle).
//!    Portals in views at depth `max_depth - 1` are drawn pink.
//! 2. Each child view is a pooled camera that renders into its own image, deepest first (camera
//!    order). Its near plane is replaced with the exit portal's plane (an oblique projection), so
//!    nothing between the virtual camera and the exit portal is drawn.
//! 3. Every view gets its own set of portal-surface entities, on that view's render layer, showing
//!    the children's images. (The same portal shows a different image in each view.)
//!
//! Mirroring: a view seen through a mirroring portal would need a mirrored camera, which can't
//! render. Each view keeps a `mirrored` flag instead and is rendered by the matching proper
//! camera, whose image is the true view flipped left-right. A surface undoes the flip with its
//! `mirror_u` parameter. The player's own view is flipped by negating the projection's X (with
//! inverted culling).
//!
//! Spaces: entities with a `PortalSpace(n)` are on render layer `1 + n`; untagged ones on layer 0.
//! A view renders layer 0 and its own space's layer, so rooms built far apart never show on the
//! horizon of each other.

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ClearColorConfig, Exposure, Hdr, RenderTarget};
use bevy::light::cluster::{ClusterConfig, ClusterFarZMode, ClusterZConfig};
use bevy::light::{AmbientLight, GlobalAmbientLight};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::render::view::Msaa;
use bevy::render::view::Tonemapping;

use super::geom::{PortalGeom, PortalSet};
use super::material::PortalSurfaceMaterial;
use super::projection::PortalProjection;
use super::traversal::PortalTraveller;

pub const SPACE_LAYER_BASE: usize = 1;
pub const MAX_SPACES: usize = 16;
pub const NODE_LAYER_BASE: usize = 24;

pub fn space_layer(space: i32) -> usize {
    SPACE_LAYER_BASE + (space.max(0) as usize).min(MAX_SPACES - 1)
}

/// Layers of everything that appears in `space` (or in every space, for `None`).
pub fn layers_for_space(space: Option<i32>) -> RenderLayers {
    match space {
        Some(s) => RenderLayers::layer(space_layer(s)),
        None => RenderLayers::layer(0),
    }
}

/// What a view renders: layer 0, its own space (every space if it has none) and its own portal
/// surfaces' layer.
pub fn view_layers(space: Option<i32>, node_layer: usize) -> RenderLayers {
    let mut l = RenderLayers::layer(0).with(node_layer);
    match space {
        Some(s) => l = l.with(space_layer(s)),
        None => {
            for s in 0..MAX_SPACES as i32 {
                l = l.with(space_layer(s));
            }
        }
    }
    l
}

/// Depth (m) the light clusters of every view reach. Lights further away still light the view;
/// they share the last slice.
const CLUSTER_FAR: f32 = 100.0;

/// The light clustering every view must use (the player's camera and the pooled view cameras).
///
/// Bevy's default sizes a camera's cluster grid from the depth of the lights that camera saw a few
/// frames ago (the figure is read back from the GPU). That only works for a camera that moves
/// smoothly. A pooled view camera jumps to a different view whenever the view tree changes, and
/// the player's camera jumps when it goes through a portal; with the stale depth, the lights of
/// the new place fall outside the grid and the view is lit by ambient light alone until the
/// statistics catch up: a portal that goes dark for about three frames.
pub fn view_cluster_config() -> ClusterConfig {
    ClusterConfig::FixedZ {
        total: 4096,
        z_slices: 24,
        z_config: ClusterZConfig { first_slice_depth: 5.0, far_z_mode: ClusterFarZMode::Constant(CLUSTER_FAR) },
        dynamic_resizing: true,
    }
}

/// Marks the player's camera.
#[derive(Component)]
pub struct PortalCamera;

#[derive(Resource, Debug, Clone)]
pub struct PortalSettings {
    /// Recursion depth: views up to `max_depth - 1` are rendered, deeper portals draw pink.
    pub max_depth: usize,
    /// Upper limit on portal views per frame; further portals are drawn pink.
    pub max_captures: usize,
    /// Render-target resolution multiplier per recursion level beyond the first.
    pub resolution_falloff: f32,
    /// Pull the near plane in (down to 1 mm) near portals, so the camera never clips through one.
    pub dynamic_near_plane: bool,
    /// Draw every portal flat pink and skip all extra views.
    pub debug: bool,
    /// Hide portal surfaces.
    pub hide: bool,
    /// Exposure of the player's camera, shared by every portal view (each view's pixels are
    /// already exposed when the surface shows them).
    pub exposure_ev100: f32,
    /// Ambient light (colour, brightness) per space, set on every view that renders that space.
    /// Spaces not listed use the `GlobalAmbientLight`.
    pub space_ambient: Vec<(i32, Color, f32)>,
    /// Cameras (and images) created up front for each recursion depth. A camera created during
    /// play shows a blank image for a few frames, so the pool should already hold as many views as
    /// one frame can need. Each costs one render target; unused ones are inactive.
    pub prewarm_per_depth: usize,
}

impl Default for PortalSettings {
    fn default() -> Self {
        Self { max_depth: 4, max_captures: 32, resolution_falloff: 1.0, dynamic_near_plane: true, debug: false, hide: false, exposure_ev100: Exposure::EV100_BLENDER, space_ambient: Vec::new(), prewarm_per_depth: 0 }
    }
}

/// The space the player's camera is in (`None`: the map doesn't use spaces).
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct PlayerSpace(pub Option<i32>);

/// Statistics for the on-screen debug line.
#[derive(Resource, Default, Debug)]
pub struct PortalStats {
    pub views: usize,
    pub captures: usize,
    pub near_plane: f32,
}

const NEAR_MIN: f32 = 0.001;
const NEAR_MAX: f32 = 0.1;

#[derive(Clone, Copy, Debug)]
struct Rect2 {
    min: Vec2,
    max: Vec2,
}

impl Rect2 {
    const FULL: Rect2 = Rect2 { min: Vec2::ZERO, max: Vec2::ONE };

    fn intersect(self, o: Rect2) -> Option<Rect2> {
        let min = self.min.max(o.min);
        let max = self.max.min(o.max);
        (min.x < max.x && min.y < max.y).then_some(Rect2 { min, max })
    }
}

struct Child {
    portal: usize,
    node: Option<usize>,
}

struct ViewNode {
    location: Vec3,
    rotation: Quat,
    forward: Vec3,
    right: Vec3,
    up: Vec3,
    mirrored: bool,
    screen: Rect2,
    exit: Option<usize>,
    clip_base: Vec3,
    clip_normal: Vec3,
    depth: usize,
    space: Option<i32>,
    children: Vec<Child>,
}

struct BuildParams<'a> {
    set: &'a PortalSet,
    tan_x: f32,
    tan_y: f32,
    near: f32,
    max_depth: usize,
    max_captures: usize,
}

fn build_node(p: &BuildParams, nodes: &mut Vec<ViewNode>, location: Vec3, rotation: Quat, mirrored: bool, exit: Option<usize>, screen: Rect2, depth: usize, space: Option<i32>) -> usize {
    let index = nodes.len();
    let mut node = ViewNode {
        location,
        rotation,
        forward: rotation * Vec3::NEG_Z,
        right: rotation * Vec3::X * if mirrored { -1.0 } else { 1.0 },
        up: rotation * Vec3::Y,
        mirrored,
        screen,
        exit,
        clip_base: Vec3::ZERO,
        clip_normal: Vec3::ZERO,
        depth,
        space,
        children: Vec::new(),
    };
    if let Some(e) = exit {
        // Keep only what lies beyond the exit portal. The plane is pulled towards the camera by
        // the near-plane distance so geometry touching the portal (door frames) isn't clipped.
        let g = &p.set.list[e];
        let mut normal = g.normal;
        if (location - g.pos).dot(normal) > 0.0 {
            normal = -normal;
        }
        node.clip_normal = normal;
        node.clip_base = g.pos - normal * p.near;
    }
    nodes.push(node);

    for (i, portal) in p.set.list.iter().enumerate() {
        if Some(i) == exit {
            continue;
        }
        if let (Some(s), Some(ps)) = (space, portal.space)
            && s != ps
        {
            continue; // belongs to another space, which this view doesn't render
        }
        let side = portal.signed_distance(location);
        if side.abs() < 1e-6 {
            continue;
        }
        let corners = portal.corners();
        {
            let n = &nodes[index];
            if exit.is_some() && !corners.iter().any(|c| (*c - n.clip_base).dot(n.clip_normal) >= 0.0) {
                continue; // entirely on the clipped side of this view's exit portal
            }
        }
        let Some(rect) = screen_rect(p, &nodes[index], &corners) else { continue };
        let Some(rect) = rect.intersect(nodes[index].screen) else { continue };

        let mut child = Child { portal: i, node: None };
        let target = if side > 0.0 { portal.front } else { portal.back };
        if let Some(target) = target
            && depth + 1 < p.max_depth
            && nodes.len() - 1 < p.max_captures
            && let Some(ti) = p.set.list.iter().position(|g| g.entity == target)
        {
            let tg: &PortalGeom = &p.set.list[ti];
            let warp = portal.warp_to(tg);
            let n = &nodes[index];
            let new_loc = warp.transform_point3(n.location);
            let new_fwd = warp.transform_vector3(n.forward).normalize_or_zero();
            let new_up = warp.transform_vector3(n.up).normalize_or_zero();
            // The camera is always a proper rotation; a reflection in the warp flips the view instead.
            let new_rot = Transform::IDENTITY.looking_to(new_fwd, new_up).rotation;
            let new_mirrored = n.mirrored != (warp.determinant() < 0.0);
            child.node = Some(build_node(p, nodes, new_loc, new_rot, new_mirrored, Some(ti), rect, depth + 1, tg.space));
        }
        nodes[index].children.push(child);
    }
    index
}

fn screen_rect(p: &BuildParams, node: &ViewNode, corners: &[Vec3; 4]) -> Option<Rect2> {
    let mut min = Vec2::splat(f32::MAX);
    let mut max = Vec2::splat(f32::MIN);
    let mut behind = 0;
    for c in corners {
        let rel = *c - node.location;
        let z = rel.dot(node.forward);
        if z <= 1e-5 {
            behind += 1;
            continue;
        }
        let u = 0.5 + 0.5 * rel.dot(node.right) / (z * p.tan_x);
        let v = 0.5 - 0.5 * rel.dot(node.up) / (z * p.tan_y);
        min = min.min(Vec2::new(u, v));
        max = max.max(Vec2::new(u, v));
    }
    if behind == 4 {
        return None;
    }
    // A rectangle crossing the camera plane can cover any part of the screen.
    Some(if behind > 0 { Rect2::FULL } else { Rect2 { min, max } })
}

struct NodeCamera {
    entity: Entity,
    image: Handle<Image>,
    size: UVec2,
    /// The recursion depth this camera always serves. Its image is sized for that depth, so it is
    /// never resized when the view tree changes shape (a resize leaves the image black for a frame).
    depth: usize,
}

struct SurfaceSlot {
    entity: Entity,
    material: Handle<PortalSurfaceMaterial>,
}

/// Pooled cameras, images and surface entities.
#[derive(Resource, Default)]
pub struct PortalRenderState {
    cameras: Vec<NodeCamera>,
    surfaces: Vec<SurfaceSlot>,
    quad: Option<Handle<Mesh>>,
}

fn new_target(images: &mut Assets<Image>, size: UVec2) -> Handle<Image> {
    let mut image = Image::new_uninit(
        Extent3d { width: size.x, height: size.y, depth_or_array_layers: 1 },
        TextureDimension::D2,
        TextureFormat::Rgba16Float,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    images.add(image)
}

/// Creates one pooled view camera (inactive) with its render target.
#[allow(clippy::too_many_arguments)]
fn spawn_view_camera(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    settings: &PortalSettings,
    global_ambient: &GlobalAmbientLight,
    index: usize,
    depth: usize,
    size: UVec2,
) -> NodeCamera {
    let image = new_target(images, size);
    let entity = commands
        .spawn((
            Name::new(format!("PortalView{}", index + 1)),
            Camera3d::default(),
            Camera { order: -(depth as isize) * 64 - (index as isize % 64), is_active: false, clear_color: ClearColorConfig::Custom(Color::BLACK), ..default() },
            RenderTarget::Image(image.clone().into()),
            Hdr,
            Tonemapping::None,
            bevy::core_pipeline::tonemapping::DebandDither::Disabled,
            // Same anti-aliasing as the player's view, or edges seen through a portal crawl.
            Msaa::Sample4,
            Exposure { ev100: settings.exposure_ev100 },
            view_cluster_config(),
            AmbientLight { color: global_ambient.color, brightness: global_ambient.brightness, ..default() },
            Projection::Perspective(PerspectiveProjection::default()),
            Transform::IDENTITY,
            RenderLayers::layer(0),
        ))
        .id();
    NodeCamera { entity, image, size, depth }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn render_portals(
    mut commands: Commands,
    settings: Res<PortalSettings>,
    set: Res<PortalSet>,
    player_space: Res<PlayerSpace>,
    mut state: ResMut<PortalRenderState>,
    mut stats: ResMut<PortalStats>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<PortalSurfaceMaterial>>,
    mut main: Query<(&mut Transform, &mut Projection, &mut Camera, &mut RenderLayers), With<PortalCamera>>,
    pawn: Query<&PortalTraveller, With<super::traversal::PortalPawn>>,
    mut others: Query<
        (&mut Transform, &mut Projection, &mut Camera, &mut RenderLayers, &mut Visibility, &mut Exposure),
        (Without<PortalCamera>, Without<PortalSurfaceTag>),
    >,
    mut surfaces: Query<(&mut Transform, &mut RenderLayers, &mut Visibility), (With<PortalSurfaceTag>, Without<PortalCamera>)>,
    mut ambients: Query<&mut AmbientLight>,
    main_entity: Query<Entity, With<PortalCamera>>,
    global_ambient: Res<GlobalAmbientLight>,
) {
    // Each view lights the space it renders with that space's ambient light.
    let mut set_ambient = |entity: Entity, space: Option<i32>| {
        let (color, brightness) = space
            .and_then(|s| settings.space_ambient.iter().find(|a| a.0 == s))
            .map_or((global_ambient.color, global_ambient.brightness), |a| (a.1, a.2));
        if let Ok(mut a) = ambients.get_mut(entity)
            && (a.brightness != brightness || a.color != color)
        {
            a.color = color;
            a.brightness = brightness;
        }
    };
    if let Ok(e) = main_entity.single() {
        set_ambient(e, player_space.0);
    }
    let Ok((cam_t, mut cam_proj, cam, mut cam_layers)) = main.single_mut() else { return };
    let Ok(traveller) = pawn.single() else { return };
    let Projection::Custom(custom) = &mut *cam_proj else { return };
    let Some(pp) = custom.get_mut::<PortalProjection>() else { return };

    // Dynamic near plane.
    let near = if settings.dynamic_near_plane { (set.nearest_distance(cam_t.translation) * 0.5).clamp(NEAR_MIN, NEAR_MAX) } else { 0.05 };
    pp.inner.near = near;
    pp.flip_x = traveller.mirrored;
    let (tan_x, tan_y, aspect, fov_y) = (pp.tan_half_x(), pp.tan_half_y(), pp.inner.aspect_ratio, pp.inner.fov);
    let cam_mirrored = traveller.mirrored;
    let cam_size = cam.physical_target_size().unwrap_or(UVec2::new(1280, 720));
    let mut cam = cam;
    cam.invert_culling = cam_mirrored;
    let cam_location = cam_t.translation;
    let cam_rotation = cam_t.rotation;
    let space = player_space.0;
    *cam_layers = view_layers(space, NODE_LAYER_BASE);

    stats.near_plane = near;
    stats.views = 0;
    stats.captures = 0;

    // Deactivate everything when there's nothing to draw.
    let disabled = set.list.is_empty() || settings.debug || settings.hide || cam_size.x == 0 || cam_size.y == 0;
    let mut nodes: Vec<ViewNode> = Vec::new();
    if !disabled {
        let params = BuildParams { set: &set, tan_x, tan_y, near, max_depth: settings.max_depth.max(1), max_captures: settings.max_captures };
        build_node(&params, &mut nodes, cam_location, cam_rotation, cam_mirrored, None, Rect2::FULL, 0, space);
    }
    stats.views = nodes.len();
    stats.captures = nodes.len().saturating_sub(1);

    // Make sure there are enough cameras and surfaces.
    if state.quad.is_none() {
        state.quad = Some(meshes.add(Rectangle::new(2.0, 2.0)));
    }
    // Cameras are pooled per depth: each node takes the next free camera of its own depth, so a
    // camera keeps its depth (and image size) however the tree changes from frame to frame.
    let size_for_depth = |depth: usize| {
        let scale = settings.resolution_falloff.clamp(0.1, 1.0).powi(depth.saturating_sub(1) as i32);
        UVec2::new(((cam_size.x as f32 * scale).round() as u32).max(16), ((cam_size.y as f32 * scale).round() as u32).max(16))
    };
    // Fill the pool once, so no camera is created while the player walks.
    if state.cameras.is_empty() && settings.prewarm_per_depth > 0 {
        for depth in 1..settings.max_depth.max(1) {
            for _ in 0..settings.prewarm_per_depth {
                let k = state.cameras.len();
                let size = size_for_depth(depth);
                state.cameras.push(spawn_view_camera(&mut commands, &mut images, &settings, &global_ambient, k, depth, size));
            }
        }
    }
    let mut node_cam: Vec<usize> = vec![usize::MAX; nodes.len()];
    let mut cam_taken = vec![false; state.cameras.len()];
    for n in 1..nodes.len() {
        let depth = nodes[n].depth;
        let found = (0..state.cameras.len()).find(|&k| !cam_taken[k] && state.cameras[k].depth == depth);
        let k = match found {
            Some(k) => k,
            None => {
                let k = state.cameras.len();
                let size = size_for_depth(depth);
                info!("portal view camera {} created during play for depth {depth} ({} views this frame); raise prewarm_per_depth", k + 1, nodes.len());
                state.cameras.push(spawn_view_camera(&mut commands, &mut images, &settings, &global_ambient, k, depth, size));
                cam_taken.push(false);
                k
            }
        };
        cam_taken[k] = true;
        node_cam[n] = k;
    }
    let mut cam_node: Vec<Option<usize>> = vec![None; state.cameras.len()];
    for (n, &k) in node_cam.iter().enumerate().skip(1) {
        cam_node[k] = Some(n);
    }

    // Position the cameras.
    for (k, camera) in state.cameras.iter_mut().enumerate() {
        let Ok((mut t, mut proj, mut cam, mut layers, _, mut exposure)) = others.get_mut(camera.entity) else { continue };
        if exposure.ev100 != settings.exposure_ev100 {
            exposure.ev100 = settings.exposure_ev100;
        }
        let Some(n) = cam_node[k] else {
            if cam.is_active {
                cam.is_active = false;
            }
            continue;
        };
        let node = &nodes[n];
        // Only a change of the window size resizes an image.
        let want = size_for_depth(camera.depth);
        if want != camera.size {
            if let Some(mut img) = images.get_mut(&camera.image) {
                img.resize(Extent3d { width: want.x, height: want.y, depth_or_array_layers: 1 });
                camera.size = want;
            }
        }
        t.translation = node.location;
        t.rotation = node.rotation;
        if let Projection::Perspective(p) = &mut *proj {
            p.fov = fov_y;
            p.aspect_ratio = aspect;
            p.near = near;
            p.far = 2000.0;
            // The exit portal's plane, in this camera's view space. Keeps the side the normal points to.
            let inv_rot = node.rotation.inverse();
            let n_view = inv_rot * node.clip_normal;
            let d = node.clip_normal.dot(node.location - node.clip_base);
            p.near_clip_plane = if std::env::var("HEXFOLD_NOCLIP").is_ok() { Vec4::new(0.0, 0.0, -1.0, -near) } else { Vec4::new(n_view.x, n_view.y, n_view.z, d) };
            if std::env::var("HEXFOLD_DEBUG").is_ok() {
                eprintln!("node {n} depth {} loc {:?} fwd {:?} space {:?} clip n_view {:?} d {d} size {:?}", node.depth, node.location, node.forward, node.space, n_view, camera.size);
            }
        }
        set_ambient(camera.entity, node.space);
        cam.order = -(node.depth as isize) * 64 - (k as isize % 64);
        cam.is_active = true;
        *layers = view_layers(node.space, NODE_LAYER_BASE + n);
    }

    // Surfaces: one per (view, visible portal).
    let mut used = 0usize;
    for (n, node) in nodes.iter().enumerate() {
        for child in &node.children {
            let portal = &set.list[child.portal];
            if state.surfaces.len() <= used {
                let material = materials.add(PortalSurfaceMaterial::pink());
                let mesh = state.quad.clone().unwrap();
                let entity = commands
                    .spawn((
                        PortalSurfaceTag,
                        Name::new("PortalSurface"),
                        Mesh3d(mesh),
                        MeshMaterial3d(material.clone()),
                        Transform::IDENTITY,
                        Visibility::Hidden,
                        RenderLayers::layer(NODE_LAYER_BASE),
                        bevy::light::NotShadowCaster,
                        bevy::light::NotShadowReceiver,
                    ))
                    .id();
                state.surfaces.push(SurfaceSlot { entity, material });
            }
            let slot = &state.surfaces[used];
            used += 1;
            let Ok((mut t, mut layers, mut vis)) = surfaces.get_mut(slot.entity) else { continue };
            let mut tr = portal.transform;
            tr.scale *= Vec3::new(portal.half_extent.x, portal.half_extent.y, 1.0);
            *t = tr;
            *layers = RenderLayers::layer(NODE_LAYER_BASE + n);
            *vis = Visibility::Inherited;
            let wanted = match child.node {
                Some(c) if c >= 1 => {
                    let image = state.cameras[node_cam[c]].image.clone();
                    let parent_flipped = n != 0 && node.mirrored; // the player's own view is flipped by its projection instead
                    Some((image, parent_flipped != nodes[c].mirrored))
                }
                _ => None,
            };
            if let Some(m) = materials.get(&slot.material) {
                let differs = match &wanted {
                    Some((img, mirror)) => m.texture.as_ref() != Some(img) || m.params.x != 0.0 || (m.params.y > 0.5) != *mirror,
                    None => m.params.x < 0.5,
                };
                if differs && let Some(mut m) = materials.get_mut(&slot.material) {
                    match &wanted {
                        Some((img, mirror)) => m.show(img, *mirror),
                        None => m.show_pink(),
                    }
                }
            }
        }
    }
    for slot in state.surfaces.iter().skip(used) {
        if let Ok((_, _, mut vis)) = surfaces.get_mut(slot.entity)
            && *vis != Visibility::Hidden
        {
            *vis = Visibility::Hidden;
        }
    }
}

/// Marks the plugin's own surface entities.
#[derive(Component)]
pub struct PortalSurfaceTag;

/// The player's camera looks wherever the pawn looks.
pub fn update_pawn_camera(
    pawn: Query<(&Transform, &super::traversal::PortalPawn), Without<PortalCamera>>,
    mut camera: Query<&mut Transform, With<PortalCamera>>,
) {
    let (Ok((t, p)), Ok(mut c)) = (pawn.single(), camera.single_mut()) else { return };
    c.translation = p.eye_at(t.translation);
    c.rotation = p.view_rotation();
}
