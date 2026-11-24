//! Compiles every render pipeline the level needs while it loads, so none is compiled during play.
//!
//! Bevy builds a pipeline (a shader variant) the first time a camera draws a mesh and material
//! with a new combination of settings. On native builds that happens on a background thread. On
//! the web it happens on the main thread, and a WebGL2 shader link can take a second (Firefox
//! keeps no shader cache across runs), so the game froze the first time the player saw a portal
//! view, walked into a new space, and so on.
//!
//! A pipeline depends on the mesh's vertex layout, the material's settings, and the camera's
//! projection type and culling direction. So while the level loads, every (mesh, material) pair
//! in the world gets a proxy entity far below the level, drawn by three hidden cameras: one like
//! the player's camera (custom projection), the same one with inverted culling (the player's
//! camera while mirrored), and one like a portal view camera (perspective). Hidden entities get a
//! proxy too (pooled portal surfaces, crossing copies, the hex seal). Once loading has settled the
//! cameras and proxies are despawned.

use std::collections::HashSet;

use bevy::asset::{RenderAssetUsages, UntypedAssetId};
use bevy::camera::visibility::{NoFrustumCulling, RenderLayers};
use bevy::camera::{ClearColorConfig, Hdr, RenderTarget};
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::pbr::Material;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::render::view::Msaa;
use bevy::world_serialization::WorldAssetRoot;

use super::projection::PortalProjection;
use super::render::{PortalCamera, view_cluster_config};

/// Where the proxies are drawn: far below the level, beyond every real camera's far plane.
const PROXY_POS: Vec3 = Vec3::new(0.0, -5000.0, 0.0);
/// Frames with no new (mesh, material) pair and nothing loading before the prewarm ends.
const QUIET_FRAMES: u32 = 30;
/// The prewarm ends after this many frames whatever is still loading.
const MAX_FRAMES: u32 = 3000;

/// Marks a prewarm proxy.
#[derive(Component)]
pub struct PrewarmProxy;

/// State of the pipeline prewarm.
#[derive(Resource, Default)]
pub struct PipelinePrewarm {
    started: bool,
    done: bool,
    frames: u32,
    quiet: u32,
    cameras: Vec<Entity>,
    /// (mesh, material) pairs that already have a proxy.
    seen: HashSet<(AssetId<Mesh>, UntypedAssetId)>,
    /// Proxies whose mesh or material hasn't loaded yet (counted afresh each frame).
    pending: usize,
    /// New proxies this frame.
    added: usize,
}

impl PipelinePrewarm {
    /// The prewarm has finished.
    pub fn done(&self) -> bool {
        self.done
    }
}

/// Starts the prewarm: spawns the three cameras, matching the player's camera's MSAA and HDR.
pub fn start_prewarm(
    mut commands: Commands,
    mut state: ResMut<PipelinePrewarm>,
    mut images: ResMut<Assets<Image>>,
    player: Query<(Option<&Msaa>, Has<Hdr>), With<PortalCamera>>,
) {
    if state.started {
        return;
    }
    let Ok((msaa, hdr)) = player.single() else { return };
    state.started = true;
    let msaa = msaa.copied().unwrap_or_default();
    let mut image = Image::new_uninit(
        Extent3d { width: 64, height: 64, depth_or_array_layers: 1 },
        TextureDimension::D2,
        TextureFormat::Rgba16Float,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    let image = images.add(image);
    // Looking straight down at the proxies.
    let transform = Transform::from_translation(PROXY_POS + Vec3::Y * 20.0).looking_at(PROXY_POS, Vec3::Z);
    let custom = || {
        let mut p = PortalProjection::default();
        p.fov_x = 120f32.to_radians();
        Projection::custom(p)
    };
    let views = [(custom(), false), (custom(), true), (Projection::Perspective(PerspectiveProjection::default()), false)];
    for (i, (projection, invert_culling)) in views.into_iter().enumerate() {
        let mut e = commands.spawn((
            Name::new(format!("PipelinePrewarm{i}")),
            Camera3d::default(),
            Camera { order: -100_000 + i as isize, invert_culling, clear_color: ClearColorConfig::Custom(Color::BLACK), ..default() },
            RenderTarget::Image(image.clone().into()),
            projection,
            msaa,
            view_cluster_config(),
            transform,
            RenderLayers::layer(0),
        ));
        if hdr {
            e.insert(Hdr);
        }
        state.cameras.push(e.id());
    }
}

/// Gives every new (mesh, material) pair of material type `M` a proxy, and counts the proxies
/// whose assets are still loading.
#[allow(clippy::type_complexity)]
pub fn add_proxies<M: Material>(
    mut commands: Commands,
    mut state: ResMut<PipelinePrewarm>,
    meshes: Res<Assets<Mesh>>,
    materials: Res<Assets<M>>,
    entities: Query<(&Mesh3d, &MeshMaterial3d<M>, Has<NotShadowCaster>, Has<NotShadowReceiver>), Without<PrewarmProxy>>,
    proxies: Query<(&Mesh3d, &MeshMaterial3d<M>), With<PrewarmProxy>>,
) {
    if !state.started || state.done {
        return;
    }
    for (mesh, material, no_cast, no_receive) in &entities {
        if !state.seen.insert((mesh.0.id(), material.0.id().untyped())) {
            continue;
        }
        state.added += 1;
        let mut e = commands.spawn((
            PrewarmProxy,
            Name::new("PipelinePrewarmProxy"),
            Mesh3d(mesh.0.clone()),
            MeshMaterial3d(material.0.clone()),
            Transform::from_translation(PROXY_POS),
            Visibility::Visible,
            NoFrustumCulling,
            RenderLayers::layer(0),
        ));
        if no_cast {
            e.insert(NotShadowCaster);
        }
        if no_receive {
            e.insert(NotShadowReceiver);
        }
    }
    state.pending += proxies.iter().filter(|(mesh, material)| !meshes.contains(&mesh.0) || !materials.contains(&material.0)).count();
}

/// Ends the prewarm once nothing new has turned up for a while, every scene has loaded and been
/// spawned, and every proxy's assets are in; then despawns the cameras and proxies.
pub fn finish_prewarm(
    mut commands: Commands,
    mut state: ResMut<PipelinePrewarm>,
    assets: Res<AssetServer>,
    scenes: Query<(&WorldAssetRoot, Option<&Children>)>,
    proxies: Query<Entity, With<PrewarmProxy>>,
) {
    if !state.started || state.done {
        return;
    }
    state.frames += 1;
    let loading = scenes.iter().any(|(root, children)| !assets.is_loaded_with_dependencies(&root.0) || children.is_none_or(|c| c.is_empty()));
    let busy = loading || state.pending > 0 || state.added > 0;
    state.quiet = if busy { 0 } else { state.quiet + 1 };
    state.pending = 0;
    state.added = 0;
    if state.quiet < QUIET_FRAMES && state.frames < MAX_FRAMES {
        return;
    }
    state.done = true;
    info!("pipeline prewarm done after {} frames ({} mesh/material pairs)", state.frames, state.seen.len());
    for e in state.cameras.drain(..).chain(proxies.iter()) {
        commands.entity(e).despawn();
    }
}
