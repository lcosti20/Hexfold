//! Picking up, carrying and throwing physics objects, also through portals.
//!
//! The held object is pulled towards a point in front of the eye, keeping its orientation
//! relative to the player's heading. Everything is worked out in the player's own frame
//! (position, view yaw, size and mirrored state), so carrying an object through a portal warps it
//! along with the player:
//!
//! * The object usually reaches a portal first. When its centre crosses, its own traversal warps
//!   it (resized and/or mirrored), and the hold point follows it through.
//! * When the player crosses after it, the two warps cancel, and the object stays put in front
//!   of them.
//!
//! So an object carried through a shrinking portal comes out shrunk with the player, and one
//! carried through a mirroring portal comes out mirrored. To change an object relative to the
//! player, throw it through, or carry it through and walk back around.
//!
//! Aiming also looks through portals (up to two): an object seen through a portal can be picked
//! up. Objects whose bounding radius is above `max_object_radius` (relative to the player's size)
//! can't be lifted.

use avian3d::prelude::*;
use bevy::prelude::*;

use super::clip::PortalClipped;
use super::geom::{PortalSet, matrix_scale};
use super::traversal::{PortalObject, PortalPawn, PortalTraveller, PortalTraversed, decompose_object};

/// Add to the player next to [`PortalPawn`]. Distances are in metres at the player's normal size.
#[derive(Component, Debug, Clone)]
pub struct PortalGrab {
    /// How far away an object can be picked up.
    pub grab_range: f32,
    /// Gap between the eye and the held object's bounding sphere.
    pub hold_gap: f32,
    /// Largest bounding-sphere radius that can be picked up (relative to the player's size).
    pub max_object_radius: f32,
    /// Speed of a thrown object (m/s).
    pub throw_speed: f32,
    /// The object is dropped if it stays this far from where it should be for half a second.
    pub break_distance: f32,
    /// The object being carried.
    pub held: Option<Held>,
}

impl Default for PortalGrab {
    fn default() -> Self {
        Self { grab_range: 2.5, hold_gap: 0.5, max_object_radius: 0.9, throw_speed: 7.0, break_distance: 1.2, held: None }
    }
}

/// A carried object and how it is held.
#[derive(Debug, Clone)]
pub struct Held {
    /// The object.
    pub entity: Entity,
    /// Held object's orientation and size in the player's frame (no translation).
    in_frame: Mat4,
    /// Maps the player's side to the held object's side: the portals between them.
    warp: Mat4,
    /// Eye to object centre, in units of the player's size.
    distance: f32,
    /// How far the object may be from its hold point before it's dropped (player's size units).
    allowed_distance: f32,
    /// How long the object has been further than `allowed_distance` (s).
    stuck_time: f32,
    /// What to restore on release.
    saved_layers: CollisionLayers,
    saved_gravity: Option<GravityScale>,
}

/// Ask the grab to let go of an object (if it is held).
#[derive(Message, Debug, Clone)]
pub struct ReleaseObject(pub Entity);

/// How fast the allowed distance shrinks from where the object was picked up to
/// `break_distance` (player's size units per second).
const PULL_IN_SPEED: f32 = 3.0;
/// Seconds an object may stay out of reach before it is dropped.
const STUCK_TIME_LIMIT: f32 = 0.5;

/// The player's frame: eye position, heading, size, and a left-right flip while mirrored.
fn player_frame(eye: Vec3, yaw: f32, scale: f32, mirrored: bool) -> Mat4 {
    Mat4::from_translation(eye) * Mat4::from_rotation_y(yaw) * Mat4::from_scale(Vec3::new(if mirrored { -scale } else { scale }, scale, scale))
}

/// The first object a ray from the eye hits, looking through up to two portals. Returns the
/// object and the warp that maps the player's side to the object's side.
#[allow(clippy::too_many_arguments)]
fn trace_for_object(
    spatial: &SpatialQuery,
    set: &PortalSet,
    grab: &PortalGrab,
    player: Entity,
    eye: Vec3,
    forward: Vec3,
    size: f32,
    // Radius at the current size for a dynamic portal object, or None for anything else.
    object_radius: &dyn Fn(Entity) -> Option<f32>,
) -> Option<(Entity, Mat4)> {
    let mut from = eye;
    let mut to = eye + forward * (grab.grab_range * size);
    let mut warp = Mat4::IDENTITY;
    let mut skip: Option<Entity> = None;
    let filter = SpatialQueryFilter::from_excluded_entities([player]).with_mask([crate::level::Layer::World, crate::level::Layer::Prop]);
    for _ in 0..3 {
        // The first portal along the segment.
        let mut through: Option<usize> = None;
        let mut fraction = 1.0f32;
        for (i, p) in set.list.iter().enumerate() {
            if Some(p.entity) == skip || !p.intersects(from, to, Vec3::ZERO) {
                continue;
            }
            let da = p.signed_distance(from);
            let db = p.signed_distance(to);
            let t = da / (da - db);
            if t < fraction {
                fraction = t;
                through = Some(i);
            }
        }
        let end = from + (to - from) * fraction;
        let dir = (end - from).normalize_or_zero();
        let len = (end - from).length();
        if let Ok(d) = Dir3::new(dir)
            && len > 1e-4
            && let Some(hit) = spatial.cast_ray(from, d, len, true, &filter)
        {
            let radius = object_radius(hit.entity)?;
            let relative = radius / matrix_scale(&warp);
            return (relative <= grab.max_object_radius * size).then_some((hit.entity, warp));
        }
        let link = through.and_then(|i| set.list[i].link_for(from))?;
        let ti = set.list.iter().position(|g| g.entity == link)?;
        let w = set.list[through?].warp_to(&set.list[ti]);
        from = w.transform_point3(end);
        to = w.transform_point3(to);
        warp = w * warp;
        skip = Some(link);
    }
    None
}

/// The grab, once a frame: follows the portals the player and the held object go through,
/// handles [`ReleaseObject`], E (grab / drop) and the left mouse button (throw), and pulls the
/// held object to its hold point (dropping it if it gets stuck).
#[allow(clippy::type_complexity)]
pub fn update_grab(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    state: Res<State<crate::game::AppState>>,
    set: Res<PortalSet>,
    time: Res<Time>,
    spatial: SpatialQuery,
    mut pawn: Query<(Entity, &mut PortalGrab, &PortalPawn, &PortalTraveller, &Transform), Without<PortalObject>>,
    mut bodies: Query<(&mut Transform, &mut LinearVelocity, &mut AngularVelocity, &CollisionLayers, Option<&mut GravityScale>, &PortalTraveller, &RigidBody, Option<&PortalClipped>), (With<PortalObject>, Without<PortalPawn>)>,
    mut commands: Commands,
    mut traversals: MessageReader<PortalTraversed>,
    mut release_requests: MessageReader<ReleaseObject>,
) {
    let Ok((player, mut grab, pawn, traveller, ptf)) = pawn.single_mut() else { return };
    let size = pawn.scale;
    let eye = pawn.eye_at(ptf.translation);
    let forward = pawn.forward();
    let playing = *state.get() == crate::game::AppState::Playing;

    // Traversals: the player moved by `warp` but the held object didn't (and the other way round).
    for m in traversals.read() {
        let Some(held) = grab.held.as_mut() else { continue };
        if m.entity == player {
            if let (Some(from), Some(to)) = (set.get(m.from), set.get(m.to)) {
                held.warp *= to.warp_to(from);
            }
        } else if m.entity == held.entity
            && let (Some(from), Some(to)) = (set.get(m.from), set.get(m.to))
        {
            held.warp = from.warp_to(to) * held.warp;
        }
    }

    for m in release_requests.read() {
        if grab.held.as_ref().is_some_and(|h| h.entity == m.0) {
            release(&mut grab, &mut bodies, &mut commands, None);
        }
    }

    // Input.
    if playing {
        if keys.just_pressed(KeyCode::KeyE) {
            if grab.held.is_some() {
                release(&mut grab, &mut bodies, &mut commands, None);
            } else if let Some((entity, warp)) = trace_for_object(&spatial, &set, &grab, player, eye, forward, size, &|e| bodies.get(e).ok().and_then(|b| matches!(b.6, RigidBody::Dynamic).then_some(b.7.map_or(0.45, |c| c.radius) * b.0.scale.y.abs()))) {
                let frame = player_frame(eye, pawn.yaw, size, traveller.mirrored);
                if let Ok((t, _, _, layers, gravity, tr, _, clipped)) = bodies.get_mut(entity) {
                    let flip = if tr.mirrored { Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0)) } else { Mat4::IDENTITY };
                    let here = warp.inverse() * t.to_matrix() * flip;
                    let mut in_frame = frame.inverse() * here;
                    in_frame.w_axis = Vec4::W;
                    let radius = clipped.map_or(0.45, |c| c.radius) * t.scale.y.abs();
                    let rel_radius = radius / matrix_scale(&warp);
                    let distance = (grab.hold_gap * size + rel_radius) / size;
                    let held = Held {
                        entity,
                        in_frame,
                        warp,
                        distance,
                        allowed_distance: 0.0,
                        stuck_time: 0.0,
                        saved_layers: *layers,
                        saved_gravity: gravity.as_ref().map(|g| GravityScale(g.0)),
                    };
                    // The pull starts wherever the object is: allow for that, then shrink it.
                    let target = hold_target(&held, eye, forward, size, pawn.yaw, traveller.mirrored);
                    let allowed = t.translation.distance(target.0) / (size * matrix_scale(&warp)) + grab.break_distance;
                    // While held, the object ignores the player and gravity.
                    // Triggers still see it, so a held rune can be seated by bringing it to a socket.
                    commands.entity(entity).insert(CollisionLayers::new(crate::level::Layer::Prop, [crate::level::Layer::World, crate::level::Layer::Trigger]));
                    match gravity {
                        Some(mut g) => g.0 = 0.0,
                        None => {
                            commands.entity(entity).insert(GravityScale(0.0));
                        }
                    }
                    grab.held = Some(Held { allowed_distance: allowed, ..held });
                }
            }
        }
        if mouse.just_pressed(MouseButton::Left)
            && let Some(held) = grab.held.clone()
        {
            let velocity = held.warp.transform_vector3(forward * (grab.throw_speed * size));
            release(&mut grab, &mut bodies, &mut commands, Some(velocity));
        }
    }

    // Carry.
    let break_distance = grab.break_distance;
    let Some(held) = grab.held.as_mut() else { return };
    let Ok((t, mut lv, mut av, ..)) = bodies.get_mut(held.entity) else {
        grab.held = None;
        return;
    };
    // Snap the warp to exactly identity once the player and the object are on the same side
    // again, so rounding doesn't build up over many trips.
    if held.warp.abs_diff_eq(Mat4::IDENTITY, 1e-5) {
        held.warp = Mat4::IDENTITY;
    }
    let (target_pos, target_rot) = hold_target(held, eye, forward, size, pawn.yaw, traveller.mirrored);
    let distance = t.translation.distance(target_pos) / (size * matrix_scale(&held.warp));
    held.allowed_distance = (held.allowed_distance - time.delta_secs() * PULL_IN_SPEED).max(break_distance);
    held.stuck_time = if distance > held.allowed_distance { held.stuck_time + time.delta_secs() } else { 0.0 };
    if held.stuck_time > STUCK_TIME_LIMIT {
        info!("dropped the held object: it got stuck");
        release(&mut grab, &mut bodies, &mut commands, None);
        return;
    }
    // A stiff spring, as velocities: the object keeps up with a walking player.
    let dt = time.delta_secs().max(1e-4);
    let to_target = target_pos - t.translation;
    lv.0 = (to_target / dt.max(1.0 / 20.0) * 0.6).clamp_length_max(25.0 * size);
    let dq = target_rot * t.rotation.inverse();
    let (axis, angle) = dq.normalize().to_axis_angle();
    let angle = if angle > std::f32::consts::PI { angle - std::f32::consts::TAU } else { angle };
    av.0 = axis * angle * 12.0;
}

/// Where the held object should be and how it should be turned: in front of the eye at its hold
/// distance, in the player's frame, then through the portals between the player and the object.
fn hold_target(held: &Held, eye: Vec3, forward: Vec3, size: f32, yaw: f32, mirrored: bool) -> (Vec3, Quat) {
    let frame = player_frame(eye, yaw, size, mirrored);
    let mut target = frame * held.in_frame;
    target.w_axis = (eye + forward * (held.distance * size)).extend(1.0);
    let m = held.warp * target;
    let (loc, rot, _, _) = decompose_object(m);
    (loc, rot)
}

/// Lets go of the held object: restores its collision layers and gravity, stops its spin, and
/// gives it the `throw` velocity (or most of its speed taken away, when dropped).
#[allow(clippy::type_complexity)]
fn release(
    grab: &mut PortalGrab,
    bodies: &mut Query<(&mut Transform, &mut LinearVelocity, &mut AngularVelocity, &CollisionLayers, Option<&mut GravityScale>, &PortalTraveller, &RigidBody, Option<&PortalClipped>), (With<PortalObject>, Without<PortalPawn>)>,
    commands: &mut Commands,
    throw: Option<Vec3>,
) {
    let Some(held) = grab.held.take() else { return };
    if let Ok((_, mut lv, mut av, _, gravity, _, _, _)) = bodies.get_mut(held.entity) {
        commands.entity(held.entity).insert(held.saved_layers);
        if let Some(mut g) = gravity {
            g.0 = held.saved_gravity.map_or(1.0, |s| s.0);
        }
        av.0 = Vec3::ZERO;
        match throw {
            Some(v) => lv.0 = v,
            None => lv.0 *= 0.2,
        }
    }
}

