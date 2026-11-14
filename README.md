# Hexfold

3D puzzle project for my Game Tech course at SCAD.

Hexfold is a first-person puzzle game built around non-Euclidean portals: doorways that lead somewhere else, rooms that are bigger on the inside, and portals that change your size. It's made in Unreal Engine 5.8 with Blueprints only (no C++ in the project).

## Requirements

- **Unreal Engine 5.8**
- **Git LFS.** All `.uasset`/`.umap` files and the plugin's prebuilt `.dll` are stored in LFS. Run `git lfs install` once before cloning.
- **Windows.** The PortalWorlds plugin ships a prebuilt Win64 editor binary only.
- **To package a build:** Visual Studio with the "Game development with C++" workload. Unreal compiles a game executable that includes the plugin's code. The project itself stays Blueprint-only.

## Getting started

```
git lfs install
git clone https://github.com/lcosti20/Hexfold.git
```

Open `Hexfold.uproject`. The editor loads the PortalWorlds plugin from its prebuilt binary without compiling anything. If it asks to rebuild modules, the LFS files probably weren't pulled; run `git lfs pull`.

To try the portals, open `Content/Levels/L_PortalTest` and press Play. The editor still starts in the template's `Lvl_FirstPerson`.

## Controls

| Action | Keyboard / mouse | Gamepad |
|---|---|---|
| Move | WASD | Left stick |
| Look | Mouse | Right stick |
| Jump | Space | A (bottom face button) |
| Grab / drop | E | X (left face button) |
| Throw | Left mouse | Right trigger |

## Portal test level

`L_PortalTest` has four areas:

1. **Doorway pair:** two doorways 10 m apart. Walk through one and come out of the other.
2. **Bigger inside:** a 3 × 3 m shed whose door opens into a 20 × 20 m hall hidden underground.
3. **Shrink portal:** a full-size doorway linked to a half-size one. Walking through halves your size.
4. **Physics cubes:** grab them, carry them through portals, or throw them through.

## Project layout

| Path | Contents |
|---|---|
| `Content/FirstPerson/` | Player Blueprints from the UE first-person template, adapted for portals |
| `Content/Input/` | Enhanced Input actions and mapping contexts |
| `Content/Levels/` | Hexfold levels |
| `Content/LevelPrototyping/` | Template prototyping meshes and materials |
| `Plugins/PortalWorlds/` | Portal rendering, traversal and grabbing (see its `README.md`) |

## How the portals are wired up

- `BP_FirstPersonCameraManager` inherits from `PortalPlayerCameraManager`, which renders the portal views each frame.
- `BP_FirstPersonCharacter` has a **Portal Traversal** component, which teleports the player through portals. It also has a **Portal Grab** component, which carries objects, even through portals.
- `Config/DefaultEngine.ini` enables `r.AllowGlobalClipPlane` (required by the plugin) and disables occlusion queries (stops geometry popping in portal views).

The plugin's own [README](Plugins/PortalWorlds/README.md) covers placing and linking portals, scaling and mirroring portals, spaces, and every Blueprint node.

## Credits

The PortalWorlds plugin is a port of the technique in CodeParade's [NonEuclidean](https://github.com/HackerPoet/NonEuclidean) engine. See `Plugins/PortalWorlds/THIRD_PARTY_NOTICES.md`.

## License

MIT. See [LICENSE](LICENSE).
