//! Hexfold: a first-person portal puzzle game. See `docs/` for the design and architecture.
//!
//! * [`portal`]: the PortalWorlds plugin (portals, traversal, recursive rendering, grabbing). It
//!   knows nothing about the game.
//! * [`level`]: spawns the Observatory from the placement data exported from the Unreal level.
//! * [`game`]: the player, runes, the hex seal, the puzzles, the ending, UI and sound.
//! * [`ue`]: Unreal-to-Bevy coordinate conversion.

mod game;
mod level;
mod portal;
mod ue;

use avian3d::prelude::*;
use bevy::prelude::*;

/// Builds the app: Bevy's defaults, avian physics in `PostUpdate`, the portal plugin and the game.
/// `HEXFOLD_PHYSDEBUG` also draws every collider.
fn main() {
    let mut app = App::new();
    let defaults = DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Hexfold".into(),
            // Web: draw into the page's `#hexfold` canvas and keep it the size of its parent, so
            // the game fills whatever frame it's embedded in (itch.io's iframe, fullscreen, ...).
            canvas: Some("#hexfold".into()),
            fit_canvas_to_parent: true,
            ..default()
        }),
        ..default()
    }).set(AssetPlugin { file_path: asset_dir(), ..default() });
    // The web build plays its sound through Web Audio itself (see `game/ambience.rs`).
    #[cfg(target_arch = "wasm32")]
    let defaults = defaults.disable::<bevy::audio::AudioPlugin>();
    app.add_plugins((
            defaults,
            // Physics runs every frame (variable step) so the portal systems, which test the
            // camera's path against portal rectangles, see exactly the motion that was rendered.
            PhysicsPlugins::new(PostUpdate),
            portal::PortalWorldsPlugin,
            game::HexfoldPlugin,
        ));
    if std::env::var("HEXFOLD_PHYSDEBUG").is_ok() {
        app.add_plugins(PhysicsDebugPlugin);
    }
    app.run();
}

/// Where the assets are. Under cargo or with `BEVY_ASSET_ROOT`, Bevy's usual `assets` (relative
/// to that root). Otherwise the first `assets/` folder found beside the executable or in one of
/// its parent folders, so `target/release/hexfold.exe` run directly finds the repo's assets and a
/// shipped build only needs `assets/` copied next to the exe.
fn asset_dir() -> String {
    let default = "assets".to_string();
    if std::env::var_os("BEVY_ASSET_ROOT").is_some() || std::env::var_os("CARGO_MANIFEST_DIR").is_some() {
        return default;
    }
    let Ok(exe) = std::env::current_exe() else { return default };
    exe.ancestors()
        .skip(1)
        .map(|dir| dir.join("assets"))
        .find(|dir| dir.is_dir())
        .map_or(default, |dir| dir.to_string_lossy().into_owned())
}
