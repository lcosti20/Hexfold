# Hexfold

First-person portal puzzle game in Unreal Engine 5.8, built on the PortalWorlds plugin (`Plugins/PortalWorlds/`; its `README.md` is the reference for portals, traversal, grabbing, spaces and Blueprint nodes).

## Hard rules

- **Blueprints only.** Never add C++, a `Source/` folder, or a game module. Gameplay goes in Blueprints.
- **Don't edit the plugin's source.** Its prebuilt `Plugins/PortalWorlds/Binaries/Win64/UnrealEditor-PortalWorlds.dll` is committed (through LFS) so the editor never needs to compile. Changing `Source/` would make the binary stale and force a C++ build. If the plugin needs changes, they happen in the `NonEuclideanTest` project and a rebuilt copy is brought over.
- **Binary assets are in Git LFS.** `.uasset`, `.umap` and plugin `.dll` files can't be diffed or edited as text. Change them through the editor (Unreal MCP), never with file tools.

## Working in the editor (Unreal MCP)

`.mcp.json` points at the editor's MCP server (`http://127.0.0.1:8000/mcp`). The editor has to be running. The server starts automatically with the editor (`bAutoStartServer=True` in `Config/DefaultEditorPerProjectUserSettings.ini`). If it isn't running, enter `ModelContextProtocol.StartServer` in the editor console. Use `list_toolsets` / `describe_toolset`, then `call_tool` with `toolset_name` and `tool_name` given separately. For multi-step edits, `editor_toolset.toolsets.programmatic.ProgrammaticToolset.execute_tool_script` batches calls in one sandboxed Python script. Call `get_execution_environment` once first.

Gotchas learned so far:
- **Plugins and `.uproject` changes need an editor restart** before their classes (`/Script/PortalWorlds.*`) exist.
- **`BlueprintTools.get_node_type_pins` can destroy an existing node with the same name.** It once deleted a freshly created event node. After wiring, check the result with `get_connected_subgraph` or `find_nodes`, and compile with `warnings_as_errors: true`.
- **No create-asset tool.** To make an Input Action, duplicate an existing one (`AssetTools.duplicate`, e.g. from `IA_Jump`). Create levels by duplicating an existing level, then `load_level`.
- **`save_assets` returns false on failure.** Check `LogsToolset.GetLogEntries` for the reason, such as read-only files.
- **Player portal crossings can't be tested by setting properties in PIE.** Pushing the character by setting `CharMoveComp` `Velocity` (even with friction off) moves it through a portal without traversing, in any level. Object traversal can be tested in Simulate (a physics actor leaning into a portal). Player crossings need a real walk-through.
- **PIE runs in slow motion when the editor is in the background** (3 fps, physics capped per frame). Turn off `bThrottleCPUWhenNotForeground` on `/Script/UnrealEd.Default__EditorPerformanceSettings` before PIE tests.
- **A pawn spawned already inside a trigger doesn't fire its BeginOverlap.** Spawn the player outside and push it in (set `CharMoveComp` `GroundFriction`/`BrakingDecelerationWalking`/`BrakingFrictionFactor` to 0 and `Velocity`). Pushing works for triggers, just not for portal crossings.
- **Graph DSL (`write_graph_dsl`):** object equality is `(== a b)` (the read-back shows `Utilities|Equal(Object)`, which can't be written); casts are `(bind x (Utilities|Casting|CastToX :Object o) (:then ...) (:CastFailed))`. The read-back omits default argument values and may show a same-named getter from another class (`Class|Character|GetMesh`); check with `get_node_infos` before assuming a bug.
- **Setting a collision profile resets Collision Enabled.** Set `collisionEnabled` in a separate `set_properties` call after `collisionProfileName`, and don't rely on Blueprint template changes reaching already-placed instances; set runtime state in BeginPlay.
- **`StaticMeshTools.import_file` won't overwrite.** Delete the asset first, re-import, then reassign `StaticMesh` on level actors (the delete clears their references).
- **Loading a level fails if the current one has unsaved changes.** Save it first; if the changes were throwaway tests, `git checkout` the `.umap` afterwards.
- **`CaptureEditorImage` captures the front window,** which may be the Message Log. Close it with `SlateInspectorToolset.Windows` first.
- **`CaptureEditorImage` output is too large to return inline.** It is saved to a tool-results file; decode the base64 `data` to a PNG to view it.

## Portal rules

- **Portal scale must be uniform** (only the sign of X may differ, for mirroring). The quad surface is the root component, so scaling the surface scales the actor. Doorways use the default 2 × 2 m surface; a different-size portal (uniform scale) resizes whatever passes through.
- A portal's surface lies in its local XZ plane, its front faces local −Y, and it's centred on the actor origin. You exit through the target's back side. Link both ways by setting **Front Link** and **Back Link** on both portals.
- Physics objects need **Mobility = Movable**, simulated physics, and a **Portal Traversal** component to pass through portals.

## Project map

- `Content/FirstPerson/Blueprints/`: `BP_FirstPersonCharacter` (has PortalTraversal and PortalGrab components; IA_Grab → Toggle Grab, IA_Throw → Throw), `BP_FirstPersonCameraManager` (parent: `PortalPlayerCameraManager`), `BP_FirstPersonPlayerController`, `BP_FirstPersonGameMode` (the global default game mode).
- `Content/Input/`: `IMC_Default` holds all mappings. Grab = E / gamepad X, Throw = LMB / right trigger.
- `Content/Levels/L_Observatory`: the game level and default map (editor and game). Space 0 = outside (Observatory exterior at the origin, porch facing +Y, entrance portal at (0, 500, 150)); space 1 = Rotunda at (50000, 0, 0), inside entrance portal at (50000, 2000, 150). Both entrance portals and their doorframes have yaw 0 and uniform scale 1.5 (a 3 × 3 m door; equal scales, so no resizing).
- `Content/Levels/L_PortalTest`: test level (doorway pair, bigger-inside shed → underground hall, half-size shrink portal, physics cubes).
- `Content/Hexfold/Blueprints/`: `BP_Rune` (physics rune, `RuneIndex`), `BP_HexSeal` (door seal, six sockets, barrier; see `Docs/Design/Plan.md` → Core loop). `Content/Hexfold/Materials/`: `MI_Rune`, `MI_HexWeb`, `MI_Glyph_Idle`, `MI_Glyph_Lit` (children of `M_PhysicsCube_PortalClip`).
- `Content/Hexfold/Meshes/`: meshes built in Blender. Sources and build scripts are in `Art/Blender/` (see `Docs/Design/Plan.md` → Models). `Docs/Design/Plan.md` is the game design and milestone plan.
- `Config/DefaultEngine.ini`: `r.AllowGlobalClipPlane=True` (required by the plugin), `r.AllowOcclusionQueries=False`.

## Git

- `.gitignore` excludes `Binaries/`, `Intermediate/`, `Saved/`, `DerivedDataCache/` and `*.dll`, with an exception for the PortalWorlds editor `.dll` and `.modules` files. The plugin's `.pdb` debug file stays ignored.
- Assets are not LFS-`lockable`, so they aren't checked out read-only.
