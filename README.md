# Hexfold

3D puzzle project for my Game Tech course at SCAD.

Hexfold is a first-person puzzle game built around non-Euclidean portals: doorways that lead somewhere else, rooms that are bigger on the inside, and portals that change your size. It's written in Rust with the [Bevy](https://bevy.org) engine (0.20), with [avian3d](https://github.com/avianphysics/avian) (0.8) for physics.

It started as an Unreal Engine 5 Blueprint project built on a C++ plugin called PortalWorlds. This branch rewrites both the game and the plugin in Rust and combines them into one codebase: `src/portal/` is the plugin, `src/game/` is the game.

## The game

You arrive outside the **Observatory**, a small domed building, and walk in. The inside is far bigger than the outside.

As soon as you're through the door, a **hex** seals it behind you. The hex shows six runes, each a different shape. To break it, you have to find all six runes inside the Observatory and bring them back to the door.

Finding a rune isn't enough. Some are the wrong size or a mirror image of the shape on the hex, so you have to shrink, grow or flip them with portals before they fit. Each rune is guarded by a portal puzzle: rooms nested inside rooms, doorways that change your size, mirrored corridors, and portals that lead somewhere new each time you go through. When the last rune is in, the hex breaks, the sun comes up, and you can walk back out.

The design is in [`docs/Design/Plan.md`](docs/Design/Plan.md). How the code is put together is in [`docs/Architecture.md`](docs/Architecture.md), and the portal plugin has its own guide in [`docs/PortalWorlds/README.md`](docs/PortalWorlds/README.md).

## Running it

You need a recent Rust toolchain (the project uses edition 2024) and a GPU with Vulkan, Metal or DirectX 12.

```
cargo run --release
```

The first build compiles Bevy and takes a few minutes. `cargo run` (debug) works too; the game itself is built with light optimisation and its dependencies with full optimisation, even in debug builds.

To run the executable directly instead of through cargo, set `BEVY_ASSET_ROOT` to the repository root so it finds `assets/`.

**Web build.** `cargo run --target wasm32-unknown-unknown` builds for the browser and serves it with [`wasm-server-runner`](https://github.com/jakobhellermann/wasm-server-runner) (`cargo install wasm-server-runner`; set up in `.cargo/config.toml`). It runs on WebGL2, with a smaller light-cluster grid than the native build.

## Controls

| Action | Keyboard / mouse |
|---|---|
| Move | WASD |
| Look | Mouse |
| Jump | Space |
| Grab / drop | E |
| Throw | Left mouse |
| Pause | Esc |

You can pick up a rune through a portal, and a rune you carry through a portal changes with you. You walk through runes rather than pushing them.

## Project layout

| Path | Contents |
|---|---|
| `src/portal/` | The PortalWorlds plugin: portals, traversal, recursive rendering, spaces, grabbing, objects cut at a portal. See [`docs/PortalWorlds/README.md`](docs/PortalWorlds/README.md) |
| `src/game/` | The game: player, runes, hex seal, puzzles, ending, UI, sound, the scripted test driver |
| `src/level/` | Builds the Observatory from `data.rs`, which is generated from the Unreal level (don't edit it by hand) |
| `src/ue.rs` | Unreal-to-Bevy coordinate conversion |
| `assets/` | `models/` (glTF meshes converted from `art/Export/`), `shaders/` (WESL), `sounds/` |
| `art/Blender/` | Blender source files and the Python scripts that build the meshes; `export_glb.py` converts the FBX exports to glTF |
| `tools/` | `gen_level_data.py` and the JSON dumps of the Unreal level it reads |
| `docs/` | Design plan, architecture, plugin guide |

## Rebuilding generated files

```
python tools/gen_level_data.py                            # tools/unreal_dump/*.json -> src/level/data.rs
blender -b --python art/Blender/scripts/export_glb.py     # art/Export/*.fbx -> assets/models/*.glb
```

## Automated checks

The game can be driven by a script, with no keyboard, for testing and screenshots. `HEXFOLD_SCRIPT` takes steps separated by `;` (waiting, teleporting, walking, turning, pressing keys, moving runes, screenshots); the full list is at the top of [`src/game/debug.rs`](src/game/debug.rs). A scripted run skips the title screen. For example, from the repository root:

```
BEVY_ASSET_ROOT=. HEXFOLD_SCRIPT="wait:300;tp:50000,1560,100,90,0,1;wait:60;shot:hex;quit" ./target/debug/hexfold.exe
```

writes `shots/hex.png`, a view of the hex from inside the Rotunda. Positions are in Unreal centimetres, as in the level data.

Other environment variables:

| Variable | Effect |
|---|---|
| `HEXFOLD_TRACE` | A line per frame on stderr: frame time, player position and velocity, portal crossings, view count, and any player contact that isn't a level floor |
| `HEXFOLD_DEBUG` | Print the portal view tree every frame |
| `HEXFOLD_NOCLIP` | Turn off the portal views' oblique near plane |
| `HEXFOLD_PHYSDEBUG` | Draw every collider |
| `HEXFOLD_SKIP_TITLE` | Start playing straight away |

## Known gaps

- Greybox look: rooms have flat colours (the Unreal version used grid materials), and the HUD's rune slots show roman numerals instead of rune icons.
- A copy of an object partway through a portal has no collision (see the plugin guide's limitations).
- Restart relaunches the executable, so it does nothing useful in the web build.

## Credits

The PortalWorlds plugin is a port of the technique in CodeParade's [NonEuclidean](https://github.com/HackerPoet/NonEuclidean) engine (MIT). See [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md).

## License

MIT. See [LICENSE](LICENSE).
