//! The first-person player: a collider body, mouse look and walking, and a visible body model.
//!
//! The portal plugin owns the view angles and size ([`PortalPawn`]), because crossing a portal
//! changes both. This module reads them: movement scales with the player's size, mouse yaw and
//! strafing are reversed while mirrored.

use avian3d::prelude::*;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::Exposure;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::camera::Hdr;
use bevy::render::view::Tonemapping;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use crate::level::Layer;
use crate::portal::render::NODE_LAYER_BASE;
use crate::portal::{PortalCamera, PortalGrab, PortalPawn, PortalProjection, PortalTraveller};
use crate::ue;

/// Collider radius at normal size (m).
pub const RADIUS: f32 = 0.34;
/// Half the collider's height at normal size (m): the body's origin is this far above the feet.
pub const HALF_HEIGHT: f32 = 0.88;
/// Eye height above the body's origin at normal size (m).
pub const EYE_HEIGHT: f32 = 0.70;
/// Walking speed at normal size (m/s).
pub const WALK_SPEED: f32 = 5.0;
/// Upward speed of a jump at normal size (m/s).
pub const JUMP_SPEED: f32 = 4.2;
/// How fast the walking speed changes on the ground (m/s²); 35% of it in the air.
pub const ACCEL: f32 = 40.0;
/// Radians of turn per pixel of mouse movement.
pub const MOUSE_SENSITIVITY: f32 = 0.0022;
/// Mass at normal size (kg); it scales with the cube of the size.
pub const MASS: f32 = 80.0;
/// Gap kept between the bottom of the collider and the floor (see [`walk`]).
pub const HOVER: f32 = 0.025;

/// The gap for a player of this size. A small player keeps the full gap: any closer and the
/// floor's triangle edges start pushing back.
fn hover(scale: f32) -> f32 {
    HOVER * scale.max(1.0)
}
/// How far below its feet the player still counts as standing (and is pulled down to the floor).
const SNAP: f32 = 0.08;
/// Time after a jump during which the player isn't held to the floor.
const JUMP_RELEASE: f32 = 0.2;

/// The player body.
#[derive(Component)]
pub struct Player {
    /// The size the collider and movement were last built for.
    pub applied_scale: f32,
    /// Standing on the level (props don't count).
    pub grounded: bool,
    /// Seconds left before the floor may hold the player again after a jump.
    pub jump_release: f32,
}

/// Where the player started: the respawn point outside.
pub const START: [f32; 3] = [0.0, 2500.0, 100.0];
/// The Unreal yaw at the start: facing the Observatory's door.
pub const START_YAW: f32 = -90.0;

/// The player's collider: a cylinder with chamfered ends. A capsule's rounded bottom would let the
/// player ride up over low objects (a rune lying on the floor), like a ramp; Unreal's character
/// had Can Step Up On = No on runes so they'd be pushed instead. A flat bottom does the same.
///
/// The bottom stops [`HOVER`] short of the feet: the player is held at that height by [`walk`]
/// and doesn't rest on the floor.
pub fn capsule(scale: f32) -> Collider {
    /// Points around the circumference.
    const SEGMENTS: usize = 20;
    /// Size of the chamfer at each end (m, at normal size).
    const CHAMFER: f32 = 0.04;
    let (r, h) = (RADIUS * scale, HALF_HEIGHT * scale);
    let c = CHAMFER * scale;
    let mut points = Vec::with_capacity(SEGMENTS * 4);
    for k in 0..SEGMENTS {
        let a = k as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
        let (s, co) = a.sin_cos();
        let foot = -h + hover(scale);
        for (y, rr) in [(foot, r - c), (foot + c, r), (h - c, r), (h, r - c)] {
            points.push(Vec3::new(co * rr, y, s * rr));
        }
    }
    Collider::convex_hull(&points).expect("player hull")
}

/// Spawns the player body (a dynamic rigid body with locked rotation, the portal pawn, traveller
/// and grab) at the start, and the player's camera.
pub fn spawn_player(mut commands: Commands) {
    let pos = ue::pos(START);
    commands.spawn((
        Name::new("Player"),
        Player { applied_scale: 1.0, grounded: false, jump_release: 0.0 },
        PortalPawn { yaw: ue::view_yaw(START_YAW), pitch: 0.0, scale: 1.0, eye_height: EYE_HEIGHT },
        PortalTraveller::default(),
        // 0.45 m: over a 1x rune (0.43 m) and under the 2x heavy rune (0.86 m), so the heavy rune
        // can only be lifted by a player who has grown to 2x (the limit scales with the player).
        PortalGrab { max_object_radius: 0.45, ..default() },
        Transform::from_translation(pos),
        RigidBody::Dynamic,
        capsule(1.0),
        Mass(MASS),
        LockedAxes::ROTATION_LOCKED,
        Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
        Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
        CollisionLayers::new(Layer::Player, [Layer::World, Layer::Trigger]),
        SleepingDisabled,
        LinearDamping(0.0),
    ));
    commands.spawn((
        Name::new("PlayerCamera"),
        PortalCamera,
        Camera3d::default(),
        Camera { order: 0, ..default() },
        Projection::custom(PortalProjection::default()),
        Hdr,
        Tonemapping::TonyMcMapface,
        Exposure { ev100: super::EXPOSURE_EV100 },
        crate::portal::render::view_cluster_config(),
        bevy::light::AmbientLight::default(),
        Msaa::Sample4,
        Transform::from_translation(pos),
        RenderLayers::layer(0),
    ));
}

/// The player's visible body (`models/Player/SM_Player.glb`, built by `art/Blender/scripts/build_player.py`).
#[derive(Component)]
pub struct PlayerBody;

/// Portal views rendered per frame at most (`PortalSettings::max_captures` defaults to 32).
const MAX_VIEW_LAYERS: usize = 32;

/// The body is drawn by portal views (node layers 1..) but never by the player's own camera
/// (node layer 0), which sits inside its head. So in a portal you see yourself, not from within.
fn body_layers() -> RenderLayers {
    RenderLayers::from_layers(&(1..=MAX_VIEW_LAYERS).map(|n| NODE_LAYER_BASE + n).collect::<Vec<_>>())
}

/// Spawns the visible body model, on the portal views' layers only.
pub fn spawn_player_body(mut commands: Commands, assets: Res<AssetServer>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let layers = body_layers();
    // Same colours as the Blender materials: cloth, gold trim (like the runes), glowing visor.
    let parts = [
        StandardMaterial { base_color: Color::srgb(0.13, 0.16, 0.24), perceptual_roughness: 0.75, ..default() },
        StandardMaterial { base_color: Color::srgb(0.95, 0.72, 0.22), metallic: 0.7, perceptual_roughness: 0.35, ..default() },
        StandardMaterial { base_color: Color::srgb(0.6, 0.85, 1.0), emissive: LinearRgba::rgb(0.8, 1.8, 3.0), perceptual_roughness: 0.2, ..default() },
    ]
    .map(|m| materials.add(m));
    // The model faces Blender's -Y, which glTF export turns into +Z; the game's forward is -Z.
    let visual = commands
        .spawn((Name::new("PlayerBodyVisual"), Transform::from_rotation(Quat::from_rotation_y(std::f32::consts::PI)), Visibility::Inherited, layers.clone()))
        .with_children(|c| {
            for (i, material) in parts.into_iter().enumerate() {
                let mesh: Handle<Mesh> = assets.load(format!("models/Player/SM_Player.glb#Mesh0/Primitive{i}"));
                c.spawn((Mesh3d(mesh), MeshMaterial3d(material), layers.clone()));
            }
        })
        .id();
    commands
        .spawn((Name::new("PlayerBody"), PlayerBody, Transform::default(), Visibility::Inherited, layers))
        .add_child(visual);
}

/// The body stands where the collider does, facing the way the player faces, at the player's size.
pub fn update_body(
    player: Query<(&Transform, &PortalPawn), (With<Player>, Without<PlayerBody>)>,
    mut body: Query<&mut Transform, With<PlayerBody>>,
) {
    let (Ok((t, pawn)), Ok(mut b)) = (player.single(), body.single_mut()) else { return };
    b.translation = t.translation - Vec3::Y * HALF_HEIGHT * pawn.scale;
    b.rotation = Quat::from_rotation_y(pawn.yaw);
    b.scale = Vec3::splat(pawn.scale);
}

/// Locks and hides the mouse cursor while playing; frees it on the menus.
///
/// It writes `CursorOptions` only when the state changes: every write makes bevy_winit grab the
/// cursor again, which in a browser is a fresh pointer-lock request each frame.
///
/// In a browser the first Esc never reaches the game: the browser uses it to release the pointer
/// lock. So on the web, losing the lock while playing pauses the game.
pub fn grab_cursor(
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
    state: Res<State<super::AppState>>,
    mut next: ResMut<NextState<super::AppState>>,
    #[cfg(target_arch = "wasm32")] mouse: Res<ButtonInput<MouseButton>>,
    #[cfg(target_arch = "wasm32")] mut was_locked: Local<bool>,
) {
    let playing = *state.get() == super::AppState::Playing;
    let (grab_mode, visible) = if playing { (CursorGrabMode::Locked, false) } else { (CursorGrabMode::None, true) };
    // A browser refuses a lock asked for without a click, or within about a second of the player
    // pressing Esc; a click in the game asks again.
    #[cfg(target_arch = "wasm32")]
    let retry = playing && !*was_locked && mouse.just_pressed(MouseButton::Left);
    #[cfg(not(target_arch = "wasm32"))]
    let retry = false;
    if retry || state.is_changed() || cursor.grab_mode != grab_mode || cursor.visible != visible {
        cursor.grab_mode = grab_mode;
        cursor.visible = visible;
    }

    #[cfg(target_arch = "wasm32")]
    {
        let locked = web_sys::window()
            .and_then(|w| w.document())
            .is_some_and(|d| d.pointer_lock_element().is_some());
        // The lock is granted a little after it is asked for, so only a lock that was held and
        // then lost counts.
        if playing && *was_locked && !locked {
            next.set(super::AppState::Paused);
        }
        *was_locked = playing && locked;
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = &mut next;
}

/// Mouse look: turns the pawn's yaw and pitch (pitch limited to about ±86°).
pub fn look(
    motion: Res<AccumulatedMouseMotion>,
    state: Res<State<super::AppState>>,
    mut player: Query<(&mut PortalPawn, &PortalTraveller)>,
) {
    if *state.get() != super::AppState::Playing {
        return;
    }
    let Ok((mut pawn, traveller)) = player.single_mut() else { return };
    let d = motion.delta;
    // While mirrored the image is flipped, so turning right on screen turns the other way.
    let yaw_sign = if traveller.mirrored { 1.0 } else { -1.0 };
    pawn.yaw += yaw_sign * d.x * MOUSE_SENSITIVITY;
    pawn.pitch = (pawn.pitch - d.y * MOUSE_SENSITIVITY).clamp(-1.5, 1.5);
    pawn.yaw = pawn.yaw.rem_euclid(std::f32::consts::TAU);
}

/// Walking, jumping and standing.
///
/// The player doesn't rest on the floor. A flat-bottomed hull lying on a flat floor is the worst
/// case for the contact solver: every so often a contact comes back with a tilted normal (at the
/// edges between a floor mesh's triangles, on very large boxes, and far from the origin, where
/// the rooms are), and walking into a tilted contact at speed throws the player into the air.
/// So the hull ends [`HOVER`] above the feet, and this system holds the feet on the floor found by
/// a few rays: the vertical speed is set to close the gap. The hull only touches the floor when
/// landing from a fall.
pub fn walk(
    time: Res<Time>,
    gravity: Res<Gravity>,
    substeps: Res<SubstepCount>,
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<super::AppState>>,
    spatial: SpatialQuery,
    mut player: Query<(Entity, &mut Player, &PortalPawn, &PortalTraveller, &Transform, &mut LinearVelocity)>,
) {
    let Ok((entity, mut p, pawn, traveller, t, mut vel)) = player.single_mut() else { return };
    let scale = pawn.scale;

    let dt = time.delta_secs();
    p.jump_release = (p.jump_release - dt).max(0.0);

    // The floor under the feet: the highest hit of five rays, so the player stays up while any
    // part of the body is over the floor. Props (a rune on the floor) are no floor to stand on:
    // they're pushed by the hull instead.
    let feet = t.translation.y - HALF_HEIGHT * scale;
    let world = SpatialQueryFilter::from_excluded_entities([entity]).with_mask([Layer::World]);
    let mut floor: Option<f32> = None;
    for (ox, oz) in [(0.0, 0.0), (0.6, 0.0), (-0.6, 0.0), (0.0, 0.6), (0.0, -0.6)] {
        let origin = t.translation + Vec3::new(ox, 0.0, oz) * RADIUS * scale;
        let Some(hit) = spatial.cast_ray(origin, Dir3::NEG_Y, (HALF_HEIGHT + SNAP) * scale, true, &world) else { continue };
        let y = origin.y - hit.distance;
        // Anything higher than the bottom of the hull is a step, which the hull has to deal with.
        if y <= feet + hover(scale) * 1.5 {
            floor = Some(floor.map_or(y, |f| f.max(y)));
        }
    }
    let held = floor.filter(|_| p.jump_release <= 0.0);
    if let Some(floor) = held {
        // Close the gap over a few frames, and cancel the fall gravity adds during this step
        // (it is applied once per substep, so on average a little over half of it counts).
        let rate = 15f32.min(0.5 / dt.max(1e-4));
        let n = substeps.0.max(1) as f32;
        vel.y = (floor - feet) * rate + gravity.0.length() * scale * dt * (n + 1.0) / (2.0 * n);
    }
    // The player passes through props, so only the world counts as ground.
    let filter = SpatialQueryFilter::from_excluded_entities([entity]).with_mask([Layer::World]);
    p.grounded = held.is_some() || spatial.cast_ray(t.translation, Dir3::NEG_Y, (HALF_HEIGHT + 0.06) * scale, true, &filter).is_some();

    let mut input = Vec2::ZERO;
    let mut jump = false;
    if *state.get() == super::AppState::Playing {
        if keys.pressed(KeyCode::KeyW) {
            input.y += 1.0;
        }
        if keys.pressed(KeyCode::KeyS) {
            input.y -= 1.0;
        }
        if keys.pressed(KeyCode::KeyD) {
            input.x += 1.0;
        }
        if keys.pressed(KeyCode::KeyA) {
            input.x -= 1.0;
        }
        jump = keys.just_pressed(KeyCode::Space);
    }
    // While mirrored, strafing is reflected so "right" still moves to the right on screen.
    if traveller.mirrored {
        input.x = -input.x;
    }
    let yaw_rot = Quat::from_rotation_y(pawn.yaw);
    let wish = yaw_rot * Vec3::new(input.x, 0.0, -input.y);
    let wish = if wish.length_squared() > 1.0 { wish.normalize() } else { wish };
    let target = wish * WALK_SPEED * scale;
    let accel = ACCEL * scale * if p.grounded { 1.0 } else { 0.35 };
    let current = Vec3::new(vel.x, 0.0, vel.z);
    let diff = target - current;
    let step = accel * dt;
    let new = if diff.length() <= step { target } else { current + diff.normalize() * step };
    vel.x = new.x;
    vel.z = new.z;
    if jump && p.grounded {
        vel.y = JUMP_SPEED * scale;
        p.jump_release = JUMP_RELEASE;
    }
}

/// Rebuilds the collider, mass and gravity when the player's size changes (a scaling portal).
pub fn apply_scale(mut player: Query<(&mut Player, &PortalPawn, &mut Collider, &mut Mass, Option<&mut GravityScale>)>, mut commands: Commands, entity: Query<Entity, With<Player>>) {
    let Ok((mut p, pawn, mut collider, mut mass, gravity)) = player.single_mut() else { return };
    if (p.applied_scale - pawn.scale).abs() < 1e-6 {
        return;
    }
    p.applied_scale = pawn.scale;
    *collider = capsule(pawn.scale);
    *mass = Mass(MASS * pawn.scale.powi(3));
    if let Some(mut g) = gravity {
        g.0 = pawn.scale;
    } else if let Ok(e) = entity.single() {
        commands.entity(e).insert(GravityScale(pawn.scale));
    }
}
