//! The hex: the win condition. It seals the player in when they walk into the Rotunda, judges
//! runes placed in its six sockets, and opens the door when all are in.
//!
//! Socket check (in order): the hex is sealed and not broken; the socket is enabled and empty;
//! it is the right rune; its world size is 1 (within 10%); it is not mirrored. A rejected rune is
//! pushed gently out and the reason is shown. An accepted one is taken out of the player's hands,
//! frozen in the socket, and lights its glyph.

use avian3d::prelude::*;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;

use crate::level::Layer;
use crate::portal::*;
use crate::ue;

use super::mats::Mats;
use super::player::Player;
use super::rune::Rune;
use super::ui::Toast;

pub const SEAL_LOC: [f32; 3] = [50000.0, 1960.0, 150.0];
pub const SEAL_YAW: f32 = 180.0;
pub const SEAL_SPACE: i32 = 1;

/// Socket k sits on the hex corner at 90 + 60 k degrees (radius 100 cm): socket 0 at the top, then
/// anticlockwise as seen from the room side. Local Unreal coordinates (x, z) in cm.
fn socket_xz(k: usize) -> (f32, f32) {
    let a = (90.0 + 60.0 * k as f32).to_radians();
    (a.cos() * 100.0, a.sin() * 100.0)
}

#[derive(Component)]
pub struct HexSeal {
    pub enabled: [bool; 6],
    pub filled: [bool; 6],
    pub sealed: bool,
    pub broken: bool,
    root: Entity,
    barrier: Entity,
    plates: [Entity; 6],
    glyphs: [Entity; 6],
    seal_trigger: Entity,
    triggers: [Entity; 6],
    seen: [Vec<Entity>; 6],
}

#[derive(Component)]
pub struct FlashPlate(pub Timer);

/// Sent when a rune is locked in or the hex breaks, for the HUD and the ending.
#[derive(Message, Debug, Clone)]
pub enum HexEvent {
    Sealed,
    RuneLit(u8),
    Broken,
}

fn local(x: f32, y: f32, z: f32) -> Vec3 {
    ue::pos([x, y, z])
}

pub fn spawn_hex(mut commands: Commands, assets: Res<AssetServer>, mats: Res<Mats>) {
    let root_t = ue::transform(SEAL_LOC, [0.0, SEAL_YAW, 0.0], [1.0, 1.0, 1.0]);
    let layers = layers_for_space(Some(SEAL_SPACE));
    let hex_root = commands.spawn((Name::new("HexRoot"), Transform::IDENTITY, Visibility::Hidden)).id();

    let ring: Handle<Mesh> = assets.load("models/Hex/SM_Hex_Ring.glb#Mesh0/Primitive0");
    let plate_mesh: Handle<Mesh> = assets.load("models/Hex/SM_Hex_SocketPlate.glb#Mesh0/Primitive0");
    let web = commands.spawn((Name::new("Web"), Mesh3d(ring), MeshMaterial3d(mats.hex_web.clone()), Transform::IDENTITY, NotShadowCaster)).id();
    commands.entity(hex_root).add_child(web);

    let mut plates = [Entity::PLACEHOLDER; 6];
    let mut glyphs = [Entity::PLACEHOLDER; 6];
    let mut triggers = [Entity::PLACEHOLDER; 6];
    for k in 0..6 {
        let (x, z) = socket_xz(k);
        let plate = commands
            .spawn((
                Name::new(format!("Plate{k}")),
                Mesh3d(plate_mesh.clone()),
                MeshMaterial3d(mats.plate_idle.clone()),
                Transform { translation: local(x, -1.0, z), scale: ue::scale([1.2, 1.0, 1.0]), ..default() },
            ))
            .id();
        let glyph_mesh: Handle<Mesh> = assets.load(format!("models/Runes/SM_Glyph_{k}.glb#Mesh0/Primitive0"));
        let glyph = commands
            .spawn((
                Name::new(format!("Glyph{k}")),
                Mesh3d(glyph_mesh),
                MeshMaterial3d(mats.glyph_idle.clone()),
                Transform { translation: local(x, 4.0, z), scale: ue::scale([0.9, 1.0, 0.9]), ..default() },
                NotShadowCaster,
            ))
            .id();
        commands.entity(hex_root).add_children(&[plate, glyph]);
        plates[k] = plate;
        glyphs[k] = glyph;
        // Box trigger 20 cm in front of each socket (half extents 24, 22, 28 cm).
        triggers[k] = commands
            .spawn((
                Name::new(format!("Trigger{k}")),
                Transform::from_translation(local(x, 20.0, z)),
                Collider::cuboid(0.48, 0.56, 0.44),
                Sensor,
                CollidingEntities::default(),
                CollisionLayers::new(Layer::Trigger, [Layer::Prop]),
            ))
            .id();
    }

    // The barrier across the doorway (half extents 155 x 6 x 155 cm): collision off until sealed.
    let barrier = commands
        .spawn((
            Name::new("Barrier"),
            Transform::from_translation(local(0.0, -2.0, 0.0)),
            RigidBody::Static,
            Collider::cuboid(3.1, 3.1, 0.12),
            ColliderDisabled,
            CollisionLayers::new(Layer::World, [Layer::Player, Layer::Prop]),
        ))
        .id();
    // The player walking 4 m into the room (half extents 300 x 100 x 200 cm) seals the door.
    let seal_trigger = commands
        .spawn((
            Name::new("SealTrigger"),
            Transform::from_translation(local(0.0, 400.0, 0.0)),
            Collider::cuboid(6.0, 4.0, 2.0),
            Sensor,
            CollidingEntities::default(),
            CollisionLayers::new(Layer::Trigger, [Layer::Player]),
        ))
        .id();

    let mut e = commands.spawn((
        Name::new("HexSeal"),
        root_t,
        Visibility::Inherited,
        PortalSpace(SEAL_SPACE),
        layers.clone(),
        HexSeal {
            enabled: [true; 6],
            filled: [false; 6],
            sealed: false,
            broken: false,
            root: hex_root,
            barrier,
            plates,
            glyphs,
            seal_trigger,
            triggers,
            seen: Default::default(),
        },
    ));
    e.add_children(&[hex_root, barrier, seal_trigger]);
    e.add_children(&triggers);
}

#[allow(clippy::type_complexity)]
pub fn hex_logic(
    time: Res<Time>,
    mats: Res<Mats>,
    mut commands: Commands,
    mut seals: Query<&mut HexSeal>,
    sensors: Query<&CollidingEntities>,
    player: Query<Entity, With<Player>>,
    runes: Query<(&Rune, &Transform, &PortalTraveller), Without<HexSeal>>,
    seal_tf: Query<&GlobalTransform, With<HexSeal>>,
    mut release: MessageWriter<ReleaseObject>,
    mut events: MessageWriter<HexEvent>,
    mut toast: MessageWriter<Toast>,
    mut flashes: Query<(Entity, &mut FlashPlate)>,
) {
    let Ok(mut seal) = seals.single_mut() else { return };
    let Ok(seal_gt) = seal_tf.single() else { return };

    // Flash timers: put the plate back to idle.
    for (e, mut f) in &mut flashes {
        f.0.tick(time.delta());
        if f.0.is_finished() {
            let idx = seal.plates.iter().position(|p| *p == e);
            let filled = idx.is_some_and(|i| seal.filled[i]);
            commands.entity(e).remove::<FlashPlate>().insert(MeshMaterial3d(if filled { mats.plate_lit.clone() } else { mats.plate_idle.clone() }));
        }
    }

    // Walking into the room seals the door.
    if !seal.sealed && !seal.broken
        && let Ok(c) = sensors.get(seal.seal_trigger)
        && player.single().is_ok_and(|p| c.contains(&p))
    {
        seal.sealed = true;
        commands.entity(seal.root).insert(Visibility::Inherited);
        commands.entity(seal.barrier).remove::<ColliderDisabled>();
        events.write(HexEvent::Sealed);
        toast.write(Toast("A seal closes the door behind you.".into()));
    }
    if !seal.sealed || seal.broken {
        return;
    }

    // Runes entering a socket trigger.
    for k in 0..6 {
        let Ok(c) = sensors.get(seal.triggers[k]) else { continue };
        let now: Vec<Entity> = c.iter().copied().collect();
        let fresh: Vec<Entity> = now.iter().copied().filter(|e| !seal.seen[k].contains(e)).collect();
        seal.seen[k] = now;
        for rune_entity in fresh {
            let Ok((rune, t, tr)) = runes.get(rune_entity) else { continue };
            if !seal.enabled[k] || seal.filled[k] {
                continue;
            }
            // The checks, in order.
            let reason = if rune.index as usize != k {
                Some("That rune doesn't belong here.")
            } else if (t.scale.y - 1.0).abs() > 0.1 {
                Some(if t.scale.y > 1.0 { "The rune is too big for the socket." } else { "The rune is too small for the socket." })
            } else if tr.mirrored {
                Some("The rune is a mirror image.")
            } else {
                None
            };
            release.write(ReleaseObject(rune_entity));
            match reason {
                None => {
                    // Accept: freeze it in the socket, facing the room, and light the socket.
                    let (x, z) = socket_xz(k);
                    let to_world = seal_gt.to_matrix();
                    let loc = to_world.transform_point3(local(x, 7.5, z));
                    let rot = seal_gt.to_scale_rotation_translation().1;
                    commands.entity(rune_entity).insert((
                        RigidBody::Static,
                        Transform { translation: loc, rotation: rot, scale: Vec3::splat(1.0) },
                        Position(loc),
                        Rotation(rot),
                        LinearVelocity::ZERO,
                        AngularVelocity::ZERO,
                        ColliderDisabled,
                    ));
                    commands.entity(seal.plates[k]).insert(MeshMaterial3d(mats.plate_lit.clone()));
                    commands.entity(seal.glyphs[k]).insert(MeshMaterial3d(mats.glyph_lit.clone()));
                    seal.filled[k] = true;
                    events.write(HexEvent::RuneLit(k as u8));
                    toast.write(Toast("The rune locks into place.".into()));
                    if (0..6).all(|i| !seal.enabled[i] || seal.filled[i]) {
                        seal.broken = true;
                        commands.entity(seal.root).insert(Visibility::Hidden);
                        commands.entity(seal.barrier).insert(ColliderDisabled);
                        events.write(HexEvent::Broken);
                        toast.write(Toast("The seal shatters. The door is open.".into()));
                    }
                }
                Some(why) => {
                    // Reject: push it gently into the room and show why on the plate.
                    let room = seal_gt.to_scale_rotation_translation().1 * Vec3::Z; // local +Y (Unreal) points into the room
                    commands.entity(rune_entity).insert((LinearVelocity(room * 2.5 + Vec3::Y * 1.5), AngularVelocity::ZERO));
                    commands.entity(seal.plates[k]).insert((MeshMaterial3d(mats.plate_lit.clone()), FlashPlate(Timer::from_seconds(0.4, TimerMode::Once))));
                    toast.write(Toast(why.into()));
                }
            }
        }
    }
}
