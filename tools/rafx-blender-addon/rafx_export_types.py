import bpy
import logging
logging = logging.getLogger(__name__)

from . import rafx_project
from . import rafx_export_properties
from . import rafx_blender_paths

class ObjectKey:
    name: str
    library: str

    def __init__(self, name, library):
        self.name = name
        self.library = library

class ExportContext:
    operator: bpy.types.Operator
    project_settings: rafx_project.ProjectSettings
    export_properties: rafx_export_properties.RafxExportProperties
    exported_images: set[ObjectKey]
    exported_materials: set[ObjectKey]
    exported_meshes: set[ObjectKey]
    exported_animation: set[ObjectKey]
    exported_scene_as_model: set[ObjectKey]
    exported_scene_as_prefab: set[ObjectKey]
    exported_single_mesh_as_model: set[ObjectKey]
    
    def __init__(self, operator: bpy.types.Operator, context: bpy.types.Context):
        self.operator = operator
        self.project_settings = rafx_blender_paths.find_project_settings_for_current_blender_file()
        self.export_properties = context.scene.rafx_export_properties
        self.exported_images = set()
        self.exported_materials = set()
        self.exported_meshes = set()
        self.exported_animation = set()
        self.exported_scene_as_model = set()
        self.exported_scene_as_prefab = set()
        self.exported_single_mesh_as_model = set()

    def summary_text(self) -> str:
        return "exported {} images, {} materials, {} meshes, {} models, {} animations, and {} prefabs".format(
            len(self.exported_images), 
            len(self.exported_materials),
            len(self.exported_meshes),
            len(self.exported_animation),
            len(self.exported_scene_as_model) + len(self.exported_single_mesh_as_model),
            len(self.exported_scene_as_prefab)
        )

    def visit_image(self, image: bpy.types.Image) -> bool:
        key = ObjectKey(image.name, image.library.filepath if image.library else "")
        if key not in self.exported_images:
            self.exported_images.add(key)
            return True
        else:
            return False

    def visit_material(self, material: bpy.types.Material) -> bool:
        # If someone ever wants a "Dots Stroke" material, they need to set it to use nodes anyways to work with this exporter
        if material.name == "Dots Stroke" and not material.use_nodes:
            self.info("Ignoring default material 'Dots Stroke'")
            return False

        key = ObjectKey(material.name, material.library.filepath if material.library else "")
        if key not in self.exported_materials:
            self.exported_materials.add(key)
            return True
        else:
            return False

    def visit_mesh(self, mesh_object: bpy.types.Object) -> bool:
        assert(mesh_object.type == "MESH")
        key = ObjectKey(mesh_object.name, mesh_object.library.filepath if mesh_object.library else "")
        if key not in self.exported_meshes:
            self.exported_meshes.add(key)
            return True
        else:
            return False
    
    def visit_animation(self, armature_object: bpy.types.Object) -> bool:
        assert(armature_object.type == "ARMATURE")
        key = ObjectKey(armature_object.name, armature_object.library.filepath if armature_object.library else "")
        if key not in self.exported_animation:
            self.exported_animation.add(key)
            return True
        else:
            return False

    def visit_scene_as_model(self, scene: bpy.types.Scene) -> bool:
        key = ObjectKey(scene.name, scene.library.filepath if scene.library else "")
        if key not in self.exported_scene_as_model:
            self.exported_scene_as_model.add(key)
            return True
        else:
            return False

    def visit_scene_as_prefab(self, scene: bpy.types.Scene) -> bool:
        key = ObjectKey(scene.name, scene.library.filepath if scene.library else "")
        if key not in self.exported_scene_as_prefab:
            self.exported_scene_as_prefab.add(key)
            return True
        else:
            return False

    def visit_single_mesh_as_model(self, mesh_object: bpy.types.Object) -> bool:
        assert(mesh_object.type == "MESH")
        key = ObjectKey(mesh_object.name, mesh_object.library.filepath if mesh_object.library else "")
        if key not in self.exported_single_mesh_as_model:
            self.exported_single_mesh_as_model.add(key)
            return True
        else:
            return False
    
    def info(self, str: str):
        self.operator.report({'INFO'}, str)
        logging.info(str)

    def warn(self, str: str):
        self.operator.report({'WARNING'}, str)
        logging.error(str)

    def error(self, str: str):
        self.operator.report({'ERROR'}, str)
        logging.error(str)