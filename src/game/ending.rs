//! The night sky, and the ending: breaking the hex starts a 12 s sunrise (the moon turns gold
//! and sinks towards the horizon, the sky turns orange, the stars fade, the outdoor ambient light
//! warms up); walking back outside shows the escape screen two seconds later.

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::light::{DirectionalLight, NotShadowCaster, NotShadowReceiver};
use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::portal::{PortalSettings, PortalSpace, layers_for_space};
use crate::ue;

use super::hex::HexEvent;
use super::player::Player;
use super::{AppState, Ending};

/// The moon: the outdoor directional light. At the sunrise it turns gold, brightens and sinks
/// towards the horizon, becoming the dawn sun.
#[derive(Component)]
pub struct MoonLight;

/// The sky: a gradient dome (night blue to an orange dawn) with a star dome inside it.
#[derive(Resource)]
pub struct Sky {
    /// The star dome's material (faded out during the sunrise).
    stars: Handle<StandardMaterial>,
    /// The gradient dome's material (unlit; the colours are in the mesh's vertex colours).
    gradient: Handle<StandardMaterial>,
    /// The gradient dome's mesh, repainted as the sunrise progresses.
    mesh: Handle<Mesh>,
    /// The direction of each of the mesh's vertices from the centre.
    dirs: Vec<Vec3>,
}

/// Moonlight on the outdoor scene (lux) before the sunrise.
const MOON_LUX: f32 = 3.0;
/// Outdoor ambient light (linear colour, brightness) at night and at dawn.
const NIGHT_AMBIENT: (Vec3, f32) = (Vec3::new(0.35, 0.45, 0.85), 0.3);
/// See [`NIGHT_AMBIENT`].
const DAWN_AMBIENT: (Vec3, f32) = (Vec3::new(1.0, 0.7, 0.5), 7.0);

/// The ambient light outside at sunrise progress `sunrise` (0 = night, 1 = dawn).
pub fn outdoor_ambient(sunrise: f32) -> (Color, f32) {
    let a = smooth(sunrise);
    let c = NIGHT_AMBIENT.0.lerp(DAWN_AMBIENT.0, a);
    (Color::linear_rgb(c.x, c.y, c.z), NIGHT_AMBIENT.1 + (DAWN_AMBIENT.1 - NIGHT_AMBIENT.1) * a)
}

/// Smoothstep: eases `t` (clamped to 0..1) in and out.
fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A star field on black, drawn additively over the gradient. Stars are widened towards the poles
/// so they don't stretch into streaks on the sphere.
fn star_texture() -> Image {
    let (w, h) = (4096u32, 2048u32);
    let mut data = vec![0u8; (w * h * 4) as usize];
    for px in data.as_chunks_mut::<4>().0 {
        px[3] = 255;
    }
    let mut seed: u32 = 0x1234_5678;
    let mut rand = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed as f32 / u32::MAX as f32
    };
    for _ in 0..7000 {
        let x = (rand() * w as f32) as i32;
        let lat = (2.0 * rand() - 1.0).asin(); // uniform on the sphere
        let y = ((0.5 - lat / std::f32::consts::PI) * h as f32) as i32;
        let spread = (1.0 / lat.cos().max(0.05)).min(12.0);
        let b = 0.25 + rand() * rand() * 0.75;
        let tint = [0.85 + 0.15 * rand(), 0.9 + 0.1 * rand(), 1.0];
        for dy in -1i32..=1 {
            let reach = (spread * if dy == 0 { 1.0 } else { 0.5 }).round() as i32;
            for dx in -reach..=reach {
                let falloff = (1.0 - dx.abs() as f32 / (reach as f32 + 1.0)) * if dy == 0 { 1.0 } else { 0.4 };
                let (px, py) = ((x + dx).rem_euclid(w as i32) as u32, (y + dy).clamp(0, h as i32 - 1) as u32);
                let i = ((py * w + px) * 4) as usize;
                for c in 0..3 {
                    data[i + c] = data[i + c].max((255.0 * b * falloff * tint[c]) as u8);
                }
            }
        }
    }
    Image::new(Extent3d { width: w, height: h, depth_or_array_layers: 1 }, TextureDimension::D2, data, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::default())
}

/// Spawns the gradient and star domes, the moonlight and a faint dusk sun, all outside only
/// (space 0), and inserts the [`Sky`] resource.
pub fn spawn_sky(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>, mut images: ResMut<Assets<Image>>) {
    let tex = images.add(star_texture());
    let stars = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(tex),
        unlit: true,
        alpha_mode: AlphaMode::Add,
        cull_mode: None,
        ..default()
    });
    let gradient = materials.add(StandardMaterial { base_color: Color::WHITE, unlit: true, cull_mode: None, ..default() });
    let mut dome = Sphere::new(1.0).mesh().uv(96, 48);
    let dirs: Vec<Vec3> = match dome.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(VertexAttributeValues::Float32x3(v)) => v.iter().map(|p| Vec3::from(*p).normalize_or_zero()).collect(),
        _ => Vec::new(),
    };
    dome.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![[0.0f32, 0.0, 0.0, 1.0]; dirs.len()]);
    let mesh = meshes.add(dome);
    // The domes are only seen from outside (space 0).
    let layers = layers_for_space(Some(0));
    commands.spawn((
        Name::new("SkyGradient"),
        Mesh3d(mesh.clone()),
        MeshMaterial3d(gradient.clone()),
        Transform::from_scale(Vec3::splat(600.0)),
        PortalSpace(0),
        layers.clone(),
        NotShadowCaster,
        NotShadowReceiver,
    ));
    commands.spawn((
        Name::new("StarDome"),
        Mesh3d(meshes.add(Sphere::new(1.0).mesh().uv(64, 32))),
        MeshMaterial3d(stars.clone()),
        Transform::from_scale(Vec3::splat(580.0)),
        PortalSpace(0),
        layers,
        NotShadowCaster,
        NotShadowReceiver,
    ));
    commands.insert_resource(Sky { stars, gradient, mesh, dirs });
    // The moon: blue light from yaw -120, pitch -38 (Unreal). Outside only.
    commands.spawn((
        Name::new("Moonlight"),
        MoonLight,
        DirectionalLight { illuminance: MOON_LUX * super::MOON_SCALE, color: Color::srgb(0.5, 0.62, 1.0), shadow_maps_enabled: true, ..default() },
        Transform::from_rotation(moon_rotation(-38.0)),
        PortalSpace(0),
        RenderLayers::layer(crate::portal::space_layer(0)),
    ));
    // A faint dusk sun, as in the Unreal level.
    commands.spawn((
        Name::new("Sun"),
        DirectionalLight { illuminance: 0.3 * super::MOON_SCALE, color: Color::srgb(1.0, 0.85, 0.7), shadow_maps_enabled: false, ..default() },
        Transform::from_rotation(ue::rot([-15.0, 30.0, 0.0]) * Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
        PortalSpace(0),
        RenderLayers::layer(crate::portal::space_layer(0)),
    ));
}

/// The moon's rotation at an Unreal `pitch` (degrees; yaw -120). The light shines along its local
/// forward (X in Unreal, -Z in Bevy).
fn moon_rotation(pitch: f32) -> Quat {
    // Unreal's forward is its local X; a Bevy light shines along -Z.
    ue::rot([pitch, -120.0, 0.0]) * Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)
}

/// The moon's pitch (degrees) at sunrise progress `sunrise`: from -38 (night) to -10 (dawn).
fn sunrise_pitch(sunrise: f32) -> f32 {
    -38.0 + 28.0 * sunrise
}

/// What the sky looks like in direction `dir` at sunrise progress `a`, as displayed (linear) pixels.
fn sky_colour(dir: Vec3, sun: Vec3, a: f32) -> Vec3 {
    let t = dir.y.clamp(0.0, 1.0);
    let night = Vec3::new(0.012, 0.02, 0.05).lerp(Vec3::new(0.002, 0.004, 0.02), t.sqrt());

    let dawn_zenith = Vec3::new(0.05, 0.10, 0.32);
    let dawn_mid = Vec3::new(0.40, 0.22, 0.28);
    let dawn_horizon = Vec3::new(0.95, 0.38, 0.08);
    let mut dawn = if t < 0.35 { dawn_horizon.lerp(dawn_mid, (t / 0.35).sqrt()) } else { dawn_mid.lerp(dawn_zenith, ((t - 0.35) / 0.65).sqrt()) };
    // A glow where the sun is coming up.
    let flat = |v: Vec3| Vec3::new(v.x, 0.0, v.z).normalize_or_zero();
    let toward_sun = flat(dir).dot(flat(sun)).max(0.0).powi(4);
    let low = (1.0 - t).powi(3);
    dawn += Vec3::new(1.4, 0.7, 0.2) * toward_sun * low;
    dawn += Vec3::new(2.5, 1.6, 0.7) * dir.dot(sun).max(0.0).powi(80);
    night.lerp(dawn, smooth(a))
}

/// Paints the sky for the current sunrise and fades the stars.
pub fn update_sky(
    sky: Option<Res<Sky>>,
    ending: Res<Ending>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut painted: Local<f32>,
) {
    let Some(sky) = sky else { return };
    if *painted != ending.sunrise + 1.0 {
        *painted = ending.sunrise + 1.0;
        let sun = moon_rotation(sunrise_pitch(ending.sunrise)) * Vec3::Z;
        let colours: Vec<[f32; 4]> = sky
            .dirs
            .iter()
            .map(|d| {
                let c = sky_colour(*d, sun, ending.sunrise);
                [c.x, c.y, c.z, 1.0]
            })
            .collect();
        if let Some(mut m) = meshes.get_mut(&sky.mesh) {
            m.insert_attribute(Mesh::ATTRIBUTE_COLOR, colours);
        }
    }
    // Unlit materials skip the camera exposure, so the sky shows exactly the colours painted here.
    if let Some(mut m) = materials.get_mut(&sky.gradient) {
        m.base_color = Color::WHITE;
    }
    if let Some(mut m) = materials.get_mut(&sky.stars) {
        // The stars fade out as the sky brightens.
        let b = (1.0 - ending.sunrise * 1.5).clamp(0.0, 1.0) * 0.9;
        m.base_color = Color::linear_rgb(b, b, b);
    }
}

/// Starts the sunrise when the hex breaks, and the countdown to the escape screen.
pub fn ending_logic(
    time: Res<Time>,
    mut events: MessageReader<HexEvent>,
    mut ending: ResMut<Ending>,
    mut moon: Query<(&mut DirectionalLight, &mut Transform), With<MoonLight>>,
    mut clear: ResMut<ClearColor>,
    mut settings: ResMut<PortalSettings>,
    player: Query<&Transform, (With<Player>, Without<MoonLight>)>,
    mut next: ResMut<NextState<AppState>>,
) {
    for e in events.read() {
        if matches!(e, HexEvent::Broken) {
            ending.broken = true;
        }
    }
    if !ending.broken {
        return;
    }
    // SunriseStep: 12 seconds from night to a gold dawn.
    if ending.sunrise < 1.0 {
        ending.sunrise = (ending.sunrise + time.delta_secs() / 12.0).min(1.0);
        let a = ending.sunrise;
        if let Ok((mut light, mut t)) = moon.single_mut() {
            let k = smooth(a);
            light.color = Color::srgb(0.5 + 0.5 * k, 0.62 + 0.08 * k, 1.0 - 0.65 * k);
            light.illuminance = (MOON_LUX + 40.0 * k) * super::MOON_SCALE;
            *t = Transform::from_rotation(moon_rotation(sunrise_pitch(a)));
        }
        // Whatever shows past the sky dome takes the horizon colour.
        clear.0 = Color::srgb(0.01 + 0.5 * a, 0.015 + 0.25 * a, 0.04 + 0.1 * a);
        let (colour, brightness) = outdoor_ambient(a);
        if let Some(entry) = settings.space_ambient.iter_mut().find(|e| e.0 == 0) {
            entry.1 = colour;
            entry.2 = brightness;
        }
    }
    // CheckEscape: back outside (more than 250 m from the Rotunda's side of the world).
    if !ending.escaped
        && let Ok(t) = player.single()
        && t.translation.x < 250.0
    {
        ending.escaped = true;
        ending.escape_timer = 2.0;
    }
    if ending.escaped && ending.escape_timer > 0.0 {
        ending.escape_timer -= time.delta_secs();
        if ending.escape_timer <= 0.0 {
            next.set(AppState::Escaped);
        }
    }
}
