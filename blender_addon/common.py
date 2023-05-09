import bpy


def relink_object(object: bpy.types.Object, new_collection: bpy.types.Collection):
    # Unlink object from all collections and link it to a new one
    for c in object.users_collection:
        c.objects.unlink(object)

    new_collection.objects.link(object)
