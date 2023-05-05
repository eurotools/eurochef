import bpy
import json
import math
import os

from bpy.props import (StringProperty, BoolProperty)
from bpy_extras.io_utils import (ImportHelper)
from mathutils import Euler

from . import trigger_vis


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
    autosmooth: BoolProperty(name="Autosmooth meshes", default=True)

    trigger_visualizations: BoolProperty(
        name="Visualize special triggers", default=False)

    import_triggers: BoolProperty(name="Import triggers", default=True)

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

        set_active_collection(self.collection)
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

            obj.location = egx_to_blender_pos(
                tuple(placement['position']))

            obj.rotation_mode = 'XYZ'
            obj.rotation_euler = egx_to_blender_rot(
                tuple(placement['rotation']))

            obj.scale = egx_to_blender_scale(tuple(placement['scale']))
            self.collection.objects.link(obj)

            if object_id not in object_cache:
                # Make the material double-sided. We're only doing this for normal placements
                for mat in obj.material_slots:
                    mat.material.use_backface_culling = False

                if self.autosmooth:
                    bpy.ops.object.shade_smooth()

                object_cache[object_id] = obj

            if self.lock_objects:
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

            # Re-enable backface culling for some merged materials
            for mat in obj.material_slots:
                mat.material.use_backface_culling = True

            if self.autosmooth:
                bpy.ops.object.shade_smooth()

            if self.lock_objects:
                obj.hide_select = True

        if self.merge_materials:
            self.merge_all_materials()

        if self.import_triggers:
            print("Importing triggers")
            self.load_triggers(self.data['triggers'])

    # Merge all duplicate materials
    def merge_all_materials(self):
        all_base_materials = {}

        # Find all materials
        duplicates = 0
        # for obj in self.collection.objects:
        for mat in bpy.data.materials:
            basename = mat.name[:mat.name.rfind('.')]
            if basename == mat.name or mat.name.endswith(".png"):
                all_base_materials[mat.name] = mat
            else:
                duplicates += 1

        print(
            f"Merging {duplicates} duplicate materials ({len(all_base_materials)} base materials in total)")

        # Reassign materials
        for obj in self.collection.objects:
            for i, mat in enumerate(obj.material_slots):
                basename = mat.name[:mat.name.rfind('.')]
                print(f"Mat {mat.name}...", end='')
                if basename == mat.name or mat.name.endswith(".png"):
                    print(" Is original!")
                    continue
                else:
                    print(" Is a dupe!")
                    obj.material_slots[i].material = all_base_materials[basename]

    def load_triggers(self, triggers):
        self.trigger_collection = bpy.data.collections.new("triggers")
        self.collection.children.link(self.trigger_collection)
        set_active_collection_child(self.collection, self.trigger_collection)

        for i, t in enumerate(triggers):
            bpy.ops.object.empty_add(type='PLAIN_AXES', align='WORLD', location=egx_to_blender_pos(tuple(
                t['position'])), rotation=egx_to_blender_rot(tuple(t['rotation'])), scale=egx_to_blender_scale(tuple(t['scale'])))
            obj = bpy.context.active_object

            obj.name = f"{i}#{t['ttype']}"
            obj.show_name = True

            if t['tsubtype']:
                obj['subtype'] = t['tsubtype']

            for di, d in enumerate(t['data']):
                if d != 0:
                    obj[f'data[0x{di:x}]'] = f"0x{d:x}"

            for li, l in enumerate(t['links']):
                if l != -1:
                    obj[f'links[{li}]'] = str(l)

            if self.trigger_visualizations:
                trigger_vis.process_triggers(
                    t['data'], t['links'], obj, i, t['ttype'])


def set_active_collection(collection: bpy.types.Collection):
    bpy.context.view_layer.active_layer_collection = bpy.context.view_layer.layer_collection.children[
        collection.name]

# TODO(cohae): Could be better


def set_active_collection_child(collection: bpy.types.Collection, child: bpy.types.Collection):
    bpy.context.view_layer.active_layer_collection = bpy.context.view_layer.layer_collection.children[
        collection.name].children[child.name]


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
