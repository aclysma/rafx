import bpy

from . import gltf_blender_image, rafx_blender_paths, rafx_utils
from .rafx_export_types import ExportContext
import os
import shutil
import pathlib
import logging

logging = logging.getLogger(__name__)

# This function is a bit odd, it could take a fast path to copy an existing image to the intended
# export location or write out a new image
def export(export_context: ExportContext, image: bpy.types.Image):
    if not export_context.visit_image(image):
        return

    export_context.info("Exporting image {}".format(image.name_full))

    export_path = rafx_blender_paths.find_export_path_for_blender_data_block(export_context.project_settings, image)
    export_dir = os.path.dirname(export_path)
    pathlib.Path(export_dir).mkdir(parents=True, exist_ok=True)
    if not image.is_dirty:
        image_filepath = image.filepath_raw
        # HACK: Blender sometimes returns backslashes in this path on macOS, don't know why. I think forward
        # slashes will still work on windows. This may be something that needs to be addressed more systemically
        # but I haven't seen this happen elsewhere yet
        #image_filepath.replace("\\", "/")
        #print("filepath_raw", image.filepath_raw, "filepath", image.filepath, "library", image.library, "fixed", image_filepath)
        src_path = bpy.path.abspath(image_filepath, library = image.library).replace("\\", "/")

        logging.info("  copy image from {} to {}".format(src_path, export_path))
        shutil.copyfile(src_path, export_path)
        return None
    else:
        export_image = gltf_blender_image.ExportImage.from_blender_image(image)
        file_format = export_image.file_format
        data = export_image.encode(file_format)
        rafx_utils.write_bytes_to_file(export_path, data)
        return data
