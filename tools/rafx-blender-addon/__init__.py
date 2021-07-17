bl_info = {
    "name" : "rafx_addon",
    "author" : "aclysma",
    "description" : "",
    "blender" : (2, 80, 0),
    "version" : (0, 0, 1),
    "location" : "",
    "warning" : "",
    "category" : "Generic"
}

def reload_package(module_dict_main):
    import importlib
    from pathlib import Path

    def reload_package_recursive(current_dir, module_dict):
        for path in current_dir.iterdir():
            if "__init__" in str(path) or path.stem not in module_dict:
                continue

            if path.is_file() and path.suffix == ".py":
                importlib.reload(module_dict[path.stem])
            elif path.is_dir():
                reload_package_recursive(path, module_dict[path.stem].__dict__)

    reload_package_recursive(Path(__file__).parent, module_dict_main)


if "bpy" in locals():
    reload_package(locals())

import bpy
import os
import logging
from bpy.props import StringProperty, PointerProperty, CollectionProperty, BoolProperty, IntProperty

from . import rafx_export_operators
from . import rafx_level_operators
from . import rafx_panels
from . import rafx_level_properties
from . import rafx_level_panels
from . import rafx_slice

#rafx_depsgraph_update_pre_driver_key = "RAFX_DEPSGRAPH_UPDATE_PRE"
#rafx_save_post_driver_key = "RAFX_SAVE_POST"
#rafx_load_post_driver_key = "RAFX_LOAD_POST"

#@bpy.app.handlers.persistent
#def rafx_depsgraph_update_pre(scene):
#    pass

# @bpy.app.handlers.persistent
# def rafx_save_post(context):
#     #print("auto-saving slices")
#     #rafx_level_operators.do_autosave_all()
            
#             # for slice in scene.rafx_level_properties.slices:
#             #     slice_collection = bpy.data.collections.get(slice.slice_path)
#             #     if slice_collection:
#             #         #abspath = bpy.path.abspath(slice.slice_path)
#             #         #abspath += ".txt"
#             #         #outfile = bpy.path.relpath(abspath)

#             #         rafx_slice.save_slice(slice_collection, slice.slice_path)


# @bpy.app.handlers.persistent
# def rafx_load_post(context):
#     #print("auto-loading slices")
#     #rafx_level_operators.do_autoreload_all()
#     # for scene in bpy.data.scenes:
#     #     if scene.rafx_level_properties.save_level_when_file_saved:
#     #         rafx_level_operators.do_save_all(scene)
            
#     #         # for slice in scene.rafx_level_properties.slices:
#     #         #     slice_collection = bpy.data.collections.get(slice.slice_path)
#     #         #     if slice_collection:
#     #         #         #abspath = bpy.path.abspath(slice.slice_path)
#     #         #         #abspath += ".txt"
#     #         #         #outfile = bpy.path.relpath(abspath)

#     #         #         rafx_slice.save_slice(slice_collection, slice.slice_path)

classes = (
    #
    # Level editing properties
    #
    rafx_level_properties.RafxSliceProperties,
    rafx_level_properties.RafxLevelProperties,

    #
    # Misc/Debug operators
    #
    #rafx_export_operators.RafxTestOp,
    #rafx_export_operators.RafxPrintProjectSettings,
    #rafx_export_operators.RafxFindExportPathForSelected,

    #
    # Export operators
    #
    rafx_export_operators.RafxExportImageOp,
    rafx_export_operators.RafxExportMaterialOp,
    rafx_export_operators.RafxExportMeshOp,
    rafx_export_operators.RafxExportSceneAsModelOp,
    rafx_export_operators.RafxExportSceneAsPrefabOp,
    rafx_export_operators.RafxExportAllOp,

    #
    # Level editing operators
    #
    rafx_level_operators.RafxLevelNewOp,
    rafx_level_operators.RafxLevelLoadOp,
    rafx_level_operators.RafxLevelSaveOp,
    rafx_level_operators.RafxLevelCloseOp,
    rafx_level_operators.RafxLevelReloadOp,
    rafx_level_operators.RafxLevelSliceLoad,
    #rafx_level_operators.RafxLevelSliceSave,
    rafx_level_operators.RafxLevelSliceUnload,
    rafx_level_operators.RafxLevelAddSliceOp,
    rafx_level_operators.RafxLevelRemoveSliceOp,
    rafx_level_operators.RafxLevelMoveSelectedToSlice,
    rafx_level_operators.RafxLevelMoveUnassignedToSlice,
    rafx_level_operators.RafxLevelHideSlice,
    rafx_level_operators.RafxLevelShowSlice,
    rafx_level_operators.RafxLevelSelectSlice,
    rafx_level_operators.RafxLevelDeselectSlice,
    rafx_level_operators.RafxLevelSaveLevelAs,
    rafx_level_operators.RafxLevelSaveSliceAs,

    #
    # Exporting UI
    #

    # These are in the "n" menu for the respective editing contexts
    rafx_panels.RafxImageEditorPanel,
    rafx_panels.RafxMaterialEditorPanel,
    rafx_panels.Rafx3DViewportEditorPanel,

    # Property panels
    rafx_panels.RafxPropertyPanelMaterial,
    rafx_panels.RafxPropertyPanelScene,

    #
    # Level editing UI
    #
    rafx_level_panels.RAFX_UL_level_ui_list,
    rafx_level_panels.Rafx3DViewportLevelPanel,
)

def unregister_handler(handler_list, handler_key):
    handler = bpy.app.driver_namespace.get(handler_key)
    if handler and handler in handler_list:
        #print("remove handler", handler)
        handler_list.remove(handler)

def register_handler(handler_list, handler_key, handler):
    unregister_handler(handler_list, handler_key)
    #print("register handler", handler)
    handler_list.append(handler)
    bpy.app.driver_namespace[handler_key] = handler


def register():
    logging.basicConfig(level=logging.WARN)
    logging.getLogger('werkzeug').setLevel(logging.WARN)
    logging.getLogger('rafx_addon').setLevel(logging.INFO)

    for c in classes:
        #print("register", c)
        bpy.utils.register_class(c)

    bpy.types.Collection.rafx_is_model = bpy.props.BoolProperty(name="Export Scene As Model", default=True)
    bpy.types.Collection.rafx_is_prefab = bpy.props.BoolProperty(name="Export Scene As Prefab", default=False)

    bpy.types.Scene.rafx_level_properties = PointerProperty(type=rafx_level_properties.RafxLevelProperties)

    #register_handler(bpy.app.handlers.depsgraph_update_pre, rafx_depsgraph_update_pre_driver_key, rafx_depsgraph_update_pre)
    #register_handler(bpy.app.handlers.save_post, rafx_save_post_driver_key, rafx_save_post)
    #register_handler(bpy.app.handlers.load_post, rafx_load_post_driver_key, rafx_load_post)
    

def unregister():
    #unregister_handler(bpy.app.handlers.depsgraph_update_pre, rafx_depsgraph_update_pre_driver_key)
    #unregister_handler(bpy.app.handlers.save_post, rafx_save_post_driver_key)
    #unregister_handler(bpy.app.handlers.load_post, rafx_load_post_driver_key)

    for c in classes:
        #print("unregister", c)
        bpy.utils.unregister_class(c)
    
    del bpy.types.Collection.rafx_is_model
    del bpy.types.Collection.rafx_is_prefab
    del bpy.types.Scene.rafx_level_properties
    
