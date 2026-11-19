"""Build the leafless trees and rocks for the Observatory's outside, in the open Blender file.

Objects (origin at the base, on the ground; metres, 1 Blender unit = 1 m):
    SM_Out_DeadTree_A / _B / _C   bare trees with a tapering trunk and forked, drooping branches
    SM_Out_Rock_A / _B / _C       lumpy boulders
    SM_Out_Rock_Slab              tall standing stone
Run inside Blender; it replaces the Outdoor collection. Then run export_fbx.py (group "Outdoor").
"""
import bpy, bmesh, math, random
from mathutils import Vector, Matrix, noise

SIDES = 7


def get_collection():
    col = bpy.data.collections.get("Outdoor")
    if col:
        for o in list(col.objects):
            bpy.data.objects.remove(o, do_unlink=True)
    else:
        col = bpy.data.collections.new("Outdoor")
        bpy.context.scene.collection.children.link(col)
    return col


def add_tube(bm, pts, radii):
    """Tapered tube along pts (list of Vector); closed at the ends with a point."""
    rings = []
    prev_x = None
    for i, (p, r) in enumerate(zip(pts, radii)):
        if i < len(pts) - 1:
            d = (pts[i + 1] - p).normalized()
        else:
            d = (p - pts[i - 1]).normalized()
        if prev_x is None:
            ref = Vector((1, 0, 0)) if abs(d.x) < 0.9 else Vector((0, 1, 0))
            x = d.cross(ref).normalized()
        else:
            x = (prev_x - d * prev_x.dot(d)).normalized()
        prev_x = x
        y = d.cross(x)
        ring = [bm.verts.new(p + (x * math.cos(a) + y * math.sin(a)) * r)
                for a in (2 * math.pi * k / SIDES for k in range(SIDES))]
        rings.append(ring)
    for a, b in zip(rings, rings[1:]):
        for k in range(SIDES):
            bm.faces.new((a[k], a[(k + 1) % SIDES], b[(k + 1) % SIDES], b[k]))
    bm.faces.new(rings[0][::-1])
    tip = bm.verts.new(pts[-1] + (pts[-1] - pts[-2]).normalized() * radii[-1] * 1.5)
    for k in range(SIDES):
        bm.faces.new((rings[-1][k], rings[-1][(k + 1) % SIDES], tip))
    return rings[-1]


def branch(bm, rng, start, direction, length, radius, depth, droop):
    n = 4
    pts, radii = [start], [radius]
    d = direction.copy()
    for i in range(1, n + 1):
        d = (d + Vector((rng.uniform(-.25, .25), rng.uniform(-.25, .25), -droop + rng.uniform(-.05, .08)))).normalized()
        pts.append(pts[-1] + d * (length / n))
        radii.append(max(radius * (1 - i / (n + 0.6)), 0.012))
    add_tube(bm, pts, radii)
    if depth <= 0:
        return
    kids = rng.choice((2, 2, 3))
    for _ in range(kids):
        idx = rng.randint(2, n)
        base = pts[idx]
        side = Vector((rng.uniform(-1, 1), rng.uniform(-1, 1), rng.uniform(0.0, 0.6)))
        nd = (d * 0.5 + side).normalized()
        branch(bm, rng, base, nd, length * rng.uniform(0.55, 0.75), radii[idx] * 0.8, depth - 1, droop)


def make_tree(name, seed, height, depth, lean):
    rng = random.Random(seed)
    bm = bmesh.new()
    n = 6
    pts = [Vector((0, 0, -0.15))]
    radii = [height * 0.05]
    d = Vector((lean, 0, 1)).normalized()
    for i in range(1, n + 1):
        d = (d + Vector((rng.uniform(-.12, .12), rng.uniform(-.12, .12), 0.1))).normalized()
        pts.append(pts[-1] + d * (height / n) + Vector((0, 0, 0.15 if i == 1 else 0)))
        radii.append(max(height * 0.05 * (1 - i / (n + 1.2)), 0.03))
    # flared root
    radii[0] *= 1.5
    add_tube(bm, pts, radii)
    for i in range(2, n + 1):
        for _ in range(rng.choice((1, 2))):
            a = rng.uniform(0, 2 * math.pi)
            up = rng.uniform(0.15, 0.7)
            nd = Vector((math.cos(a), math.sin(a), up)).normalized()
            branch(bm, rng, pts[i], nd, height * rng.uniform(0.28, 0.42) * (1.1 - i / (n + 3)), radii[i] * 0.75, depth, 0.08)
    return bm_to_obj(bm, name)


def make_rock(name, seed, size, squash, spike=0.0):
    rng = random.Random(seed)
    off = Vector((rng.uniform(0, 50), rng.uniform(0, 50), rng.uniform(0, 50)))
    bm = bmesh.new()
    bmesh.ops.create_icosphere(bm, subdivisions=3, radius=1.0)
    for v in bm.verts:
        p = v.co.copy()
        n1 = noise.noise(p * 1.4 + off)
        n2 = noise.noise(p * 3.5 + off) * 0.25
        v.co = p * (1 + 0.45 * n1 + n2)
        v.co.x *= size * squash[0]
        v.co.y *= size * squash[1]
        v.co.z *= size * squash[2] * (1 + spike * max(p.z, 0))
    # sit on the ground, flatten the bottom slightly
    zmin = min(v.co.z for v in bm.verts)
    for v in bm.verts:
        v.co.z -= zmin
        if v.co.z < 0.08 * size:
            v.co.z = 0.08 * size * 0.5 + v.co.z * 0.5
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return bm_to_obj(bm, name)


def bm_to_obj(bm, name):
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    ob = bpy.data.objects.new(name, me)
    for p in me.polygons:
        p.use_smooth = False
    return ob


def run():
    col = get_collection()
    mat_name = "M_Out_Dark"
    mat = bpy.data.materials.get(mat_name) or bpy.data.materials.new(mat_name)
    mat.use_nodes = True
    bsdf = next(n for n in mat.node_tree.nodes if n.type == "BSDF_PRINCIPLED")
    bsdf.inputs["Base Color"].default_value = (0.08, 0.075, 0.07, 1)
    objs = [
        make_tree("SM_Out_DeadTree_A", 3, 6.5, 2, 0.12),
        make_tree("SM_Out_DeadTree_B", 11, 8.5, 2, -0.08),
        make_tree("SM_Out_DeadTree_C", 27, 5.0, 1, 0.2),
        make_rock("SM_Out_Rock_A", 5, 1.2, (1.3, 1.0, 0.7)),
        make_rock("SM_Out_Rock_B", 9, 2.2, (1.1, 1.2, 0.8)),
        make_rock("SM_Out_Rock_C", 21, 0.6, (1.2, 0.9, 0.7)),
        make_rock("SM_Out_Rock_Slab", 33, 1.1, (0.55, 0.9, 1.8), spike=0.3),
    ]
    for i, ob in enumerate(objs):
        ob.data.materials.append(mat)
        ob.location = (i * 12 - 36, 0, 0)
        col.objects.link(ob)
    return [o.name for o in objs]


print(run())
