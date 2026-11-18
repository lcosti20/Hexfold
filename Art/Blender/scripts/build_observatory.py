"""Build the Observatory exterior and the shared portal doorframe in the open Blender file.

Run inside Blender (Text Editor > Run Script, or exec() over the Blender MCP).
It replaces everything in the Doorframe and Exterior collections.

SM_Obs_Doorframe
    Stone frame around a portal. Origin = centre of the 2 x 2 m portal, which is how the
    PortalWorlds default surface is centred, so frame and portal share one transform.
    The opening is 1.96 x 1.98 m: the frame overlaps the portal's sides and top by 2 cm,
    so no gap shows at the portal edge. Left-right symmetric (safe next to mirroring portals).
    Front faces -Y, like the portal. UCX boxes: two jambs and the lintel.

SM_Obs_Exterior
    The small domed observatory seen from outside: drum 8 m across and 4 m high, cornice,
    copper dome with a shutter slit and a telescope poking out, porch and a stone plinth.
    Origin = centre of the drum at door-floor level. The porch faces -Y.
    The door recess is 1 m deep behind the portal plane, so the player's capsule reaches
    the portal before it reaches the back wall.
    Place SM_Obs_Doorframe (and the entrance portal) at DOOR_SOCKET.
    No UCX: in Unreal, set Collision Complexity to "Use Complex Collision As Simple".

Units: metres (1 Blender unit = 1 m = 100 Unreal units).
"""
import bpy, bmesh, math
from mathutils import Matrix, Vector

# ---- dimensions ---------------------------------------------------------------------------
OPEN_HW, OPEN_TOP = 0.98, 0.98          # opening half width, top (relative to portal centre)
FRAME_HW, FRAME_TOP, FRAME_D = 1.2, 1.25, 0.2

DRUM_R, DRUM_H = 4.0, 4.0
PORCH_HW, PORCH_FRONT, PORCH_BACK, PORCH_H = 1.6, -5.0, -3.5, 3.0
RECESS_DEPTH = 1.0
DOOR_SOCKET = Vector((0.0, PORCH_FRONT, 1.0))   # where the doorframe/portal origin goes

SEG = 64


def _mat(name, rgb, metallic=0.0, rough=0.7):
    m = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    m.use_nodes = True
    bsdf = next(n for n in m.node_tree.nodes if n.type == "BSDF_PRINCIPLED")
    bsdf.inputs["Base Color"].default_value = (*rgb, 1)
    bsdf.inputs["Metallic"].default_value = metallic
    bsdf.inputs["Roughness"].default_value = rough
    m.diffuse_color = (*rgb, 1)
    return m


def _clear(coll_name):
    coll = bpy.data.collections.get(coll_name)
    if coll:
        for o in list(coll.objects):
            me = o.data
            bpy.data.objects.remove(o, do_unlink=True)
            if me and me.users == 0:
                bpy.data.meshes.remove(me)
    else:
        coll = bpy.data.collections.new(coll_name)
        bpy.context.scene.collection.children.link(coll)
    return coll


def _box(bm, lo, hi, mat_index=0):
    r = bmesh.ops.create_cube(bm, size=1.0)
    lo, hi = Vector(lo), Vector(hi)
    size, ctr = hi - lo, (hi + lo) / 2
    bmesh.ops.transform(bm, matrix=Matrix.Translation(ctr) @ Matrix.Diagonal((*size, 1)), verts=r["verts"])
    for f in {f for v in r["verts"] for f in v.link_faces}:
        f.material_index = mat_index
    return r["verts"]


def _cyl(bm, r, z0, z1, mat_index=0, seg=SEG, r_top=None):
    res = bmesh.ops.create_cone(bm, cap_ends=True, segments=seg, radius1=r,
                                radius2=r if r_top is None else r_top, depth=z1 - z0)
    bmesh.ops.translate(bm, vec=(0, 0, (z0 + z1) / 2), verts=res["verts"])
    for f in {f for v in res["verts"] for f in v.link_faces}:
        f.material_index = mat_index
    return res["verts"]


def _obj(name, bm, coll, mats=()):
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    for m in mats:
        me.materials.append(m)
    ob = bpy.data.objects.new(name, me)
    coll.objects.link(ob)
    return ob


def _difference(target, cutter_bm, coll):
    cutter = _obj("_cutter", cutter_bm, coll)
    m = target.modifiers.new("cut", 'BOOLEAN')
    m.operation, m.solver, m.object = 'DIFFERENCE', 'EXACT', cutter
    with bpy.context.temp_override(object=target, active_object=target):
        bpy.ops.object.modifier_apply(modifier=m.name)
    me = cutter.data
    bpy.data.objects.remove(cutter, do_unlink=True)
    bpy.data.meshes.remove(me)


# ---- doorframe ----------------------------------------------------------------------------
def build_doorframe():
    coll = _clear("Doorframe")
    trim = _mat("M_Obs_Trim", (0.55, 0.52, 0.47))
    bm = bmesh.new()
    b = -1.0                                     # floor (portal bottom edge)
    d = FRAME_D
    _box(bm, (-FRAME_HW, -d, b), (-OPEN_HW, d, FRAME_TOP))            # left jamb
    _box(bm, (OPEN_HW, -d, b), (FRAME_HW, d, FRAME_TOP))              # right jamb
    _box(bm, (-OPEN_HW, -d, OPEN_TOP), (OPEN_HW, d, FRAME_TOP))       # lintel
    _box(bm, (-FRAME_HW - 0.05, -d - 0.04, b), (-OPEN_HW, d + 0.04, b + 0.25))   # jamb bases
    _box(bm, (OPEN_HW, -d - 0.04, b), (FRAME_HW + 0.05, d + 0.04, b + 0.25))
    _box(bm, (-FRAME_HW - 0.06, -d - 0.04, FRAME_TOP - 0.08), (FRAME_HW + 0.06, d + 0.04, FRAME_TOP + 0.06))  # cap
    _box(bm, (-0.16, -d - 0.05, OPEN_TOP - 0.06), (0.16, d + 0.05, FRAME_TOP + 0.14))  # keystone
    bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=1e-5)
    frame = _obj("SM_Obs_Doorframe", bm, coll, [trim])

    for k, (lo, hi) in enumerate([((-FRAME_HW, -d, b), (-OPEN_HW, d, FRAME_TOP)),
                                  ((OPEN_HW, -d, b), (FRAME_HW, d, FRAME_TOP)),
                                  ((-OPEN_HW, -d, OPEN_TOP), (OPEN_HW, d, FRAME_TOP))]):
        cb = bmesh.new()
        _box(cb, lo, hi)
        col = _obj(f"UCX_SM_Obs_Doorframe_{k:02d}", cb, coll)
        col.parent = frame
        col.display_type = 'WIRE'
        col.hide_render = True
    return frame


# ---- exterior -----------------------------------------------------------------------------
def build_exterior():
    coll = _clear("Exterior")
    stone = _mat("M_Obs_Stone", (0.62, 0.58, 0.52))
    trim = _mat("M_Obs_Trim", (0.55, 0.52, 0.47))
    copper = _mat("M_Obs_Copper", (0.25, 0.45, 0.38), metallic=0.6, rough=0.45)
    dark = _mat("M_Obs_Dark", (0.03, 0.03, 0.04), rough=0.9)
    STONE, TRIM, COPPER, DARK = range(4)

    bm = bmesh.new()
    _cyl(bm, 4.7, -0.4, 0.0, TRIM)                                   # plinth
    _cyl(bm, DRUM_R, 0.0, DRUM_H, STONE)                             # drum
    _cyl(bm, DRUM_R + 0.1, 0.0, 0.35, TRIM)                          # base course
    _cyl(bm, DRUM_R + 0.25, DRUM_H, DRUM_H + 0.25, TRIM)            # cornice

    # dome: upper hemisphere
    dome_z, dome_r = DRUM_H + 0.25, DRUM_R - 0.05
    res = bmesh.ops.create_uvsphere(bm, u_segments=SEG, v_segments=32, radius=dome_r)
    dv = res["verts"]
    bmesh.ops.delete(bm, geom=[v for v in dv if v.co.z < -1e-4], context='VERTS')
    dv = [v for v in dv if v.is_valid]
    bmesh.ops.translate(bm, vec=(0, 0, dome_z), verts=dv)
    for f in {f for v in dv for f in v.link_faces}:
        f.material_index = COPPER
    rim = [e for e in bm.edges if e.is_boundary and all(abs(v.co.z - dome_z) < 1e-3 for v in e.verts)]
    bmesh.ops.holes_fill(bm, edges=rim)

    # shutter slit: a dark strip with copper ribs either side, along the front meridian
    def meridian_strip(half_w, lift, mat, a0=math.radians(18), a1=math.radians(95), steps=24):
        verts = []
        for i in range(steps + 1):
            a = a0 + (a1 - a0) * i / steps                 # angle from the front horizon over the top
            r = dome_r + lift
            y, z = -r * math.cos(a), dome_z + r * math.sin(a)
            verts.append((bm.verts.new((-half_w, y, z)), bm.verts.new((half_w, y, z))))
        for (l0, r0), (l1, r1) in zip(verts, verts[1:]):
            f = bm.faces.new((l0, r0, r1, l1))
            f.material_index = mat
    meridian_strip(0.45, 0.01, DARK)
    for side in (-1, 1):                                     # ribs as thin boxes swept along the arc
        for i in range(24):
            a = math.radians(18) + (math.radians(95) - math.radians(18)) * (i + 0.5) / 24
            r = dome_r + 0.04
            c = Vector((side * 0.52, -r * math.cos(a), dome_z + r * math.sin(a)))
            seg_len = r * (math.radians(95) - math.radians(18)) / 24 + 0.02
            vs = _box(bm, (-0.06, -seg_len / 2, -0.05), (0.06, seg_len / 2, 0.05), COPPER)
            rot = Matrix.Rotation(a - math.pi / 2, 4, 'X')
            bmesh.ops.transform(bm, matrix=Matrix.Translation(c) @ rot.inverted(), verts=vs)

    # telescope tube poking out of the slit
    tube = bmesh.new()
    _cyl(tube, 0.32, -1.6, 1.4, TRIM, seg=32)
    _cyl(tube, 0.38, 1.2, 1.5, COPPER, seg=32)
    tv = list(tube.verts)
    bmesh.ops.transform(tube, matrix=Matrix.Translation((0, -2.3, dome_z + 2.6))
                        @ Matrix.Rotation(math.radians(55), 4, 'X'), verts=tv)
    me_tmp = bpy.data.meshes.new("_tube")
    tube.to_mesh(me_tmp)
    tube.free()
    bm.from_mesh(me_tmp)
    bpy.data.meshes.remove(me_tmp)

    # porch (boolean-cut below) with a gabled roof
    porch = bmesh.new()
    _box(porch, (-PORCH_HW, PORCH_FRONT, 0.0), (PORCH_HW, PORCH_BACK, PORCH_H), STONE)
    _box(porch, (-PORCH_HW - 0.12, PORCH_FRONT - 0.12, PORCH_H), (PORCH_HW + 0.12, PORCH_BACK, PORCH_H + 0.18), TRIM)
    gable = [porch.verts.new(p) for p in [(-PORCH_HW - 0.12, PORCH_FRONT - 0.12, PORCH_H + 0.18),
                                          (PORCH_HW + 0.12, PORCH_FRONT - 0.12, PORCH_H + 0.18),
                                          (0, PORCH_FRONT - 0.12, PORCH_H + 0.9)]]
    back = [porch.verts.new((v.co.x, PORCH_BACK, v.co.z)) for v in gable]
    for f in (porch.faces.new(gable), porch.faces.new(list(reversed(back))),
              porch.faces.new((gable[0], back[0], back[1], gable[1])),
              porch.faces.new((gable[1], back[1], back[2], gable[2])),
              porch.faces.new((gable[2], back[2], back[0], gable[0]))):
        f.material_index = TRIM
    porch_ob = _obj("_porch", porch, coll, [stone, trim, copper, dark])
    cut = bmesh.new()
    _box(cut, (-OPEN_HW, PORCH_FRONT - 0.5, -0.01), (OPEN_HW, PORCH_FRONT + RECESS_DEPTH, DOOR_SOCKET.z + OPEN_TOP))
    _difference(porch_ob, cut, coll)
    # dark back wall of the recess (only visible if the portal is off)
    for f in porch_ob.data.polygons:
        if abs(f.center.y - (PORCH_FRONT + RECESS_DEPTH)) < 1e-3 and f.normal.y < -0.9:
            f.material_index = DARK

    bm.from_mesh(porch_ob.data)
    me = porch_ob.data
    bpy.data.objects.remove(porch_ob, do_unlink=True)
    bpy.data.meshes.remove(me)

    ext = _obj("SM_Obs_Exterior", bm, coll, [stone, trim, copper, dark])
    # smooth curved surfaces, keep hard edges (exported as FBX smoothing groups)
    ext.data.shade_smooth()
    ext.data.set_sharp_from_angle(angle=math.radians(30))
    return ext


def build_all():
    frame = build_doorframe()
    ext = build_exterior()
    frame.location = DOOR_SOCKET            # preview only; exported at the origin
    return [(o.name, tuple(round(x, 2) for x in o.dimensions), len(o.data.polygons)) for o in (frame, ext)]


if __name__ == "__main__":
    for row in build_all():
        print(row)
