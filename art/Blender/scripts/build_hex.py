"""Build the hex seal and pedestal meshes in the open Blender file (Hex.blend).

Run inside Blender (Text Editor > Run Script, or exec() over the Blender MCP).
It replaces everything in the Hex collection.

SM_Hex_Ring         The hex web over the inside door, in the XZ plane, front -Y (+Y in Unreal).
                    Origin at the hex centre (= the portal centre). A pointy-top hexagon with
                    corner radius RING_R (corners at 90, 150, 210, 270, 330, 30 degrees; the
                    BP_HexSocket actors sit on them), spokes from the centre to each corner, and
                    spokes from each corner out to the edge of the 3 x 3 m doorway (DOOR_HALF).
SM_Hex_SocketPlate  Hexagonal backing plate for one socket (corner radius 0.32 m, 5 cm thick)
                    with a raised rim on the front. Origin at the plate centre, back face at y=+0.025.
SM_Hex_Pedestal     Column for a rune: base, shaft and top plate, 1.0 m tall. Origin at the
                    floor centre. UCX: one box around the whole column.

Units: metres.
"""
import bpy, bmesh, math
from mathutils import Matrix, Vector

RING_R = 1.0
DOOR_HALF = 1.47          # half width/height of the 3 x 3 m doorway opening
BAR_W, BAR_D = 0.08, 0.06
PLATE_R, PLATE_T, RIM_W = 0.32, 0.05, 0.035


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


def _prism_xz(bm, pts, y0, y1):
    """Convex polygon in XZ extruded from y0 to y1."""
    a = [bm.verts.new((x, y0, z)) for x, z in pts]
    b = [bm.verts.new((x, y1, z)) for x, z in pts]
    bm.faces.new(a)
    bm.faces.new(list(reversed(b)))
    n = len(pts)
    for i in range(n):
        j = (i + 1) % n
        bm.faces.new((a[i], a[j], b[j], b[i]))


def _bar(bm, p, q, w, y0, y1):
    p, q = Vector(p), Vector(q)
    d = (q - p).normalized()
    n = Vector((-d.y, d.x)) * w / 2
    ext = d * w / 2
    pts = [tuple(p - ext - n), tuple(q + ext - n), tuple(q + ext + n), tuple(p - ext + n)]
    _prism_xz(bm, pts, y0, y1)


def _hex_pts(r, start_deg=0.0):
    return [(r * math.cos(math.radians(start_deg + 60 * k)), r * math.sin(math.radians(start_deg + 60 * k))) for k in range(6)]


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


def _cyl(bm, r, z0, z1, seg=32):
    res = bmesh.ops.create_cone(bm, cap_ends=True, segments=seg, radius1=r, radius2=r, depth=z1 - z0)
    bmesh.ops.translate(bm, vec=(0, 0, (z0 + z1) / 2), verts=res["verts"])


def build_all():
    coll = _clear("Hex")
    hexmat = _mat("M_Hex", (0.45, 0.2, 0.75))
    stone = _mat("M_Hex_Stone", (0.55, 0.52, 0.47))

    # web: hexagon bars, centre spokes, and outer spokes to the doorway edge
    bm = bmesh.new()
    corners = _hex_pts(RING_R, 30)
    for k in range(6):
        c = Vector(corners[k])
        _bar(bm, corners[k], corners[(k + 1) % 6], BAR_W, -BAR_D / 2, BAR_D / 2)
        _bar(bm, (0, 0), corners[k], BAR_W * 0.5, -BAR_D / 4, BAR_D / 4)
        d = c.normalized()
        t = min(DOOR_HALF / abs(d.x) if abs(d.x) > 1e-6 else 1e9, DOOR_HALF / abs(d.y) if abs(d.y) > 1e-6 else 1e9)
        _bar(bm, corners[k], tuple(d * t), BAR_W * 0.6, -BAR_D / 3, BAR_D / 3)
    ring = _obj("SM_Hex_Ring", bm, coll, [hexmat])

    # socket plate: hex plate + front rim
    bm = bmesh.new()
    _prism_xz(bm, _hex_pts(PLATE_R, 30), -PLATE_T / 2, PLATE_T / 2)
    rim = _hex_pts(PLATE_R - RIM_W / 2, 30)
    for k in range(6):
        _bar(bm, rim[k], rim[(k + 1) % 6], RIM_W, -PLATE_T / 2 - 0.02, -PLATE_T / 2 + 0.001)
    plate = _obj("SM_Hex_SocketPlate", bm, coll, [stone])

    # pedestal
    bm = bmesh.new()
    _cyl(bm, 0.38, 0.0, 0.12)
    _cyl(bm, 0.22, 0.12, 0.92)
    _cyl(bm, 0.34, 0.92, 1.0)
    ped = _obj("SM_Hex_Pedestal", bm, coll, [stone])
    cb = bmesh.new()
    r = bmesh.ops.create_cube(cb, size=1.0)
    bmesh.ops.transform(cb, matrix=Matrix.Translation((0, 0, 0.5)) @ Matrix.Diagonal((0.68, 0.68, 1.0, 1)), verts=r["verts"])
    col = _obj("UCX_SM_Hex_Pedestal_00", cb, coll)
    col.parent = ped
    col.display_type = 'WIRE'

    ring.location = (0, 0, 1.5)
    plate.location = (2.0, 0, 1.0)
    ped.location = (3.2, 0, 0)
    return [(o.name, tuple(round(x, 3) for x in o.dimensions)) for o in (ring, plate, ped)]


if __name__ == "__main__":
    for row in build_all():
        print(row)
