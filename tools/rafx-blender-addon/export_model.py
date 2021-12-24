
import bpy
import mathutils
import json
import logging
import os

logging = logging.getLogger(__name__)

from . import rafx_blender_paths, rafx_errors, rafx_utils, export_mesh
from .rafx_export_types import ExportContext

# Ideally there is just 1, but we return a list so caller can do their own error handling
def find_lod0_collection_names(scene: bpy.types.Scene, throw_if_none: bool, throw_if_many: bool) -> list[str]:
    scene_collection = scene.collection
    lod0_collection_names = []
    if len(scene_collection.children) == 1:
        lod0_collection_names.append(scene_collection.children[0].name)
    else:
        for collection in scene_collection.children:
            if collection.name.lower().endswith("lod0"):
                if throw_if_many and len(lod0_collection_names) > 0:
                    error_string = "The scene {} is configured to export as a model, has multiple collections for the same lod (LOD0)".format(scene)
                    logging.error(error_string)
                    raise rafx_errors.RafxModenlSceneInvalidLodCollections(error_string)

                lod0_collection_names.append(collection.name)
    
    if throw_if_none and len(lod0_collection_names) == 0:
        error_string = "The scene {} is configured to export as a model, has no LOD0 collection".format(scene)
        logging.error(error_string)
        raise rafx_errors.RafxModelSceneInvalidLodCollections(error_string)

    return lod0_collection_names

def export(export_context: ExportContext, scene: bpy.types.Scene):
    if not export_context.visit_scene_as_model(scene):
        return

    log_str = "Exporting scene {} as model".format(scene.name_full)
    export_context.info(log_str)

    export_path = rafx_blender_paths.find_export_path_for_blender_data_block(export_context.project_settings, scene)
    export_dir = os.path.dirname(export_path)
    scene_collection = scene.collection
    if not scene_collection.rafx_is_model:
        error_string = "Scene {} is not configured to export as a model".format(scene)
        logging.error(error_string)
        raise rafx_errors.RafxSceneIsNotAModel(error_string)

    if len(scene_collection.objects) > 0:
        error_string_format = "The scene {} is configured to export as a model, but an object {} was found in the scene collection. " \
            " All objects must be part of a collection for a specific LOD"
        error_string = error_string_format.format(scene, scene_collection.objects[0])
        logging.error(error_string)
        raise rafx_errors.RafxModelSceneHasRootObject(error_string)
            
    if len(scene_collection.children) == 0:
        error_string = "The scene {} is configured to export as a model, has no collections. Exactly one collection is required".format(scene)
        logging.error(error_string)
        raise rafx_errors.RafxModelSceneHasNoCollections(error_string)

    # Either assume there is exactly one collection, or that there is a collection ending with "LOD0"
    lod0_collection_name = find_lod0_collection_names(scene, True, True)[0]

    if not lod0_collection_name:
        error_string = "The scene {} is configured to export as a model, has no LOD0 collection".format(scene)
        logging.error(error_string)
        raise rafx_errors.RafxModelSceneInvalidLodCollections(error_string)

    collection = scene_collection.children[lod0_collection_name]

    if len(collection.objects) > 1:
        # Alert the problem but export what we can
        error_string = "The collection {} in scene {} has more than one object. Multiple meshes in a LOD not yet supported. Only the first mesh object will be exported.".format(collection, scene)
        export_context.error(error_string)
        #raise rafx_errors.RafxModelSceneCollectionHasMultipleMeshes(error_string)
    
    if len(collection.objects) == 0:
        error_string = "The collection {} in scene {} has no object. Exactly one mesh object is required".format(collection, scene)
        logging.error(error_string)
        raise rafx_errors.RafxModelSceneCollectionHasNoMeshes(error_string)

    if collection.objects[0].type != 'MESH':
        error_string = "The object in collection {} in scene {} is not a mesh object. Exactly one mesh object is required".format(collection, scene)
        logging.error(error_string)
        raise rafx_errors.RafxModelSceneCollectionHasNoMeshes(error_string)

    #TODO: Do I need to adjust for objects transform?
    #TODO: If we ever support more than one collection/object, we need to update 
    object = collection.objects[0]

    mesh_path = rafx_blender_paths.find_export_path_for_blender_data_block(export_context.project_settings, object)
    mesh_path = rafx_blender_paths.make_cross_platform_relative_path(mesh_path, export_dir)
    
    model = {
        "lods": [{
            "mesh": mesh_path
        }]
    }
    
    model_as_json = json.dumps(model, indent=4)
    rafx_utils.write_string_to_file(export_path, model_as_json)

def export_model_for_mesh_object(export_context: ExportContext, object: bpy.types.Object):
    if not export_context.visit_single_mesh_as_model(object):
        return

    export_path = rafx_blender_paths.find_export_path_for_blender_data_block_with_extension(export_context.project_settings, object, "blender_model")
    export_dir = os.path.dirname(export_path)

    #TODO: Verify no children, or correctly iterate them

    mesh_path = rafx_blender_paths.find_export_path_for_blender_data_block(export_context.project_settings, object)
    mesh_path = rafx_blender_paths.make_cross_platform_relative_path(mesh_path, export_dir)

    model = {
        "lods": [{
            "mesh": mesh_path
        }]
    }

    model_as_json = json.dumps(model, indent=4)
    rafx_utils.write_string_to_file(export_path, model_as_json)
