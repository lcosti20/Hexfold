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
- **Blueprints can be created** with `BlueprintTools.create` (`folder_path`, `asset_name`, `asset_type` = e.g. `/Script/Engine.Actor`); add components with `ActorTools.add_component` and functions with `add_function_graph` + `add_function_param`. Component properties go through `ObjectTools.set_properties` (`instance`, `values` as a JSON string); collision profiles aren't exposed there, so set collision in BeginPlay. Don't rewrite an existing function with `write_graph_dsl` if it has object-typed parameters: the read-back loses the types and the write fails. Add a new function and wire it in with `create_node` + `connect_pins` instead. Setters of Blueprint variables are `Variables|Default|SetX`, calls to another Blueprint's function are `Class|BPName|Function`, and `Set Control Rotation` takes the controller (`Pawn|GetController`).
- **No create-asset tool for other types.** To make an Input Action, duplicate an existing one (`AssetTools.duplicate`, e.g. from `IA_Jump`). Create levels by duplicating an existing level, then `load_level`.
- **`save_assets` returns false on failure.** Check `LogsToolset.GetLogEntries` for the reason, such as read-only files.
- **To test a player portal crossing in PIE, teleport in two short steps.** `ActorTools.set_actor_transform` the player to ~5 cm in front of the portal plane, then (in a separate call, so a frame passes) to ~30 cm past it; the traversal takes the jump as a crossing. Pushing with `CharMoveComp` `Velocity` does *not* trigger a crossing. A long teleport between rooms is fine as long as the straight path doesn't pass through a portal rectangle.
- **A PIE teleport sets the capsule's rotation, not the view.** The camera follows the controller rotation, which `set_actor_transform` doesn't change (and `ControlRotation` can't be set as a property). To face a direction, start PIE with that rotation in `startTransform`.
- **Blueprint pure nodes are re-evaluated each time they're read.** A `bind` of a variable getter isn't a snapshot: after `Set X`, reading the bound value gives the new X. Take decisions before setting.
- **Binding a C++ delegate (e.g. On Portal Traversed):** use `create_node` with `Portal|AssignOnPortalTraversed` (it makes the bind node and a matching custom event), wire it with `connect_pins`, and call your function from the custom event. `CreateEvent` + a Blueprint function won't bind, because Blueprint floats are double and the delegate's is single.
- **Graph DSL:** a latent node like `Delay` is written as a plain statement (`(Utilities|FlowControl|Delay :Duration 0.5)`) followed by the rest; it has no named `Completed` continuation.
- **PIE runs in slow motion when the editor is in the background** (3 fps, physics capped per frame). Turn off `bThrottleCPUWhenNotForeground` on `/Script/UnrealEd.Default__EditorPerformanceSettings` before PIE tests.
- **A pawn spawned already inside a trigger doesn't fire its BeginOverlap.** Spawn the player outside and push it in (set `CharMoveComp` `GroundFriction`/`BrakingDecelerationWalking`/`BrakingFrictionFactor` to 0 and `Velocity`). Pushing works for triggers, just not for portal crossings.
- **Graph DSL (`write_graph_dsl`):** object equality is `(== a b)` (the read-back shows `Utilities|Equal(Object)`, which can't be written); casts are `(bind x (Utilities|Casting|CastToX :Object o) (:then ...) (:CastFailed))`. The read-back omits default argument values and may show a same-named getter from another class (`Class|Character|GetMesh`); check with `get_node_infos` before assuming a bug.
- **Setting a collision profile resets Collision Enabled.** Set `collisionEnabled` in a separate `set_properties` call after `collisionProfileName`, and don't rely on Blueprint template changes reaching already-placed instances; set runtime state in BeginPlay.
- **`StaticMeshTools.import_file` won't overwrite.** Delete the asset first, re-import, then reassign `StaticMesh` on level actors (the delete clears their references).
- **Loading a level fails if the current one has unsaved changes.** Save it first; if the changes were throwaway tests, `git checkout` the `.umap` afterwards.
- **`CaptureEditorImage` captures the front window,** which may be the Message Log. Close it with `SlateInspectorToolset.Windows` first.
- **`CaptureEditorImage` output is too large to return inline.** It is saved to a tool-results file; decode the base64 `data` to a PNG to view it.

## Packaging

- `RunUAT.bat BuildCookRun -project=<Hexfold.uproject> -platform=Win64 -clientconfig=Development -build -cook -stage -pak -archive -archivedirectory=<dir>` works (about 3 minutes, 1.1 GB). Run it from PowerShell with `&` (a path with spaces breaks it in Git Bash).
- **Close the editor first.** UBT refuses to build while Live Coding is active.
- The build compiles the plugin from `Source/` and links it into `Hexfold.exe`, so a package has no plugin DLL and there is nothing extra to commit. It does rebuild `Plugins/PortalWorlds/Binaries/Win64/UnrealEditor-PortalWorlds.dll`; run `git checkout` on it afterwards so the committed binary stays unchanged.

## Portal rules

- **Portal scale must be uniform** (only the sign of X may differ, for mirroring). The quad surface is the root component, so scaling the surface scales the actor. Doorways use the default 2 × 2 m surface; a different-size portal (uniform scale) resizes whatever passes through.
- A portal's surface lies in its local XZ plane, its front faces local −Y, and it's centred on the actor origin. You exit through the target's back side. Link both ways by setting **Front Link** and **Back Link** on both portals.
- Physics objects need **Mobility = Movable**, simulated physics, and a **Portal Traversal** component to pass through portals.

## Project map

- `Content/FirstPerson/Blueprints/`: `BP_FirstPersonCharacter` (has PortalTraversal and PortalGrab components; IA_Grab → Toggle Grab, IA_Throw → Throw), `BP_FirstPersonCameraManager` (parent: `PortalPlayerCameraManager`), `BP_FirstPersonPlayerController`, `BP_FirstPersonGameMode` (the global default game mode).
- `Content/Input/`: `IMC_Default` holds all mappings. Grab = E / gamepad X, Throw = LMB / right trigger.
- `Content/Levels/L_Observatory`: the game level and default map (editor and game). Space 0 = outside (Observatory exterior at the origin, porch facing +Y, entrance portal at (0, 500, 150)); space 1 = Rotunda at (50000, 0, 0), inside entrance portal at (50000, 2000, 150). Both entrance portals and their doorframes have yaw 0 and uniform scale 1.5 (a 3 × 3 m door; equal scales, so no resizing).
- `Content/Levels/L_PortalTest`: test level (doorway pair, bigger-inside shed → underground hall, half-size shrink portal, physics cubes).
- `Content/Hexfold/Blueprints/`: `BP_Rune` (physics rune, `RuneIndex`), `BP_HexSeal` (door seal, six sockets, barrier; see `Docs/Design/Plan.md` → Core loop), `BP_CabinetPuzzle` (rune 2's relinking cabinets), `BP_Telescope` (rune 6's dial: floor plates that rescale/mirror the lens portal at runtime), `BP_KillVolume` (returns runes to their pedestals and respawns the player; `RespawnTransform` per instance) and `BP_RestoreFont` (resets the player's size and mirroring; see `Docs/Design/Plan.md` → Safety nets). Wings (spaces 2–9, at x = 80000 / 110000 / 140000 / 170000 / 200000 / 230000 / 260000 / 290000) are listed in `Docs/Design/Plan.md` → Runes 1–2, 3–4 and 5–6. Alcoves 1, 2, 3, 5, 6, 7 lead to runes 6, 5, 4, 3, 2, 1; the niche (k = 4) is empty. Template changes to `BP_Rune`'s mesh don't reach placed runes; set them on each instance too. `Content/Hexfold/Materials/`: `MI_Rune`, `MI_HexWeb`, `MI_Glyph_Idle`, `MI_Glyph_Lit` (children of `M_PhysicsCube_PortalClip`).
- `Content/Hexfold/Meshes/`: meshes built in Blender. Sources and build scripts are in `Art/Blender/` (see `Docs/Design/Plan.md` → Models). `Docs/Design/Plan.md` is the game design and milestone plan.
- `Config/DefaultEngine.ini`: `r.AllowGlobalClipPlane=True` (required by the plugin), `r.AllowOcclusionQueries=False`.

## Git

- `.gitignore` excludes `Binaries/`, `Intermediate/`, `Saved/`, `DerivedDataCache/` and `*.dll`, with an exception for the PortalWorlds editor `.dll` and `.modules` files. The plugin's `.pdb` debug file stays ignored.
- Assets are not LFS-`lockable`, so they aren't checked out read-only.
