# PortalWorlds plugin

PortalWorlds is the Unreal Engine 5.8 runtime plugin that provides Hexfold's portals. It is a port of the technique in CodeParade's [NonEuclidean](https://github.com/HackerPoet/NonEuclidean) engine (MIT licence; see `Plugins/PortalWorlds/THIRD_PARTY_NOTICES.md`).

It provides:

- **Rendering.** Portals show the scene through other portals, recursively, using scene captures. Portal edges are invisible and there is no frame lag.
- **Traversal.** The player and physics objects pass through portals seamlessly. A portal of a different size scales whatever passes through it. A mirroring portal flips it left-right.
- **Grabbing.** Physics objects can be picked up, carried and thrown, including through portals.
- **Spaces.** Alternative versions of a room can share the same apparent location without seeing each other.

## Documents

| Document | Contents |
| --- | --- |
| [Guide](Guide.md) | How the plugin is set up in Hexfold and how to build puzzles with it: placing and linking portals, scaling, mirroring, physics objects, carrying, spaces and troubleshooting |
| [API reference](API.md) | Every class, property, function, event, console variable, asset and log message, with Blueprint names and defaults |

The plugin's own [`README.md`](../../Plugins/PortalWorlds/README.md) is the upstream reference. These documents are written for this project: Blueprint-only, with C++-only members marked as such.

## At a glance

| Class | Kind | Role |
| --- | --- | --- |
| `APortal` (**Portal**) | Actor | A portal surface. Each side links to another portal. |
| `UPortalTraversalComponent` (**Portal Traversal**) | Actor component | Moves its owner through portals. Goes on the player pawn and on every physics object that should pass through. |
| `UPortalGrabComponent` (**Portal Grab**) | Actor component | Picks up, carries and throws physics objects. Goes on the player pawn. |
| `APortalPlayerCameraManager` (**Portal Player Camera Manager**) | Player camera manager | Renders the portal views once the camera is final each frame and reverses mouse yaw while mirrored. |
| `UPortalSubsystem` (**Portal Subsystem**) | World subsystem | Owns the portal list, view tree, render targets and spaces. |

## Plugin layout

```
Plugins/PortalWorlds/
  PortalWorlds.uplugin           Runtime module "PortalWorlds", EnabledByDefault: false
  Binaries/Win64/                Prebuilt editor DLL (committed through Git LFS)
  Content/Materials/             M_Portal, MF_PortalClip, M_PortalMirrorView, RT_PortalDefault
  Content/Meshes/                SM_PortalQuad
  Source/PortalWorlds/Public/    Portal.h, PortalTraversalComponent.h, PortalGrabComponent.h,
                                 PortalPlayerCameraManager.h, PortalSubsystem.h, PortalWorlds.h
```

> **Don't edit `Source/`.** The editor loads the committed prebuilt DLL. Changing the source would leave the DLL stale and force a C++ build. Plugin changes are made in the `NonEuclideanTest` project, and a rebuilt copy is brought over (see `CLAUDE.md`).

## Requirements

- Unreal Engine 5.8 on Windows (D3D12). That is the only tested platform.
- `r.AllowGlobalClipPlane=True` in `Config/DefaultEngine.ini` (required).
- `r.AllowOcclusionQueries=False` (recommended).
- Packaging needs Visual Studio with the "Game development with C++" workload, because Unreal builds a game executable that contains the plugin's code. The project itself stays Blueprint-only.
