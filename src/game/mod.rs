//! Hexfold: the game itself, built on the portal plugin.
//!
//! * [`player`]: the first-person body and controls.
//! * [`rune`], [`hex`]: the six runes and the seal that wants them back.
//! * [`puzzles`]: the orrery cabinets, the telescope dial, kill volumes and the restore font.
//! * [`ending`]: the night sky and the sunrise after the hex breaks.
//! * [`ui`], [`ambience`], [`world_text`]: HUD, menus, sound, and text in the world.
//! * [`mats`]: shared materials.
//! * [`debug`](mod@debug): the scripted driver for automated checks (`HEXFOLD_SCRIPT`, `HEXFOLD_TRACE`).

pub mod ambience;
pub mod debug;
pub mod ending;
pub mod hex;
pub mod mats;
pub mod player;
pub mod puzzles;
pub mod rune;
pub mod ui;
pub mod world_text;

use avian3d::prelude::*;
use bevy::light::GlobalAmbientLight;
use bevy::prelude::*;

use crate::level::{self, LevelPlugin};
use crate::portal::PortalSettings;

/// Camera exposure: the Unreal level's lights are in lumens tuned for its auto-exposure.
pub const EXPOSURE_EV100: f32 = 3.5;
/// Outdoors the lights are far dimmer (moonlight), so the camera opens up, as Unreal's auto-exposure did.
pub const EXPOSURE_OUTDOORS_EV100: f32 = 0.0;
/// Scale from Unreal lumens to Bevy lumens.
pub const LIGHT_SCALE: f32 = 1.0;
/// Scale for the outdoor lights (lux).
pub const MOON_SCALE: f32 = 1.0;

/// Which screen the game is on. Time only runs while `Playing`.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    /// The title screen (skipped by a scripted run or `HEXFOLD_SKIP_TITLE`).
    #[default]
    Title,
    Playing,
    /// Esc pressed.
    Paused,
    /// The player walked out after breaking the hex: the end screen.
    Escaped,
}

/// Progress of the ending.
#[derive(Resource, Default, Debug)]
pub struct Ending {
    /// The hex has broken.
    pub broken: bool,
    /// Sunrise progress, 0 (night) to 1 (dawn), over 12 s after the hex breaks.
    pub sunrise: f32,
    /// The player has walked back outside after the hex broke.
    pub escaped: bool,
    /// Seconds left before the escape screen.
    pub escape_timer: f32,
}

/// The whole game: the level, the actors, their systems, and the portal settings it plays with
/// (recursion depth 6, resolution falloff 0.75, 10 pooled cameras per depth).
pub struct HexfoldPlugin;

impl Plugin for HexfoldPlugin {
    fn build(&self, app: &mut App) {
        let scripted = if let Some(script) = debug::load_script() {
            app.insert_resource(script);
            true
        } else {
            false
        };
        let skip_title = scripted || debug::var("HEXFOLD_SKIP_TITLE").is_some();
        app.add_plugins(LevelPlugin)
            .insert_state(if skip_title { AppState::Playing } else { AppState::Title })
            .add_observer(ui::on_button)
            .init_resource::<Ending>()
            .insert_resource(crate::portal::PlayerSpace(Some(0)))
            .init_resource::<ui::ToastTimer>()
            .add_message::<ui::Toast>()
            .add_message::<hex::HexEvent>()
            .add_message::<rune::ReturnToPedestal>()
            .insert_resource(Gravity(Vec3::NEG_Y * 9.81))
            .insert_resource(PortalSettings { max_depth: 6, resolution_falloff: 0.75, exposure_ev100: EXPOSURE_EV100, prewarm_per_depth: 10, space_ambient: vec![(0, ending::outdoor_ambient(0.0).0, ending::outdoor_ambient(0.0).1)], ..default() })
            .insert_resource(GlobalAmbientLight { color: Color::srgb(0.6, 0.65, 0.8), brightness: 8.0, ..default() })
            .insert_resource(ClearColor(Color::srgb(0.01, 0.015, 0.04)))
            .add_systems(
                Startup,
                (
                    mats::setup_materials,
                    level::spawn_level,
                    player::spawn_player,
                    player::spawn_player_body,
                    rune::spawn_runes,
                    hex::spawn_hex,
                    puzzles::setup_cabinets,
                    puzzles::spawn_telescope,
                    puzzles::spawn_kill_volumes,
                    puzzles::spawn_restore_font,
                    ending::spawn_sky,
                    world_text::spawn_world_text,
                    ambience::spawn_ambience,
                    ui::spawn_hud,
                )
                    .chain(),
            )
            .add_systems(
                PostUpdate,
                player::update_body.after(crate::portal::PortalSystems::Camera).before(TransformSystems::Propagate),
            )
            .add_systems(Last, debug::trace.run_if(|| debug::var("HEXFOLD_TRACE").is_some()))
            .add_systems(OnEnter(AppState::Title), ui::open_title)
            .add_systems(OnExit(AppState::Title), ui::close_menus)
            .add_systems(OnEnter(AppState::Paused), ui::open_pause)
            .add_systems(OnExit(AppState::Paused), ui::close_menus)
            .add_systems(OnEnter(AppState::Escaped), ui::open_escaped)
            .add_systems(
                Update,
                (
                    debug::run_script.before(crate::portal::grab::update_grab).before(player::walk),
                    (player::grab_cursor, player::look, player::walk, player::apply_scale).chain(),
                    (ui::pause_key, ui::freeze_time, ui::toast_system, ui::hud_events),
                    (hex::hex_logic, puzzles::cabinet_logic, puzzles::telescope_logic, puzzles::kill_volume_logic, puzzles::restore_font_logic, rune::return_to_pedestal),
                    (ending::ending_logic, ending::update_sky, ambience::update_ambience, world_text::finish_bakes, adapt_exposure),
                ),
            );
        if debug::var("HEXFOLD_PIPELINES").is_some()
            && let Some(render) = app.get_sub_app_mut(bevy::render::RenderApp)
        {
            render.add_systems(bevy::render::Render, debug::log_new_pipelines.in_set(bevy::render::RenderSystems::Cleanup));
        }
    }
}

/// The eye adapts: wide open outside, stopped down in the lit rooms. Every portal view uses the
/// same exposure as the player's camera (see `PortalSettings::exposure_ev100`).
fn adapt_exposure(
    time: Res<Time<Real>>,
    space: Res<crate::portal::PlayerSpace>,
    ending: Res<Ending>,
    mut settings: ResMut<PortalSettings>,
    mut camera: Query<&mut bevy::camera::Exposure, With<crate::portal::PortalCamera>>,
) {
    let outside = space.0.is_none_or(|s| s == 0);
    // After the sunrise the outside is bright again.
    let outdoors = EXPOSURE_OUTDOORS_EV100 + (EXPOSURE_EV100 - EXPOSURE_OUTDOORS_EV100) * ending.sunrise;
    let target = if outside { outdoors } else { EXPOSURE_EV100 };
    let k = (time.delta_secs() * 1.5).min(1.0);
    let ev = settings.exposure_ev100 + (target - settings.exposure_ev100) * k;
    settings.exposure_ev100 = ev;
    if let Ok(mut e) = camera.single_mut() {
        e.ev100 = ev;
    }
}
