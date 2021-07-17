from genericpath import exists
import rafx_errors
import bpy
import json
import os

from . import rafx_utils

import logging
logging = logging.getLogger(__name__)

class RafxSlice:
    def __init__(self, path, slice_data):
        self.path = path
        self.slice_data = slice_data

class RafxLevel:
    def __init__(self, slices, level_data):
        self.slices = slices
        self.level_data = level_data

# returns object with "slices": array of "slice_path" and "slice_data" - which is all keys but the slice path
# and "level_data" which is all keys but the slices
def load_level(level_file: str) -> RafxLevel:
    level_path = bpy.path.abspath(level_file)
    level_dir = os.path.dirname(level_file)

    with open(level_path, 'r') as myfile:
        text=myfile.read()
        level_data = json.loads(text)
    
    if not "slices" in level_data:
        logging.warning("Failed to load slice, no contents attribute found")
        return

    loaded_slices = []

    for slice_data in level_data["slices"]:
        if not "path" in slice_data:
            logging.warning("No path defined in slice within level file")
            return

        slice_path = slice_data["path"]
        if not os.path.isabs(slice_path):
            slice_path = os.path.join(level_dir, slice_path)

        slice_path = bpy.path.relpath(slice_path)

        del slice_data["path"]

        loaded_slices.append(RafxSlice(slice_path, slice_data))
    
    del level_data["slices"]

    return RafxLevel(loaded_slices, level_data)

# saves file with just paths to all slices
def save_level(level_file: str, level: RafxLevel) -> None:
    level_abspath = bpy.path.abspath(level_file)
    level_base_dir = os.path.dirname(level_abspath)

    # load the existing file on disk, we will try to preserve any data we don't know about
    if os.path.exists(level_abspath):
        existing_level = load_level(level_file)
        existing_level_data = existing_level.level_data
        existing_slice_data = {}
        for slice in existing_level.slices:
            existing_slice_data[slice.path] = slice.slice_data
    else:
        existing_level_data = {}
        existing_slice_data = {}
    
    level_slices = []
    for slice in level.slices:
        slice_abspath = bpy.path.abspath(slice.path)
        slice_path = os.path.relpath(slice_abspath, level_base_dir)

        # start from the data in the json file for this slice if it already existed
        slice_data = existing_slice_data.get(slice.path)
        if not slice_data:
            slice_data = {}

        # save the slice data we know about on top of it
        for k, v in slice.slice_data.items():
            slice_data[k] = v
        
        # add the path attribute
        slice_data["path"] = slice_path
        level_slices.append(slice_data)
    
    # start from the data in the json file if it already existed
    level_data = existing_level_data

    # save the level data we know about on top of it
    for k, v in level.level_data.items():
        level_data[k] = v
    
    # add the slices
    level_data["slices"] = level_slices

    s = json.dumps(level_data, indent=4)
    rafx_utils.write_string_to_file(bpy.path.abspath(level_file), s)


