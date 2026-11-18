# Hexfold: design and build plan

## Premise

1. The player starts outside the **Observatory**, a small domed building, and walks in. The inside is much bigger than the outside.
2. Once the player is inside, a **hex** seals the door. It shows six runes, each a different shape. To break it, the player has to bring all six runes back to the door.
3. The runes are scattered through the Observatory. Some have to be **resized** or **flipped** (mirrored) before they fit.
4. Each rune is guarded by a portal puzzle built with the PortalWorlds plugin.

## How the plugin's rules shape the design

These facts come from [`Docs/PortalWorlds/Guide.md`](../PortalWorlds/Guide.md), and the puzzles are built on them.

| Plugin behaviour | What it means for puzzles |
|---|---|
| A portal linked to a different-size portal scales whatever passes through by the size ratio. | Shrink and grow rooms. A rune's world size can be changed. |
| One portal of a pair with negative X scale mirrors whatever passes through. Objects really become mirrored (negative scale), and **Is Mirrored** on their Portal Traversal component reports it. | Runes can be flipped. Every pass through a mirror flips the rune again, so what matters is whether the rune has been through an **odd or even** number of mirrors. |
| **A carried object changes together with the player.** Carry a rune through a shrink portal and both shrink, so it still looks the same size to the player. To change a rune *relative to the player*, throw it through, or carry it through and walk back another way. | The core trick of the size puzzles: the rune's *world* size is what the hex checks, and the player has to work out which route changes it. |
| Objects with a bounding radius over **Max Object Radius** (90 cm at the player's size) can't be lifted. | An oversized rune has to be pushed, not carried. |
| Grab traces look through up to two portals. | A rune can be grabbed from the far side of a portal. |
| **On Portal Traversed** fires after each trip, and **Front Link** / **Back Link** can be changed at runtime. | Doors that lead somewhere new each time, and a telescope dial that changes where a portal goes. |
| Rooms that share an apparent location are built far apart and tagged `PortalSpace.<n>`. | The bigger-on-the-inside Observatory and every room inside it. |

## The Observatory

### Outside (space 0)

A small stone observatory, about 8 m across, with one door, on a hilltop at dusk. The player spawns a short walk away, so they can see the building is small before entering.

The front door is a 2 × 2 m portal linked to the inside door.

### Inside: the Rotunda (space 1)

A rotunda about 40 m across, built a few hundred metres away from (or below) the exterior. A large telescope stands at the centre, and six wings lead off it, one per rune.

- The inside door is the other end of the entrance pair. Rotate it so the player comes out facing into the room (you exit through the target's back).
- Wings connect to the Rotunda through ordinary corridors where possible. Portals are used only where a puzzle needs them, which keeps the number of scene captures down (`Portal.MaxCaptures` is 32, and every visible portal is a full scene render).
- Each wing that's really somewhere else gets its own space (2–7).

### The hex

When the player first comes through the entrance, the hex appears over the inside of the door: a hexagonal seal with six **sockets**, one per rune.

- It's a blocking barrier in front of the door. The doorway stays visible through it, so the player can see the outside they've been cut off from.
- Each socket shows its rune's glyph at the **correct size and handedness**. This is the player's reference for what each rune has to look like.
- **Every rune shape is chiral** (no mirror symmetry), even ones that never need flipping. Otherwise a mirrored rune would look right, and the player couldn't tell which runes might need flipping.

**Socket check.** When a rune is released into a socket's trigger:

| Check | Passes when |
|---|---|
| Right rune | The rune's index matches the socket |
| Right size | The rune's world scale is 1 (within ±10%) |
| Right way round | **Is Mirrored** on the rune's Portal Traversal component is false |

- **Accepted:** physics off, the rune slides into the socket and attaches, and the glyph lights up.
- **Rejected:** the rune is pushed gently out, and the glyph shows why: its size outline pulses for a wrong size, or it flips back and forth for a mirrored rune.

When all six are in, the hex breaks and the barrier is removed. Walking out the door ends the game. Outside, it's now night and the sky is full of stars.

## The six runes

The order below teaches one idea at a time and then combines them. The rune names are working titles.

### 1. Through the Glass (looking and grabbing through portals)

The rune sits on a ledge across a chasm that can't be crossed. A portal window on the player's side looks onto the ledge from close up.

- **Solution:** grab the rune through the window (grab traces pass through portals) and pull it through.
- **Teaches:** portals are windows you can reach through.

### 2. The Orrery Cabinets (rooms inside rooms, doors that relink)

A cabinet in the Rotunda opens into a study. The study has its own cabinets, which open into further rooms. The doors relink after each trip (**On Portal Traversed** sets **Front Link** / **Back Link**), so the way through is a *sequence* of doors, hinted at by the orrery's planets on the walls.

- **Solution:** follow the sequence to the innermost room, take the rune, and follow the way back out.
- **Teaches:** spaces nest, and a door doesn't always lead to the same place.

### 3. The Heavy Rune (shrinking an object)

The rune is at 2× size, too big to lift. It sits in a sloped hall that ends in a 4 m portal linked to a 2 m one.

- **Solution:** push it (the player pushes physics objects by walking into them) down the slope through the large portal. It comes out of the small one at 1×.
- **Teaches:** portals resize objects, not just the player.
- **Tuning:** the rune's 2× bounding radius must be over **Max Object Radius** (90 cm), and its 1× radius well under it.

### 4. The Mouse Door (growing an object, and the carry rule)

The rune is at 0.5× size. A half-size doorway leads into a full-size room on the far side, so going through it grows things by 2×.

- **Trap:** the player shrinks themselves to reach the rune, then carries it out through the half-size doorway. Both grow, and the rune is still 0.5×.
- **Solution:** throw the rune through the half-size doorway from outside (it grows to 1×), or carry it through and come back another way.
- **Teaches:** a carried object changes with you. Only its size relative to the world counts.

### 5. Hall of Reflections (mirroring and parity)

The rune is a mirror image of its glyph. The wing is a symmetric gallery with a mirroring portal pair, and the only way back to the Rotunda passes through one mirror.

- **Trap:** the player goes in through a mirror, picks up the rune (which now looks right to them, because they're mirrored too), and walks back out. Both flip again, so the rune is still mirrored.
- **Solution:** send the rune through an **odd** number of mirrors in total, for example by throwing it through the gallery's mirror window once before leaving.
- **Teaches:** each pass through a mirror flips the rune, carried or not. Count the mirrors.
- **Build note:** geometry, textures and lighting around mirroring portals must be left-right symmetric, or there's a visible seam.

### 6. The Telescope (combining everything)

The central telescope's eyepiece is a portal. A dial on its base rotates the dome and changes what the eyepiece links to: one of several "constellation" rooms, each with a different transform (mirrored, half size, double size).

The last rune starts both mirrored and at the wrong size.

- **Solution:** choose the dial settings and the order of trips so the rune ends up the right size and the right way round.
- **Teaches:** nothing new. It's the final exam.

## Blueprints and assets

| Asset | Type | Purpose |
|---|---|---|
| `L_Observatory` | Level | Outside, Rotunda and all six wings, in spaces 0–7. Becomes the default map. |
| `BP_Rune` | Actor | Root static mesh (Movable, Simulate Physics, centred pivot, simple box or convex collision on one body) with a Portal Traversal component. `RuneIndex` (0–5), `SpawnTransform`. Handles **Return To Pedestal**. |
| `SM_Rune_0` … `SM_Rune_5` | Static meshes | Six chiral rune shapes, modelled in Blender (see [Models](#models-blender)). |
| `SM_Obs_*` | Static meshes | Observatory exterior, Rotunda, telescope, hex seal, pedestals, doorframes, modelled in Blender |
| `M_Rune` / `MI_Rune_*` | Material | Masked, with `MF_PortalClip` so runes are cut at portals, and an emissive glow. |
| `BP_HexSeal` | Actor | Barrier, six socket triggers and glyph displays. Binds to the player's **On Portal Traversed** to seal itself when the player enters, runs the socket check, and breaks when all six are in. |
| `BP_RelinkingDoor` | Actor or component | Holds a list of links and switches to the next one on **On Portal Traversed**. Used in rune 2. |
| `BP_TelescopeDial` | Actor | Interactable dial that sets the eyepiece portal's links. Used in rune 6. |
| `BP_RestoreFont` | Actor | A spot in the Rotunda that resets the player to normal size and unmirrored. |
| `BP_KillVolume` | Actor | Returns runes that fall out of reach to their pedestals, and puts the player back at the last checkpoint. |

**Resets** follow the guide's rules: after moving anything by script, call **Reset Tracking** on its Portal Traversal component. For the player, also call **Set Mirrored (false)** and **Apply Scale Factor** with the inverse of their current scale. For a rune, reset its actor scale, call **Set Mirrored (false)** and then **Refresh Mirrored Rendering**.

**Editor tooling limits** (from `CLAUDE.md`): the Unreal MCP has no create-asset tool. New levels, Blueprints and materials are made by duplicating existing ones, such as `L_PortalTest` for the level, and `M_PhysicsCube_PortalClip` for the rune material (it's already Masked with `MF_PortalClip`, so duplicating it is safe). A Blueprint class may be quicker to create by hand in the Content Browser, with the graph then built through MCP.

## Models (Blender)

Meshes are modelled in Blender through the Blender MCP, exported as FBX and imported into Unreal.

**Files.** Source `.blend` files go in `Art/Blender/`, exports in `Art/Export/<group>/` (both LFS-tracked by `.gitattributes`; `.blend1` backups are git-ignored). Imported meshes go in `Content/Hexfold/Meshes/<group>/`. One `.blend` per group: `Runes.blend`, `Observatory_Exterior.blend`, later `Rotunda.blend`, `Props.blend`.

**Scripts.** Meshes are generated by Python scripts in `Art/Blender/scripts/`, so they can be rebuilt and reviewed as text:

| Script | Builds |
|---|---|
| `build_runes.py` | `SM_Rune_0`–`5` with `UCX_` hulls, and `SM_Glyph_0`–`5` |
| `build_observatory.py` | `SM_Obs_Doorframe` (with `UCX_` hulls) and `SM_Obs_Exterior` |
| `build_rotunda.py` | `SM_Obs_Rotunda` (in `Rotunda.blend`) |
| `export_fbx.py` | Exports every top-level `SM_*` object in the open file, at the origin, with its `UCX_` children |

Run a build script in the `.blend` file it belongs to, save, then run `export_fbx.py`. Opening a file and exporting must be separate script runs (the FBX exporter needs a fresh context).

**Export conventions.**

- Model in metres at real size: a 2 × 2 m doorway is 2 × 2 Blender units. Export FBX with unit scale 1 and **Apply Scalings = FBX Units Scale (`FBX_SCALE_NONE`)**, Blender's default. (`FBX_SCALE_ALL` makes meshes arrive 100× too small.)
- **Axes:** Blender X → Unreal X, Blender Z → Unreal Z, Blender −Y → Unreal **+Y**. A mesh's Blender front (−Y) faces +Y in Unreal. The portal's front is local −Y, so rotate a placed frame or building 180° in yaw if its front must match a portal's front.
- One object per exported mesh, named `SM_<Name>`, origin where the pivot should be.
- No UVs yet: the scripts build untextured meshes for flat-colour materials. Add UVs before textured materials go on (milestone 7).
- Import with the Unreal MCP's `StaticMeshTools.import_file` into `/Game/Hexfold/Meshes/<group>`, materials off. Re-importing means deleting the asset first.

**Imported so far** (milestone 1b):

| Mesh | Size (cm) | Collision |
|---|---|---|
| `SM_Rune_0`–`5` | 24–29 × 8 × 40–48 | 3–4 convex boxes from `UCX_` |
| `SM_Glyph_0`–`5` | same outline, 1 cm thick | auto (set **No Collision** when placed in sockets) |
| `SM_Obs_Doorframe` | 252 × 50 × 239, origin at the portal centre | 3 convex boxes (jambs, lintel) |
| `SM_Obs_Exterior` | 940 × 982 × 1 069 (drum 6 m high, porch 4.5 m), origin at door-floor centre; 3 × 3 m door recess | complex as simple |
| `SM_Obs_Rotunda` | 40 m room (5 040 × 5 040 × 3 030 with alcoves), origin at floor centre; entrance on −Y in Blender (+Y in Unreal, socket (0, 2000, 150)); 3 × 3 m door recess; six 4 × 6 m wing arches and a niche at 45° steps | complex as simple |

Greybox materials: the `MI_PrototypeGrid_*` instances from `LevelPrototyping` are assigned to the Observatory meshes' slots.

The exterior's door socket (where the doorframe and the entrance portal go) is at **(0, +500, 150)** cm relative to the building, with the porch facing +Y. The doorframe and the entrance portals there are placed at **scale 1.5** (a 3 × 3 m door). Both entrance portals use the same scale, so passing through doesn't resize anything.

**Runes** have extra requirements because they're physics objects that pass through portals:

- **Origin at the centre of the bounds** (the plugin tracks the root's origin, so it has to cross the portal).
- **Chiral:** no mirror plane in any direction. Check by mirroring a copy and confirming it can't be rotated to match.
- **Simple collision on one body:** one or a few convex hulls exported as `UCX_SM_Rune_N_00` … alongside the mesh. Mirrored physics needs boxes, spheres or convex hulls, not complex collision.
- **Size and the grab limit.** Runes are 40–48 cm tall and 8 cm thick, with bounding radii of 25–28 cm. At 2× that's 49–57 cm. The default **Max Object Radius** (90 cm) would let a 2× rune be lifted, so it's lowered to **45 cm** on `BP_FirstPersonCharacter`'s Portal Grab (milestone 2). That's under every 2× rune and over every 1× rune, and the 50 cm test cubes (radius 43 cm) can still be lifted. Rune 3's mesh is the same size; the level places it at scale 2.
- **Chiral in 3D:** each outline has no mirror symmetry, and only the front (−Y in Blender) is chamfered, while the back is flat. A flat slab with identical faces would have a mirror plane through its middle, so a mirrored rune could just be turned over to match.
- A matching flat glyph (`SM_Glyph_N`, 1 cm thick) for the hex sockets.
- **Tested:** `SM_Rune_0` as a physics actor with Portal Traversal and the portal-clip material toppled through `L_PortalTest`'s doorway pair (log: `traversed Portal_DoorA -> Portal_DoorB`) and came to rest flat on the floor beyond, 4.0 cm up (exactly half its thickness).

**Portal doorframes** are modelled around a 2 × 2 m opening centred on the origin, matching the default portal surface, so the frame and portal share a transform. Frames used with mirroring portals must be left-right symmetric.

**Workflow per asset:** script in Blender → check with `look` → save → `export_fbx.py` → `import_file` in Unreal → check bounds and collision → set materials → place.

## Greybox hub layout

`L_Observatory` (duplicated from `L_PortalTest`, keeping its sky, sun and sky light). Coordinates in cm.

| Actor | Location | Space | Notes |
|---|---|---|---|
| `Ground` | (0, 0, −70), scale 400 × 400 × 1 | 0 | 400 m square, top at −20 so the plinth shows |
| `PlayerStart` | (0, 2500, 100), yaw −90 | – | 25 m in front of the porch, facing it |
| `Obs_Exterior` | origin | 0 | Porch faces +Y |
| `Obs_DoorframeOut`, `Portal_Entrance_Out` | (0, 500, 150), yaw 0, scale 1.5 | 0 | The player enters through the portal's back side |
| `Rot_Shell` | (50000, 0, 0) | 1 | 500 m from the building. **Cast Shadow As Two Sided** is on, so the sun doesn't leak through the single-sided dome |
| `Rot_DoorframeIn`, `Portal_Entrance_In` | (50000, 2000, 150), yaw 0, scale 1.5 | 1 | Same rotation and scale as the outside portal, so the player keeps facing −Y and walks into the room |
| `Rot_LightDome`, `Rot_LightRing0`–`7` | inside the Rotunda | – | Movable point lights: 20 000 lm under the dome, 8 × 5 000 lm at 13 m radius |

Both entrance portals link to each other on both sides. The sun and sky are untagged, so they show in both spaces.

Known greybox issues: the Rotunda is overexposed (auto-exposure; tune in milestone 7), the prototype materials have no UVs to rely on (they're world-aligned), and the FBX importer warns about degenerate tangents for the same reason.

## Milestones

Each milestone ends with a playable build in PIE.

| # | Milestone | Done when |
|---|---|---|
| 0 | Branch, README, plan | This document is merged |
| 1 | **Greybox hub.** `L_Observatory` with the outside, the entrance portal pair and an empty Rotunda in their own spaces. Set as the default map. | Built (see [Greybox hub layout](#greybox-hub-layout)). The Rotunda shows through the door from outside, and the outside shows from inside. **Still to confirm by hand:** walking through the door both ways. |
| 1b | **First models.** The six rune meshes with collision and glyphs, the doorframe and the Observatory exterior shell, from Blender | ✅ They import at the right size, pivots are centred, and a rune passes through a portal cleanly |
| 2 | **Core loop.** `BP_Rune`, `M_Rune`, `BP_HexSeal`, Max Object Radius 45 cm, with one rune on a pedestal in the Rotunda and the other five sockets temporarily disabled | Entering seals the door. Placing the rune is accepted, a scaled or mirrored test rune is rejected, and breaking the hex opens the door |
| 3 | **Runes 1–2** (through the glass, cabinets) | Both solvable with no debug help |
| 4 | **Runes 3–4** (size) | Both solvable. The carry trap in rune 4 rejects at the socket for the right reason. |
| 5 | **Runes 5–6** (mirror, telescope) | Both solvable, with no visible seams at mirroring portals |
| 6 | **Safety nets.** Kill volumes, rune return, restore font, socket feedback | Nothing can soft-lock the game: every rune can always be recovered |
| 7 | **Art, sound and lighting.** Final Blender models for the Rotunda, wings and telescope, Observatory look, glyph VFX, hex break, ending sky. Check portal edges under Lumen, auto-exposure and TSR (see the guide's Limitations). | A full playthrough looks finished |
| 8 | **Performance and packaging.** Check `Portal.ShowStats 1` in the Rotunda, tune `Portal.MaxDepth` and `Portal.ResolutionFalloff`, and package a Win64 build | Stable frame rate and a working packaged build |

## Risks

- **Too many portals in view.** The Rotunda could see several portals at once, each a full scene capture. Mitigation: ordinary corridors for wings, doors that close, and checks with `Portal.ShowStats 1`.
- **Mirror seams.** Mirroring portals need left-right symmetric surroundings. Mitigation: build the Hall of Reflections as a straight, symmetric corridor and test early.
- **Rune collision.** Chiral shapes can tempt complex collision, but mirrored physics needs simple collision on one body. Mitigation: give each rune a single convex hull or box.
- **Lighting.** The plugin was verified in unlit, fixed-exposure scenes. Mitigation: test portal edges as soon as real lighting goes in (milestone 7, or earlier in one wing).
- **Puzzle clarity.** Size and mirror differences need to be easy to read. Mitigation: socket glyphs show the target size and handedness, and rejection feedback says which check failed.
