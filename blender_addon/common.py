import bpy


def relink_object(object: bpy.types.Object, new_collection: bpy.types.Collection):
    # Unlink object from all collections and link it to a new one
    for c in object.users_collection:
        c.objects.unlink(object)

    new_collection.objects.link(object)


def create_srgb_node_group():
    if 'srgbApprox' in bpy.data.node_groups:
        return

    group = bpy.data.node_groups.new('srgbApprox', 'ShaderNodeTree')
    group_inputs = group.nodes.new('NodeGroupInput')
    group_inputs.location = (-350, 0)
    group.inputs.new('NodeSocketColor', 'color_input')

    group_outputs = group.nodes.new('NodeGroupOutput')
    group_outputs.location = (350, 0)
    group.outputs.new('NodeSocketColor', 'color_output')

    split_rgb = group.nodes.new('ShaderNodeSeparateRGB')
    split_rgb.location = (-200, 0)

    power_r = group.nodes.new('ShaderNodeMath')
    power_r.operation = 'POWER'
    power_r.location = (0, 100)
    power_r.inputs[1].default_value = 2.2

    power_g = group.nodes.new('ShaderNodeMath')
    power_g.operation = 'POWER'
    power_g.location = (0, 0)
    power_g.inputs[1].default_value = 2.2

    power_b = group.nodes.new('ShaderNodeMath')
    power_b.operation = 'POWER'
    power_b.location = (0, -100)
    power_b.inputs[1].default_value = 2.2

    combine_rgb = group.nodes.new('ShaderNodeCombineRGB')
    combine_rgb.location = (200, 0)

    group.links.new(group_inputs.outputs[0], split_rgb.inputs[0])
    group.links.new(split_rgb.outputs[0], power_r.inputs[0])
    group.links.new(split_rgb.outputs[1], power_g.inputs[0])
    group.links.new(split_rgb.outputs[2], power_b.inputs[0])
    group.links.new(power_r.outputs[0], combine_rgb.inputs[0])
    group.links.new(power_g.outputs[0], combine_rgb.inputs[1])
    group.links.new(power_b.outputs[0], combine_rgb.inputs[2])
    group.links.new(combine_rgb.outputs[0], group_outputs.inputs[0])
