import bpy
import os
from bpy.props import StringProperty, PointerProperty, CollectionProperty, BoolProperty, IntProperty

import logging
logging = logging.getLogger(__name__)

class RafxExportProperties(bpy.types.PropertyGroup):
    enable_prefab_export : BoolProperty(
        name="Export Prefabs",
        default=True
    )

    enable_model_export : BoolProperty(
        name="Export Models",
        default=True
    )

    enable_mesh_export : BoolProperty(
        name="Export Meshes",
        default=True
    )

    enable_material_export : BoolProperty(
        name="Export Materials",
        default=True
    )

    enable_image_export : BoolProperty(
        name="Export Images",
        default=True
    )

    enable_animation_export : BoolProperty(
        name="Export Animations",
        default=True
    )
    