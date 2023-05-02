import bpy
import json
import math
import os

from bpy.props import (StringProperty, BoolProperty)
from bpy_extras.io_utils import (ImportHelper)
from mathutils import Euler


class EcmLoader(bpy.types.Operator, ImportHelper):
    """Import BSP map files from the Source engine"""
    bl_idname = "eurochefutil.ecm"
    bl_description = "Import Eurochef map files"
    bl_label = "Import Eurochef ECM"

    filename_ext = ".ecm"
    filter_glob: StringProperty(
        default="*.ecm",
        options={'HIDDEN'},
    )

    filepath: StringProperty(subtype="FILE_PATH")
    merge_materials: BoolProperty(
        name="Merge materials (recommended)", default=True)
    lock_objects: BoolProperty(name="Make objects unselectable", default=False)

    def execute(self, context):
        self.data = json.load(open(self.filepath, 'r'))
        self.directory = os.path.dirname(self.filepath)
        print("Loading data from {}".format(self.directory))
        if (not self.load()):
            return {'CANCELLED'}

        return {'FINISHED'}

    def scale_pos(self, pos):
        return [
            pos[0] * 0.1,
            pos[1] * 0.1,
            pos[2] * 0.1,
        ]

    def load(self):
        if (not self.data):
            return False

        self.collection = bpy.data.collections.new(
            os.path.basename(os.path.dirname(self.filepath)))
        bpy.context.scene.collection.children.link(self.collection)

        object_cache = {}

        for placement in self.data['placements']:
            object_id = f"{placement['object_ref']:x}"
            model_path = os.path.join(
                self.directory, f"{object_id}.gltf")
            print(f"[ECM] Loading {model_path}")
            if not os.path.exists(model_path):
                print("Couldn't find model {}/{:x}".format(
                    object_id, placement['hashcode']))
                continue

            obj = None
            if object_id in object_cache:
                obj = object_cache[object_id].copy()
                print(f"Copied {object_id} from cache")
            else:
                bpy.ops.import_scene.gltf(filepath=model_path)
                obj = bpy.context.active_object
                bpy.context.scene.collection.objects.unlink(obj)

            obj.location = egx_to_blender_pos(
                tuple(placement['position']))

            obj.rotation_mode = 'XYZ'
            obj.rotation_euler = egx_to_blender_rot(
                tuple(placement['rotation']))

            obj.scale = egx_to_blender_scale(tuple(placement['scale']))
            self.collection.objects.link(obj)

            if object_id not in object_cache:
                object_cache[object_id] = obj

            if (self.lock_objects):
                obj.hide_select = True

        for mapzone in self.data['mapzone_entities']:
            object_id = f"ref_{mapzone['entity_refptr']}"
            model_path = os.path.join(
                self.directory, f"{object_id}.gltf")
            print(f"[ECM] Loading {model_path}")
            if not os.path.exists(model_path):
                print("Couldn't find model ref_{}".format(
                    mapzone['entity_refptr']))
                continue

            bpy.ops.import_scene.gltf(filepath=model_path)
            obj = bpy.context.active_object
            bpy.context.scene.collection.objects.unlink(obj)
            self.collection.objects.link(obj)

            if (self.lock_objects):
                obj.hide_select = True

        if self.merge_materials:
            self.merge_all_materials()

    # Merge all duplicate materials
    def merge_all_materials(self):
        original_materials = {}

        # Find all materials
        duplicates = 0
        for obj in self.collection.objects:
            for mat in obj.material_slots:
                basename = mat.name[:mat.name.rfind('.')]
                if basename == mat.name or mat.name.endswith(".png"):
                    original_materials[mat.name] = mat.material
                else:
                    duplicates += 1

        print(
            f"Merging {duplicates} duplicate materials into {len(original_materials)}")

        # Reassign materials
        for obj in self.collection.objects:
            for i, mat in enumerate(obj.material_slots):
                basename = mat.name[:mat.name.rfind('.')]
                if basename == mat.name or mat.name.endswith(".png"):
                    continue
                else:
                    obj.material_slots[i].material = original_materials[basename]


def egx_to_blender_pos(pos: tuple):
    return (
        -pos[0],
        -pos[2],
        pos[1],
    )


def egx_to_blender_rot(pos: tuple):
    return (
        pos[0],
        pos[2],
        -pos[1],
    )


def egx_to_blender_scale(pos: tuple):
    return (
        pos[0],
        pos[2],
        pos[1],
    )


def menu_import(self, context):
    self.layout.operator(EcmLoader.bl_idname, text='Eurochef ECM (.ecm)')


def register():
    bpy.utils.register_class(EcmLoader)
    bpy.types.TOPBAR_MT_file_import.append(menu_import)


def unregister():
    bpy.types.TOPBAR_MT_file_import.remove(menu_import)
    bpy.utils.unregister_class(EcmLoader)
