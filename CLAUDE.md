# Hexfold

First-person portal puzzle game in Rust + Bevy 0.20 (physics: avian3d 0.8). Rewritten from an Unreal Engine 5 Blueprint game that used the C++ PortalWorlds plugin; both now live in this crate. The design docs (`docs/Design/Plan.md`, `docs/Architecture.md`) describe the game; the Unreal wording in `Plan.md` (Blueprints, MCP tooling) is historical.

## Layout

- `src/portal/` is the PortalWorlds plugin and knows nothing about Hexfold's puzzles. `geom` (portal planes and warps), `traversal` (walking and throwing through portals), `render` (recursive views), `clip` (objects cut at a portal, space render layers), `grab` (carry and throw), `material` + `projection` (support).
- `src/game/` is the game. `src/level/` builds the level from `level/data.rs`, which is **generated** by `python tools/gen_level_data.py` from `tools/unreal_dump/*.json` (dumps of the Unreal level). Don't edit `data.rs` by hand.
- `src/ue.rs` converts Unreal coordinates to Bevy: `bevy = (ue.x, ue.z, ue.y) / 100`, Unreal yaw `psi` is `ue::view_yaw(psi)` for the player and `ue::rot` for objects. The Blender -> glTF pipeline already uses this mapping, so a mesh looks the same as it did in Unreal.

## Commands

```
cargo run --release          # play
cargo check                  # fast compile check
python tools/gen_level_data.py   # regenerate src/level/data.rs
blender -b --python art/Blender/scripts/export_glb.py   # art/Export/*.fbx -> assets/models/*.glb
```

Running the exe directly (not through cargo) needs `BEVY_ASSET_ROOT=<repo root>`.

## Automated checks (no keyboard needed)

`HEXFOLD_SCRIPT` drives the game: steps separated by `;`: `wait:N`, `tp:x,y,z,yaw,pitch,space` (Unreal cm and degrees), `walk:dx,dy,dz,frames` (m/s, Unreal axes), `fwd:speed,frames` (walk the way the player faces, so it carries on through portals), `turn:dyaw,dpitch,frames` (degrees per frame, runs alongside the next steps), `key:E`, `click`, `runetp:i,x,y,z`, `runevel:i,vx,vy,vz`, `sunrise:f` (jump the ending to f in 0..1), `pos` (logs player and runes), `shot:name` (writes `shots/name.png`), `burst:name,N` (a screenshot every frame for N frames, alongside the next steps), `quit`. Example:

```
BEVY_ASSET_ROOT=. HEXFOLD_SCRIPT="wait:300;tp:50000,1560,100,90,0,1;wait:60;shot:hex;quit" ./target/debug/hexfold.exe
```

To hunt a flicker, `burst` through the moment and compare neighbouring frames; to hunt a bump, run with `HEXFOLD_TRACE` and look for a jump in the vertical velocity. Wait ~300 frames at the start so assets load. A scripted run skips the title screen. Other env vars: `HEXFOLD_TRACE` (a line per frame on stderr: frame time, player position and velocity, crossings, view count, plus any player contact that isn't a level floor), `HEXFOLD_DEBUG` (print the portal view tree), `HEXFOLD_NOCLIP` (no oblique clip), `HEXFOLD_PHYSDEBUG` (draw colliders), `HEXFOLD_SKIP_TITLE`.

## Things that bit us (Bevy 0.20 / avian)

- Shaders must be `.wesl` (WESL imports like `import bevy_pbr::render::...`); `.wgsl` files get no `#import` processing.
- `CollisionLayers` is an immutable component: replace it with `commands.entity(e).insert(..)`. Avian's `Position` / `Rotation` are authoritative: when teleporting, set them as well as `Transform`, and order systems `.after(PhysicsSystems::Writeback)`.
- Don't toggle a static collider with `ColliderDisabled`: one spawned disabled never collides after the marker is removed. Swap its `CollisionLayers` (`CollisionLayers::NONE` when off) instead.
- A player filter must include `Layer::Trigger`, or sensors never see the player.
- `RenderLayers::layer(n)` panics above 63; use `from_layers(&[n])` / `.with(n)`.
- Query conflicts only show as a runtime panic (B0001) naming no system; split queries with `Without<>` or merge them.
- The portal cameras need the same `Exposure` as the player's camera, or interiors seen through a portal render almost black.
- The player's collider is a flat-bottomed chamfered cylinder on purpose: a capsule rides up over a rune lying on the floor.
- The player must not rest on the floor. A hull lying on a floor mesh gets a tilted contact normal now and then and is thrown upward mid-walk. `player::walk` holds the feet on the floor with ground rays, and the hull ends 2.5 cm above them (not scaled down for a small player: at 1.25 cm the bumps come back).
- Every camera needs `portal::render::view_cluster_config()` (a constant cluster depth). Bevy's default takes the depth from a GPU readback a few frames old, so a camera that jumps (a pooled view camera given another view, the player's camera after a portal) loses its point lights for about three frames: portals flash dark.

## Portal rules

- Portals must be root entities (their `Transform` is the world transform). Scale must be uniform, except the sign of X (mirroring).
- A portal's surface lies in its local XY plane, centred on the entity; its front faces local -Z. Entering the front exits through the target's back. Link both sides with `Portal { front_link, back_link }`.
- Rooms that share an apparent location are built far apart and given a `PortalSpace(n)`. Space `n` is render layer `1 + n`; a view renders layer 0 and its own space. `PlayerSpace` follows the portals the player walks through; set it when you teleport the player.
- Objects that cross portals are a root with a positive uniform scale and the rigid body, plus a `PortalObject { visual }` child that carries the mirror flip (negative X scale) and the meshes. Use `ClipMaterial` so they are cut at the portal.
