//! Hexfold: a first-person portal puzzle game. See `docs/` for the design and architecture.

mod game;
mod level;
mod portal;
mod ue;

use avian3d::prelude::*;
use bevy::prelude::*;

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
        app.add_plugins(PhysicsDebugPlugin::default());
    }
    app.run();
}
