"""Build the six Hexfold rune meshes in the open Blender file.

Run inside Blender (Text Editor > Run Script, or exec() over the Blender MCP).
It replaces everything in the Rune_* collections.

Per rune N it makes:
  SM_Rune_N            visual mesh: strokes 8 cm wide, 8 cm thick, front (-Y) edges chamfered,
                       back flat, so the 3D shape is chiral. Origin at the centre of its bounds.
  UCX_SM_Rune_N_##     one box hull per stroke segment (children of SM_Rune_N), for Unreal's
                       simple collision. Boxes are fine at negative (mirrored) scale.
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
