import bpy
import os
from bpy.props import StringProperty, PointerProperty, CollectionProperty, BoolProperty, IntProperty

from . import rafx_export_operators
from . import rafx_level_operators
from . import rafx_panels
from . import rafx_slice
from . import rafx_level

import logging
logging = logging.getLogger(__name__)

#def level_get(self, value):

def level_path_get(self):
    level_path = self.get("level_path")
    return level_path or ""

def level_path_set(self, value):
    self["level_path"] = value

def slice_object_count_get(self):
    c = bpy.data.collections.get(self.slice_path)
    if c:
        return len(c.objects)
    else:
        return 0

class RafxSliceProperties(bpy.types.PropertyGroup):
    slice_path : StringProperty(name="", description="Slice Path")
    object_count : IntProperty(name="", description="Number of Objects", get=slice_object_count_get)

class RafxLevelProperties(bpy.types.PropertyGroup):
    level_path : StringProperty(
        name="",
        description="Path to Level File",
        default="",
        maxlen=1024,
    )

    slices : CollectionProperty(
        type=RafxSliceProperties
    )

    active_slice : IntProperty(

    )

    # TODO: Don't think this works, it's not used/exposed currently
    save_level_when_file_saved : BoolProperty(
        default=True
    )
    