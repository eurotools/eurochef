from . import ecm

bl_info = {
    "name": "Eurochef Utility",
    "author": "cohaereo",
    "description": "",
    "blender": (2, 80, 0),
    "version": (0, 0, 1),
    "location": "File -> Import",
    "category": "Import-Export"
}


def register():
    ecm.register()


def unregister():
    ecm.unregister()
