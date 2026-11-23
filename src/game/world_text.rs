//! Text in the world (the mural's numerals, the mirror gallery's inscription, the telescope
//! dial's names). Bevy draws `Text2d` only through 2D cameras, so each label is baked once into an
//! image by a throwaway 2D camera, then shown on an unlit quad.

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ClearColorConfig, RenderTarget};
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::render::view::Msaa;

use crate::level::data::TEXTS;
use crate::portal::{PortalSpace, layers_for_space};
use crate::ue;

const IMAGE_W: u32 = 1024;
const IMAGE_H: u32 = 256;
/// First render layer used for baking (portal views use 24..).
const BAKE_LAYER: usize = 100;

#[derive(Component)]
pub struct TextBake {
    frames: u8,
}

#[allow(clippy::too_many_arguments)]
fn spawn_label(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    index: usize,
    text: &str,
    letter_height: f32,
    transform: Transform,
    space: Option<i32>,
) {
    let mut image = Image::new_uninit(
        Extent3d { width: IMAGE_W, height: IMAGE_H, depth_or_array_layers: 1 },
        TextureDimension::D2,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    let handle = images.add(image);
    let layer = RenderLayers::from_layers(&[BAKE_LAYER + index]);

    commands.spawn((
        Name::new(format!("TextBake{index}")),
        TextBake { frames: 0 },
        Camera2d,
        Camera { order: -200 - index as isize, clear_color: ClearColorConfig::Custom(Color::srgba(1.0, 1.0, 1.0, 0.0)), ..default() },
        RenderTarget::Image(handle.clone().into()),
        Msaa::Off,
        layer.clone(),
    ));
    commands.spawn((
        Text2d::new(text),
        TextFont { font_size: FontSize::Px(120.0), ..default() },
        TextColor(Color::WHITE),
        TextLayout::justify(Justify::Center),
        layer,
    ));

    // The quad: IMAGE_W : IMAGE_H, with the letters about half its height.
    let h = letter_height * 2.0;
    let w = h * IMAGE_W as f32 / IMAGE_H as f32;
    let material = materials.add(StandardMaterial {
        base_color: Color::BLACK,
        emissive: LinearRgba::rgb(1.6, 1.5, 1.3),
        emissive_texture: Some(handle.clone()),
        base_color_texture: Some(handle),
        unlit: false,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    });
    let mut e = commands.spawn((
        Name::new(format!("Label {text}")),
        Mesh3d(meshes.add(Rectangle::new(w, h))),
        MeshMaterial3d(material),
        transform,
        NotShadowCaster,
        NotShadowReceiver,
    ));
    match space {
        Some(s) => {
            e.insert((PortalSpace(s), layers_for_space(Some(s))));
        }
        None => {}
    }
}

pub fn spawn_world_text(mut commands: Commands, mut images: ResMut<Assets<Image>>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let mut index = 0;
    // The quad faces +Z; Unreal's text faces its actor's +X, which is Bevy's local +X.
    let align = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
    for t in TEXTS {
        let tf = Transform { translation: ue::pos(t.loc) + Vec3::Y * t.size * ue::CM, rotation: ue::rot(t.rot) * align, scale: Vec3::ONE };
        spawn_label(&mut commands, &mut images, &mut meshes, &mut materials, index, t.text, t.size * ue::CM, tf, (t.space >= 0).then_some(t.space));
        index += 1;
    }
    // The telescope dial's three plates are labelled GEMINI, LYNX and CYGNUS.
    let names = ["GEMINI - the Twins", "LYNX - the Small", "CYGNUS - the Great"];
    for (k, name) in names.iter().enumerate() {
        let x = [450.0, 750.0, 1050.0][k];
        let loc = [290000.0 + x, 700.0, 150.0];
        let tf = Transform { translation: ue::pos(loc) + Vec3::Y * 0.16, rotation: ue::rot([0.0, 90.0, 0.0]) * align, scale: Vec3::ONE };
        spawn_label(&mut commands, &mut images, &mut meshes, &mut materials, index, name, 0.16, tf, Some(9));
        index += 1;
    }
}

/// A baked label's camera has done its job after a few frames: stop rendering it.
pub fn finish_bakes(mut cameras: Query<(&mut TextBake, &mut Camera)>) {
    for (mut bake, mut camera) in &mut cameras {
        if bake.frames < 8 {
            bake.frames += 1;
        } else if camera.is_active {
            camera.is_active = false;
        }
    }
}
