import bpy
import json
import os
import mathutils
import pathlib
from mathutils import Vector 
import logging
logging = logging.getLogger(__name__)

from . import rafx_utils

c_light_type_names_from_blender = {
    "POINT": "Point",
    "SUN": "Directional",
    "SPOT": "Spot",
}

c_light_type_names_to_blender = {v: k for k, v in c_light_type_names_from_blender.items()}

# TODO: Implement loading lower LODs
def load_model_library(model_library_path, models):
    #logging.info("load model library", model_library_path)
    with bpy.data.libraries.load(model_library_path, link=True) as (data_from, data_to):
        for collection in data_from.collections:
            if collection.endswith("LOD0"):
                data_to.collections.append(collection)
                models[model_library_path] = collection, model_library_path
    

def add_object(slice_object, slice_collection, models, base_dir):
    p = slice_object["position"]
    r = slice_object["rotation"]
    s = slice_object["scale"]
            
    if slice_object["ty"] == "MESH":
        model_path = bpy.path.relpath(os.path.join(base_dir, slice_object["mesh"]["path"]))
        
        if not model_path in models:
            load_model_library(model_path, models)
        
        collection_name, library_path = models[model_path]
        c = bpy.data.collections[collection_name, library_path]
        if c:                
            instance_obj = bpy.data.objects.new(
                name=slice_object["name"], 
                object_data=None
            )
            instance_obj.rotation_mode = "QUATERNION"
            instance_obj.location = (p[0], p[1], p[2])
            instance_obj.rotation_quaternion = (r[3], r[0], r[1], r[2])
            instance_obj.scale = (s[0], s[1], s[2])
            instance_obj.instance_collection = c
            instance_obj.instance_type = "COLLECTION"
            
            return instance_obj
        else:
            logging.warning("can't find collection {} {}".format(collection_name, library_path))
    elif slice_object["ty"] == "LIGHT":
        l = slice_object["light"]
        color = l["color"]
        light_type = c_light_type_names_to_blender[l["light_type"]]
        intensity = l["intensity"]
        
        light_data = bpy.data.lights.new(
            name=slice_object["name"], 
            type=light_type
        )
        light_data.color = (color[0], color[1], color[2])
        light_data.energy = intensity
        
        if light_type == "SPOT":
            outer_angle = l["spot"]["outer_angle"]
            inner_angle = l["spot"]["inner_angle"]
            
            light_data.spot_size = outer_angle * 2.0
            light_data.spot_blend = 1.0 - (inner_angle / max(0.001, outer_angle))
            
        
        light_obj = bpy.data.objects.new(
            name=slice_object["name"], 
            object_data=light_data
        )
        light_obj.rotation_mode = "QUATERNION"
        light_obj.location = (p[0], p[1], p[2])
        light_obj.rotation_quaternion = (r[3], r[0], r[1], r[2])
        light_obj.scale = (s[0], s[1], s[2])
        
        
        return light_obj
    elif slice_object["ty"] == "EMPTY":
        empty_obj = bpy.data.objects.new(
            name=slice_object["name"], 
            object_data=None
        )
        
        empty_obj.rotation_mode = "QUATERNION"
        empty_obj.location = (p[0], p[1], p[2])
        empty_obj.rotation_quaternion = (r[3], r[0], r[1], r[2])
        empty_obj.scale = (s[0], s[1], s[2])
        
        if slice_object.get("children"):
            for child in slice_object["children"]:
                child_obj = add_object(child, slice_collection, models, base_dir)
                if child_obj:
                    child_obj.parent = empty_obj
                    slice_collection.objects.link(child_obj)
        
        return empty_obj
    else:
        logging.warning("unknown model type {}".format(slice_object["ty"]))
        

def load_slice(dst_collection, slice_file):    
    bpy.ops.outliner.orphans_purge(do_local_ids=True, do_linked_ids=True, do_recursive=True)

    slice_json_path = bpy.path.abspath(slice_file)
    slice_basedir = os.path.dirname(slice_json_path)
    logging.info("load slice from ", slice_json_path, "into collection", dst_collection)

    with open(slice_json_path, 'r') as myfile:
        text=myfile.read()
        slice_data = json.loads(text)

    models = {}

    if not "contents" in slice_data:
        logging.warning("Failed to load slice, no contents attribute found")
        return

    for slice_object in slice_data.get("contents"):
        obj = add_object(slice_object, dst_collection, models, slice_basedir)
        if obj:
            dst_collection.objects.link(obj)


def get_object_info(object, base_dir):
    ty = object.type
    
    is_mesh = object.type == 'EMPTY' and object.instance_type != 'NONE'
    is_leaf_object = object.type != 'EMPTY' or is_mesh

    if is_leaf_object and object.children:
        logging.warning("Object is type {}, instance type {} and has children, this is not handled correctly".format(object.type, object.instance_type))
        return None
    
    attributes = {}
    attributes['name'] = object.name
    attributes['ty'] = object.type
    if is_mesh:
        attributes['ty'] = "MESH"
    
    p = object.location
    if object.rotation_mode == 'QUATERNION':
        r = object.rotation_quaternion
    elif object.rotation_mode == 'AXIS_ANGLE':
        r = mathutils.Quaternion(object.rotation_axis_angle[1:4], object.rotation_axis_angle[0])
    else:
        r = object.rotation_euler.to_quaternion()
    
    s = object.scale
    
    attributes['position'] = [p.x, p.y, p.z]
    attributes['rotation'] = [r.x, r.y, r.z, r.w]
    attributes['scale'] = [s.x, s.y, s.z]
    
    if object.type == 'EMPTY' and object.instance_type == "COLLECTION":
        if not object.instance_collection:
            logging.warning("Skip object {}, it has no instance_collection")

        collection_library = object.instance_collection.library
        if not collection_library:
            logging.warning("Skip object {}, the collection is not linked from an external blend file")

        collection_abspath = bpy.path.abspath(collection_library.filepath)
        collection_relpath = os.path.relpath(collection_abspath, base_dir)

        attributes['mesh'] = {
            'path': collection_relpath
        }
    elif object.type == 'LIGHT':
        light = object.data
        if light.type not in c_light_type_names_from_blender:
            logging.warning("unsupported light type " + light.type)
        else:
            c = light.color
            attributes["light"] = {
                "color": [c.r, c.g, c.b],
                "light_type": c_light_type_names_from_blender[light.type],
                "intensity": light.energy
            }

            if light.type == "SPOT":
                outer_angle = light.spot_size * 0.5
                inner_angle = outer_angle - outer_angle * light.spot_blend
                attributes["light"]["spot"] = {
                    "outer_angle": outer_angle,
                    "inner_angle": inner_angle
                }
    elif object.type == 'EMPTY':    
        children = []
        
        for child_object in object.children:
            obj_info = get_object_info(child_object, base_dir)
            if obj_info:
                children.append(obj_info)
        
        if children:
            attributes['children'] = children
        
    return attributes


def save_slice(src_collection, slice_file):
    #
    # Get local paths
    #
    slice_abspath = bpy.path.abspath(slice_file)
    slice_basedir = os.path.dirname(slice_abspath)

    logging.info("save collection ", src_collection.name, "to", slice_file)

    object_info = []
    for object in src_collection.objects:
        # include only objects in the "root" of the collection
        if object.parent:
            continue

        obj_info = get_object_info(object, slice_basedir)
        if obj_info:
            object_info.append(obj_info)

    slice_data = {
        "contents": object_info
    }

    s = json.dumps(slice_data, indent=4)

    rafx_utils.write_string_to_file(bpy.path.abspath(slice_file), s)

    bpy.ops.outliner.orphans_purge(do_local_ids=True, do_linked_ids=True, do_recursive=True)


def move_selected_to_slice(context, dst_collection):

    # Push the selected flag up the object tree. This should not iterate selected_objects because
    # selected_objects changes (I think, not certain!)
    for object in bpy.data.objects:
        if object.select_get():
            parent = object.parent
            while parent:
                parent.select_set(True)
                parent = parent.parent
    
    # Push the selected flag back down the object tree.
    for object in bpy.data.objects:
        if object.select_get():
            continue

        parent_selected = False
        parent = object.parent
        while parent:
            if parent.select_get():
                object.select_set(True)
                break

            parent = parent.parent

    for object in context.selected_objects:
        collections = []
        for c in object.users_collection:
            collections.append(c)
        
        for c in collections:
            c.objects.unlink(object)
            dst_collection.objects.link(object)
