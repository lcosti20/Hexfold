//! Conversion from Unreal Engine's coordinates (the level data came from there) to Bevy's.
//!
//! Unreal is left-handed: X forward, Y right, Z up, centimetres. Bevy is right-handed: X right,
//! Y up, -Z forward, metres. The art pipeline (Blender -> glTF) maps Unreal's (x, y, z) to Bevy's
//! (x, z, y), so levels are converted with the same axis swap: `bevy = (ue.x, ue.z, ue.y) / 100`.
//! A swap is a reflection, which is exactly what takes a left-handed system to a right-handed
//! one without mirroring anything on screen.

use bevy::prelude::*;

pub const CM: f32 = 0.01;

/// A position in Unreal centimetres to Bevy metres.
pub fn pos(v: [f32; 3]) -> Vec3 {
    Vec3::new(v[0], v[2], v[1]) * CM
}

/// A direction or extent (no unit change).
pub fn dir(v: Vec3) -> Vec3 {
    Vec3::new(v.x, v.z, v.y)
}

/// Unreal (pitch, yaw, roll) in degrees to a Bevy rotation. Uses Unreal's `FRotationMatrix`.
pub fn rot(pyr: [f32; 3]) -> Quat {
    let (sp, cp) = pyr[0].to_radians().sin_cos();
    let (sy, cy) = pyr[1].to_radians().sin_cos();
    let (sr, cr) = pyr[2].to_radians().sin_cos();
    // The rotated frame's axes in Unreal world coordinates.
    let x = Vec3::new(cp * cy, cp * sy, sp);
    let y = Vec3::new(sr * sp * cy - cr * sy, sr * sp * sy + cr * cy, -sr * cp);
    let z = Vec3::new(-(cr * sp * cy + sr * sy), cy * sr - cr * sp * sy, cr * cp);
    // Bevy's local X, Y, Z are Unreal's local X, Z, Y.
    Quat::from_mat3(&Mat3::from_cols(dir(x), dir(z), dir(y)))
}

/// Unreal's per-axis scale to Bevy's local axes (Y and Z swap; the sign of X survives).
pub fn scale(s: [f32; 3]) -> Vec3 {
    Vec3::new(s[0], s[2], s[1])
}

pub fn transform(loc: [f32; 3], pyr: [f32; 3], scl: [f32; 3]) -> Transform {
    Transform { translation: pos(loc), rotation: rot(pyr), scale: scale(scl) }
}

/// The Bevy view yaw (see `PortalPawn`) that faces the way Unreal's yaw `deg` faces.
pub fn view_yaw(deg: f32) -> f32 {
    -(deg + 90.0).to_radians()
}
