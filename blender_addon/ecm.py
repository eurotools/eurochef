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
    downscale: BoolProperty(name="Rescale map (recommended)", default=True)
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
            bpy.path.display_name_from_filepath(self.filepath))
        bpy.context.scene.collection.children.link(self.collection)

        for placement in self.data['placements']:
            model_path = os.path.join(
                self.directory, "{:x}.gltf".format(placement['object_ref']))
            if not os.path.exists(model_path):
                print("Couldn't find model {:x}/{:x}".format(
                    placement['object_ref'], placement['hashcode']))
                continue

            bpy.ops.import_scene.gltf(filepath=model_path)
            obj = bpy.context.active_object

            obj.location = egx_to_blender_pos(
                tuple(placement['position']))

            obj.rotation_mode = 'XYZ'
            obj.rotation_euler = egx_to_blender_rot(
                tuple(placement['rotation']))

            obj.scale = egx_to_blender_scale(tuple(placement['scale']))

        for mapzone in self.data['mapzone_entities']:
            model_path = os.path.join(
                self.directory, "ref_{}.gltf".format(mapzone['entity_refptr']))
            if not os.path.exists(model_path):
                print("Couldn't find model ref_{}".format(
                    mapzone['entity_refptr']))
                continue

            bpy.ops.import_scene.gltf(filepath=model_path)


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
