//! The first-person player: a capsule body, mouse look and walking.
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
use crate::portal::{PortalCamera, PortalGrab, PortalPawn, PortalProjection, PortalTraveller};
use crate::ue;

pub const RADIUS: f32 = 0.34;
pub const HALF_HEIGHT: f32 = 0.88;
pub const EYE_HEIGHT: f32 = 0.70;
pub const WALK_SPEED: f32 = 5.0;
pub const JUMP_SPEED: f32 = 4.2;
pub const ACCEL: f32 = 40.0;
pub const MOUSE_SENSITIVITY: f32 = 0.0022;
pub const MASS: f32 = 80.0;

/// The player body.
#[derive(Component)]
pub struct Player {
    /// The size the collider and movement were last built for.
    pub applied_scale: f32,
    pub grounded: bool,
}

/// Where the player started: the respawn point outside.
pub const START: [f32; 3] = [0.0, 2500.0, 100.0];
pub const START_YAW: f32 = -90.0;

/// The player's collider: a cylinder with chamfered ends. A capsule's rounded bottom would let the
/// player ride up over low objects (a rune lying on the floor), like a ramp; Unreal's character
/// had Can Step Up On = No on runes so they'd be pushed instead. A flat bottom does the same.
pub fn capsule(scale: f32) -> Collider {
    const SEGMENTS: usize = 20;
    const CHAMFER: f32 = 0.04;
    let (r, h) = (RADIUS * scale, HALF_HEIGHT * scale);
    let c = CHAMFER * scale;
    let mut points = Vec::with_capacity(SEGMENTS * 4);
    for k in 0..SEGMENTS {
        let a = k as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
        let (s, co) = a.sin_cos();
        for (y, rr) in [(-h, r - c), (-h + c, r), (h - c, r), (h, r - c)] {
            points.push(Vec3::new(co * rr, y, s * rr));
        }
    }
    Collider::convex_hull(&points).expect("player hull")
}

pub fn spawn_player(mut commands: Commands) {
    let pos = ue::pos(START);
    commands.spawn((
        Name::new("Player"),
        Player { applied_scale: 1.0, grounded: false },
        PortalPawn { yaw: ue::view_yaw(START_YAW), pitch: 0.0, scale: 1.0, eye_height: EYE_HEIGHT },
        PortalTraveller::default(),
        PortalGrab::default(),
        Transform::from_translation(pos),
        RigidBody::Dynamic,
        capsule(1.0),
        Mass(MASS),
        LockedAxes::ROTATION_LOCKED,
        Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
        Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
        CollisionLayers::new(Layer::Player, [Layer::World, Layer::Prop, Layer::Trigger]),
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
        Msaa::Sample4,
        Transform::from_translation(pos),
        RenderLayers::layer(0),
    ));
}

pub fn grab_cursor(mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>, keys: Res<ButtonInput<KeyCode>>, mouse: Res<ButtonInput<MouseButton>>, state: Res<State<super::AppState>>) {
    let playing = *state.get() == super::AppState::Playing;
    if playing {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    } else {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
    let _ = (&keys, &mouse);
}

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

pub fn walk(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<super::AppState>>,
    spatial: SpatialQuery,
    mut player: Query<(Entity, &mut Player, &PortalPawn, &PortalTraveller, &Transform, &mut LinearVelocity)>,
) {
    let Ok((entity, mut p, pawn, traveller, t, mut vel)) = player.single_mut() else { return };
    let scale = pawn.scale;

    // Ground check.
    let filter = SpatialQueryFilter::from_excluded_entities([entity]).with_mask([Layer::World, Layer::Prop]);
    p.grounded = spatial.cast_ray(t.translation, Dir3::NEG_Y, (HALF_HEIGHT + 0.06) * scale, true, &filter).is_some();

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
    let dt = time.delta_secs();
    let accel = ACCEL * scale * if p.grounded { 1.0 } else { 0.35 };
    let current = Vec3::new(vel.x, 0.0, vel.z);
    let diff = target - current;
    let step = accel * dt;
    let new = if diff.length() <= step { target } else { current + diff.normalize() * step };
    vel.x = new.x;
    vel.z = new.z;
    if jump && p.grounded {
        vel.y = JUMP_SPEED * scale;
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
