from typing import Collection
import bpy
import os
import logging

logging = logging.getLogger(__name__)

from . import rafx_project
from .rafx_errors import *

def find_base_export_path_for_blender_file(project_settings, current_path):
    # this is an absolute path
    art_dir = project_settings[rafx_project.ART_DIR]
    # make the current file an absolute path
    current_path_abs = os.path.abspath(current_path)

    common_base_path = os.path.commonpath([art_dir, current_path_abs])
    if common_base_path != art_dir:
        error_string = "{} is not a parent of {}".format(art_dir, current_path_abs)
        logging.error(error_string)
        raise RafxFileNotWithinArtDir(error_string)

    relative_path = os.path.relpath(current_path_abs, art_dir)
    logging.debug("Relative path within art dir: %s", relative_path)

    # Remove the file extension since we're creating a directory
    relative_path = os.path.splitext(relative_path)[0]

    export_base_path = project_settings[rafx_project.ASSETS_DIR]
    export_path = os.path.join(export_base_path, relative_path)
    logging.debug("Out path: %s", export_path)

    return export_path

def find_base_export_path_for_current_blender_file(project_settings):
    current_path = bpy.data.filepath
    if not current_path:
        raise RafxFileNotSaved("Cannot determine export paths, this file has not been saved")

    return find_base_export_path_for_blender_file(project_settings, current_path)

def find_project_settings_for_current_blender_file():
    current_path = bpy.data.filepath
    if not current_path:
        raise RafxFileNotSaved("Cannot find project settings, this file has not been saved")
    
    return rafx_project.find_project_settings(current_path)

# Normalize extensions that are sometimes driven by blender's file_format enum
image_extension_aliases = {
    "jpeg": "jpg",
    "tiff": "tif",
    "targa": "tga",
}

def get_export_extension(data_block):
    if isinstance(data_block, bpy.types.Image):
        abspath = bpy.path.abspath(data_block.filepath)
        path, ext = os.path.splitext(abspath)
        if ext.startswith("."):
            ext = ext[1:]
        extension_alias = image_extension_aliases.get(ext)
        if extension_alias:
            return extension_alias
        return ext.lower()
        #assume we export as PNG
        #return "png"
    elif isinstance(data_block, bpy.types.Material):
        return "blender_material"
    elif isinstance(data_block, bpy.types.Object) and data_block.type == "MESH":
        return "blender_mesh"
    elif isinstance(data_block, bpy.types.Object) and data_block.type == "ARMATURE":
        return "blender_anim"
    elif isinstance(data_block, bpy.types.Collection):
        if data_block.rafx_is_model:
            return "blender_model"
        else:
            return "blender_prefab"
    elif isinstance(data_block, bpy.types.Scene):
        return get_export_extension(data_block.collection)
    elif isinstance(data_block, bpy.types.Action):
        return "blender_anim"
    elif isinstance(data_block, bpy.types.Armature):
        return "blender_skel"
    
    logging.warn("get_export_extension cannot determine extension for data block %s", data_block)

    return None

def find_base_export_path_for_data_block(project_settings, data_block):
    blend_filepath = bpy.data.filepath
    if data_block.library != None:
        blend_filepath = bpy.path.abspath(data_block.library.filepath)

    base_dir = find_base_export_path_for_blender_file(project_settings, blend_filepath)

    return base_dir

def find_export_path_for_blender_data_block_with_extension(project_settings, data_block, extension):
    base_dir = find_base_export_path_for_data_block(project_settings, data_block)
    name_without_extension = data_block.name
    
    # See if the image block ends with an extension so that we can avoid saving
    # image files named like image.png.png. This logic should handle jpeg/tiff
    # being equivalent to jpeg and tif
    f, data_block_name_ext = os.path.splitext(name_without_extension.lower())
    if image_extension_aliases.get(data_block_name_ext):
        data_block_name_ext = image_extension_aliases.get(data_block_name_ext)

    if data_block_name_ext.lower() == ".{}".format(extension):
        export_file_name = name_without_extension
    else:
        export_file_name = "{}.{}".format(name_without_extension, extension)

    return os.path.join(base_dir, export_file_name)

# This is generally the right function call to find where the export data of a blender data block will be written
def find_export_path_for_blender_data_block(project_settings, data_block):
    extension = get_export_extension(data_block)
    if not extension:
        raise RafxNoExtensionForDataBlockType
    
    return find_export_path_for_blender_data_block_with_extension(project_settings, data_block, extension)

def make_cross_platform_relative_path(path, relative_to_dir):
    path = os.path.relpath(path, relative_to_dir)
    return path.replace('\\', '/')
