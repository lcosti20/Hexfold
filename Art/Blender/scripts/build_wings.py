"""Build the greybox wing rooms in the open Blender file (Wings.blend).

Run inside Blender (Text Editor > Run Script, or exec() over the Blender MCP).
It replaces everything in the Wings collection.

COORDINATES IN THIS SCRIPT ARE UNREAL'S (x, y, z in metres, +Y into the room).
Each mesh is mirrored in Y at the end, so after FBX import (Blender -Y -> Unreal +Y)
it comes out exactly as written here.

Every room is a solid shell (walls, floor and ceiling WALL_T thick, so it shadows
properly) with its origin at the floor centre of its entrance door, the door in the
-Y wall at y = 0. Each door is an opening cut into a wall plus a recess 1 m deep
behind the portal plane (the portal sits on the wall's inner face), closed at the
back, so the player's capsule reaches the portal before it hits anything.

Door openings are for SM_Obs_Doorframe at a given portal scale s: the opening is
(1.96 s) wide and (1.98 s) tall (the frame overlaps the portal by 2 cm x s).

  SM_Wing_Chasm   rune 1 (Through the Glass). 12 x 26 m gallery, 14 m tall (z -6..8).
                  Near floor y 0..10 and far ledge y 20..26 at z 0; a 10 m pit between
                  them, 6 m deep, with a railing along y = 10. Entrance for scale 2.
  SM_Wing_Study   rune 2 hub. 12 x 12 x 5 m. Entrance (scale 2) in -Y; three cabinet
                  doors (scale 1) in the +Y wall at x = -4, 0, 4.
  SM_Wing_Closet  2 x ... small storeroom 4 x 3 x 3 m with shelves; entrance scale 1.
  SM_Wing_Vault   8 x 8 x 4 m; entrance scale 1.
  SM_Wing_Push    rune 3. 10 x 16 x 6 m hall. Entrance scale 2; a scale-2 door in the +Y wall;
                  a scale-1 door in the -X wall at y = 4.
  SM_Wing_Mouse   rune 4. 10 x 12 x 5 m. Entrance scale 2; a scale-0.5 mouse door in the +Y
                  wall at x = -3; a scale-1 door in the +X wall at y = 6.
  SM_Wing_Mirror  rune 5. 10 x 20 x 6 m hall, symmetric about x = 0 and y = 10. Entrance
                  scale 2 (-Y), a matching blind doorway (+Y), and scale-0.5 window openings
                  with a 0.9 m sill in both side walls at y = 10.
  SM_Wing_Telescope rune 6. 24 x 18 x 8 m. Entrance scale 2 (-Y); the lens doorway (+Y),
                  opening sized for scale 2.

Material slots: Floor, Wall, Trim, Dark. No UVs (use world-aligned materials).
No UCX: in Unreal, use complex collision as simple.
"""
import bpy, bmesh, math
from mathutils import Matrix, Vector

WALL_T = 0.4
RECESS = 1.0
FLOOR, WALL, TRIM, DARK = range(4)


def _clear(name):
    coll = bpy.data.collections.get(name)
    if coll:
        for o in list(coll.objects):
            me = o.data
            bpy.data.objects.remove(o, do_unlink=True)
            if me and me.users == 0:
                bpy.data.meshes.remove(me)
    else:
        coll = bpy.data.collections.new(name)
        bpy.context.scene.collection.children.link(coll)
    return coll


def _mats():
    out = []
    for n, rgb in (("M_Wing_Floor", (0.35, 0.33, 0.3)), ("M_Wing_Wall", (0.62, 0.58, 0.52)),
                   ("M_Wing_Trim", (0.5, 0.47, 0.42)), ("M_Wing_Dark", (0.04, 0.04, 0.05))):
        m = bpy.data.materials.get(n) or bpy.data.materials.new(n)
        m.diffuse_color = (*rgb, 1)
        out.append(m)
    return out


def _box(bm, lo, hi, mat=WALL):
    r = bmesh.ops.create_cube(bm, size=1.0)
    lo, hi = Vector(lo), Vector(hi)
    bmesh.ops.transform(bm, matrix=Matrix.Translation((hi + lo) / 2) @ Matrix.Diagonal((*(hi - lo), 1)),
                        verts=r["verts"])
    for f in {f for v in r["verts"] for f in v.link_faces}:
        f.material_index = mat


def _obj(name, bm, coll, mats):
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    for m in mats:
        me.materials.append(m)
    ob = bpy.data.objects.new(name, me)
    coll.objects.link(ob)
    return ob


def _boolean(target, bm, op, coll, mats):
    other = _obj("_bool", bm, coll, mats)
    m = target.modifiers.new(op, 'BOOLEAN')
    m.operation, m.solver, m.object = op, 'EXACT', other
    with bpy.context.temp_override(object=target, active_object=target):
        bpy.ops.object.modifier_apply(modifier=m.name)
    me = other.data
    bpy.data.objects.remove(other, do_unlink=True)
    bpy.data.meshes.remove(me)


def _door_boxes(door, inner):
    """door = (wall, centre, scale[, sill]); returns (recess cut box, vestibule block box).
    sill raises the opening above the door floor (for windows)."""
    wall, c, s = door[:3]
    sill = door[3] if len(door) > 3 else 0.0
    hw, top = 0.98 * s, 1.98 * s
    x0, x1, y0, y1, z0, z1 = inner
    z0 += sill
    if wall == "-y":
        cut = ((c - hw, y0 - RECESS, z0), (c + hw, y0 + 0.5, z0 + top))
        blk = ((c - hw - WALL_T, y0 - RECESS - WALL_T, z0 - WALL_T), (c + hw + WALL_T, y0, z0 + top + WALL_T))
    elif wall == "+y":
        cut = ((c - hw, y1 - 0.5, z0), (c + hw, y1 + RECESS, z0 + top))
        blk = ((c - hw - WALL_T, y1, z0 - WALL_T), (c + hw + WALL_T, y1 + RECESS + WALL_T, z0 + top + WALL_T))
    elif wall == "-x":
        cut = ((x0 - RECESS, c - hw, z0), (x0 + 0.5, c + hw, z0 + top))
        blk = ((x0 - RECESS - WALL_T, c - hw - WALL_T, z0 - WALL_T), (x0, c + hw + WALL_T, z0 + top + WALL_T))
    else:
        cut = ((x1 - 0.5, c - hw, z0), (x1 + RECESS, c + hw, z0 + top))
        blk = ((x1, c - hw - WALL_T, z0 - WALL_T), (x1 + RECESS + WALL_T, c + hw + WALL_T, z0 + top + WALL_T))
    return cut, blk


def room(name, coll, mats, inner, doors, door_floor=0.0, extras=()):
    """inner = (x0, x1, y0, y1, z0, z1). doors = [(wall, centre, portal scale)].
    door_floor: z of the door sill (the room floor may be lower, e.g. the chasm).
    extras = [(lo, hi, mat)] solid boxes added inside (floors, ledges, railings, shelves)."""
    x0, x1, y0, y1, z0, z1 = inner
    t = WALL_T
    bm = bmesh.new()
    _box(bm, (x0 - t, y0 - t, z0 - t), (x1 + t, y1 + t, z1 + t), WALL)
    shell = _obj(name, bm, coll, mats)
    door_inner = (x0, x1, y0, y1, door_floor, z1)
    for d in doors:
        cut, blk = _door_boxes(d, door_inner)
        b = bmesh.new()
        _box(b, *blk, WALL)
        _boolean(shell, b, 'UNION', coll, mats)
    b = bmesh.new()
    _box(b, (x0, y0, z0), (x1, y1, z1), WALL)
    _boolean(shell, b, 'DIFFERENCE', coll, mats)
    for d in doors:
        cut, blk = _door_boxes(d, door_inner)
        b = bmesh.new()
        _box(b, *cut, WALL)
        _boolean(shell, b, 'DIFFERENCE', coll, mats)
    for lo, hi, mat in extras:              # one at a time: a multi-box operand loses pieces
        b = bmesh.new()
        _box(b, lo, hi, mat)
        _boolean(shell, b, 'UNION', coll, mats)

    # materials by face direction: up-facing at floor levels = floor, recess backs = dark
    for p in shell.data.polygons:
        if p.normal.z > 0.9:
            p.material_index = FLOOR
        elif p.normal.z < -0.9:
            p.material_index = WALL
    for d in doors:
        cut, _ = _door_boxes(d, door_inner)
        (cx0, cy0, cz0), (cx1, cy1, cz1) = cut
        for p in shell.data.polygons:
            c = p.center
            if d[0] == "-y" and abs(c.y - cy0) < 1e-3 and p.normal.y > 0.9:
                p.material_index = DARK
            if d[0] == "+y" and abs(c.y - cy1) < 1e-3 and p.normal.y < -0.9:
                p.material_index = DARK

    # mirror Y so it comes out right in Unreal (Blender -Y -> Unreal +Y)
    bm = bmesh.new()
    bm.from_mesh(shell.data)
    bmesh.ops.transform(bm, matrix=Matrix.Diagonal((1, -1, 1, 1)), verts=bm.verts)
    bmesh.ops.reverse_faces(bm, faces=bm.faces)
    bm.to_mesh(shell.data)
    bm.free()
    return shell


def build_all():
    coll = _clear("Wings")
    mats = _mats()
    rooms = []

    rail = []
    for x in (-5.8, -3.5, -1.2, 1.2, 3.5, 5.8):
        rail.append(((x - 0.06, 9.82, -0.05), (x + 0.06, 9.98, 1.15), TRIM))   # posts deeper than the bar (no coplanar faces)
    rail.append(((-6.0, 9.85, 1.0), (6.0, 9.95, 1.1), TRIM))
    rooms.append(room("SM_Wing_Chasm", coll, mats, (-6, 6, 0, 26, -6, 8), [("-y", 0.0, 2.0)], 0.0,
                      [((-6, 0, -6), (6, 10, 0), FLOOR), ((-6, 20, -6), (6, 26, 0), FLOOR)] + rail))
    rooms.append(room("SM_Wing_Study", coll, mats, (-6, 6, 0, 12, 0, 5),
                      [("-y", 0.0, 2.0), ("+y", -4.0, 1.0), ("+y", 0.0, 1.0), ("+y", 4.0, 1.0)]))
    shelves = [((-2.0, 1.0, z), (-1.6, 2.8, z + 0.05), TRIM) for z in (0.6, 1.2, 1.8)]
    shelves += [((1.6, 1.0, z), (2.0, 2.8, z + 0.05), TRIM) for z in (0.6, 1.2, 1.8)]
    rooms.append(room("SM_Wing_Closet", coll, mats, (-2, 2, 0, 3, 0, 3), [("-y", 0.0, 1.0)], 0.0, shelves))
    rooms.append(room("SM_Wing_Vault", coll, mats, (-4, 4, 0, 8, 0, 4), [("-y", 0.0, 1.0)]))
    # rune 3: push the 2x rune into the big portal (+Y wall, scale 2); it comes out of the
    # small door (-X wall at y = 4, scale 1) at 1x
    rooms.append(room("SM_Wing_Push", coll, mats, (-5, 5, 0, 16, 0, 6),
                      [("-y", 0.0, 2.0), ("+y", 0.0, 2.0), ("-x", 4.0, 1.0)]))
    # rune 4: mouse door (+Y wall at x = -3, scale 0.5) linked to a normal door (+X wall at y = 6, scale 1)
    rooms.append(room("SM_Wing_Mouse", coll, mats, (-5, 5, 0, 12, 0, 5),
                      [("-y", 0.0, 2.0), ("+y", -3.0, 0.5), ("+x", 6.0, 1.0)]))
    # rune 5: hall of reflections, symmetric about x = 0 (entrance mirror) and about y = 10
    # (the window mirrors). Entrance -Y and a matching blind doorway +Y; mirror windows
    # (scale 0.5, sill 0.9 m) in the -X and +X walls at y = 10.
    rooms.append(room("SM_Wing_Mirror", coll, mats, (-5, 5, 0, 20, 0, 6),
                      [("-y", 0.0, 2.0), ("+y", 0.0, 2.0), ("-x", 10.0, 0.5, 0.9), ("+x", 10.0, 0.5, 0.9)]))
    # rune 6: the telescope room. Entrance -Y; the lens doorway in +Y, sized for the lens at
    # scale 2 (the lens portal is rescaled at runtime, standing on the floor)
    rooms.append(room("SM_Wing_Telescope", coll, mats, (-12, 12, 0, 18, 0, 8),
                      [("-y", 0.0, 2.0), ("+y", 0.0, 2.0)]))

    for i, r in enumerate(rooms):
        r.data.shade_smooth()
        r.data.set_sharp_from_angle(angle=math.radians(30))
        r.location = (i * 30.0, 0, 0)          # preview only; exported at the origin
    return [(r.name, tuple(round(v, 2) for v in r.dimensions), len(r.data.polygons)) for r in rooms]


if __name__ == "__main__":
    for row in build_all():
        print(row)
