# PortalWorlds guide

How the plugin is set up in Hexfold and how to build portal puzzles with it. Everything here is done in the editor or in Blueprints. For exact names, types and defaults, see the [API reference](API.md).

- [Project setup](#project-setup)
- [Portals](#portals)
- [Scaling portals](#scaling-portals)
- [Mirroring portals](#mirroring-portals)
- [Physics objects](#physics-objects)
- [Carrying and throwing](#carrying-and-throwing)
- [Spaces](#spaces)
- [Reacting to traversal in Blueprints](#reacting-to-traversal-in-blueprints)
- [Respawning and scripted teleports](#respawning-and-scripted-teleports)
- [Performance](#performance)
- [Troubleshooting](#troubleshooting)
- [Limitations](#limitations)

## Project setup

Hexfold is already set up. This is what the setup consists of, in case it needs checking or reproducing in another project.

| Piece | Hexfold | Why |
| --- | --- | --- |
| Plugin enabled | `Hexfold.uproject` lists `PortalWorlds` | The plugin is off by default (`EnabledByDefault: false`) |
| Global clip plane | `r.AllowGlobalClipPlane=True` in `Config/DefaultEngine.ini` | Each portal view clips away everything in front of its exit portal. Without this, portal views show geometry that is behind the exit portal, and the plugin logs a warning at play. |
| Occlusion queries off | `r.AllowOcclusionQueries=False` | Portal views jump between very different viewpoints, and stale occlusion results make geometry pop for a frame. The plugin logs a note if they are on. |
| Camera manager | `BP_FirstPersonCameraManager` (parent: `PortalPlayerCameraManager`), set on `BP_FirstPersonPlayerController` | Renders the portal views after the camera's final position is known, and reverses mouse yaw while mirrored |
| Traversal | **Portal Traversal** component on `BP_FirstPersonCharacter` | Moves the player through portals |
| Grabbing | **Portal Grab** component on `BP_FirstPersonCharacter`. `IA_Grab` (E / gamepad X) calls **Toggle Grab**, and `IA_Throw` (LMB / right trigger) calls **Throw**. | Carrying and throwing |

A camera manager that doesn't derive from `PortalPlayerCameraManager` can't render portals from Blueprints. The hook it needs (`UPortalSubsystem::RenderPortals`) is C++ only, so always reparent camera managers onto `PortalPlayerCameraManager`.

Changes to plugins or to `.uproject` take effect only after an editor restart, when the `/Script/PortalWorlds.*` classes become available.

## Portals

### Placing

Drag a **Portal** actor into the level (Place Actors, or search "Portal").

- The surface is a 2 × 2 m double-sided quad (`SM_PortalQuad`) in the actor's **local XZ plane**, centred on the actor origin. X is width and Z is height.
- The **front** faces local **−Y**. **Get Normal** returns this direction in world space.
- The quad is the root component, so scaling the surface scales the actor.
- **Scale must be uniform.** Only the sign of X may differ, which makes the portal mirroring. To fit a narrower doorway, scale the whole actor or use a custom surface (see below).

### Linking

Each portal has two links, set per instance in the Details panel under **Portal**:

- **Front Link:** where you go when you enter from the front side, the side the normal points to.
- **Back Link:** where you go when you enter from the back.

A link can be empty, and that side then renders as an ordinary surface. To link two portals both ways, set **both links on both portals** to the other portal. At runtime, the **Connect (A, B)** node does the same thing.

Links are `EditInstanceOnly`. They can be set on placed actors, but not in a Blueprint subclass's defaults. Both links can be read and written from Blueprints at runtime, which is how to open, close or redirect a portal during play.

### Where you come out

Passing through keeps your position and orientation **relative to the portal**:

```
Warp = Source.LocalToWorld⁻¹ × Target.LocalToWorld
```

There is no built-in 180° flip, so **entering the front of A leaves through the back of B**, facing the same way relative to B. To come out of B's front instead, rotate B 180° about Z.

Practical check: stand in front of portal A, facing it. Then picture standing behind portal B, facing the same way relative to B. That is what you see through A and where you arrive.

### Custom surfaces

A portal can use any surface mesh that lies in its local XZ plane. Set **Surface Half Extent** to match it. The default is (100, 100) cm, the half width and half height before actor scale. Traversal and visibility use this rectangle, not the mesh bounds.

A replacement material must have:

| Parameter | Type | Purpose |
| --- | --- | --- |
| `PortalTex` | Texture | The view through the portal. Sample it at **ScreenPosition → ViewportUV**. |
| `Pink` | Scalar | 1 = draw a flat colour. It is shown where recursion stops (`Portal.MaxDepth`, `Portal.MaxCaptures`) and in `Portal.Debug`. |
| `MirrorU` | Scalar | Needed only on mirroring portals. When it is 1, sample at U' = 1 − U. |

The simplest approach is a material instance of `/PortalWorlds/Materials/M_Portal`.

## Scaling portals

If the two linked portals are different sizes (uniform scale), everything that passes through is scaled by the ratio. A doorway linked to a half-size doorway halves the player:

- Pawns are scaled with **Apply Scale Factor**. It covers actor scale (capsule and camera height) and every character movement distance, speed and acceleration, including gravity, so the player moves the same relative to their own size.
- Objects have their whole transform scaled. Their linear and angular velocity are carried through, and their mass is recomputed.
- The **On Portal Traversed** event reports the **Scale Factor** (1 for a plain portal).

Grab tuning values (**Grab Range**, **Hold Gap**, **Throw Speed**, **Break Distance**) are in cm at the player's normal size, so they scale with the player automatically.

## Mirroring portals

Give **one** of two linked portals a negative X scale, for example (−1, 1, 1), or (−0.6, 1, 1) for a 0.6-wide doorway. The link then reflects space across the portal's width. The reflection plane passes through the portal's centre, perpendicular to its width. **Is Mirrored Link To** reports whether a link mirrors.

**Seams.** The image through the portal is the far side mirrored, so the surroundings must look the same mirrored:

- Use left-right symmetric geometry around both portals, such as a straight corridor with the portal across it.
- Use symmetric textures. A checker should be centred on the portal's centre line, not split by it.
- Use symmetric lighting: a light whose direction lies in the reflection plane, or lighting that doesn't differ between the left and right walls. With directional lighting, run the corridor along the light's horizontal direction.

**What the player gets.** A camera can't render mirrored, so the plugin renders normally and flips the image:

- The screen is flipped left-right by `/PortalWorlds/Materials/M_PortalMirrorView`. It runs after tonemapping, so the UI and HUD aren't flipped. In a level with any mirroring portal, this pass runs from the start (unflipped) so its first use doesn't hitch.
- Mouse yaw is reversed by `PortalPlayerCameraManager`.
- Movement input is reflected left-right (**Mirror Movement Input**, on by default).
- **Is Mirrored** on the player's Portal Traversal component gives the state. **Set Mirrored** forces it, for example on respawn. **Is View Mirrored** on the subsystem tells you whether last frame's view was flipped.

**Not handled:**

- Stereo audio isn't mirrored, so a sound on the left of the screen is heard on the right.
- A visible body or first-person arms is flipped with the world. Give it a negative scale while mirrored to cancel the flip.
- The flip assumes a centred (symmetric) projection.

Objects that pass through a mirroring portal really become mirrored (negative scale). See [Physics objects](#physics-objects).

## Physics objects

Requirements for an object to pass through portals:

1. The root is a primitive with **Mobility = Movable** and **Simulate Physics** on.
2. It has a **Portal Traversal** component.
3. It has a **centred pivot**. The point that must cross is the root component's origin.
4. For mirroring, use **simple collision** made of boxes, spheres or convex hulls on a single body. Chaos handles these at negative scale. Bodies welded from several components don't rescale correctly.

When the object's origin crosses, its whole transform goes through the portal: position, rotation, scale and mirroring. Velocity goes with it. If the object is tagged with a space, it moves to the exit portal's space.

### Partway through: the crossing copy

While an object straddles a portal, a copy of its **static meshes** is shown at the linked portal (**Show Copy While Crossing**, on by default), so the part already through is visible on the far side.

- The copy is solid to **pawns only** (query-only, using the meshes' simple collision). A character walking into it pushes the real object, using the character movement's push settings.
- It doesn't collide with other objects or simulate. The part that's through only becomes physical once the object's centre crosses.

### Cutting at the portal plane

With **Clip At Portal** (on by default), the object and its copy are cut at the portal plane. The object keeps the side its centre is on, and the copy keeps only the part that's through. Each part then appears exactly once.

The cut is done in the **material**, so every material on a traversing object must support it:

1. Create a **new** material. Don't duplicate an opaque one, because a duplicate can keep the saved "masked can be assumed opaque" flag and then ignores the cut even though it says Masked.
2. Set **Blend Mode = Masked**.
3. Add the material function `/PortalWorlds/Materials/MF_PortalClip` and connect its **Mask** output to **Opacity Mask**. If there is already a mask, multiply the two.
4. Optionally set **Two Sided**. The cut leaves the mesh open, which looks better from the side.

The plane is passed through **custom primitive data indices 32–35** on every primitive of the object and its copy. Don't use those indices for anything else on traversing objects.

A material without `MF_PortalClip` draws the whole object, so the part that's through sticks out of the back of the entry portal. The portal views still look right, because they hide the object instead. In the editor and PIE, the first cut logs a `LogPortalWorlds` warning naming the materials that don't support it.

While cut, the object stops colliding with pawns standing on the cut-away side near the portal, so a player can't bump into the invisible part.

### Mirroring objects yourself

A mesh chooses its face culling from the sign of its scale when its render state is built. Traversal handles this automatically. If you set a negative scale (or undo one) in a Blueprint, call **Refresh Mirrored Rendering** on the actor afterwards, or it renders inside out.

## Carrying and throwing

The **Portal Grab** component on the player:

- **Grab** picks up the simulating object under the crosshair within **Grab Range**. Aiming also looks through up to two portals, so objects seen through a portal can be grabbed.
- Objects with a bounding radius over **Max Object Radius** (relative to the player's size) can't be lifted.
- The held object is pulled by a physics handle to a point **Hold Gap** beyond the eye, keeping its orientation relative to the player's heading. While held, it ignores collision with pawns.
- It's dropped automatically if it stays more than **Break Distance** from its hold point for half a second, for example when caught on a wall.
- **Throw** releases it at **Throw Speed** along the view.
- **Get Aimed Component** returns what **Grab** would pick up right now, which is useful for an interaction prompt. **Is Holding** and **Get Held Component** describe the current state.

**Carrying through portals.** Hold points are worked out in the player's own frame (position, heading, size and mirrored state). The object usually reaches the portal first and is warped by its own traversal component, and the hold point follows it. When the player crosses, the two warps cancel. So **a carried object changes together with the player**: carry a cube through a shrink portal and both shrink, and it still looks the same size to the player.

To change an object **relative to the player**, throw it through the portal, or carry it through and walk back around. **Get Held Relative Scale** returns the held object's size relative to the player. It is 1 when both have been through the same portals. This is useful for puzzles where the cube must be a particular size.

## Spaces

Rooms that occupy the same apparent location (such as a shed that is bigger inside) are built far apart, typically 200–500 m. To stop them appearing on each other's horizon, put each one in a **space**:

- Tag every actor of space *n*, both geometry and portals, with `PortalSpace.<n>`, for example `PortalSpace.1`. Use the actor's **Tags** array, not component tags.
- Untagged actors, such as the sky, lights and player, appear in every space.
- Each view renders only its own space and only looks through portals in that space.
- The player's space is the one whose bounds contain the camera. **Get Current Space** returns it, or −1 if the level doesn't use spaces.
- A portal outside its own space is invisible, along with everything seen through it, but it can still be walked through.

At runtime:

- **Set Actor Space (Actor, n)** tags an actor. −1 removes the tag, which puts it in every space. Spaces rebuild on the next frame.
- After spawning tagged actors or changing tags by hand, call **Refresh Spaces**.
- **Get Actor Space** reads an actor's space.
- Physics objects tagged with a space move to the exit portal's space when they traverse.

At play, a `LogPortalWorlds` warning lists each portal that stands inside another space's area but outside its own. Such a portal can't be seen from where it stands, so either retag it or move it.

`Portal.Spaces 0` disables filtering for debugging.

## Reacting to traversal in Blueprints

Bind **On Portal Traversed** on a Portal Traversal component, either on the player or on an object. It fires after each trip with **From Portal**, **To Portal** and **Scale Factor**. Uses include:

- Puzzle triggers, such as "the player went through portal X while small".
- Swapping links after a crossing, so walking back leads somewhere new. Set **Front Link** and **Back Link** on the portal in the handler.
- Counting trips with **Get Traversal Count**.

On the player, use **Is Mirrored** after the event to check handedness. On objects, it tells you whether the object is now a mirror image.

## Respawning and scripted teleports

After moving a pawn or object by any means other than a portal (respawn, Set Actor Location, Teleport):

1. Call **Reset Tracking** on its Portal Traversal component. Otherwise the jump may be taken as a walk through a portal on the jump's path.
2. If the player should come back unmirrored, call **Set Mirrored (false)**.
3. If the player should come back at normal size, call **Apply Scale Factor** with the inverse of their current scale. For example, use 2 if they are at half size.

## Performance

Every portal view is a full scene capture.

- `Portal.MaxDepth` (default 4) sets recursion depth. Lower it for scenes with many portals in view.
- `Portal.MaxCaptures` (default 32) caps captures per frame. Portals beyond the cap draw `Pink`.
- `Portal.ResolutionFalloff` (default 1.0) multiplies render-target resolution at each deeper level. For example, 0.5 halves it per level.
- Visibility is tested on the CPU, so portals hidden behind walls are still captured. Avoid many portals that are in front of the camera but blocked by walls.
- `Portal.ShowStats 1` shows view and capture counts and the near plane on screen. **Get Last Capture Count** returns the capture count in Blueprints.

## Troubleshooting

| Symptom | Likely cause / fix |
| --- | --- |
| Portal shows flat pink | No link on that side, recursion limit reached, `Portal.MaxCaptures` hit, `Portal.Debug 1`, or no `PortalPlayerCameraManager` (nothing is rendering) |
| Portal view shows things behind the exit portal | `r.AllowGlobalClipPlane` is off (check the log for the warning). Restart the editor after changing it. |
| Geometry pops for a frame in portal views | Occlusion queries are on. Set `r.AllowOcclusionQueries=False`. |
| Player doesn't go through | No Portal Traversal component, the portal has no link on that side, or the camera isn't crossing the portal rectangle (check **Surface Half Extent** on custom surfaces) |
| Object doesn't go through | Not simulating physics, not Movable, no Portal Traversal component, or its pivot isn't centred (its root origin never crosses) |
| Object sticks out of the back of the entry portal while crossing | Its material doesn't use `MF_PortalClip`, isn't Masked, or is a duplicate of an opaque material. Check the log warning. |
| Object renders inside out | It was mirrored outside of traversal. Call **Refresh Mirrored Rendering**. |
| Mirrored object collides strangely | Use simple collision (box, sphere or convex) on a single body |
| Visible seam at a mirroring portal | The surroundings aren't left-right symmetric (geometry, texture alignment or lighting) |
| Other rooms visible on the horizon | Actors aren't tagged `PortalSpace.<n>`, or `Portal.Spaces` is 0 |
| Portal invisible from where it stands | It is tagged with one space but stands in another's area. See the log warning. |
| Player arrives facing backwards | Rotate the target portal 180° about Z (you exit through the target's back) |
| Player teleports after a respawn | Call **Reset Tracking** after moving the pawn |
| `/Script/PortalWorlds.*` classes missing | The plugin is disabled or the editor hasn't been restarted since it was enabled. Also check that LFS pulled the DLL (`git lfs pull`). |
| Packaged game: "module 'PortalWorlds' could not be found" | The plugin copy lost `"EnabledByDefault": false` (for example, it was rebuilt with raw `RunUAT BuildPlugin`). Use the packaging script's output. |

Turn on `LogPortalWorlds` (`log LogPortalWorlds Verbose`) to see traversals, grabs and render-target creation.

## Limitations

- Every capture is a full scene render.
- Visibility is tested on the CPU, so occluded portals are still captured.
- The look was verified in unlit, fixed-exposure scenes. With Lumen, auto-exposure or TSR/TAA, check the portal edges in your own scenes. A portal linked to itself should be invisible.
- Single player and single view only: no split-screen or VR.
- The cut at a portal is a whole plane, not just the portal's rectangle. If an object straddles the edge of a free-standing portal, the part beside the portal on the far side is hidden too.
- The crossing copy only blocks pawns. The part that's through doesn't collide with anything else until the object's centre crosses.
- A fast player can outrun a held object, which then trails behind. This is still correct, just less tidy.
- Portal scale must be uniform, apart from the sign of X.
