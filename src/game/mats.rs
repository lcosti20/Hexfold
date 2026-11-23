//! Shared materials for the game's own actors.

use bevy::prelude::*;

#[derive(Resource)]
pub struct Mats {
    pub rune_gold: StandardMaterial,
    pub rune_plate: StandardMaterial,
    pub plate_idle: Handle<StandardMaterial>,
    pub plate_lit: Handle<StandardMaterial>,
    pub glyph_idle: Handle<StandardMaterial>,
    pub glyph_lit: Handle<StandardMaterial>,
    pub hex_web: Handle<StandardMaterial>,
    pub font_pad: Handle<StandardMaterial>,
}

pub fn setup_materials(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    commands.insert_resource(Mats {
        rune_gold: StandardMaterial {
            base_color: Color::srgb(0.95, 0.72, 0.22),
            metallic: 0.7,
            perceptual_roughness: 0.35,
            emissive: LinearRgba::rgb(0.6, 0.4, 0.05),
            cull_mode: None,
            ..default()
        },
        rune_plate: StandardMaterial { base_color: Color::srgb(0.18, 0.16, 0.15), perceptual_roughness: 0.8, cull_mode: None, ..default() },
        plate_idle: materials.add(StandardMaterial { base_color: Color::srgb(0.2, 0.19, 0.18), perceptual_roughness: 0.8, ..default() }),
        plate_lit: materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.8, 0.3),
            emissive: LinearRgba::rgb(1.4, 0.9, 0.2),
            ..default()
        }),
        glyph_idle: materials.add(StandardMaterial {
            base_color: Color::srgb(0.35, 0.3, 0.12),
            emissive: LinearRgba::rgb(0.25, 0.18, 0.03),
            cull_mode: None,
            ..default()
        }),
        glyph_lit: materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.85, 0.35),
            emissive: LinearRgba::rgb(6.0, 4.0, 0.8),
            cull_mode: None,
            ..default()
        }),
        hex_web: materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.45, 0.9),
            emissive: LinearRgba::rgb(0.4, 1.0, 3.0),
            cull_mode: None,
            ..default()
        }),
        font_pad: materials.add(StandardMaterial {
            base_color: Color::srgb(0.9, 0.95, 1.0),
            emissive: LinearRgba::rgb(1.5, 2.2, 3.5),
            ..default()
        }),
    });
}
