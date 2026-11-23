"""Build the player's body (SM_Player) and export it to assets/models/Player/SM_Player.glb.

Run headless:  blender -b --python art/Blender/scripts/build_player.py
(or inside Blender, e.g. over the Blender MCP; it then also leaves the mesh in the open file).

A simple, left-right symmetric figure: boots, legs, belt, torso, arms, head with a glowing visor.
Units: metres. Origin at the centre of the feet, standing on z = 0, front = -Y (Blender's front).
It is 1.76 m tall with the eyes at 1.58 m and fits inside the game's collider (radius 0.34 m,
height 1.76 m, eye 0.70 m above the collider centre), so the body and the camera agree.

Three material slots, one glTF primitive each:
  0  M_PlayerCloth  dark blue-grey fabric (legs, torso, arms, head)
  1  M_PlayerTrim   gold (boots, belt, cuffs, collar), matching the runes
  2  M_PlayerVisor  emissive pale blue visor strip
"""
import bpy, bmesh, math, os
from mathutils import Matrix, Vector

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", ".."))
OUT = os.path.join(ROOT, "assets", "models", "Player", "SM_Player.glb")

CLOTH, TRIM, VISOR = 0, 1, 2


def _material(name, rgb, emissive=None, metallic=0.0, roughness=0.6):
    m = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    m.use_nodes = True
    bsdf = next(n for n in m.node_tree.nodes if n.type == "BSDF_PRINCIPLED")
    bsdf.inputs["Base Color"].default_value = (*rgb, 1)
    bsdf.inputs["Metallic"].default_value = metallic
    bsdf.inputs["Roughness"].default_value = roughness
    if emissive:
        bsdf.inputs["Emission Color"].default_value = (*emissive, 1)
        bsdf.inputs["Emission Strength"].default_value = 3.0
    m.diffuse_color = (*rgb, 1)
    return m


def _box(bm, centre, size, mat, taper=1.0):
    """Axis-aligned box; `taper` scales the bottom face's width (a cheap tapered limb)."""
    geom = bmesh.ops.create_cube(bm, size=1.0, matrix=Matrix.Diagonal((*size, 1.0)))
    verts = geom["verts"]
    for v in verts:
        if v.co.z < 0 and taper != 1.0:
            v.co.x *= taper
            v.co.y *= taper
        v.co += Vector(centre)
    faces = {f for v in verts for f in v.link_faces}
    for f in faces:
        f.material_index = mat
    return verts


def _sphere(bm, centre, radius, mat, squash=(1.0, 1.0, 1.0)):
    geom = bmesh.ops.create_uvsphere(bm, u_segments=20, v_segments=12, radius=radius)
    for v in geom["verts"]:
        v.co = Vector((v.co.x * squash[0], v.co.y * squash[1], v.co.z * squash[2])) + Vector(centre)
    for f in {f for v in geom["verts"] for f in v.link_faces}:
        f.material_index = mat


def build():
    for name in ("SM_Player",):
        old = bpy.data.objects.get(name)
        if old:
            me = old.data
            bpy.data.objects.remove(old, do_unlink=True)
            bpy.data.meshes.remove(me)

    bm = bmesh.new()
    for s in (-1, 1):
        # Boot (gold), leg (cloth). Legs are 0.8 m, boots the bottom 0.12 m.
        _box(bm, (s * 0.115, -0.02, 0.06), (0.17, 0.30, 0.12), TRIM)
        _box(bm, (s * 0.115, 0.0, 0.46), (0.16, 0.20, 0.68), CLOTH, taper=0.85)
        # Arm (cloth) hanging at the side, gold cuff at the wrist, a small hand-block below.
        _box(bm, (s * 0.285, 0.0, 1.10), (0.10, 0.14, 0.50), CLOTH)
        _box(bm, (s * 0.285, 0.0, 0.84), (0.115, 0.155, 0.06), TRIM)
        _box(bm, (s * 0.285, 0.0, 0.76), (0.09, 0.12, 0.10), CLOTH)
        # Shoulder pad.
        _sphere(bm, (s * 0.255, 0.0, 1.355), 0.085, TRIM, squash=(1.0, 1.0, 0.7))

    # Belt and torso (a bit wider at the shoulders).
    _box(bm, (0, 0, 0.82), (0.43, 0.25, 0.07), TRIM)
    _box(bm, (0, 0, 1.12), (0.42, 0.23, 0.52), CLOTH)
    _box(bm, (0, 0, 1.36), (0.45, 0.24, 0.10), CLOTH)
    # Collar.
    _box(bm, (0, 0, 1.43), (0.24, 0.20, 0.05), TRIM)
    # Neck and head, with a visor strip on the front (-Y) and a small crest on top.
    _box(bm, (0, 0, 1.47), (0.10, 0.10, 0.06), CLOTH)
    _sphere(bm, (0, 0, 1.59), 0.15, CLOTH, squash=(0.95, 1.0, 1.05))
    _box(bm, (0, -0.125, 1.60), (0.22, 0.06, 0.05), VISOR)
    _box(bm, (0, 0.0, 1.745), (0.04, 0.16, 0.03), TRIM)

    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    me = bpy.data.meshes.new("SM_Player")
    bm.to_mesh(me)
    bm.free()
    me.materials.append(_material("M_PlayerCloth", (0.13, 0.16, 0.24), roughness=0.75))
    me.materials.append(_material("M_PlayerTrim", (0.95, 0.72, 0.22), metallic=0.7, roughness=0.35))
    me.materials.append(_material("M_PlayerVisor", (0.6, 0.85, 1.0), emissive=(0.4, 0.8, 1.0), roughness=0.2))
    for p in me.polygons:
        p.use_smooth = False
    ob = bpy.data.objects.new("SM_Player", me)
    bpy.context.scene.collection.objects.link(ob)
    return ob


def export(ob):
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    for o in bpy.context.scene.objects:
        o.select_set(o is ob)
    bpy.context.view_layer.objects.active = ob
    bpy.ops.export_scene.gltf(filepath=OUT, export_format='GLB', export_yup=True, export_apply=True,
                              export_materials='EXPORT', use_selection=True,
                              export_cameras=False, export_lights=False)


if __name__ == "__main__":
    if bpy.app.background:
        bpy.ops.wm.read_factory_settings(use_empty=True)
    player = build()
    d = player.dimensions
    print("SM_Player dims(m) = %.2f x %.2f x %.2f, tris %d" % (d.x, d.y, d.z, sum(len(p.vertices) - 2 for p in player.data.polygons)))
    export(player)
    print("exported", OUT)
