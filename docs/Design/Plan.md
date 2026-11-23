# Hexfold: design and build plan

> **Note:** this plan was written for the Unreal Engine version. The game design (premise, rooms, the six runes, the hex) is unchanged in the Rust/Bevy rewrite. References to Blueprints, the Unreal MCP, `Content/` paths and `CLAUDE.md` tooling notes are historical; see [`Architecture.md`](Architecture.md) for how the Bevy version is built. Distances below are Unreal centimetres; the Bevy version uses metres.

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

The rune is at 2× size, too big to lift. It lies in a hall that ends in a 4 m portal linked to a 2 m door in the side wall.

- **Solution:** push it (the player pushes physics objects by walking into them) across the hall into the large portal. It comes out of the small door at 1×. (Runes have **Can Character Step Up On = No**, so the player pushes them instead of climbing onto them.) Walking into the big portal yourself shrinks you to 0.5×; the small door takes you back.
- **Teaches:** portals resize objects, not just the player.
- **Tuning:** the rune's 2× bounding radius must be over **Max Object Radius** (90 cm), and its 1× radius well under it.

### 4. The Mouse Door (growing an object, and the carry rule)

The rune is at 0.5× size. A 1 × 1 m mouse door (scale 0.5) is linked to a normal 2 × 2 m door (scale 1) in the same room, so anything going in the mouse door comes out the big door at 2×, and the other way round at 0.5×.

- **Solutions:** throw the rune into the mouse door (it comes out of the big door at 1×). Or walk into the big door yourself, come out of the mouse door at half size, pick up the rune (it now looks normal-sized), and carry it back through the mouse door: you both grow 2×.
- **Trap:** carrying the 0.5× rune into the big door at normal size shrinks it to 0.25×.
- **Teaches:** small-to-big grows, big-to-small shrinks, and a carried rune changes with you, so only its size relative to the world counts. (An earlier draft had the carry-out route as the trap; it isn't, because a carried rune does change world size.)

### 5. Hall of Reflections (mirroring and parity)

The wing's entrance is a mirroring portal pair, so the player enters the gallery mirrored. The rune is world-correct, which means it looks reversed to the mirrored player. Any way back to the Rotunda passes through that mirror.

- **Trap:** pick up the rune and carry it out. The mirror flips it, so the hex rejects it as a mirror image.
- **Solution:** throw the rune through the gallery's mirror window (a mirroring portal pair high in the side walls) first, then carry it out: two flips in total. A "COUNT THE MIRRORS" inscription on the wall reads backwards while you're mirrored.
- **Teaches:** each pass through a mirror flips the rune, carried or not. Count the mirrors. (An earlier draft had the rune start mirrored and the carry-out as the trap; that's backwards, because the exit mirror would fix it.)
- **Build note:** geometry, textures and lighting around mirroring portals must be left-right symmetric, or there's a visible seam.

### 6. The Telescope (combining everything)

The telescope has its own room behind the last alcove. Its eyepiece is a doorway in the middle of the room. Walking or throwing into it brings you out of the "lens", a portal in the room's far wall. Three floor plates (the dial) turn the dome to a constellation, which sets the lens: **Gemini** mirrors, **Lynx** halves, **Cygnus** doubles. Before any plate is stepped on, the lens is a plain 1:1 doorway.

The last rune starts both mirrored and at 0.5×.

- **Solution:** send it through once on Gemini and once on Cygnus (in either order; throw it, or carry it and accept that you change too). Walking back from the lens into the eyepiece undoes the current setting, so the player can always restore themselves.
- **Teaches:** nothing new. It's the final exam.

## Blueprints and assets

| Asset | Type | Purpose |
|---|---|---|
| `L_Observatory` | Level | Outside, Rotunda and all six wings, in spaces 0–7. Becomes the default map. |
| `BP_Rune` | Actor | Root static mesh (Movable, Simulate Physics, centred pivot, simple box or convex collision on one body) with a Portal Traversal component. `RuneIndex` (0–5). Built in milestone 2; `SpawnTransform` and **Return To Pedestal** come in milestone 6. |
| `SM_Rune_0` … `SM_Rune_5` | Static meshes | Six chiral rune shapes, modelled in Blender (see [Models](#models-blender)). |
| `SM_Obs_*` | Static meshes | Observatory exterior, Rotunda, telescope, hex seal, pedestals, doorframes, modelled in Blender |
| `MI_Rune`, `MI_Glyph_*`, `MI_HexWeb` | Material instances | Children of `M_PhysicsCube_PortalClip` (Masked, `MF_PortalClip`), so runes are cut at portals. An emissive `M_Rune` comes with the art pass. |
| `BP_HexSeal` | Actor | Web, barrier, six socket triggers and glyph displays. Seals when the player walks into a trigger 4 m inside the door (simpler than binding **On Portal Traversed**), runs the socket check, and breaks when every enabled socket is filled. |
| `SM_Hex_*` | Static meshes | Hex web, socket plate and pedestal, built by `build_hex.py` |
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
| `build_hex.py` | `SM_Hex_Ring`, `SM_Hex_SocketPlate`, `SM_Hex_Pedestal` (in `Hex.blend`) |
| `build_wings.py` | `SM_Wing_Chasm`, `SM_Wing_Study`, `SM_Wing_Closet`, `SM_Wing_Vault` (in `Wings.blend`) |
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
- **Hex backing plate:** each rune also has a 2 cm pointy-top hexagonal plate behind its strokes (corner radius 32.5 cm, material slot 1 = `MI_RunePlate`, one extra `UCX_` hull), so a rune's facing is obvious and it seats in a socket. The meshes are now about 56 × 65 cm with a bounding radius of about 43 cm, still under the 45 cm grab limit at 1× and over it at 2× (86 cm).
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

## Core loop (milestone 2)

**`BP_Rune`** (`Content/Hexfold/Blueprints`): root `Mesh` (static mesh, Movable, Simulate Physics, `PhysicsActor` profile, overlap events on) and `PortalTraversal`. `RuneIndex` (int, instance editable). BeginPlay: if the actor's X scale is negative, **Set Mirrored (true)**, so a rune placed mirrored in the level counts as mirrored. Set the mesh (`SM_Rune_N`) per instance. Rune meshes use `MI_Rune` (gold, a child of `M_PhysicsCube_PortalClip`, so runes are cut at portals).

**`BP_HexSeal`**: one actor for the web, the six sockets and the barrier. Place it just inside the entrance with **yaw 180** so its front faces the room. In `L_Observatory` it's at (50000, 1960, 150).

| Component | Purpose |
|---|---|
| `HexRoot` → `Web` (`SM_Hex_Ring`), `Plate0`–`5` (`SM_Hex_SocketPlate`, component scale 1.2 × 1 × 1.2 so the rune plates fit inside the rim), `Glyph0`–`5` (`SM_Glyph_N`) | Visuals, hidden until the door seals. Socket *k* sits on the hex corner at 90° + 60°·*k* (radius 1 m): socket 0 at the top, then anticlockwise as seen from the room side. |
| `Trigger0`–`5` | Box triggers 20 cm in front of each socket. A `BP_Rune` overlapping one calls `TryPlace(Rune, k)`. |
| `Barrier` | Box across the doorway (310 × 12 × 310 cm), `BlockAll`. Collision is off until the door seals (set in BeginPlay). |
| `SealTrigger` | 6 × 2 × 4 m box, 4 m into the room. The player overlapping it calls `SealDoor`. |

Variables: `SocketEnabled` (bool ×6, instance editable: the level enables only socket 0 for now), `SocketFilled`, `bSealed`, `bBroken`.

Functions: `SealDoor` (show the web and enabled sockets, barrier on), `TryPlace` (checks, in order: sealed and not broken, socket enabled and empty, `RuneIndex` matches, |scale Z| within 0.9–1.1, not **Is Mirrored**), `AcceptRune` (release from the player's grab, physics and collision off, snap 3.5 cm in front of the glyph, so the rune's plate sits in the socket's rim facing the room, light the plate with `MI_Glyph_Lit`, then `BreakHex` if `AllFilled`), `RejectRune` (release, push it 250 cm/s into the room and 150 cm/s up, print the reason), `BreakHex` (hide everything, barrier off), `ReleaseIfHeld`, `AllFilled` (every enabled socket is filled).

Feedback is `Print String` for now; proper glyph effects are milestone 6/7.

**In the level:** `Rune0` on `Ped_Rune0` at (50000, 700) in front of the dais, plus `Test_RuneSmall` (scale 0.5) and `Test_RuneMirrored` (scale −1, 1, 1), all rune 0, on pedestals 4.5 m to either side. Remove the test runes when the real puzzles go in.

**Player:** `BP_FirstPersonCharacter`'s Portal Grab **Max Object Radius** is 45 (was 90).

## Runes 1–2 (milestone 3)

**Wings and alcoves.** Each wing is reached through a 4 × 4 m portal pair (scale 2 on both, with `SM_Obs_Doorframe` at scale 2): one at the back of a Rotunda alcove, 23.4 m from the centre, with yaw = atan2(−uₓ, u_y) so its local +Y points out of the room along the alcove direction u = (sin 45k°, cos 45k°); the other at the wing room's origin, yaw 0, scale 2, centred 2 m up. You walk out of the Rotunda facing out and arrive facing into the wing (+Y). Wing rooms are built far apart, each in its own space. Alcove numbering *k* counts 45° steps from the entrance (k = 0); k = 4 is the niche.

| Rune (index) | Alcove | Rooms | Space | Origin (cm) |
|---|---|---|---|---|
| 1 Through the Glass (0) | 7 | `SM_Wing_Chasm` | 2 | (80000, 0, 0) |
| 2 Orrery Cabinets (1) | 6 | `SM_Wing_Study` | 3 | (110000, 0, 0) |
| | | `SM_Wing_Closet` | 4 | (140000, 0, 0) |
| | | `SM_Wing_Vault` | 5 | (170000, 0, 0) |

Wing meshes are built by `build_wings.py` (`Wings.blend`), written in Unreal coordinates and mirrored in Y for export.

**Rune 1: Through the Glass.** A 12 × 26 m gallery: near floor (y 0–10 m), a 10 m pit 6 m deep, and a far ledge (y 20–26 m) with rune 0 on a pedestal at y = 23 m. A railing and an invisible blocker (`Wing1_ChasmBlocker`, `InvisibleWall`, hidden in game) stop the player from crossing or falling. Two 60 cm portal windows (scale 0.3, so nothing is resized): `WindowA` on a stand on the player's side at (−3, 8, 1.4) m, and `WindowB` on the ledge 1.2 m in front of the rune, both facing +Y. They're one-way windows: A's front links to B, and B's back links to A. Backing plates cover A's back and B's front, so the unlinked sides are never seen. Looking into A shows the rune about a metre away, within Grab Range (250), so the player grabs it through the window and pulls it through. The 60 cm window is narrower than the player's capsule, so the player can't climb through.

**Rune 2: the Orrery Cabinets.** The study has three cabinet doors (scale 1) in its far wall: Mercury (x −4 m), Venus (0) and Mars (+4 m), each with a coloured planet above it. A mural on the left wall shows the order: Mars, Venus, Mercury, numbered I, II, III. **`BP_CabinetPuzzle`** (in the study) holds `Cabinets` [Mercury, Venus, Mars], `ClosetDoor`, `VaultDoor`, `Sequence` [2, 1, 0] and `Progress`.
- BeginPlay waits 0.5 s, then binds the player's Portal Traversal **On Portal Traversed** (with `AssignOnPortalTraversed`, which creates a matching custom event; a Create Event bound to a function doesn't work because the delegate's ScaleFactor is a single-precision float) to `HandleTraversal`.
- `HandleTraversal`: going from cabinet *i* into the closet links the closet door back to *i*, then advances `Progress` if *i* is the next cabinet in `Sequence`. Otherwise it resets Progress (to 1 if *i* is the first cabinet in the sequence, else 0). Entering the vault links the vault door back and resets Progress. It prints "clicks forward", "turns into alignment" or "grinds back".
- `UpdateLinks`: every cabinet leads to the closet, except the last cabinet in the sequence, which leads to the vault once `Progress` reaches the last step. Because portals show their destination, the player sees the vault through the Mercury door once it's open.

The hex now has sockets 0 and 1 enabled. The test runes from milestone 2 are removed.

## Runes 3–4 (milestone 4)

| Rune (index) | Alcove | Room | Space | Origin (cm) |
|---|---|---|---|---|
| 3 The Heavy Rune (2) | 5 | `SM_Wing_Push` (10 × 16 × 6 m) | 6 | (200000, 0, 0) |
| 4 The Mouse Door (3) | 3 | `SM_Wing_Mouse` (10 × 12 × 5 m) | 7 | (230000, 0, 0) |

**Heavy Rune:** `Rune2` (scale 2, lying flat) at (0, 9) m in the hall. `Portal_Wing3_Big` (scale 2, yaw 0) is in the far wall at y = 16 m; `Portal_Wing3_Small` (scale 1, yaw −90 so its back faces into the room) is in the left wall at y = 4 m. They link to each other on both sides: scale ×0.5 going in the big one, ×2 coming back.

**Mouse Door:** `Rune3` (scale 0.5) on a pedestal at (−2, 7) m. `Portal_Wing4_MouseDoor` (scale 0.5, yaw 0) is in the far wall at x = −3 m, at floor level; `Portal_Wing4_BigDoor` (scale 1, yaw 90 so its back faces into the room) is in the right wall at y = 6 m. A normal-size player doesn't fit the mouse door (its opening is 98 × 99 cm).

The hex now has sockets 0–3 enabled.

## Runes 5–6 (milestone 5)

**Hall of Reflections** (rune index 4): alcove 2 → `SM_Wing_Mirror` (10 × 20 × 6 m, space 8, origin (260000, 0, 0)). `Portal_Alcove2` is scale (2, 2, 2) and `Portal_Wing5_In` is scale (−2, 2, 2), so the pair mirrors. The hall is symmetric about x = 0 (the entrance mirror's reflection plane), with the rune, pedestal and lights on the centre line. It's also symmetric about y = 10 m (the window mirrors' plane), with a blind doorway at the far end matching the entrance. Mirror windows: `Portal_Wing5_WindowW` (west wall, yaw 90, scale 0.5) and `Portal_Wing5_WindowE` (east wall, yaw 90, scale (−0.5, 0.5, 0.5)), centred 1.4 m up on a 0.9 m sill, too small for the player. W's front links to E, and E's back links to W, so a rune thrown into either comes out of the other, flipped. `Rune4` is world-correct, at scale 1.

**The Telescope** (rune index 5): its own wing behind alcove 1 → `SM_Wing_Telescope` (24 × 18 × 8 m, space 9, origin (290000, 0, 0)). Positions below are relative to the wing origin. (It was first built in the Rotunda and moved here so every rune has its own room; the niche is empty again.)
- `Portal_Telescope_Eyepiece` (scale 1, yaw 0, front facing the entrance) stands at (0, 7, 1) m with a backing plate behind it. Its front links to `Portal_Telescope_Lens` in the far-wall doorway at (0, 18) m, yaw 180, back facing the room. The doorway is sized for the lens at scale 2. The lens's back links to the eyepiece.
- **`BP_Telescope`** at (0, 7, 0) m: three plates with triggers and labels at x = +4.5, +7.5, +10.5 m (Gemini, Lynx, Cygnus). Variables: `Lens`, `LensFrame`, `LensBase` (290000, 1800, 0) and `Setting` (−1 at start).
- `ApplySetting(i)` sets the lens and its frame to scale (−1, 1, 1), 0.5 or 2, standing on the floor (centre at |scale| × 1 m), and prints the constellation.
- `Rune5` starts at scale (−0.5, 0.5, 0.5) (mirrored, half size) on a pedestal at (−6, 7) m.

All six hex sockets are now enabled, so the hex breaks only when all six runes are in.

## Safety nets (milestone 6)

- **`BP_Rune` return.** BeginPlay stores `SpawnTransform` and `SpawnSpace`. **`ReturnToPedestal`** zeroes the physics velocities, teleports the rune back (scale included), calls **Reset Tracking**, **Set Mirrored** from the sign of its X scale, **Refresh Mirrored Rendering** and **Set Actor Space**, then releases it if the player is holding it.
- **`BP_KillVolume`** (box, overlap only; `RespawnTransform` per instance). A `BP_Rune` overlapping it calls `ReturnToPedestal`. The player is stopped, rescaled to 1× (**Apply Scale Factor** with the inverse of the current scale), unmirrored, teleported to `RespawnTransform`, turned to face the same way (**Set Control Rotation**) and given **Reset Tracking**.
- **Placement.** One 60 × 60 × 10 m slab per space about 15 m below the floor (`KV_Outside` is 600 × 600 m, the Rotunda's 80 × 80 m), plus `KV_ChasmPit` (12 × 10 × 1 m) just above the pit floor in the chasm wing. The `Volume` box keeps its default 32 cm Y/Z extent (`BoxExtent` can't be set through MCP), so each slab's size comes from the **actor scale** (Y = X extent / 32, Z = 500 / 32). The slabs are thick so a fast-falling rune can't skip over them between frames.
- **Runes use CCD.** `BP_Rune`'s `Mesh` (template and every placed rune) has **Use CCD** on. The room shells use complex-as-simple (triangle-mesh) collision, which has no thickness, so a thin rune falling or pushed fast enough could pass through a floor or wall. In a drop test from 27 m in the Rotunda, 2 of 6 runes fell through without CCD and none did with it. Respawn points are 3 m inside each wing's entrance, facing +Y. The closet and vault respawn in the study, and the Hall of Reflections respawns in the Rotunda (at (50000, 800, 100)), so nobody lands in the mirrored hall unmirrored. Outside respawns at the `PlayerStart`.
- **`BP_RestoreFont`** (`RestoreFont`, tagged `PortalSpace.1`, at (50000, 1100, 0) in the Rotunda). A glowing pad inside a box trigger: the player is rescaled to 1× and unmirrored if they aren't already, and it prints "The font's light restores you." The pad's collision is turned off in BeginPlay.
- **Socket feedback.** `RejectRune` now also calls `FlashPlate(Index)`, which lights that socket's plate with `MI_Glyph_Lit` for 0.4 s; `EndFlash` puts every unfilled plate back to `MI_Glyph_Idle`. The reason text is still printed. Proper glyph effects are milestone 7.

## Performance (milestone 8)

Measured in PIE (so the absolute numbers include editor overhead; compare them with each other) with a temporary probe Blueprint that logged average frame time and **Get Last Capture Count** every 2 s. The worst cases are the Rotunda looking toward alcoves 1 and 7 and the entrance, and the telescope room looking at the eyepiece and lens.

| Setting | Rotunda ms (captures) | Telescope ms (captures) |
|---|---|---|
| `MaxDepth` 4, falloff 1 (plugin default) | 20.7 (10) | 16.5 (6) |
| `MaxDepth` 3, falloff 1 | 18.0 (8) | 14.2 (4) |
| `MaxDepth` 2, falloff 1 | 13.3 (5) | 8.7 (2) |
| `MaxDepth` 4, falloff 0.5 | 19.1 (10) | 14.2 (6) |
| **`MaxDepth` 3, falloff 0.5 (chosen)** | 16.5 (8) | about 12.5 (not measured; 14.2 − 1.7 from the other falloff gains) |
| `MaxDepth` 6, falloff 1 | not measured | 20.3 (8) |

`MaxCaptures` (32) is never reached. Depth 3 shows no pink fallback portals in the telescope room. Depth 2 would be cheapest but a portal seen through a portal would turn pink, which breaks the windows and cabinets, so it isn't used. The values are set in `Config/DefaultEngine.ini` under `[SystemSettings]`.

Not yet done: a check of the Hall of Reflections and the study cabinets at depth 3, and timing the packaged build (it should be faster than PIE).

## Milestones

Each milestone ends with a playable build in PIE.

| # | Milestone | Done when |
|---|---|---|
| 0 | Branch, README, plan | This document is merged |
| 1 | **Greybox hub.** `L_Observatory` with the outside, the entrance portal pair and an empty Rotunda in their own spaces. Set as the default map. | ✅ Built (see [Greybox hub layout](#greybox-hub-layout)). Walked through both ways. The door is 3 × 3 m (both entrance portals at scale 1.5). |
| 1b | **First models.** The six rune meshes with collision and glyphs, the doorframe and the Observatory exterior shell, from Blender | ✅ They import at the right size, pivots are centred, and a rune passes through a portal cleanly |
| 2 | **Core loop.** `BP_Rune`, `M_Rune`, `BP_HexSeal`, Max Object Radius 45 cm, with one rune on a pedestal in the Rotunda and the other five sockets temporarily disabled | Built (see [Core loop](#core-loop-milestone-2)). Tested in PIE: walking in seals the door; the mirrored and half-size test runes are rejected with the right reason; rune 0 locks in and the hex breaks. **Still to try by hand:** carrying the rune to the socket. |
| 3 | **Runes 1–2** (through the glass, cabinets) | Built (see [Runes 1–2](#runes-12-milestone-3)). Tested in PIE: the alcoves lead to the wings, the window shows the rune up close, and the cabinet sequence (including a wrong first try) opens the vault. **Still to try by hand:** grabbing rune 1 through the window, and carrying both runes back to the hex. |
| 4 | **Runes 3–4** (size) | Built (see [Runes 3–4](#runes-34-milestone-4)). Tested in Simulate/PIE: the 2× rune comes out of the small door at 1×, the 0.5× rune comes out of the big door at 1×, and the player shrinks through the big door and grows back through the mouse door. **Still to try by hand:** pushing the heavy rune, and throwing the small one. |
| 5 | **Runes 5–6** (mirror, telescope) | Built (see [Runes 5–6](#runes-56-milestone-5)). Tested in PIE/Simulate: entering the gallery mirrors the player with no seams at the entrance or windows; the rune comes out of the window mirrored; the Cygnus plate doubles the lens, the eyepiece grows the player to 2×, and walking back undoes it. **Still to try by hand:** both full solutions and all six runes in the hex. |
| 6 | **Safety nets.** Kill volumes, rune return, restore font, socket feedback | Built (see [Safety nets](#safety-nets-milestone-6)). Tested in PIE: a rune dropped into the chasm pit returns to its pedestal, a player who falls under the Rotunda is put back inside it, and a 2× player is restored to 1× on the font. **Still to try by hand:** a rune thrown out of reach while carried, and the rejected-socket flash. |
| 7 | **Art, sound and lighting.** Final Blender models for the Rotunda, wings and telescope, Observatory look, glyph VFX, hex break, ending sky. Check portal edges under Lumen, auto-exposure and TSR (see the guide's Limitations). | A full playthrough looks finished. **In progress:** `PPV_Global` (unbound post-process volume: histogram auto-exposure, bias −0.5, brightness 0.4–2.0) fixes the overexposed Rotunda. `BP_HexSeal.BreakHex` now calls `EndingSky`, which turns the sun gold and drops it to a low dawn angle (compiled, **not yet seen in PIE**). **Still to do:** final Blender models, glyph VFX, sound, and the Lumen/TSR portal-edge check. |
| 8 | **Performance and packaging.** Check `Portal.ShowStats 1` in the Rotunda, tune `Portal.MaxDepth` and `Portal.ResolutionFalloff`, and package a Win64 build | Tuned (see [Performance](#performance-milestone-8)): `MaxDepth` 3 and `ResolutionFalloff` 0.5 cut the worst Rotunda frame from 20.7 to 16.5 ms in PIE. A Development Win64 package builds and starts (see `CLAUDE.md` → Packaging). **Still to do:** time the packaged build and check the hall and cabinets at depth 3. |

## Risks

- **Too many portals in view.** The Rotunda could see several portals at once, each a full scene capture. Mitigation: ordinary corridors for wings, doors that close, and checks with `Portal.ShowStats 1`.
- **Mirror seams.** Mirroring portals need left-right symmetric surroundings. Mitigation: build the Hall of Reflections as a straight, symmetric corridor and test early.
- **Rune collision.** Chiral shapes can tempt complex collision, but mirrored physics needs simple collision on one body. Mitigation: give each rune a single convex hull or box.
- **Lighting.** The plugin was verified in unlit, fixed-exposure scenes. Mitigation: test portal edges as soon as real lighting goes in (milestone 7, or earlier in one wing).
- **Puzzle clarity.** Size and mirror differences need to be easy to read. Mitigation: socket glyphs show the target size and handedness, and rejection feedback says which check failed.
