import bpy
import logging
import os
import bpy_extras
import json

logging = logging.getLogger(__name__)

from . import export_material, export_mesh, export_prefab, export_image, rafx_project, rafx_blender_paths, rafx_utils, rafx_errors, export_model, rafx_slice, rafx_level
from .rafx_level import RafxLevel, RafxSlice

def get_slice_collection_by_path(slice_path):
    collection = bpy.data.collections.get(slice_path)
    return collection

def get_slice_collection(slice):
    return get_slice_collection_by_path(slice.slice_path)

def get_current_slice(context):
    slices = context.scene.rafx_level_properties.slices
    active_slice_index = context.scene.rafx_level_properties.active_slice
    if active_slice_index >= 0 and active_slice_index < len(slices):
        return context.scene.rafx_level_properties.slices[active_slice_index]

    return None

def get_current_slice_collection(context):    
    active_slice = get_current_slice(context)
    if active_slice:
        collection = get_slice_collection(active_slice)
        if collection:
            return collection

    return None

def try_get_current_slice_collection_or_report(self, context):    
    slices = context.scene.rafx_level_properties.slices
    active_slice_index = context.scene.rafx_level_properties.active_slice
    if active_slice_index >= 0 and active_slice_index < len(slices):
        active_slice = context.scene.rafx_level_properties.slices[active_slice_index]
        if active_slice:
            collection = get_slice_collection(active_slice)
            if collection:
                return collection
            else:
                self.report({'WARNING'}, "Could not find collection {}".format(active_slice))
        else:
            self.report({'WARNING'}, "Could not find active slice by index {}".format(active_slice_index))
    else:
        self.report({'WARNING'}, "No active slice")

    return None

def do_load_scene(scene, filepath):
    level_base_dir = os.path.dirname(filepath)
    relpath = bpy.path.relpath(filepath)
    level = rafx_level.load_level(relpath)

    scene.rafx_level_properties.slices.clear()
    for slice in level.slices:
        slice_path = slice.path
        f = os.path.join(level_base_dir, slice_path)
        f = bpy.path.relpath(f)

        item = scene.rafx_level_properties.slices.add()
        #item.should_slice_be_loaded = False
        item.slice_path = f
    
    scene.rafx_level_properties.level_path = relpath

def do_close_scene(scene, save_all):
    if save_all:
        do_save_scene(scene)
    
    level_props = scene.rafx_level_properties
    slices_to_unload = []
    for slice in level_props.slices:
        if bpy.data.collections.get(slice.slice_path):
            slices_to_unload.append(slice.slice_path)
    
    for slice_path in slices_to_unload:
        do_unload_slice(scene, slice_path, False)
    
    level_props.level_path = ""
    level_props.slices.clear()

def do_save_level_file(scene, level_path=None):
    level_properties = scene.rafx_level_properties
    if not level_path:
        level_path = level_properties.level_path

    slices = []
    for slice in level_properties.slices:
        slices.append(RafxSlice(slice.slice_path, {}))

    level = RafxLevel(slices, {})

    rafx_level.save_level(level_path, level)

def do_save_slice(slice, slice_path=None):
    if not slice_path:
        slice_path = slice.slice_path
    
    collection = get_slice_collection(slice)
    if collection:
        rafx_slice.save_slice(collection, slice_path)

def do_save_scene(scene):
    do_save_level_file(scene)

    level_properties = scene.rafx_level_properties
    for slice in level_properties.slices:
        do_save_slice(slice)

def do_load_slice(scene, slice_path):
    collection = bpy.data.collections.get(slice_path)
    if not collection:
        collection = bpy.data.collections.new(slice_path)
        scene.collection.children.link(collection)
        rafx_slice.load_slice(collection, slice_path)
        return True
    else:
        return False

def do_unload_slice(scene, slice_path, save_slice):
    collection = bpy.data.collections.get(slice_path)
    if collection:
        if save_slice:
            rafx_slice.save_slice(collection, slice_path)
        
        for obj in collection.objects:
            collection.objects.unlink(obj)

        bpy.data.collections.remove(collection)
        return True
    else:
        return False


def do_reload_scene(scene):
    level_path = scene.rafx_level_properties.level_path
    level_abspath = bpy.path.abspath(level_path)

    level_props = scene.rafx_level_properties
    slice_paths_to_load = []
    for slice in level_props.slices:
        if get_slice_collection(slice):
            slice_paths_to_load.append(slice.slice_path)

    do_close_scene(scene, False)
    do_load_scene(scene, level_abspath)

    for slice_path_to_load in slice_paths_to_load:
        for slice in level_props.slices:
            if slice.slice_path == slice_path_to_load:
                do_load_slice(scene, slice_path_to_load)
                break

def do_autosave_all():
    logging.info("AUTOSAVE ALL")
    for scene in bpy.data.scenes:
        if scene.rafx_level_properties.save_level_when_file_saved:
            do_save_scene(scene)


def do_autoreload_all():
    logging.info("AUTORELOAD ALL")
    for scene in bpy.data.scenes:
        do_reload_scene(scene)


class RafxLevelNewOp(bpy.types.Operator, bpy_extras.io_utils.ExportHelper):
    bl_idname = "scene.rafx_new_level_op"
    bl_label = "Create Level"

    filename_ext = ".level"
    filter_glob: bpy.props.StringProperty(
        default='*.level',
        options={'HIDDEN'}
    )

    def execute(self, context):
        logging.info("new level", self.filepath)
        level_path = self.filepath  

        # write a blank level
        empty_level = {
            "slices": []
        }

        empty_level_str = json.dumps(empty_level, indent=4)
        rafx_utils.write_string_to_file(level_path, empty_level_str)

        # add the slice to the level
        bpy.ops.scene.rafx_load_level_op(filepath=level_path)
        return {'FINISHED'}

class RafxLevelLoadOp(bpy.types.Operator, bpy_extras.io_utils.ImportHelper):
    bl_idname = "scene.rafx_load_level_op"
    bl_label = "Rafx: Load Level File"

    filename_ext = ".level"
    filter_glob: bpy.props.StringProperty(
        default='*.level',
        options={'HIDDEN'}
    )

    def execute(self, context):
        logging.info('RafxLevelLoadOp filepath={}'.format(self.filepath))
        
        do_load_scene(context.scene, self.filepath)
        
        return {'FINISHED'}


class RafxLevelSaveOp(bpy.types.Operator):
    bl_idname = "scene.rafx_save_level_op"
    bl_label = "Rafx: Save Level"

    @classmethod
    def poll(cls, context):
        if not context.scene.rafx_level_properties.level_path:
            return False

        return True

    def execute(self, context):
        logging.info("RafxLevelSaveOp")
        do_save_scene(context.scene)

        return {'FINISHED'}


class RafxLevelCloseOp(bpy.types.Operator):
    bl_idname = "scene.rafx_close_level_op"
    bl_label = "Rafx: Close Level"

    save_all: bpy.props.BoolProperty()

    @classmethod
    def poll(cls, context):
        if not context.scene.rafx_level_properties.level_path:
            return False

        return True
    
    def invoke(self, context, event):
        self.save_all = True
        return context.window_manager.invoke_props_dialog(self)

    def draw(self, context):
        row = self.layout
        row.prop(self, "save_all", text="Save Level and Loaded Slices")
    
    def execute(self, context):
        logging.info("RafxLevelCloseOp save_all={}".format(self.save_all))
        do_close_scene(context.scene, self.save_all)
        return {'FINISHED'}

class RafxLevelReloadOp(bpy.types.Operator):
    bl_idname = "scene.rafx_reload_level_op"
    bl_label = "Rafx: Reload Level"

    @classmethod
    def poll(cls, context):
        if not context.scene.rafx_level_properties.level_path:
            return False

        return True
    
    def invoke(self, context, event):
        return context.window_manager.invoke_confirm(self, event)
    
    def execute(self, context):
        do_reload_scene(context.scene)
        
        return {'FINISHED'}

class RafxLevelSliceLoad(bpy.types.Operator):
    bl_idname = "scene.rafx_load_slice_op"
    bl_label = "Rafx: Load Slice"

    slice_path: bpy.props.StringProperty(
        default="",
        options={'HIDDEN'}
    )

    def execute(self, context):
        if not do_load_slice(context.scene, self.slice_path):
            self.report({"INFO"}, "Slice {} already loaded".format(self.slice_path))

        return {'FINISHED'}

class RafxLevelSliceUnload(bpy.types.Operator):
    bl_idname = "scene.rafx_unload_slice_op"
    bl_label = "Rafx: Unload Slice"

    slice_path: bpy.props.StringProperty(
        default="",
        options={'HIDDEN'}
    )
    save_slice: bpy.props.BoolProperty()

    def invoke(self, context, event):
        self.save_slice = True
        return context.window_manager.invoke_props_dialog(self)

    def draw(self, context):
        row = self.layout
        row.prop(self, "save_slice", text="Save")

    def execute(self, context):
        if not do_unload_slice(context.scene, self.slice_path, self.save_slice):
            self.report({"WARNING"}, "Did not find collection for slice {}, not saved".format(self.slice_path))

        return {'FINISHED'}


class RafxLevelAddSliceOp(bpy.types.Operator, bpy_extras.io_utils.ExportHelper):
    bl_idname = "scene.rafx_add_slice"
    bl_label = "Add Slice"
    bl_options = {'REGISTER', 'INTERNAL'}

    filename_ext = ".slice"
    filter_glob: bpy.props.StringProperty(
        default='*.slice',
        options={'HIDDEN'}
    )

    def execute(self, context):
        logging.info("add slice", self.filepath)
        slice_path = self.filepath

        # unload it without saving, if it already existed/is loaded
        relpath = bpy.path.relpath(slice_path)
        if get_slice_collection_by_path(relpath):
            bpy.ops.scene.rafx_unload_slice_op(slice_path=relpath,save_slice=False)
        
        if not os.path.exists(slice_path):
            # write a blank slice
            empty_slice = {
                "contents": []
            }

            empty_slice_str = json.dumps(empty_slice, indent=4)
            rafx_utils.write_string_to_file(slice_path, empty_slice_str)

        # add the slice to the level
        item = context.scene.rafx_level_properties.slices.add()
        logging.info("path is", relpath)
        item.slice_path = relpath

        # load the empty slice
        bpy.ops.scene.rafx_load_slice_op(slice_path=relpath)

        do_save_level_file(context.scene)
        return {'FINISHED'}


# Need a confirm prompt for this
class RafxLevelRemoveSliceOp(bpy.types.Operator):

    bl_idname = "scene.rafx_remove_slice"
    bl_label = "Delete Slice"

    delete_slice_file: bpy.props.BoolProperty()

    def invoke(self, context, event):
        self.delete_slice_file = False
        return context.window_manager.invoke_props_dialog(self)

    def draw(self, context):
        row = self.layout
        row.prop(self, "delete_slice_file", text="Delete the Slice File")
        
    def execute(self, context):
        active_slice_index = context.scene.rafx_level_properties.active_slice
        slices = context.scene.rafx_level_properties.slices

        if active_slice_index >= 0 and active_slice_index < len(slices):
            slice_path = slices[active_slice_index].slice_path
            do_unload_slice(context.scene, slice_path, False)
            slices.remove(active_slice_index)
        
            # delete the slice file
            if self.delete_slice_file:
                slice_abspath = bpy.path.abspath(slice_path)
                logging.info("DELETE FILE", slice_abspath)
                os.remove(slice_abspath)

            # save the level file
            do_save_level_file(context.scene)

        return {'FINISHED'}


def poll_move_to_slice(cls, context):
    active_slice_index = context.scene.rafx_level_properties.active_slice
    slices = context.scene.rafx_level_properties.slices
    if active_slice_index >= 0 and active_slice_index < len(slices):
        active_slice = context.scene.rafx_level_properties.slices[active_slice_index]
        if active_slice:
            collection = bpy.data.collections.get(active_slice.slice_path)
            if collection:
                return True
    
    return False

class RafxLevelMoveSelectedToSlice(bpy.types.Operator):
    bl_idname = "object.rafx_level_move_selected_to_slice"
    bl_label = "Rafx: Move Selected Hierarchy To Active Slice"

    @classmethod
    def poll(cls, context):
        if not context.selected_objects:
            return False
        
        return poll_move_to_slice(cls, context)

    def execute(self, context):            
        collection = try_get_current_slice_collection_or_report(self, context)
        if collection:
            rafx_slice.move_selected_to_slice(context, collection)

        return {'FINISHED'}

class RafxLevelMoveUnassignedToSlice(bpy.types.Operator):
    bl_idname = "object.rafx_level_move_unassigned_to_slice"
    bl_label = "Rafx: Move Selected Hierarchy To Active Slice"

    @classmethod
    def poll(cls, context):
        if not len(context.scene.collection.objects) > 0:
            return False

        return poll_move_to_slice(cls, context)

    def execute(self, context):
        # Get the slice we will move everything into and show errors if we can't find it
        collection = try_get_current_slice_collection_or_report(self, context)
        if collection:
            # Select all unassigned things
            bpy.ops.object.select_all(action='DESELECT')
            for object in context.scene.collection.objects:
                object.select_set(True)
            
            # Move them all
            rafx_slice.move_selected_to_slice(context, collection)
                
        return {'FINISHED'}

def do_set_slice_visible(self, context, visible):
    collection = try_get_current_slice_collection_or_report(self, context)

    vl = context.view_layer
    vl_collection = vl.layer_collection.children.get(collection.name)
    if vl_collection:
        vl_collection.hide_viewport = not visible

class RafxLevelHideSlice(bpy.types.Operator):
    bl_idname = "object.rafx_level_hide_slice"
    bl_label = "Rafx: Hide Slice"

    @classmethod
    def poll(cls, context):
        if get_current_slice_collection(context):
            return True
        
        return False

    def execute(self, context):
        do_set_slice_visible(self, context, False)

        return {'FINISHED'}


class RafxLevelShowSlice(bpy.types.Operator):
    bl_idname = "object.rafx_level_show_slice"
    bl_label = "Rafx: Show Slice"

    @classmethod
    def poll(cls, context):
        if get_current_slice_collection(context):
            return True
        
        return False

    def execute(self, context):
        do_set_slice_visible(self, context, True)

        return {'FINISHED'}

def do_select_slice(self, context, selected):
    collection = try_get_current_slice_collection_or_report(self, context)
    for object in collection.all_objects:
        object.select_set(selected)


class RafxLevelSelectSlice(bpy.types.Operator):
    bl_idname = "object.rafx_level_select_slice"
    bl_label = "Rafx: Select Slice"

    @classmethod
    def poll(cls, context):
        if get_current_slice_collection(context):
            return True
        
        return False

    def execute(self, context):
        do_select_slice(self, context, True)

        return {'FINISHED'}


class RafxLevelDeselectSlice(bpy.types.Operator):
    bl_idname = "object.rafx_level_deselect_slice"
    bl_label = "Rafx: Deselect Slice"

    @classmethod
    def poll(cls, context):
        if get_current_slice_collection(context):
            return True
        
        return False

    def execute(self, context):
        do_select_slice(self, context, False)

        return {'FINISHED'}

class RafxLevelSaveLevelAs(bpy.types.Operator, bpy_extras.io_utils.ExportHelper):
    bl_idname = "scene.rafx_level_save_as"
    bl_label = "Rafx: Save Level As"

    filename_ext = ".level"
    filter_glob: bpy.props.StringProperty(
        default='*.level',
        options={'HIDDEN'}
    )

    @classmethod
    def poll(cls, context):
        if context.scene.rafx_level_properties.level_path:
            return True
        
        return False

    def execute(self, context):
        relpath = bpy.path.relpath(self.filepath)
        do_save_level_file(context.scene, relpath)
        context.scene.rafx_level_properties.level_path = relpath

        return {'FINISHED'}


class RafxLevelSaveSliceAs(bpy.types.Operator, bpy_extras.io_utils.ExportHelper):
    bl_idname = "scene.rafx_level_slice_save_as"
    bl_label = "Rafx: Save Slice As"

    filename_ext = ".slice"
    filter_glob: bpy.props.StringProperty(
        default='*.slice',
        options={'HIDDEN'}
    )

    @classmethod
    def poll(cls, context):
        if get_current_slice_collection(context):
            return True
        
        return False

    def execute(self, context):
        relpath = bpy.path.relpath(self.filepath)
        current_collection = collection = get_current_slice_collection(context)
        
        slice = get_current_slice(context)
        do_save_slice(slice, relpath)
        slice.slice_path = relpath
        current_collection.name = relpath

        do_save_level_file(context.scene)

        return {'FINISHED'}
