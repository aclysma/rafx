import bpy
import logging
import os

logging = logging.getLogger(__name__)

from . import export_material, export_mesh, export_prefab, export_image, export_animation, rafx_project, rafx_blender_paths, rafx_utils, rafx_errors, export_model, rafx_export_properties
from .rafx_export_types import ExportContext, ObjectKey

def do_export_image(export_context: ExportContext, image: bpy.types.Image, ignore_export_properties: bool):
    try:
        if ignore_export_properties or export_context.export_properties.enable_image_export:
            export_image.export(export_context, image)
    except rafx_errors.RafxError as e:
        error_str = "Failed to export {}: {}".format(image.name_full, str(e))
        export_context.error(error_str)


def do_export_material(export_context: ExportContext, material: bpy.types.Material, ignore_export_properties: bool):   
    try:
        if ignore_export_properties or export_context.export_properties.enable_material_export:
            export_material.export(export_context, material)
    except rafx_errors.RafxError as e:
        error_str = "Failed to export {}: {}".format(material.name_full, str(e))
        export_context.error(error_str)


def do_export_mesh(export_context: ExportContext, object: bpy.types.Object, ignore_export_properties: bool):
    try:
        if ignore_export_properties or export_context.export_properties.enable_mesh_export:
            export_mesh.export(export_context, object)
    except rafx_errors.RafxError as e:
        error_str = "Failed to export {}: {}".format(object.name_full, str(e))
        export_context.error(error_str)


def do_export_animation_data(export_context: ExportContext, object: bpy.types.Object, ignore_export_properties: bool):
    try:
        if ignore_export_properties or export_context.export_properties.enable_animation_export:
            export_animation.export_animation_data(export_context, object)
    except rafx_errors.RafxError as e:
        error_str = "Failed to export {}: {}".format(object.name_full, str(e))
        export_context.error(error_str)


def do_export_scene_as_prefab(export_context: ExportContext, scene: bpy.types.Scene, ignore_export_properties: bool):
    try:
        if ignore_export_properties or export_context.export_properties.enable_prefab_export:
            export_prefab.export(export_context, scene)
    except rafx_errors.RafxError as e:
        error_str = "Failed to export {}: {}".format(scene.name_full, str(e))
        export_context.error(error_str)


def do_export_scene_as_model(export_context: ExportContext, scene: bpy.types.Scene, ignore_export_properties: bool):
    if ignore_export_properties or export_context.export_properties.enable_model_export:
        try:
            export_model.export(export_context, scene)
        except rafx_errors.RafxError as e:
            error_str = "Failed to export {}: {}".format(scene.name_full, str(e))
            export_context.error(error_str)

    if ignore_export_properties or export_context.export_properties.enable_mesh_export:
        lod0_collection_names = export_model.find_lod0_collection_names(scene, True, True)

        for collection in scene.collection.children:
            # for now only export lod0
            if not collection.name in lod0_collection_names:
                continue

            print(collection.name)
            for object in collection.objects:
                if object.type == "EMPTY":
                    continue
                do_export_mesh(export_context, object, ignore_export_properties)

class RafxExportImageOp(bpy.types.Operator):
    bl_idname = "object.rafx_export_current_image"
    bl_label = "Rafx: Export Current Image"

    @classmethod
    def poll(cls, context):
        if not hasattr(context, "edit_image"):
            return False
        return context.edit_image

    def execute(self, context):
        export_context = ExportContext(self, context)
        image = context.edit_image
        do_export_image(export_context, image, True)

        return {'FINISHED'}

class RafxExportMaterialOp(bpy.types.Operator):
    bl_idname = "object.rafx_export_current_material"
    bl_label = "Rafx: Export Current Material"
    
    @classmethod
    def poll(cls, context):
        if not hasattr(context, "material"):
            return False
        return context.material

    def execute(self, context):
        export_context = ExportContext(self, context)
        material = context.material
        do_export_material(export_context, material, True)

        return {'FINISHED'}

class RafxExportMeshOp(bpy.types.Operator):
    bl_idname = "object.rafx_export_current_mesh"
    bl_label = "Rafx: Export Current Mesh"

    @classmethod
    def poll(cls, context):
        return context.active_object \
            and context.active_object.type == 'MESH' \
            and context.active_object.data

    def execute(self, context):
        export_context = ExportContext(self, context)
        object = context.active_object
        do_export_mesh(export_context, object, True)

        return {'FINISHED'}
    
class RafxExportAnimationDataOp(bpy.types.Operator):
    bl_idname = "object.rafx_export_animation_data"
    bl_label = "Rafx: Export Animation Data"

    @classmethod
    def poll(cls, context):
        return context.active_object \
            and context.active_object.type == 'ARMATURE' \
            and context.active_object.data
    
    def execute(self, context):
        export_context = ExportContext(self, context)
        object = context.active_object
        do_export_animation_data(export_context, object, True)

        return {'FINISHED'}

# class RafxExportAllAnimationDataOp(bpy.types.Operator):
#     bl_idname = "object.rafx_export_all_animation_data"
#     bl_label = "Rafx: Export Animation Data"

#     def execute(self, context):
#         project_settings = rafx_blender_paths.find_project_settings_for_current_blender_file()

#         for armature in bpy.data.armatures:
#             do_export_armature(self, armature, project_settings)
            
#         for action in bpy.data.actions:
#             do_export_action(self, action, project_settings)

#         return {'FINISHED'}


class RafxExportSceneAsPrefabOp(bpy.types.Operator):
    bl_idname = "object.rafx_export_current_scene_as_prefab"
    bl_label = "Rafx: Export Current Scene as Prefab"

    @classmethod
    def poll(cls, context):
        return context.scene.collection.rafx_is_prefab

    def execute(self, context):
        export_context = ExportContext(self, context)
        scene = context.scene
        do_export_scene_as_prefab(export_context, scene, True)
        
        return {'FINISHED'}

class RafxExportSceneAsModelOp(bpy.types.Operator):
    bl_idname = "object.rafx_export_current_scene_as_model"
    bl_label = "Rafx: Export Current Scene as Model"

    @classmethod
    def poll(cls, context):
        return context.scene.collection.rafx_is_model

    def execute(self, context):
        export_context = ExportContext(self, context)
        scene = context.scene
        #scene_collection = bpy.context.scene.collection
        do_export_scene_as_model(export_context, scene, True)
        
        return {'FINISHED'}

def do_export_external_model(export_context: ExportContext, collection: bpy.types.Collection, ignore_export_properties: bool):
    assert(collection.library)

    # Rename the scenes so we don't have any ID collisions
    #for scene in bpy.data.scenes:
    #    if not scene.library:
    #        scene.original_scene_name = scene.name
    #        scene.name = "{}_TEMP_RENAME".format(scene.name)

    # Find any scenes that are already linked so we don't unlink them
    linked_scenes_from_library = set()
    for scene in bpy.data.scenes:
        if scene.library and scene.library == collection.library:
            linked_scenes_from_library.add(scene.name)

    # Link all scenes from the given blend file
    library_path = bpy.path.abspath(collection.library.filepath)
    with bpy.data.libraries.load(library_path, link=True) as (data_from, data_to):
        data_to.scenes = data_from.scenes
    
    found = False
    for scene in bpy.data.scenes:
        if scene.library == collection.library:
            for child_collection in scene.collection.children:
                if child_collection == collection: 
                    # The passed in collection matched this scene. If it's configured to export as a model,
                    # export all LODs
                    if scene.collection.rafx_is_model:
                        if ignore_export_properties or export_context.export_properties.enable_model_export:
                            do_export_scene_as_model(export_context, scene, ignore_export_properties)
                    
                        if ignore_export_properties or export_context.export_properties.enable_mesh_export:
                            for object in child_collection.children:
                                if object.type == "EMPTY":
                                    continue
                                do_export_mesh(export_context, object, ignore_export_properties)
                    
                    found = True
                    break

        if found:
            break

    scenes_to_unlink = []
    for scene in bpy.data.scenes:
        if scene.library and scene.library.name not in linked_scenes_from_library:
            scenes_to_unlink.append(scene)
    
    for scene_to_unlink in scenes_to_unlink:
        bpy.data.scenes.remove(scene_to_unlink)
    
    #for scene in bpy.data.scenes:
    #    if not scene.library:
    #        scene.name = scene.original_scene_name
    #        scene.original_scene_name = ""


class RafxExportAllOp(bpy.types.Operator):
    bl_idname = "object.rafx_export_all"
    bl_label = "Rafx: Export All"

    def execute(self, context):
        export_context = ExportContext(self, context)
        export_context.info("-----Export All-----")

        scene = context.scene

        if export_context.export_properties.enable_image_export:
            export_context.info("Exporting images")
            for image in bpy.data.images:
                if image.users != 0 or image.use_fake_user:
                    do_export_image(export_context, image, False)
        else:
            export_context.info("Image export diabled")
        

        if export_context.export_properties.enable_material_export:
            export_context.info("Exporting materials")
            for material in bpy.data.materials:
                if material.users != 0 or material.use_fake_user:
                    do_export_material(export_context, material, False)
        else:
            export_context.info("Material export diabled")

        
        # models stored in this blend file can be found by iterating scenes
        if export_context.export_properties.enable_model_export:
            export_context.info("Exporting models")
            for collection in bpy.data.collections:
                if collection.library and collection.rafx_is_model:
                    do_export_external_model(export_context, collection, False)
        else:
            export_context.info("Model export diabled")

        for scene in bpy.data.scenes:
            export_context.info("Exporting scenes")

            if scene.collection.rafx_is_model:
                do_export_scene_as_model(export_context, scene, False)

            elif scene.collection.rafx_is_prefab:
                do_export_scene_as_prefab(export_context, scene, False)
        
        export_context.info(export_context.summary_text())

        return {'FINISHED'}



# Useless stub operators that's good for quickly testing/debugging something

# class RafxFindExportPathForSelected(bpy.types.Operator):
#     bl_idname = "object.rafx_find_export_path_for_selected"
#     bl_label = "Rafx: Find Export Path For Selected"

#     @classmethod
#     def poll(cls, context):
#         return context.active_object

#     def execute(self, context):
#         project_settings = rafx_blender_paths.find_project_settings_for_current_blender_file()
#         path = rafx_blender_paths.find_export_path_for_blender_data_block(project_settings, context.active_object)
#         print(path)
#         return {'FINISHED'}

# class RafxPrintProjectSettings(bpy.types.Operator):
#     bl_idname = "object.rafx_print_project_settings"
#     bl_label = "Rafx: Print Project Settings"

#     def execute(self, context):
#         project_settings = rafx_blender_paths.find_project_settings_for_current_blender_file()
#         print(project_settings)
#         return {'FINISHED'}

# class RafxTestOp(bpy.types.Operator):
#     bl_idname = "object.rafx_test_op"
#     bl_label = "Rafx: TestOp"

#     def execute(self, context):
#         project_settings = rafx_blender_paths.find_project_settings_for_current_blender_file()
#         obj = context.active_object

#         return {'FINISHED'}
