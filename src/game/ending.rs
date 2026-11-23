//! The night sky, and the ending: breaking the hex turns the moon gold and lets it sink towards a
//! dawn horizon; walking back outside shows the escape screen.

use bevy::camera::visibility::RenderLayers;
use bevy::light::{DirectionalLight, GlobalAmbientLight, NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::asset::RenderAssetUsages;

use crate::portal::{PortalSpace, layers_for_space};
use crate::ue;

use super::hex::HexEvent;
use super::player::Player;
use super::{AppState, Ending};

#[derive(Component)]
pub struct MoonLight;
#[derive(Component)]
pub struct StarDome(pub Handle<StandardMaterial>);

/// A star field: dark blue with scattered bright points.
fn star_texture() -> Image {
    let (w, h) = (2048u32, 1024u32);
    let mut data = vec![0u8; (w * h * 4) as usize];
    let mut seed: u32 = 0x1234_5678;
    let mut rand = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed as f32 / u32::MAX as f32
    };
    for y in 0..h {
        // A faint glow towards the horizon.
        let t = 1.0 - (y as f32 / h as f32 - 0.5).abs() * 2.0;
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            data[i] = (6.0 + 10.0 * t * t) as u8;
            data[i + 1] = (10.0 + 14.0 * t * t) as u8;
            data[i + 2] = (28.0 + 30.0 * t * t) as u8;
            data[i + 3] = 255;
        }
    }
    for _ in 0..5200 {
        let x = (rand() * w as f32) as u32 % w;
        let y = (rand() * h as f32) as u32 % h;
        let b = 90.0 + rand() * 165.0;
        for (dx, dy, f) in [(0i32, 0i32, 1.0f32), (1, 0, 0.5), (0, 1, 0.5)] {
            let (px, py) = ((x as i32 + dx) as u32 % w, (y as i32 + dy) as u32 % h);
            let i = ((py * w + px) * 4) as usize;
            let v = (b * f) as u8;
            data[i] = data[i].max(v);
            data[i + 1] = data[i + 1].max(v);
            data[i + 2] = data[i + 2].max((v as f32 * 1.05).min(255.0) as u8);
        }
    }
    Image::new(Extent3d { width: w, height: h, depth_or_array_layers: 1 }, TextureDimension::D2, data, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::default())
}

pub fn spawn_sky(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>, mut images: ResMut<Assets<Image>>) {
    let tex = images.add(star_texture());
    let mat = materials.add(StandardMaterial {
        // Unlit materials show only the base colour, so the star texture goes there.
        base_color: Color::WHITE,
        base_color_texture: Some(tex),
        unlit: true,
        cull_mode: None,
        ..default()
    });
    // The dome is only seen from outside (space 0).
    commands.spawn((
        Name::new("StarDome"),
        StarDome(mat.clone()),
        Mesh3d(meshes.add(Sphere::new(1.0).mesh().uv(48, 24))),
        MeshMaterial3d(mat),
        Transform::from_scale(Vec3::splat(600.0)),
        PortalSpace(0),
        layers_for_space(Some(0)),
        NotShadowCaster,
        NotShadowReceiver,
    ));
    // The moon: 0.6 lux of blue light from yaw -120, pitch -38 (Unreal). Outside only.
    commands.spawn((
        Name::new("Moonlight"),
        MoonLight,
        DirectionalLight { illuminance: 0.6 * super::MOON_SCALE, color: Color::srgb(0.5, 0.62, 1.0), shadow_maps_enabled: true, ..default() },
        Transform::from_rotation(moon_rotation(-38.0)),
        PortalSpace(0),
        RenderLayers::layer(crate::portal::space_layer(0)),
    ));
    // A faint dusk sun, as in the Unreal level.
    commands.spawn((
        Name::new("Sun"),
        DirectionalLight { illuminance: 0.05 * super::MOON_SCALE, color: Color::WHITE, shadow_maps_enabled: false, ..default() },
        Transform::from_rotation(ue::rot([-15.0, 30.0, 0.0]) * Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
        PortalSpace(0),
        RenderLayers::layer(crate::portal::space_layer(0)),
    ));
}

/// A directional light pointing along Unreal's (pitch, yaw 0, roll -120)-style rotation: the light
/// shines along its local forward (X in Unreal, -Z in Bevy).
fn moon_rotation(pitch: f32) -> Quat {
    // Unreal's forward is its local X; a Bevy light shines along -Z.
    ue::rot([pitch, -120.0, 0.0]) * Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)
}

/// Starts the sunrise when the hex breaks, and the countdown to the escape screen.
pub fn ending_logic(
    time: Res<Time>,
    mut events: MessageReader<HexEvent>,
    mut ending: ResMut<Ending>,
    mut moon: Query<(&mut DirectionalLight, &mut Transform), With<MoonLight>>,
    mut clear: ResMut<ClearColor>,
    mut ambient: ResMut<GlobalAmbientLight>,
    domes: Query<&StarDome>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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
            light.color = Color::srgb(0.5 + 0.5 * a, 0.62 + 0.08 * a, 1.0 - 0.65 * a);
            light.illuminance = (0.6 + 40.0 * a) * super::MOON_SCALE;
            *t = Transform::from_rotation(moon_rotation(-38.0 + 28.0 * a));
        }
        for d in &domes {
            if let Some(mut m) = materials.get_mut(&d.0) {
                // Stars fade and the sky warms towards dawn.
                let g = 1.0 - 0.6 * a;
                m.base_color = Color::linear_rgb(g + 1.2 * a, g + 0.55 * a, g * 0.9 + 0.2 * a);
            }
        }
        clear.0 = Color::srgb(0.01 + 0.5 * a, 0.015 + 0.25 * a, 0.04 + 0.1 * a);
        ambient.brightness = 8.0 + 30.0 * a;
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
