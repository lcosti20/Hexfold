//! The scripted puzzle actors and the safety nets: the orrery cabinets (rune 2), the telescope dial
//! (rune 6), kill volumes that return runes and respawn the player, and the restore font.

use avian3d::prelude::*;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;

use crate::level::data::{KILL_VOLUMES, KillDef};
use crate::level::{Layer, LevelIndex};
use crate::portal::*;
use crate::ue;

use super::mats::Mats;
use super::player::Player;
use super::rune::{Rune, ReturnToPedestal};
use super::ui::Toast;

// ---------------------------------------------------------------------------------------------
// Rune 2: the Orrery Cabinets
// ---------------------------------------------------------------------------------------------

/// Tracks the order in which the player visits the closet through the three cabinets, and relinks
/// the cabinets so the right sequence opens the vault. The only actor that reacts to portal
/// crossings.
#[derive(Resource)]
pub struct CabinetPuzzle {
    /// Mercury, Venus, Mars.
    pub cabinets: [Entity; 3],
    pub closet_door: Entity,
    pub vault_door: Entity,
    /// Indices into `cabinets`: Mars, Venus, Mercury.
    pub sequence: [usize; 3],
    pub progress: usize,
}

pub fn setup_cabinets(mut commands: Commands, index: Res<LevelIndex>) {
    commands.insert_resource(CabinetPuzzle {
        cabinets: [index.get("Portal_Cabinet_Mercury"), index.get("Portal_Cabinet_Venus"), index.get("Portal_Cabinet_Mars")],
        closet_door: index.get("Portal_ClosetDoor"),
        vault_door: index.get("Portal_VaultDoor"),
        sequence: [2, 1, 0],
        progress: 0,
    });
}

fn link_both(portals: &mut Query<&mut Portal>, from: Entity, to: Entity) {
    if let Ok(mut p) = portals.get_mut(from) {
        p.front_link = Some(to);
        p.back_link = Some(to);
    }
}

/// Every cabinet leads to the closet, except the last cabinet in the sequence, which leads to the
/// vault once the earlier steps are done.
fn update_links(puzzle: &CabinetPuzzle, portals: &mut Query<&mut Portal>) {
    let last = puzzle.sequence[puzzle.sequence.len() - 1];
    for (i, c) in puzzle.cabinets.iter().enumerate() {
        let target = if i == last && puzzle.progress >= puzzle.sequence.len() - 1 { puzzle.vault_door } else { puzzle.closet_door };
        link_both(portals, *c, target);
    }
}

pub fn cabinet_logic(
    mut messages: MessageReader<PortalTraversed>,
    puzzle: Option<ResMut<CabinetPuzzle>>,
    pawn: Query<Entity, With<PortalPawn>>,
    mut portals: Query<&mut Portal>,
    mut toast: MessageWriter<Toast>,
) {
    let Some(mut puzzle) = puzzle else { return };
    let Ok(player) = pawn.single() else { return };
    for m in messages.read() {
        if m.entity != player {
            continue;
        }
        let Some(i) = puzzle.cabinets.iter().position(|c| *c == m.from) else { continue };
        if m.to == puzzle.closet_door {
            // The closet door leads back to the cabinet you came from.
            link_both(&mut portals, puzzle.closet_door, puzzle.cabinets[i]);
            if i == puzzle.sequence[puzzle.progress.min(puzzle.sequence.len() - 1)] {
                puzzle.progress += 1;
                toast.write(Toast(if puzzle.progress + 1 >= puzzle.sequence.len() { "The orrery turns into alignment.".into() } else { "Something clicks forward.".into() }));
            } else {
                puzzle.progress = usize::from(i == puzzle.sequence[0]);
                toast.write(Toast("The mechanism grinds back.".into()));
            }
        } else if m.to == puzzle.vault_door {
            link_both(&mut portals, puzzle.vault_door, puzzle.cabinets[i]);
            puzzle.progress = 0;
        }
        update_links(&puzzle, &mut portals);
    }
}

// ---------------------------------------------------------------------------------------------
// Rune 6: the Telescope
// ---------------------------------------------------------------------------------------------

#[derive(Resource)]
pub struct Telescope {
    lens: Entity,
    frame: Entity,
    base: [f32; 3],
    setting: i32,
    plates: [Entity; 3],
}

const TELESCOPE_ROOT: [f32; 3] = [290000.0, 700.0, 0.0];
const TELESCOPE_SPACE: i32 = 9;
const PLATE_X: [f32; 3] = [450.0, 750.0, 1050.0];

pub fn spawn_telescope(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>, index: Res<LevelIndex>) {
    let disc = meshes.add(Cylinder::new(0.5, 1.0));
    let colours = [Color::srgb(0.3, 0.5, 1.0), Color::srgb(0.9, 0.7, 0.2), Color::srgb(0.85, 0.25, 0.35)];
    let mut plates = [Entity::PLACEHOLDER; 3];
    for (k, x) in PLATE_X.iter().enumerate() {
        let c = colours[k];
        let mat = materials.add(StandardMaterial { base_color: c, emissive: c.to_linear() * 0.6, ..default() });
        // The dial's floor plate: a flat disc 1 m across, 6 cm thick (a cylinder of height 1 scaled 0.06).
        commands.spawn((
            Name::new(format!("TelescopePlate{k}")),
            Mesh3d(disc.clone()),
            MeshMaterial3d(mat),
            Transform { translation: ue::pos([TELESCOPE_ROOT[0] + x, TELESCOPE_ROOT[1], 3.0]), scale: Vec3::new(1.0, 0.06, 1.0), ..default() },
            PortalSpace(TELESCOPE_SPACE),
            layers_for_space(Some(TELESCOPE_SPACE)),
            NotShadowCaster,
        ));
        plates[k] = commands
            .spawn((
                Name::new(format!("TelescopeTrigger{k}")),
                Transform::from_translation(ue::pos([TELESCOPE_ROOT[0] + x, TELESCOPE_ROOT[1], 60.0])),
                Collider::cuboid(1.0, 1.2, 1.0),
                Sensor,
                CollidingEntities::default(),
                CollisionLayers::new(Layer::Trigger, [Layer::Player]),
            ))
            .id();
    }
    commands.insert_resource(Telescope {
        lens: index.get("Portal_Telescope_Lens"),
        frame: index.get("Telescope_Lens_Frame"),
        base: [290000.0, 1800.0, 0.0],
        setting: -1,
        plates,
    });
}

pub fn telescope_logic(
    telescope: Option<ResMut<Telescope>>,
    sensors: Query<&CollidingEntities>,
    player: Query<Entity, With<Player>>,
    mut transforms: Query<&mut Transform>,
    mut toast: MessageWriter<Toast>,
) {
    let Some(mut t) = telescope else { return };
    let Ok(p) = player.single() else { return };
    for i in 0..3 {
        let Ok(c) = sensors.get(t.plates[i]) else { continue };
        if c.contains(&p) && t.setting != i as i32 {
            t.setting = i as i32;
            // Gemini mirrors, Lynx halves, Cygnus doubles; standing on the floor.
            let (sx, s, name) = match i {
                0 => (-1.0, 1.0, "Gemini: the twins. The lens mirrors."),
                1 => (0.5, 0.5, "Lynx: the lens halves."),
                _ => (2.0, 2.0, "Cygnus: the lens doubles."),
            };
            for e in [t.lens, t.frame] {
                if let Ok(mut tf) = transforms.get_mut(e) {
                    tf.translation = ue::pos([t.base[0], t.base[1], s * 100.0]);
                    tf.scale = ue::scale([sx, s, s]);
                }
            }
            toast.write(Toast(name.into()));
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Safety nets
// ---------------------------------------------------------------------------------------------

#[derive(Component)]
pub struct KillVolume {
    pub respawn: [f32; 3],
    pub respawn_yaw: f32,
    pub respawn_space: i32,
}

/// The space of the area around an x coordinate (Unreal cm); the rooms are 30 000 cm apart.
pub fn space_for_x(x: f32) -> i32 {
    match (x / 10000.0).round() as i32 {
        0 => 0,
        5 => 1,
        8 => 2,
        11 => 3,
        14 => 4,
        17 => 5,
        20 => 6,
        23 => 7,
        26 => 8,
        29 => 9,
        _ => 0,
    }
}

pub fn spawn_kill_volumes(mut commands: Commands) {
    for k in KILL_VOLUMES {
        commands.spawn(kill_volume_bundle(k));
    }
}

fn kill_volume_bundle(k: &KillDef) -> impl Bundle {
    // BoxExtent X is 30 000 for the outside slab, 600 for the chasm pit, and as wide as it is deep
    // for the others; Y and Z are the default 32 cm times the actor scale.
    let half_x = if k.name == "KV_Outside" {
        30000.0
    } else if k.name == "KV_ChasmPit" {
        600.0
    } else {
        32.0 * k.scale[1]
    };
    let half = Vec3::new(half_x, 32.0 * k.scale[2], 32.0 * k.scale[1]) * ue::CM; // (x, z up, y)
    (
        Name::new(k.name),
        Transform::from_translation(ue::pos(k.loc)),
        Collider::cuboid(half.x * 2.0, half.y * 2.0, half.z * 2.0),
        Sensor,
        CollidingEntities::default(),
        CollisionLayers::new(Layer::Trigger, [Layer::Player, Layer::Prop]),
        KillVolume { respawn: k.respawn, respawn_yaw: k.respawn_yaw, respawn_space: space_for_x(k.respawn[0]) },
    )
}

/// Puts the player back: stopped, 1x, unmirrored, facing the same way, tracking reset.
#[allow(clippy::type_complexity)]
pub fn respawn_player(
    loc: [f32; 3],
    yaw_deg: f32,
    space: i32,
    player: &mut (Mut<Transform>, Mut<PortalPawn>, Mut<PortalTraveller>, Mut<LinearVelocity>, Option<Mut<Position>>),
    player_space: &mut PlayerSpace,
) {
    let (t, pawn, tr, lv, pos) = player;
    t.translation = ue::pos(loc);
    pawn.yaw = ue::view_yaw(yaw_deg);
    pawn.pitch = 0.0;
    pawn.scale = 1.0;
    tr.mirrored = false;
    tr.reset_tracking();
    lv.0 = Vec3::ZERO;
    if let Some(p) = pos {
        p.0 = t.translation;
    }
    player_space.0 = Some(space);
}

#[allow(clippy::type_complexity)]
pub fn kill_volume_logic(
    volumes: Query<(&KillVolume, &CollidingEntities)>,
    runes: Query<(), With<Rune>>,
    mut players: Query<(&mut Transform, &mut PortalPawn, &mut PortalTraveller, &mut LinearVelocity, Option<&mut Position>), With<Player>>,
    mut player_space: ResMut<PlayerSpace>,
    mut returns: MessageWriter<ReturnToPedestal>,
) {
    for (kv, colliding) in &volumes {
        for e in colliding.iter() {
            if runes.contains(*e) {
                returns.write(ReturnToPedestal(*e));
            } else if let Ok((t, pawn, tr, lv, pos)) = players.get_mut(*e) {
                let mut p = (t, pawn, tr, lv, pos);
                respawn_player(kv.respawn, kv.respawn_yaw, kv.respawn_space, &mut p, &mut player_space);
            }
        }
    }
}

/// A glowing pad in the Rotunda: stand on it to be restored to normal size and unmirrored.
#[derive(Component)]
pub struct RestoreFont;

pub fn spawn_restore_font(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mats: Res<Mats>) {
    let centre = [50000.0, 1100.0, 150.0];
    let pad = meshes.add(Cylinder::new(0.5, 1.0));
    commands.spawn((
        Name::new("RestoreFont"),
        RestoreFont,
        Transform::from_translation(ue::pos(centre)),
        // Half extents 120 x 120 x 150 cm.
        Collider::cuboid(2.4, 3.0, 2.4),
        Sensor,
        CollidingEntities::default(),
        CollisionLayers::new(Layer::Trigger, [Layer::Player]),
        PortalSpace(1),
        layers_for_space(Some(1)),
        Visibility::Inherited,
        children![(
            Mesh3d(pad),
            MeshMaterial3d(mats.font_pad.clone()),
            // Cylinder scaled 2.4 x 2.4 x 0.03 (diameter 2.4 m, 3 cm thick), 148 cm below the volume's centre.
            Transform { translation: Vec3::new(0.0, -1.48, 0.0), scale: Vec3::new(2.4, 0.03, 2.4), ..default() },
            NotShadowCaster,
        )],
    ));
}

pub fn restore_font_logic(
    fonts: Query<&CollidingEntities, With<RestoreFont>>,
    mut players: Query<(Entity, &mut PortalPawn, &mut PortalTraveller), With<Player>>,
    mut toast: MessageWriter<Toast>,
) {
    let Ok(c) = fonts.single() else { return };
    let Ok((e, mut pawn, mut tr)) = players.single_mut() else { return };
    if c.contains(&e) && ((pawn.scale - 1.0).abs() > 0.01 || tr.mirrored) {
        pawn.scale = 1.0;
        tr.mirrored = false;
        toast.write(Toast("The font's light restores you.".into()));
    }
}
