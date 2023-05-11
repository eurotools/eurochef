import bpy
import json
import math
import os

from bpy.props import (StringProperty, BoolProperty)
from bpy_extras.io_utils import (ImportHelper)
from mathutils import Euler

from blender_addon.common import relink_object

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

    surface_blending: BoolProperty(
        name="Surface blending (WIP)", default=False)

    import_triggers: BoolProperty(name="Import triggers", default=True)

    def execute(self, context):
        self.data = json.load(open(self.filepath, 'r'))
        self.directory = os.path.dirname(self.filepath)
        print("Loading data from {}".format(self.directory))
        if (not self.load()):
            return {'CANCELLED'}

        return {'FINISHED'}

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

            obj.location = egx_to_blender_pos(
                tuple(placement['position']))

            obj.rotation_mode = 'XYZ'
            obj.rotation_euler = egx_to_blender_rot(
                tuple(placement['rotation']))

            obj.scale = egx_to_blender_scale(tuple(placement['scale']))
            relink_object(obj, self.collection)

            if object_id not in object_cache:
                # Make the material double-sided. We're only doing this for normal placements
                for mat in obj.material_slots:
                    mat.material.use_backface_culling = False

                if self.autosmooth:
                    bpy.ops.object.shade_smooth()

                object_cache[object_id] = obj

            if self.lock_objects:
                obj.hide_select = True

            self.process_blended_surfaces(obj)

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
            relink_object(obj, self.collection)

            # Re-enable backface culling for some merged materials
            for mat in obj.material_slots:
                mat.material.use_backface_culling = True

            if self.autosmooth:
                bpy.ops.object.shade_smooth()

            if self.lock_objects:
                obj.hide_select = True

            self.process_blended_surfaces(obj)

        if self.merge_materials:
            self.merge_all_materials()

        if self.import_triggers:
            print("Importing triggers")
            self.load_triggers(self.data['triggers'])

    def process_blended_surfaces(self, obj: bpy.types.Object):
        for mat in obj.material_slots:
            self.rewrite_material_vertex_lighting(mat.material)

        if not self.surface_blending:
            return

        mesh: bpy.types.Mesh = obj.data
        color_layer = mesh.vertex_colors["Col"]
        # Check if the object has any polygons with a vertex color alpha under 1.0, and modify the material if it does
        seen_materials = []
        for poly in mesh.polygons:
            material = obj.material_slots[poly.material_index].material
            if material.name not in seen_materials:
                for idx in poly.loop_indices:
                    rgb = color_layer.data[idx].color

                    if rgb[3] < 1.0:
                        print(
                            f"Surface has transparency {rgb[0]} {rgb[1]} {rgb[2]} {rgb[3]}")
                        seen_materials.append(material.name)
                        self.modify_material_for_blending(material)

    # Rewrite the material to use vertex colors as lighting
    def rewrite_material_vertex_lighting(self, material: bpy.types.Material):
        ...

    def modify_material_for_blending(self, material: bpy.types.Material):

        # Set the material's alpha mode to blend
        material.blend_method = 'HASHED'

        # Add a mix node before the material output, plugging the original material into the second slot, and a transparency node into the first. Plug the vertex color alpha into the factor

        # Create a mix node
        mix_node = material.node_tree.nodes.new("ShaderNodeMixShader")

        # Create a transparency node
        transparency_node = material.node_tree.nodes.new(
            "ShaderNodeBsdfTransparent")

        # Get the color attribute node
        vertex_color_node = material.node_tree.nodes["Color Attribute"]

        # Get the output node
        output_node = material.node_tree.nodes["Material Output"]

        # Get node attached to output node
        output_node_shader = output_node.inputs[0].links[0].from_node

        material.node_tree.links.new(
            mix_node.inputs[2], output_node_shader.outputs[0])
        material.node_tree.links.new(
            output_node.inputs["Surface"], mix_node.outputs[0])
        material.node_tree.links.new(
            mix_node.inputs[1], transparency_node.outputs["BSDF"])
        material.node_tree.links.new(
            mix_node.inputs["Fac"], vertex_color_node.outputs["Alpha"])

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
                if basename == mat.name or mat.name.endswith(".png"):
                    continue
                else:
                    base_material = all_base_materials[basename]
                    # Do not merge materials with different transparency settings
                    if base_material.blend_method != mat.material.blend_method:
                        continue
                    obj.material_slots[i].material = base_material

    def load_triggers(self, triggers):
        self.trigger_collection = bpy.data.collections.new("triggers")
        self.collection.children.link(self.trigger_collection)

        for i, t in enumerate(triggers):
            bpy.ops.object.empty_add(type='PLAIN_AXES', align='WORLD', location=egx_to_blender_pos(tuple(
                t['position'])), rotation=egx_to_blender_rot(tuple(t['rotation'])), scale=egx_to_blender_scale(tuple(t['scale'])))
            obj = bpy.context.active_object
            relink_object(obj, self.trigger_collection)

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
