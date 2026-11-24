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
    app.add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window { title: "Hexfold".into(), ..default() }),
                ..default()
            }),
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
