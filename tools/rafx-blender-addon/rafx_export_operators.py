import bpy
import logging
import os

logging = logging.getLogger(__name__)

from . import export_material, export_mesh, export_prefab, export_image, export_animation, rafx_project, rafx_blender_paths, rafx_utils, rafx_errors, export_model


def do_export_image(op, image, project_settings):
    try:
        log_str = "Exporting image {}".format(image.name_full)
        op.report({'INFO'}, log_str)
        logging.info(log_str)
        export_image.export(image, project_settings)
    except rafx_errors.RafxError as e:
        error_str = "Failed to export {}: {}".format(image.name_full, str(e))
        op.report({"ERROR"}, error_str)

def do_export_material(op, material, project_settings):
    if material.name == "Dots Stroke" and not material.use_nodes:
        op.report({'WARNING'}, "Ignoring default material 'Dots Stroke', it is not using nodes")
        return
    
    try:
        log_str = "Exporting material {}".format(material.name_full)
        op.report({'INFO'}, log_str)
        logging.info(log_str)
        export_material.export(material, project_settings)
    except rafx_errors.RafxError as e:
        error_str = "Failed to export {}: {}".format(material.name_full, str(e))
        op.report({"ERROR"}, error_str)

def do_export_mesh(op, object, project_settings):
    try:
        log_str = "Exporting mesh {}".format(object.name_full)
        op.report({'INFO'}, log_str)
        logging.info(log_str)
        export_mesh.export(object, project_settings)
    except rafx_errors.RafxError as e:
        error_str = "Failed to export {}: {}".format(object.name_full, str(e))
        op.report({"ERROR"}, error_str)



def do_export_animation_data(op, object, project_settings):
    try:
        log_str = "Exporting animation data {}".format(object.name_full)
        op.report({'INFO'}, log_str)
        logging.info(log_str)
        export_animation.export_animation_data(object, project_settings)
    except rafx_errors.RafxError as e:
        error_str = "Failed to export {}: {}".format(object.name_full, str(e))
        op.report({"ERROR"}, error_str)


# def do_export_armature(op, object, project_settings):
#     try:
#         log_str = "Exporting armature {}".format(object.name_full)
#         op.report({'INFO'}, log_str)
#         logging.info(log_str)
#         export_animation.export_armature(object, project_settings)
#     except rafx_errors.RafxError as e:
#         error_str = "Failed to export {}: {}".format(object.name_full, str(e))
#         op.report({"ERROR"}, error_str)

# def do_export_action(op, object, project_settings):
#     try:
#         log_str = "Exporting action {}".format(object.name_full)
#         op.report({'INFO'}, log_str)
#         logging.info(log_str)
#         export_animation.export_action(object, project_settings)
#     except rafx_errors.RafxError as e:
#         error_str = "Failed to export {}: {}".format(object.name_full, str(e))
#         op.report({"ERROR"}, error_str)




def do_export_scene_as_prefab(op, scene, project_settings):
    try:
        log_str = "Exporting scene {} as prefab".format(scene.name_full)
        op.report({'INFO'}, log_str)
        logging.info(log_str)
        export_prefab.export(scene, project_settings)
    except rafx_errors.RafxError as e:
        error_str = "Failed to export {}: {}".format(scene.name_full, str(e))
        op.report({"ERROR"}, error_str)

def do_export_scene_as_model(op, scene: bpy.types.Scene, project_settings):
    try:
        log_str = "Exporting scene {} as model".format(scene.name_full)
        op.report({'INFO'}, log_str)
        logging.info(log_str)
        export_model.export(scene, project_settings)
    except rafx_errors.RafxError as e:
        error_str = "Failed to export {}: {}".format(scene.name_full, str(e))
        op.report({"ERROR"}, error_str)
    
    for collection in scene.collection.children:
        for object in collection.objects:
            do_export_mesh(op, object, project_settings)


class RafxExportImageOp(bpy.types.Operator):
    bl_idname = "object.rafx_export_current_image"
    bl_label = "Rafx: Export Current Image"

    @classmethod
    def poll(cls, context):
        if not hasattr(context, "edit_image"):
            return False
        return context.edit_image

    def execute(self, context):
        image = context.edit_image
        project_settings = rafx_blender_paths.find_project_settings_for_current_blender_file()
        do_export_image(self, image, project_settings)

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
        material = context.material
        project_settings = rafx_blender_paths.find_project_settings_for_current_blender_file()
        do_export_material(self, material, project_settings)

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
        object = context.active_object
        project_settings = rafx_blender_paths.find_project_settings_for_current_blender_file()
        do_export_mesh(self, object, project_settings)

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
        object = context.active_object
        project_settings = rafx_blender_paths.find_project_settings_for_current_blender_file()
        do_export_animation_data(self, object, project_settings)

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
        scene = context.scene
        project_settings = rafx_blender_paths.find_project_settings_for_current_blender_file()
        do_export_scene_as_prefab(self, scene, project_settings)
        
        return {'FINISHED'}

class RafxExportSceneAsModelOp(bpy.types.Operator):
    bl_idname = "object.rafx_export_current_scene_as_model"
    bl_label = "Rafx: Export Current Scene as Model"

    @classmethod
    def poll(cls, context):
        return context.scene.collection.rafx_is_model

    def execute(self, context):
        scene = context.scene
        project_settings = rafx_blender_paths.find_project_settings_for_current_blender_file()
        #scene_collection = bpy.context.scene.collection
        do_export_scene_as_model(self, scene, project_settings)
        
        return {'FINISHED'}

def do_export_external_model(op, collection: bpy.types.Collection, project_settings, exported_scenes):
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
                        key = (scene.name, scene.library.filepath)
                        if key not in exported_scenes:
                            exported_scenes.add(key)

                            do_export_scene_as_model(op, scene, project_settings)
                        
                            for object in child_collection.children:
                                do_export_mesh(op, object, project_settings)
                        else:
                            logging.info("skipping", key, "it's already exported")
                    
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
        project_settings = rafx_blender_paths.find_project_settings_for_current_blender_file()

        image_count = 0
        material_count = 0
        model_count = 0
        prefab_count = 0

        logging.info("Exporting images")
        for image in bpy.data.images:
            if image.users != 0 or image.use_fake_user:
                do_export_image(self, image, project_settings)
                image_count += 1
        
        logging.info("Exporting materials")
        for material in bpy.data.materials:
            if material.users != 0 or material.use_fake_user:
                do_export_material(self, material, project_settings)
                material_count += 1
        
        logging.info("Exporting models and scenes")
        # models stored in this blend file can be found by iterating scenes
        exported_scenes = set()
        for collection in bpy.data.collections:
            if collection.library and collection.rafx_is_model:
                do_export_external_model(self, collection, project_settings, exported_scenes)
                model_count += 1

        for scene in bpy.data.scenes:
            if scene.collection.rafx_is_model:
                do_export_scene_as_model(self, scene, project_settings)
                model_count += 1

            elif scene.collection.rafx_is_prefab:
                do_export_scene_as_prefab(self, scene, project_settings)
                
                prefab_count += 1
        
        log_string = "exported {} images, {} materials, {} models, and {} prefabs".format(image_count, material_count, model_count, prefab_count)
        self.report({'INFO'}, log_string)
        logging.info(log_string)

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