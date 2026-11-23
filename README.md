# Hexfold

3D puzzle project for my Game Tech course at SCAD.

Hexfold is a first-person puzzle game built around non-Euclidean portals: doorways that lead somewhere else, rooms that are bigger on the inside, and portals that change your size. It's written in Rust with the [Bevy](https://bevy.org) engine (0.20), with [avian3d](https://github.com/avianphysics/avian) for physics.

It started as an Unreal Engine 5 Blueprint project built on a C++ plugin called PortalWorlds. This branch rewrites both the game and the plugin in Rust and combines them into one codebase: `src/portal/` is the plugin, `src/game/` is the game.

## The game

You arrive outside the **Observatory**, a small domed building, and walk in. The inside is far bigger than the outside.

As soon as you're through the door, a **hex** seals it behind you. The hex shows six runes, each a different shape. To break it, you have to find all six runes inside the Observatory and bring them back to the door.

Finding a rune isn't enough. Some are the wrong size or a mirror image of the shape on the hex, so you have to shrink, grow or flip them with portals before they fit. Each rune is guarded by a portal puzzle: rooms nested inside rooms, doorways that change your size, mirrored corridors, and portals that lead somewhere new each time you go through.

The design and build plan is in [`docs/Design/Plan.md`](docs/Design/Plan.md). How the code is put together is in [`docs/Architecture.md`](docs/Architecture.md).

## Running it

You need a recent Rust toolchain (the project uses edition 2024) and a GPU with Vulkan, Metal or DirectX 12.

```
cargo run --release
```

The first build compiles Bevy and takes a few minutes. `cargo run` (debug) works too; dependencies are optimised even in debug builds.

## Controls

| Action | Keyboard / mouse |
|---|---|
| Move | WASD |
| Look | Mouse |
| Jump | Space |
| Grab / drop | E |
| Throw | Left mouse |
| Pause | Esc |

## Project layout

| Path | Contents |
|---|---|
| `src/portal/` | The PortalWorlds plugin: portals, traversal, recursive rendering, spaces, grabbing, objects cut at a portal. See [`docs/PortalWorlds/README.md`](docs/PortalWorlds/README.md) |
| `src/game/` | The game: player, runes, hex seal, puzzles, ending, UI, sound |
| `src/level/` | Builds the Observatory from `data.rs` (generated from the Unreal level) |
| `src/ue.rs` | Unreal-to-Bevy coordinate conversion |
| `assets/models/` | glTF meshes (converted from `art/Export/`), `assets/shaders/`, `assets/sounds/` |
| `art/Blender/` | Blender source files and the Python scripts that build the meshes; `export_glb.py` converts the FBX exports to glTF |
| `tools/` | `gen_level_data.py` and the JSON dumps of the Unreal level it reads |
| `docs/` | Design plan, architecture, plugin guide |

## Credits

The PortalWorlds plugin is a port of the technique in CodeParade's [NonEuclidean](https://github.com/HackerPoet/NonEuclidean) engine (MIT).

## License

MIT. See [LICENSE](LICENSE).
