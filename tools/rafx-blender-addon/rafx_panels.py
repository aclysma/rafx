import bpy

from . import rafx_export_operators
from . import rafx_level_operators

import logging
logging = logging.getLogger(__name__)

#
# "N" menu panels
#
class RafxImageEditorPanel(bpy.types.Panel):
    bl_idname = "RAFX_PT_image_editor"
    bl_space_type = 'IMAGE_EDITOR'
    bl_region_type = 'UI'
    bl_label = "Rafx"
    bl_category = "Rafx"

    def draw(self, context):
        layout = self.layout        
        layout.operator(rafx_export_operators.RafxExportImageOp.bl_idname, text = "Export This Image")

class RafxMaterialEditorPanel(bpy.types.Panel):
    bl_idname = "RAFX_PT_material_editor"
    bl_space_type = 'NODE_EDITOR'
    bl_region_type = 'UI'
    bl_label = "Rafx"
    bl_category = "Rafx"

    def draw(self, context):
        layout = self.layout
        if context.material:
            layout.operator(rafx_export_operators.RafxExportMaterialOp.bl_idname, text = "Export This Material")
        
class Rafx3DViewportEditorPanel(bpy.types.Panel):
    bl_idname = "RAFX_PT_3d_viewport_export"
    bl_label = "Export"
    bl_space_type = 'VIEW_3D'
    bl_region_type = 'UI'
    bl_category = "Rafx"
    bl_context = "objectmode"
    
    def draw(self, context):
        layout = self.layout

        layout.prop(context.scene.collection, "rafx_is_model")
        layout.prop(context.scene.collection, "rafx_is_prefab")

        if bpy.context.scene.collection.rafx_is_model:
            layout.operator(rafx_export_operators.RafxExportSceneAsModelOp.bl_idname, text = "Export This Model")
        
        if bpy.context.scene.collection.rafx_is_prefab:
            layout.operator(rafx_export_operators.RafxExportSceneAsPrefabOp.bl_idname, text = "Export This Prefab")
        
        layout.separator()
        layout.operator(rafx_export_operators.RafxExportAllOp.bl_idname, text = "Export All Referenced Assets")

#
# Property Panels
#
class RafxPropertyPanel(bpy.types.Panel):
    bl_label = "Rafx"
    bl_space_type = 'PROPERTIES'
    bl_region_type = 'WINDOW'

class RafxPropertyPanelMaterial(RafxPropertyPanel):
    bl_idname = "RAFX_PT_material_properties"
    bl_context = 'material'
    
    @classmethod
    def poll(cls, context):
        # Hide if no material is selected
        return context.material != None
        
    def draw(self, context):
        layout = self.layout
        layout.operator(rafx_export_operators.RafxExportMaterialOp.bl_idname, text = "Export This Material")

class RafxPropertyPanelScene(RafxPropertyPanel):
    bl_idname = "RAFX_PT_scene_properties"
    bl_context = "scene"
    
    def draw(self, context):
        row = self.layout.row()
        row.prop(context.scene.collection, "rafx_is_model")
        row = self.layout.row()
        row.prop(context.scene.collection, "rafx_is_prefab")

