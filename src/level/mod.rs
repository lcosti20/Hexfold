//! Builds the Observatory level from the placement data exported from the Unreal project.

pub mod data;

use std::collections::HashMap;

use avian3d::prelude::*;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;
use bevy::world_serialization::{WorldAssetRoot, WorldInstanceReady};

use crate::portal::{Portal, PortalSpace, layers_for_space};
use crate::ue;
use data::*;

/// Physics layers.
#[derive(PhysicsLayer, Clone, Copy, Debug, Default)]
pub enum Layer {
    #[default]
    World,
    Player,
    Prop,
    /// Triggers (kill volumes, plates, seal): detect the player and props.
    Trigger,
}

/// Named entities of the level, for wiring things up.
#[derive(Resource, Default)]
pub struct LevelIndex {
    pub by_name: HashMap<String, Entity>,
}

impl LevelIndex {
    pub fn get(&self, name: &str) -> Entity {
        *self.by_name.get(name).unwrap_or_else(|| panic!("level has no entity named {name}"))
    }
}

/// The greybox palette. (Unreal used the LevelPrototyping grid materials here.)
fn colour(name: &str) -> Color {
    if name.contains("Doorframe") || name.ends_with("_Frame") {
        Color::srgb(0.30, 0.27, 0.24)
    } else if name == "Ground" {
        Color::srgb(0.06, 0.09, 0.04)
    } else if name.starts_with("Out_Tree") {
        Color::srgb(0.10, 0.08, 0.06)
    } else if name.starts_with("Out_Rock") {
        Color::srgb(0.22, 0.22, 0.22)
    } else if name == "Obs_Exterior" {
        Color::srgb(0.55, 0.52, 0.47)
    } else if name.starts_with("Rot_") {
        Color::srgb(0.62, 0.58, 0.52)
    } else if name.starts_with("Ped_") {
        Color::srgb(0.45, 0.43, 0.42)
    } else if name.starts_with("Wing") {
        Color::srgb(0.58, 0.55, 0.50)
    } else if name == "Planet_Mercury" || name == "Mural_Mercury" {
        Color::srgb(0.55, 0.50, 0.45)
    } else if name == "Planet_Venus" || name == "Mural_Venus" {
        Color::srgb(0.85, 0.72, 0.35)
    } else if name == "Planet_Mars" || name == "Mural_Mars" {
        Color::srgb(0.75, 0.22, 0.12)
    } else {
        Color::srgb(0.5, 0.5, 0.5)
    }
}

/// Puts a material on every mesh of a glTF scene once it has spawned.
#[derive(Component)]
pub struct Recolor(pub Handle<StandardMaterial>);

/// Applies [`PortalSpace`] to everything below it (the portal plugin does this each frame, but
/// doing it as soon as the scene exists avoids a frame in layer 0).
fn recolor_scene(ready: On<WorldInstanceReady>, roots: Query<&Recolor>, children: Query<&Children>, meshes: Query<(), With<Mesh3d>>, mut commands: Commands) {
    let Ok(recolor) = roots.get(ready.entity) else { return };
    for e in children.iter_descendants(ready.entity) {
        if meshes.contains(e) {
            commands.entity(e).insert(MeshMaterial3d(recolor.0.clone()));
        }
    }
}

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LevelIndex>().add_observer(recolor_scene);
    }
}

fn space_of(s: i32) -> Option<i32> {
    (s >= 0).then_some(s)
}

/// Spawns every placed actor except the gameplay Blueprints (runes, seal, ...), which the game
/// module builds.
pub fn spawn_level(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut index: ResMut<LevelIndex>,
) {
    let mut by_name: HashMap<String, Entity> = HashMap::new();

    // Static meshes
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let sphere = meshes.add(Sphere::new(0.5).mesh().uv(32, 18));
    for p in STATICS {
        if p.name == "StarDome" {
            continue; // the game builds its own star dome (game::ending)
        }
        let t = ue::transform(p.loc, p.rot, p.scale);
        let space = space_of(p.space);
        let mut e = commands.spawn((Name::new(p.name), t));
        if let Some(s) = space {
            e.insert(PortalSpace(s));
        }
        e.insert(layers_for_space(space));
        let mat = materials.add(StandardMaterial { base_color: colour(p.name), perceptual_roughness: 0.9, ..default() });
        match p.mesh {
            "Cube" | "Sphere" => {
                let is_blocker = p.name.ends_with("Blocker");
                if is_blocker {
                    // Invisible wall: a collider only.
                    e.insert((Visibility::Hidden, RigidBody::Static, Collider::cuboid(1.0, 1.0, 1.0)));
                } else if p.mesh == "Cube" {
                    e.insert((Mesh3d(cube.clone()), MeshMaterial3d(mat), RigidBody::Static, Collider::cuboid(1.0, 1.0, 1.0)));
                } else {
                    e.insert((Mesh3d(sphere.clone()), MeshMaterial3d(mat), NotShadowCaster));
                }
            }
            glb => {
                let group = if glb.starts_with("SM_Obs") {
                    "Observatory"
                } else if glb.starts_with("SM_Hex") {
                    "Hex"
                } else if glb.starts_with("SM_Out") {
                    "Outdoor"
                } else {
                    "Wings"
                };
                let path = format!("models/{group}/{glb}.glb#Scene0");
                e.insert((
                    WorldAssetRoot(assets.load(path)),
                    Recolor(mat),
                    RigidBody::Static,
                    ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMesh),
                    CollisionLayers::new(Layer::World, [Layer::World, Layer::Player, Layer::Prop]),
                ));
            }
        }
        by_name.insert(p.name.to_string(), e.id());
    }

    // Portals
    let portal_mesh = ();
    let _ = portal_mesh;
    for d in PORTALS {
        let t = ue::transform(d.loc, d.rot, d.scale);
        let mut e = commands.spawn((Name::new(d.name), t, Portal::default(), Visibility::Hidden));
        if let Some(s) = space_of(d.space) {
            e.insert(PortalSpace(s));
        }
        by_name.insert(d.name.to_string(), e.id());
    }
    for d in PORTALS {
        let me = by_name[d.name];
        let front = by_name.get(d.front).copied();
        let back = by_name.get(d.back).copied();
        commands.entity(me).insert(Portal { front_link: front, back_link: back, ..default() });
    }

    // Lights
    for l in LIGHTS {
        let mut e = commands.spawn((
            Name::new(l.name),
            PointLight {
                // Unreal's lumens, scaled to Bevy's exposure (see `game::LIGHT_SCALE`).
                intensity: l.lumens * crate::game::LIGHT_SCALE,
                range: l.radius * ue::CM,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(ue::pos(l.loc)),
        ));
        if let Some(s) = space_of(l.space) {
            e.insert((PortalSpace(s), layers_for_space(Some(s))));
        }
        by_name.insert(l.name.to_string(), e.id());
    }

    // Text
    for t in TEXTS.iter().take(0) {
        let mut e = commands.spawn((
            Name::new(t.text),
            Text2d::new(t.text),
            TextFont { font_size: FontSize::Px(100.0), ..default() },
            TextColor(Color::WHITE),
            Transform { translation: ue::pos(t.loc), rotation: ue::rot(t.rot), scale: Vec3::splat(t.size * ue::CM * 0.011) },
            NotShadowCaster,
            NotShadowReceiver,
        ));
        if let Some(s) = space_of(t.space) {
            e.insert(PortalSpace(s));
        }
    }

    index.by_name = by_name;
}
