"""Export every SM_* object in the open Blender file to Art/Export/<group>/<name>.fbx.

Run inside Blender after build_runes.py / build_observatory.py (and after saving).
Each mesh is exported at the world origin together with its UCX_ children, so the
pivot Unreal gets is the object's origin.

Unit settings: Blender scene in metres (unit scale 1) and FBX "Apply Scalings" =
FBX_SCALE_NONE, Blender's default. Unreal then imports 1 m as 100 cm.
(FBX_SCALE_ALL makes meshes arrive 100x too small.)
"""
import bpy, os

ART = os.path.normpath(os.path.join(os.path.dirname(bpy.data.filepath), ".."))
GROUPS = {"Outdoor.blend": "Outdoor", "Runes.blend": "Runes", "Observatory_Exterior.blend": "Observatory", "Rotunda.blend": "Observatory", "Hex.blend": "Hex", "Wings.blend": "Wings"}


def export_all():
    group = GROUPS.get(os.path.basename(bpy.data.filepath), "Misc")
    out_dir = os.path.join(ART, "Export", group)
    os.makedirs(out_dir, exist_ok=True)
    done = []
    for ob in [o for o in bpy.data.objects if o.type == 'MESH' and o.name.startswith("SM_") and o.parent is None]:
        saved = ob.matrix_world.copy()
        ob.matrix_world.identity()          # export at the origin, unrotated and unscaled
        bpy.context.view_layer.update()
        for o in bpy.context.view_layer.objects:
            o.select_set(False)
        for o in [ob] + list(ob.children):
            o.select_set(True)
        bpy.context.view_layer.objects.active = ob
        path = os.path.join(out_dir, ob.name + ".fbx")
        bpy.ops.export_scene.fbx(filepath=path, use_selection=True, object_types={'MESH'},
                                 apply_unit_scale=True, apply_scale_options='FBX_SCALE_NONE',
                                 mesh_smooth_type='FACE', use_mesh_modifiers=True,
                                 add_leaf_bones=False, bake_anim=False)
        ob.matrix_world = saved
        done.append((ob.name, len(ob.children), path))
    for o in bpy.context.view_layer.objects:
        o.select_set(False)
    return done


if __name__ == "__main__":
    for row in export_all():
        print(row)
