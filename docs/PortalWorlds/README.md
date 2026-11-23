# PortalWorlds for Bevy

Seamless, recursive portals for non-Euclidean spaces: tunnels longer inside than outside, rooms bigger on the inside, doorways that change your size, and portals into a mirror world. It's a Rust port of the PortalWorlds plugin for Unreal Engine (itself a port of the technique in CodeParade's [NonEuclidean](https://github.com/HackerPoet/NonEuclidean) engine; see [`THIRD_PARTY_NOTICES.md`](../../THIRD_PARTY_NOTICES.md)). It lives in `src/portal/` and depends on Bevy 0.20 and avian3d 0.8. It doesn't know anything about the game.

- **Rendering:** every portal visible in a view adds a child view, up to a set depth. Each child view is a pooled camera rendering into its own image; the portal surface shows it at the fragment's own screen position, so the edge is invisible and there's no frame lag.
- **Walking through:** the player passes through portals with no visible jump. A portal of a different size scales the player, and a mirroring portal flips the world left-right.
- **Objects:** physics objects pass through too, thrown or carried. A scaling portal resizes them and a mirroring portal flips their handedness.
- **Spaces:** keep alternative rooms out of sight of each other.

## Using it

```rust
app.add_plugins((DefaultPlugins, PhysicsPlugins::new(PostUpdate), PortalWorldsPlugin));
```

Physics must run in `PostUpdate` (variable step), so the portal systems see exactly the motion that was rendered.

### Portals

```rust
commands.spawn((
    Portal { front_link: Some(b), back_link: Some(b), ..default() },
    Transform::from_translation(..).with_scale(Vec3::splat(1.5)),
    PortalSpace(1), // optional
));
```

- A portal is a rectangle in its entity's local XY plane, 2 m x 2 m before scale (`half_extent`). Its front faces local -Z (`Transform::forward`). It must be a root entity.
- `front_link` is where you go entering from the front side, `back_link` from the back. Both can be changed at runtime (the cabinets do). `connect(&mut query, a, b)` links two portals both ways.
- The exit keeps your position relative to the portal. You come out of the target's back side. Rotate the target 180 degrees to come out facing the other way.
- **Scale:** if the target is smaller or larger (uniform scale), whatever passes through is scaled by the same factor, including the player's collider, camera height, speeds and gravity.
- **Mirroring:** give one of the two portals a negative X scale (for example `(-1, 1, 1)`). The geometry around both should be left-right symmetric, or the seam shows.

### The player

Give the player body a `PortalPawn` (view angles and size) and a `PortalTraveller`, and spawn a camera with `PortalCamera`, `Projection::custom(PortalProjection::default())` and `Hdr`. The plugin places the camera from the pawn each frame; your controller moves the body and reads/writes `PortalPawn::yaw` / `pitch`. When `pawn.scale` changes (a scaling portal), rebuild the collider, speeds and gravity. While `traveller.mirrored` is set, reverse mouse yaw and strafing.

Set `PortalSettings::exposure_ev100` to the player camera's exposure: every portal view must expose the same way.

### Objects

A root entity with a rigid body and a positive uniform scale, a `PortalTraveller`, and `PortalObject { visual }` where `visual` is a child that holds the meshes (the plugin gives it a negative X scale while the object is mirrored). Give it a centred pivot: the point that has to cross is the root's origin.

Add `PortalClipped` with meshes using `ClipMaterial` and the object is cut at the portal plane while partway through, with a copy shown at the other side. Objects follow the exit portal's `PortalSpace`.

### Carrying objects

Add `PortalGrab` to the player. The game calls nothing: the plugin reads E (grab / drop) and the left mouse button (throw) itself. It picks up the dynamic `PortalObject` under the crosshair, also through up to two portals, and carries it through portals together with the player (so a carried rune changes with the player; throw it to change it relative to the player). `ReleaseObject(entity)` makes it let go; `ObjectGrabbed` / `ObjectReleased` report what happened.

### Spaces

Rooms that share an apparent location are built far apart. Tag everything of space `n` with `PortalSpace(n)`; untagged entities appear in every space. Space `n` is render layer `1 + n`, applied to the entity and everything below it each frame. A view renders layer 0 and its own space. Keep the `PlayerSpace` resource right when you teleport the player (traversal updates it by itself).

### Messages

`PortalTraversed { entity, from, to, scale_factor, mirrored_warp }` is sent after every crossing, by the player or an object.

## Settings

`PortalSettings`: `max_depth` (default 4: views up to depth 3; deeper portals show a dim void), `max_captures` (32), `resolution_falloff` (render-target size multiplier per level beyond the first), `dynamic_near_plane` (pull the near plane in to 1 mm near portals), `debug`, `hide`, `exposure_ev100`. `PortalStats` reports the number of views and the near plane.

## How it works

Every frame, after the camera is placed, `render_portals` builds a tree of views on the CPU. The root is the player's view. For each portal visible in a view (not behind, not entirely outside the parent portal's clip plane, screen rectangle overlapping the parent portal's), a child view is made by warping the camera through the portal (`target.matrix * portal.matrix.inverse()`). Each child camera replaces its near plane with the exit portal's plane (Bevy's oblique `near_clip_plane`), pulled towards the camera by the near-plane distance, so nothing between the virtual camera and the exit portal is drawn. Cameras render deepest-first (`Camera::order`). Each view has its own set of surface entities on its own render layer, so one portal can show different images to different cameras.

Traversal tests the eye's path each frame against every portal rectangle, with the plane moved 2 mm (times the traveller's size) towards the side the eye came from. On a crossing the traveller is warped to the linked portal, and the eye is placed twice the bump past the exit plane. A pawn's location, view, velocity and size are warped; an object's whole transform is multiplied by the warp, and a reflection in the warp becomes the object's mirror flag (a proper rotation plus a negative X scale on the visual child).

A mirroring warp (negative determinant) can't be applied to a camera, so each view carries a `mirrored` flag. It is rendered with the matching proper camera, whose image is the true view flipped left-right; a surface undoes the flip with its `mirror_u` parameter (parent flipped XOR child flipped). The player's own view is flipped by negating the projection's X, with `Camera::invert_culling`.

A carried object's hold point is the player's frame times a "carry warp" that collects the portals between the player and the object: each trip the object makes is appended, and each trip the player makes is cancelled.

### Frame order

All in `PostUpdate`, after avian's `PhysicsSystems::Writeback` and before `TransformSystems::Propagate`: refresh the portal set, update eyes, traverse, place the camera, update crossing objects and space layers, render the view tree.

## Limitations

- Every view is a full scene render. Keep portal scenes light, or lower `max_depth` / `resolution_falloff`. Point lights in other rooms are on their own space's layer, so a view only processes its own room's lights.
- Single player, single view. No split screen, no VR.
- The cut at a portal is a whole plane, not just the portal's rectangle.
- The copy of an object partway through a portal has no collision, so the part that's through doesn't collide with anything until the object's centre crosses. Aiming at a copy doesn't grab the object.
- Portal scale must be uniform (apart from the sign of X for mirroring portals).
- Mirrored objects keep an unmirrored collider (use symmetric colliders).
