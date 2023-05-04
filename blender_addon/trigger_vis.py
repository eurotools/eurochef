from struct import pack, unpack
import bpy
from mathutils import Euler
import math


def raw_to_f32(raw: int):
    return unpack(">f", pack(">I", raw))[0]


def visualize_crossplane(data, parent, parent_id, parent_subclass):
    bpy.ops.mesh.primitive_plane_add()
    obj = bpy.context.active_object
    obj.name = f"{parent_id}#Visualization"
    obj.parent = parent
    bpy.ops.object.origin_clear()

    scale_x = raw_to_f32(data[0])
    scale_y = raw_to_f32(data[1])

    obj.dimensions = (scale_x, scale_y, 0)
    obj.rotation_euler = Euler((math.radians(90), 0, 0))

    if parent_subclass == "HT_TriggerSubType_LoadMap":
        obj.color = (0, 0, 255, 255)

    if parent_subclass == "HT_TriggerSubType_CloseMap":
        obj.color = (255, 0, 0, 255)


def process_triggers(data, links, parent, parent_id, parent_class):
    parent_subclass = None
    if 'subtype' in parent:
        parent_subclass = parent['subtype']

    if parent_class == "HT_TriggerType_CrossPlane":
        visualize_crossplane(data, parent, parent_id, parent_subclass)
