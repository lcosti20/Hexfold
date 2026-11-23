//! Ambient sound: wind outside, a low hum inside, and footsteps while walking.

use avian3d::prelude::*;
use bevy::audio::Volume;
use bevy::prelude::*;

use crate::portal::PlayerSpace;

use super::player::Player;
use super::{AppState, Ending};

#[derive(Component)]
pub struct WindLoop;
#[derive(Component)]
pub struct InteriorLoop;
#[derive(Component)]
pub struct Footsteps;

pub fn spawn_ambience(mut commands: Commands, assets: Res<AssetServer>) {
    let looped = |vol: f32| PlaybackSettings { mode: bevy::audio::PlaybackMode::Loop, volume: Volume::Linear(vol), ..default() };
    commands.spawn((Name::new("WindLoop"), WindLoop, AudioPlayer::new(assets.load("sounds/wind_loop.wav")), looped(0.0)));
    commands.spawn((Name::new("InteriorLoop"), InteriorLoop, AudioPlayer::new(assets.load("sounds/interior_loop.wav")), looped(0.0)));
    commands.spawn((Name::new("Footsteps"), Footsteps, AudioPlayer::new(assets.load("sounds/footsteps.wav")), looped(0.0)));
}

#[allow(clippy::type_complexity)]
pub fn update_ambience(
    time: Res<Time>,
    state: Res<State<AppState>>,
    space: Res<PlayerSpace>,
    ending: Res<Ending>,
    player: Query<(&LinearVelocity, &Player)>,
    mut wind: Query<&mut AudioSink, (With<WindLoop>, Without<InteriorLoop>, Without<Footsteps>)>,
    mut interior: Query<&mut AudioSink, (With<InteriorLoop>, Without<WindLoop>, Without<Footsteps>)>,
    mut steps: Query<&mut AudioSink, (With<Footsteps>, Without<WindLoop>, Without<InteriorLoop>)>,
    mut inside: Local<f32>,
) {
    let playing = *state.get() == AppState::Playing;
    // Crossfade between outside and inside over about a second.
    let want = if space.0.is_some_and(|s| s >= 1) { 1.0 } else { 0.0 };
    *inside += (want - *inside) * (time.delta_secs() * 2.0).min(1.0);
    let wind_level = if ending.broken { 0.25 } else { 1.0 };
    if let Ok(mut s) = wind.single_mut() {
        s.set_volume(Volume::Linear(if playing { 0.5 * wind_level * (1.0 - *inside * 0.9) } else { 0.0 }));
    }
    if let Ok(mut s) = interior.single_mut() {
        s.set_volume(Volume::Linear(if playing { 0.45 * *inside } else { 0.0 }));
    }
    if let Ok(mut s) = steps.single_mut() {
        let walking = player.single().is_ok_and(|(v, p)| p.grounded && Vec2::new(v.x, v.z).length() > 0.5 * p.applied_scale);
        s.set_volume(Volume::Linear(if playing && walking { 0.6 } else { 0.0 }));
    }
}
