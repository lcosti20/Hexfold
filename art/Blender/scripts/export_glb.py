"""Convert every FBX in art/Export/<group>/ to a glTF binary in assets/models/<group>/.

Run headless:  blender -b --python art/Blender/scripts/export_glb.py

The FBX files are the Unreal exports (metres, Blender axes). glTF export converts Blender's
Z-up to Bevy's Y-up, so a mesh's Blender front (-Y) ends up facing +Z in Bevy. UCX_ collision
hulls are dropped: colliders are rebuilt from the render mesh in the game.
"""
import bpy, os, sys

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", ".."))
SRC = os.path.join(ROOT, "art", "Export")
DST = os.path.join(ROOT, "assets", "models")


def convert(fbx, out):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.fbx(filepath=fbx)
    for ob in list(bpy.data.objects):
        if ob.name.startswith("UCX_") or ob.type != 'MESH':
            bpy.data.objects.remove(ob, do_unlink=True)
    os.makedirs(os.path.dirname(out), exist_ok=True)
    bpy.ops.export_scene.gltf(filepath=out, export_format='GLB', export_yup=True,
                              export_apply=True, export_materials='EXPORT', export_cameras=False,
                              export_lights=False)
    ob = next(iter(bpy.data.objects), None)
    if ob:
        d = ob.dimensions
        print("OK", os.path.basename(out), "dims(m)=%.2f x %.2f x %.2f" % (d.x, d.y, d.z), "loc", tuple(round(v, 2) for v in ob.location))


for group in sorted(os.listdir(SRC)):
    gdir = os.path.join(SRC, group)
    if not os.path.isdir(gdir):
        continue
    for name in sorted(os.listdir(gdir)):
        if name.lower().endswith(".fbx"):
            convert(os.path.join(gdir, name), os.path.join(DST, group, name[:-4] + ".glb"))
