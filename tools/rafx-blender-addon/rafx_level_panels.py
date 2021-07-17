import bpy

#from . import test
from . import rafx_export_operators
from . import rafx_level_operators

class RAFX_UL_level_ui_list(bpy.types.UIList):
    def draw_item(self, context, layout, data, item, icon, active_data, active_propname, index):
        layout.label(text=item.slice_path)

        if context.scene.collection.children.get(item.slice_path):
            layout.operator(rafx_level_operators.RafxLevelSliceUnload.bl_idname, icon="OUTLINER_OB_POINTCLOUD", text="").slice_path = item.slice_path
        else:
            layout.operator(rafx_level_operators.RafxLevelSliceLoad.bl_idname, icon="OUTLINER_DATA_POINTCLOUD", text="").slice_path = item.slice_path

class Rafx3DViewportLevelPanel(bpy.types.Panel):
    bl_idname = "RAFX_PT_3d_viewport_level"
    bl_label = "Level"
    bl_space_type = 'VIEW_3D'
    bl_region_type = 'UI'
    bl_category = "Rafx"
    bl_context = "objectmode"

    def draw(self, context):
        layout = self.layout

        level_properties = context.scene.rafx_level_properties
        level_path = level_properties.level_path
        if not level_path:
            row = layout.row()
            row.operator(rafx_level_operators.RafxLevelNewOp.bl_idname, text = "New Level")
            row.operator(rafx_level_operators.RafxLevelLoadOp.bl_idname, text = "Open Level")
        else:
            layout.label(text="Level: {}".format(context.scene.rafx_level_properties.level_path))
            row = layout.row()
            row.operator(rafx_level_operators.RafxLevelSaveOp.bl_idname, text = "Save All")
            row.operator(rafx_level_operators.RafxLevelCloseOp.bl_idname, text = "Close")
            row.operator(rafx_level_operators.RafxLevelReloadOp.bl_idname, text = "Reload")

            row = layout.row()
            row.operator(rafx_level_operators.RafxLevelSaveLevelAs.bl_idname, text = "Save Level As")
            row.operator(rafx_level_operators.RafxLevelSaveSliceAs.bl_idname, text = "Save Slice As")

            layout.label(text="Slices")
            row = layout.row()

            row = layout.row()
            row.template_list(
                "RAFX_UL_level_ui_list", "",  # type and unique id
                context.scene.rafx_level_properties, "slices",  # pointer to the CollectionProperty
                context.scene.rafx_level_properties, "active_slice",  # pointer to the active identifier
            )

            col = row.column(align=True)
            col.operator(rafx_level_operators.RafxLevelAddSliceOp.bl_idname, icon="ADD", text="")
            col.operator(rafx_level_operators.RafxLevelRemoveSliceOp.bl_idname, icon = "REMOVE", text="")

            row = layout.row()
            sub = row.row(align=True)
            sub.operator(rafx_level_operators.RafxLevelHideSlice.bl_idname, text="Hide")
            sub.operator(rafx_level_operators.RafxLevelShowSlice.bl_idname, text="Show")

            sub = row.row(align=True)
            sub.operator(rafx_level_operators.RafxLevelSelectSlice.bl_idname, text="Select")
            sub.operator(rafx_level_operators.RafxLevelDeselectSlice.bl_idname, text="Deselect")

            active_slice_index = level_properties.active_slice
            if active_slice_index >= 0 and active_slice_index < len(level_properties.slices):
                active_slice = level_properties.slices[active_slice_index]
                layout.label(text="{} objects in slice".format(active_slice.object_count))

            layout.operator(rafx_level_operators.RafxLevelMoveSelectedToSlice.bl_idname, text = "Move Selected to Slice")
            layout.operator(rafx_level_operators.RafxLevelMoveUnassignedToSlice.bl_idname, text = "Move Unassigned to Slice")
