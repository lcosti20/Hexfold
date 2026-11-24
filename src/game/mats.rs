//! Shared materials for the game's own actors.

use bevy::prelude::*;

/// The materials, made once at startup.
#[derive(Resource)]
pub struct Mats {
    /// A rune's strokes. Kept as a plain material: each rune wraps its own copy in a
    /// `ClipMaterial`, because each is cut at a portal on its own.
    pub rune_gold: StandardMaterial,
    /// A rune's hexagonal backing plate (also wrapped per rune).
    pub rune_plate: StandardMaterial,
    /// An empty hex socket.
    pub plate_idle: Handle<StandardMaterial>,
    /// A filled socket, or one flashing a rejection.
    pub plate_lit: Handle<StandardMaterial>,
    /// A socket's glyph while its rune is missing.
    pub glyph_idle: Handle<StandardMaterial>,
    /// A socket's glyph once its rune is in.
    pub glyph_lit: Handle<StandardMaterial>,
    /// The glowing web of the hex.
    pub hex_web: Handle<StandardMaterial>,
    /// The restore font's glowing pad.
    pub font_pad: Handle<StandardMaterial>,
}

/// Creates the [`Mats`] resource.
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
