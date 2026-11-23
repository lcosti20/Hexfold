"""Build the six Hexfold rune meshes in the open Blender file.

Run inside Blender (Text Editor > Run Script, or exec() over the Blender MCP).
It replaces everything in the Rune_* collections.

Per rune N it makes:
  SM_Rune_N            visual mesh: strokes 8 cm wide, 8 cm thick, front (-Y) edges chamfered,
                       back flat, so the 3D shape is chiral. Origin at the centre of its bounds.
  UCX_SM_Rune_N_##     one box hull per stroke segment (children of SM_Rune_N), for Unreal's
                       simple collision. Boxes are fine at negative (mirrored) scale.
                       Behind the strokes (+Y) it also has a hexagonal backing plate (material slot 1),
                       pointy-top like the sockets of the hex seal, so a rune's facing is obvious and
                       it fits into a socket. The plate is its own mesh island (strokes keep slot 0).
  SM_Glyph_N           flat 1 cm plate of the same outline, for the hex sockets.

Units: metres (1 Blender unit = 1 m = 100 Unreal units).
"""
import bpy, bmesh, math
from mathutils import Vector

U = 0.01     # grid unit: coordinates below are in cm
W = 0.08     # stroke width
T = 0.08     # thickness along Y
BEV = 0.012  # front chamfer
GLYPH_T = 0.01
PLATE_R = 0.325   # corner radius of the hex backing plate: every rune's strokes fit inside it
PLATE_T = 0.02    # plate thickness along Y (behind the back face at y = +T/2)
SPACING = 0.7

# Each rune is a list of polylines on a cm grid (x right, z up). None has a mirror symmetry.
RUNES = {
    0: [[(-8, -20), (-8, 20)], [(-8, 20), (10, 8)], [(-8, 6), (10, -6)]],          # stem, two slanted arms
    1: [[(-10, 20), (-10, 4), (10, -4), (10, -20)]],                                # zig-zag bolt
    2: [[(10, 20), (0, 20), (0, -20), (-12, -20), (-12, -10)]],                      # hook
    3: [[(-8, -20), (-8, 20)], [(-8, 20), (10, 10), (-8, 0)]],                       # flag
    4: [[(0, -20), (0, 20)], [(0, 20), (12, 10)], [(0, -20), (-12, -10)]],           # opposite barbs
    5: [[(0, -20), (0, 6)], [(0, 6), (-12, 20)], [(0, 6), (7, 14)]],                 # unequal fork
}

# Extra solid convex areas (cm grid), so closed loops don't leave holes with sharp inner corners.
FILLS = {
    3: [[(-8, 20), (-8, 0), (10, 10)]],
}


def _prism(bm, pts2d, t):
    front = [bm.verts.new((x, -t / 2, z)) for x, z in pts2d]
    back = [bm.verts.new((x, t / 2, z)) for x, z in pts2d]
    bm.faces.new(front)
    bm.faces.new(list(reversed(back)))
    n = len(pts2d)
    for i in range(n):
        j = (i + 1) % n
        bm.faces.new((front[i], front[j], back[j], back[i]))


def _ccw(poly):
    n = len(poly)
    a = sum(poly[i][0] * poly[(i + 1) % n][1] - poly[(i + 1) % n][0] * poly[i][1] for i in range(n))
    return poly if a > 0 else list(reversed(poly))


def _object(name, polys, t, coll):
    me = bpy.data.meshes.new(name)
    ob = bpy.data.objects.new(name, me)
    coll.objects.link(ob)
    bm = bmesh.new()
    for p in polys:
        _prism(bm, _ccw(p), t)
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bm.to_mesh(me)
    bm.free()
    return ob


def _seg_rect(p, q, w, ext_p=0.0, ext_q=0.0):
    p, q = Vector((p[0] * U, p[1] * U)), Vector((q[0] * U, q[1] * U))
    d = (q - p).normalized()
    n = Vector((-d.y, d.x)) * w / 2
    p, q = p - d * ext_p, q + d * ext_q
    return [tuple(p - n), tuple(q - n), tuple(q + n), tuple(p + n)]


def _octagon(c, w):
    r = (w / 2) / math.cos(math.pi / 8)
    return [(c[0] * U + r * math.cos(math.pi / 8 + k * math.pi / 4),
             c[1] * U + r * math.sin(math.pi / 8 + k * math.pi / 4)) for k in range(8)]


def _hex_pts(r):
    """Pointy-top hexagon (corners at 30, 90, ... degrees) in the XZ plane, like the socket plates."""
    return [(r * math.cos(math.radians(30 + 60 * k)), r * math.sin(math.radians(30 + 60 * k))) for k in range(6)]


def _material(name, rgb):
    m = bpy.data.materials.get(name) or bpy.data.materials.new(name)
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


def _union(base, others):
    for other in others:
        m = base.modifiers.new("union", 'BOOLEAN')
        m.operation, m.solver, m.object = 'UNION', 'EXACT', other
        with bpy.context.temp_override(object=base, active_object=base):
            bpy.ops.object.modifier_apply(modifier=m.name)
        me = other.data
        bpy.data.objects.remove(other, do_unlink=True)
        bpy.data.meshes.remove(me)


def build_rune(idx, strokes):
    coll = _clear(f"Rune_{idx}")

    # Outline = segment rectangles + octagons at every bend and every point where strokes meet.
    parts, pts = [], {}
    for s in strokes:
        parts += [_seg_rect(a, b, W) for a, b in zip(s, s[1:])]
        for k, p in enumerate(s):
            inner = 0 < k < len(s) - 1
            pts[p] = pts.get(p, 0) + (2 if inner else 1)
    parts += [_octagon(p, W) for p, n in pts.items() if n > 1]
    parts += [[(x * U, z * U) for x, z in f] for f in FILLS.get(idx, [])]

    pieces = [_object(f"_tmp_{idx}_{k}", [p], T, coll) for k, p in enumerate(parts)]
    base = pieces[0]
    base.name = base.data.name = f"SM_Rune_{idx}"
    _union(base, pieces[1:])

    bm = bmesh.new()
    bm.from_mesh(base.data)
    bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=1e-5)
    bmesh.ops.dissolve_limit(bm, angle_limit=math.radians(1), verts=bm.verts, edges=bm.edges)
    xs = [v.co.x for v in bm.verts]
    zs = [v.co.z for v in bm.verts]
    c = Vector(((max(xs) + min(xs)) / 2, 0, (max(zs) + min(zs)) / 2))
    for v in bm.verts:
        v.co -= c

    # Glyph: the unbevelled outline squashed to 1 cm.
    gme = bpy.data.meshes.new(f"SM_Glyph_{idx}")
    gbm = bm.copy()
    for v in gbm.verts:
        v.co.y *= GLYPH_T / T
    gbm.to_mesh(gme)
    gbm.free()
    glyph = bpy.data.objects.new(gme.name, gme)
    coll.objects.link(glyph)

    # Chamfer the front face's edges only.
    front = [f for f in bm.faces if f.normal.y < -0.99]
    edges = list({e for f in front for e in f.edges})
    bmesh.ops.bevel(bm, geom=edges, offset=BEV, segments=1, affect='EDGES', profile=0.5)
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    # Re-centre on the final bounds (the portal plugin tracks the origin); the glyph follows.
    xs = [v.co.x for v in bm.verts]
    zs = [v.co.z for v in bm.verts]
    c2 = Vector(((max(xs) + min(xs)) / 2, 0, (max(zs) + min(zs)) / 2))
    for v in bm.verts:
        v.co -= c2
    for v in gme.vertices:
        v.co -= c2
    c += c2

    # Hex backing plate behind the back face (+Y), as a separate island with material slot 1.
    # It overlaps the strokes by 2 mm so the two never show a gap.
    y0, y1 = T / 2 - 0.002, T / 2 + PLATE_T
    pa = [bm.verts.new((x, y0, z)) for x, z in _hex_pts(PLATE_R)]
    pb = [bm.verts.new((x, y1, z)) for x, z in _hex_pts(PLATE_R)]
    plate_faces = [bm.faces.new(pa), bm.faces.new(list(reversed(pb)))]
    plate_faces += [bm.faces.new((pa[i], pa[(i + 1) % 6], pb[(i + 1) % 6], pb[i])) for i in range(6)]
    bmesh.ops.recalc_face_normals(bm, faces=plate_faces)
    for f in bm.faces:
        f.material_index = 1 if f in plate_faces else 0
    base.data.materials.clear()
    base.data.materials.append(_material("M_Rune", (0.85, 0.65, 0.2)))
    base.data.materials.append(_material("M_RunePlate", (0.35, 0.33, 0.3)))
    bm.to_mesh(base.data)
    bm.free()

    # Collision hulls: one box per segment, lengthened by half a width only at joints
    # (free ends stay flush with the visual mesh, so runes rest on the floor).
    k = 0
    for s in strokes:
        for a, b in zip(s, s[1:]):
            # 0.4 * half width keeps the box corners inside the joint octagon
            ea = 0.4 * W / 2 if pts[a] > 1 else 0.0
            eb = 0.4 * W / 2 if pts[b] > 1 else 0.0
            poly = [(x - c.x, z - c.z) for x, z in _seg_rect(a, b, W, ea, eb)]
            col = _object(f"UCX_SM_Rune_{idx}_{k:02d}", [poly], T, coll)
            col.parent = base
            col.display_type = 'WIRE'
            col.hide_render = True
            k += 1

    # One more hull for the plate (a convex hexagonal prism).
    poly = _hex_pts(PLATE_R)
    plate_col = _object(f"UCX_SM_Rune_{idx}_{k:02d}", [poly], PLATE_T + 0.002, coll)
    for v in plate_col.data.vertices:
        v.co.y += T / 2 + (PLATE_T - 0.002) / 2
    plate_col.parent = base
    plate_col.display_type = 'WIRE'
    plate_col.hide_render = True

    base.location = (idx * SPACING, 0, 0.3)
    glyph.location = (idx * SPACING, 0, -0.15)
    for o in (base, glyph):
        o.data.update()
    return base


def build_all():
    out = []
    for idx, strokes in RUNES.items():
        ob = build_rune(idx, strokes)
        d = ob.dimensions
        radius_cm = math.sqrt((d.x / 2) ** 2 + (d.y / 2) ** 2 + (d.z / 2) ** 2) * 100
        out.append((ob.name, tuple(round(x, 3) for x in d), len(ob.data.polygons), round(radius_cm, 1)))
    return out


if __name__ == "__main__":
    for row in build_all():
        print(row)
