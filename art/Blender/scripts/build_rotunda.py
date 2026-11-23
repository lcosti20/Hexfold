"""Build the greybox Rotunda (the Observatory's inside) in the open Blender file.

Run inside Blender (Text Editor > Run Script, or exec() over the Blender MCP).
It replaces everything in the Rotunda collection.

SM_Obs_Rotunda
    A round hall 40 m across: floor, walls 10 m high and 1.2 m thick, cornice and a
    hemispherical dome (top at 30 m), seen from the inside. Origin = floor centre.
    Eight openings at 45 degree steps around the wall:
      k = 0  the entrance, on the -Y side (same side as the exterior porch, so both
             entrance portals can share a rotation). A (1.96 x 1.98 m) x DOOR_SCALE recess, 1 m deep
             behind the portal plane, like the exterior's.
      k = 4  a niche opposite the entrance (for the star map / hex focus later).
      others six wing arches, 4 x 6 m, closed 3 m deep for now (placeholders until the
             wings are built).
    A two-step dais in the centre for the telescope.
    Material slots: Floor, Wall, Trim, Dark. No UVs (use world-aligned materials).
    No UCX: in Unreal, set Collision Complexity to "Use Complex Collision As Simple".

DOOR_SOCKET is where SM_Obs_Doorframe and the inside entrance portal go (Blender coords;
in Unreal that's (0, +2000, 150) cm at scale DOOR_SCALE, because Blender -Y becomes Unreal +Y).

Units: metres.
"""
import bpy, bmesh, math
from mathutils import Matrix, Vector

R_IN, WALL_T, WALL_H = 20.0, 1.2, 10.0
R_OUT = R_IN + WALL_T
FLOOR_T = 0.3
OPEN_HW, OPEN_TOP = 0.98, 0.98
RECESS_BEHIND = 1.0
ARCH_W, ARCH_H, ALCOVE_D = 4.0, 6.0, 3.0
NICHE_D = 1.5
DOOR_SCALE = 1.5                     # entrance portal + doorframe scale (3 x 3 m door), as in build_observatory.py
DOOR_SOCKET = Vector((0.0, -R_IN, 1.0 * DOOR_SCALE))
SEG = 96


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


def _mat(name, rgb):
    m = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    m.diffuse_color = (*rgb, 1)
    return m


def _tag(verts, mat):
    for f in {f for v in verts for f in v.link_faces}:
        f.material_index = mat


def _box(bm, lo, hi, mat=0, matrix=None):
    r = bmesh.ops.create_cube(bm, size=1.0)
    lo, hi = Vector(lo), Vector(hi)
    m = Matrix.Translation((hi + lo) / 2) @ Matrix.Diagonal((*(hi - lo), 1))
    if matrix is not None:
        m = matrix @ m
    bmesh.ops.transform(bm, matrix=m, verts=r["verts"])
    _tag(r["verts"], mat)
    return r["verts"]


def _cyl(bm, r, z0, z1, mat=0, seg=SEG):
    res = bmesh.ops.create_cone(bm, cap_ends=True, segments=seg, radius1=r, radius2=r, depth=z1 - z0)
    bmesh.ops.translate(bm, vec=(0, 0, (z0 + z1) / 2), verts=res["verts"])
    _tag(res["verts"], mat)
    return res["verts"]


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


def _boolean(target, other, op):
    m = target.modifiers.new(op, 'BOOLEAN')
    m.operation, m.solver, m.object = op, 'EXACT', other
    with bpy.context.temp_override(object=target, active_object=target):
        bpy.ops.object.modifier_apply(modifier=m.name)
    me = other.data
    bpy.data.objects.remove(other, do_unlink=True)
    bpy.data.meshes.remove(me)


def _radial(k):
    """Matrix that turns local -Y (the entrance direction) to opening k (45 degree steps)."""
    return Matrix.Rotation(math.radians(45 * k), 4, 'Z')


def build():
    coll = _clear("Rotunda")
    mats = [_mat("M_Rot_Floor", (0.35, 0.33, 0.3)), _mat("M_Rot_Wall", (0.62, 0.58, 0.52)),
            _mat("M_Rot_Trim", (0.5, 0.47, 0.42)), _mat("M_Rot_Dark", (0.04, 0.04, 0.05))]
    FLOOR, WALL, TRIM, DARK = range(4)

    # --- solid walls + alcove blocks, then cut the openings ---
    bm = bmesh.new()
    _cyl(bm, R_OUT, 0.0, WALL_H, WALL)
    walls = _obj("_walls", bm, coll, mats)
    # alcove blocks and the entrance vestibule, unioned onto the ring one by one
    for k in range(1, 8):
        depth = NICHE_D if k == 4 else ALCOVE_D
        b = bmesh.new()
        _box(b, (-ARCH_W / 2 - 0.5, -(R_OUT + depth + 0.3), 0.0), (ARCH_W / 2 + 0.5, -(R_IN + 0.5), ARCH_H + 0.5),
             WALL, _radial(k))
        _boolean(walls, _obj(f"_alcove{k}", b, coll, mats), 'UNION')
    b = bmesh.new()
    _box(b, (-2.2, -(R_OUT + 0.4), 0.0), (2.2, -(R_IN + 0.5), 3.9), WALL)
    _boolean(walls, _obj("_vestibule", b, coll, mats), 'UNION')

    # inner room cylinder (hollows the ring)
    cut = bmesh.new()
    _cyl(cut, R_IN, -0.1, WALL_H + 0.1, WALL)
    _boolean(walls, _obj("_room", cut, coll, mats), 'DIFFERENCE')

    # openings
    cut = bmesh.new()
    s = DOOR_SCALE
    _box(cut, (-OPEN_HW * s, -(R_IN + RECESS_BEHIND), 0.0), (OPEN_HW * s, -(R_IN - 1.0), DOOR_SOCKET.z + OPEN_TOP * s))
    for k in range(1, 8):
        depth = NICHE_D if k == 4 else ALCOVE_D
        _box(cut, (-ARCH_W / 2, -(R_OUT + depth), 0.0), (ARCH_W / 2, -(R_IN - 1.0), ARCH_H), WALL, _radial(k))
    _boolean(walls, _obj("_openings", cut, coll, mats), 'DIFFERENCE')

    # dark backs: the entrance recess back wall and the alcove ends
    for p in walls.data.polygons:
        c = Vector(p.center)
        r = math.hypot(c.x, c.y)
        if r > R_OUT - 0.05 and c.z < ARCH_H and Vector((c.x, c.y, 0)).normalized().dot(p.normal) < -0.9:
            p.material_index = DARK
        if abs(c.y + R_IN + RECESS_BEHIND) < 1e-3 and abs(c.x) < OPEN_HW * DOOR_SCALE and p.normal.y > 0.9:
            p.material_index = DARK

    # --- floor, cornice, dome, dais ---
    bm = bmesh.new()
    bm.from_mesh(walls.data)
    me = walls.data
    bpy.data.objects.remove(walls, do_unlink=True)
    bpy.data.meshes.remove(me)

    _cyl(bm, R_OUT + ALCOVE_D + 1.0, -FLOOR_T, 0.0, FLOOR)                    # floor (covers alcoves)
    ring = bmesh.ops.create_cone(bm, cap_ends=False, segments=SEG, radius1=R_IN - 0.35, radius2=R_IN - 0.35, depth=0.4)
    bmesh.ops.translate(bm, vec=(0, 0, WALL_H - 0.2), verts=ring["verts"])   # cornice band
    _tag(ring["verts"], TRIM)

    dome = bmesh.ops.create_uvsphere(bm, u_segments=SEG, v_segments=48, radius=R_IN)
    dv = dome["verts"]
    bmesh.ops.delete(bm, geom=[v for v in dv if v.co.z < -1e-4], context='VERTS')
    dv = [v for v in dv if v.is_valid]
    bmesh.ops.translate(bm, vec=(0, 0, WALL_H), verts=dv)
    dome_faces = list({f for v in dv for f in v.link_faces})
    for f in dome_faces:
        f.material_index = WALL
    bmesh.ops.reverse_faces(bm, faces=dome_faces)                            # seen from inside

    _cyl(bm, 4.0, 0.0, 0.4, TRIM)                                            # dais
    _cyl(bm, 3.0, 0.4, 0.8, TRIM)

    me = bpy.data.meshes.new("SM_Obs_Rotunda")
    bm.to_mesh(me)
    bm.free()
    for m in mats:
        me.materials.append(m)
    rot = bpy.data.objects.new("SM_Obs_Rotunda", me)
    coll.objects.link(rot)
    me.shade_smooth()
    me.set_sharp_from_angle(angle=math.radians(30))
    return rot


def build_all():
    rot = build()
    return [(rot.name, tuple(round(x, 2) for x in rot.dimensions), len(rot.data.polygons))]


if __name__ == "__main__":
    for row in build_all():
        print(row)
