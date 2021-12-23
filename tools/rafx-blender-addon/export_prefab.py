
import bpy
import mathutils
import json
import logging
import os

logging = logging.getLogger(__name__)

from . import rafx_blender_paths, rafx_errors, rafx_utils

def object_common_attributes(object):
    attributes = {}

    attributes['position'] = object.location
    if object.rotation_mode == 'QUATERNION':
        attributes['rotation'] = object.rotation_quaternion
    elif object.rotation_mode == 'AXIS_ANGLE':
        attributes['rotation'] = mathutils.Quaternion(object.rotation_axis_angle[1:4], object.rotation_axis_angle[0])
    else:
        attributes['rotation'] = object.rotation_euler.to_quaternion()

    attributes['scale'] = object.scale
    return attributes

def iterate_object(project_settings, export_dir, out_objects, object: bpy.types.Object, transform: mathutils.Matrix):
    transform = transform @ object.matrix_basis
    
    t = transform.translation
    r = transform.to_quaternion()
    s = transform.to_scale()
    transform_attributes = {
        "position": [t.x, t.y, t.z],
        "rotation": [r.x, r.y, r.z, r.w],
        "scale": [s.x, s.y, s.z]
    }

    object_attributes = {
        "transform": transform_attributes,
    }

    light_kind_names = {
        "POINT": "Point",
        "SUN": "Directional",
        "SPOT": "Spot",
    }
    
    if object.type == 'LIGHT':
        print("LIGHT")
        print(object.data)
        light = object.data
        kind = light_kind_names.get(light.type)
        if not kind:
            #unsupported
            print("unsupported light type " + light.type)
        else:
            c = light.color
            object_attributes["light"] = {
                "color": [c.r, c.g, c.b],
                "kind": light_kind_names[light.type],
                "intensity": light.energy,
                "cutoff_distance": light.cutoff_distance if light.use_custom_distance else -1.0,
            }

            if light.type == "SPOT":
                outer_angle = light.spot_size * 0.5
                inner_angle = outer_angle - outer_angle * light.spot_blend
                object_attributes["light"]["spot"] = {
                    "outer_angle": outer_angle,
                    "inner_angle": inner_angle
                }

    
    if object.instance_collection:
        library = object.instance_collection.library
        if not library:
            raise rafx_errors.RafxPrefabSceneUnsupportedObject("Collection instance {} must be linked from external file".format(object.instance_collection))

        # HACK HACK HACK: Assume that the blender file only contains one model, and its scene name matches the filename.
        # The alternative is linking the blend file, iterating over all scenes and trying to find the one that contains
        # the given collection. do_export_external_model does this
        library_export_path = rafx_blender_paths.find_base_export_path_for_data_block(project_settings, object.instance_collection)
        library_path = bpy.path.abspath(library.filepath)
        library_name = os.path.basename(library_path)
        model_name, ext = os.path.splitext(library_name)
        model_name = "{}.blender_model".format(model_name)
        f = os.path.join(library_export_path, model_name)
        collection_export_path = rafx_blender_paths.make_cross_platform_relative_path(f, export_dir)        

        print(collection_export_path)
        object_attributes["model"] = {
            "model": collection_export_path
        }

    out_objects.append(object_attributes)

def iterate_collection(project_settings, export_dir, out_objects, collection: bpy.types.Collection, transform: mathutils.Matrix):
    for object in collection.objects:
        # include only objects in the "root" of the collection
        if object.parent:
            continue

        iterate_object(project_settings, export_dir, out_objects, object, transform)
    
    for collection in collection.children:
        iterate_collection(project_settings, export_dir, out_objects, collection, transform)

def export(scene: bpy.types.Scene, project_settings):
    export_path = rafx_blender_paths.find_export_path_for_blender_data_block(project_settings, scene)
    export_dir = os.path.dirname(export_path)
    scene_collection = scene.collection
    if scene_collection.rafx_is_model:
        error_string = "Scene {} is not configured to export as a prefab".format(scene)
        logging.error(error_string)
        raise rafx_errors.RafxSceneIsNotAPrefab(error_string)

    out_objects = []
    iterate_collection(project_settings, export_dir, out_objects, scene_collection, mathutils.Matrix())

    prefab_object = {
        "objects": out_objects
    }

    prefab_as_json = json.dumps(prefab_object, indent=4)
    rafx_utils.write_string_to_file(export_path, prefab_as_json)
