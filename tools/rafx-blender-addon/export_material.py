import bpy
import json
import hashlib
import os
from . import gltf_export, rafx_blender_paths, gltf_blender_image, rafx_errors, rafx_project, rafx_utils
from .rafx_export_types import ExportContext
from .gltf_blender_image import Channel

# def add_generic_texture_map(socket):
#     node = gltf_export.previous_node_typed(socket, bpy.types.ShaderNodeTexImage)
#     if node:
#         if node.image:
#             info = {}
#             id = node.image.get("rafx_uuid")
#             if id:
#                 info['image'] = id
#                 #tex_coords = get_tex_coord_for_image_node(node)
#                 #info['tex_coords'] = tex_coords
            
#             return info
    
#     return None

def find_bsdf_node(material):
    # Find the active BSDF node
    node_type = bpy.types.ShaderNodeBsdfPrincipled
    nodes = [n for n in material.node_tree.nodes if isinstance(n, node_type) and not n.mute]
    nodes = [node for node in nodes if gltf_export.check_if_is_linked_to_active_output(node.outputs[0])]
    if len(nodes) > 1:
        raise Exception("ERROR: Material " + str(material) + " has more than one unmuted Principled BSDF node connected to output")
    
    if not nodes:
        raise Exception("ERROR: Material " + str(material) + " must have exactly one unmuted Principled BSDF node connected to output")
    
    return nodes[0] 



def find_path_from_node_type_to_input_socket(socket: bpy.types.NodeSocket, node_type):
    assert(socket.link_limit < 2)
    assert(len(socket.links) < 2)
    if socket.links:
        link = socket.links[0]
        linked_node = link.from_node
        if isinstance(linked_node, node_type) and not linked_node.mute:
            return [{
                "node": linked_node,
                "input_socket": None, 
                "output_socket": link.from_socket
            }]
        
        for input_socket in linked_node.inputs:
            result = find_path_from_node_type_to_input_socket(input_socket, node_type)
            if result:
                result.append({
                    "node": linked_node,
                    "input_socket": input_socket, 
                    "output_socket": link.from_socket
                })
                return result
        
        return None

    return None

def find_path_from_image_to_input_socket(socket:bpy.types.NodeSocket):
    path = find_path_from_node_type_to_input_socket(socket, bpy.types.ShaderNodeTexImage)
    if not path:
        return None
        
    image_node = path[0]["node"]
    if not image_node.image:
        return None

    src_channel = None
    if path[0]["output_socket"].name == "Color":
        src_channel = 'L'
    elif path[0]["output_socket"].name == "Alpha":
        src_channel = 'A'
    
    for step in path[1:]:
        if isinstance(step["node"], bpy.types.ShaderNodeSeparateRGB):
            if step["output_socket"].name == "R":
                src_channel = "R"
            elif step["output_socket"].name == "G":
                src_channel = "G"
            elif step["output_socket"].name == "B":
                src_channel = "B"
        else:
            raise rafx_errors.RafxUnsupportedMaterialGraphNode("Graph node {} is unsupported".format(step["node"]))
    
    return {
        "image": image_node.image,
        "src_channel": src_channel,
    }

def get_channel(c):
    if c == "R":
        return Channel.R
    elif c == "G":
        return Channel.G
    elif c == "B":
        return Channel.B
    elif c == "A":
        return Channel.A
    elif c == "L":
        #TODO: Log warning that the result will be less accurate
        return Channel.R

def create_image_hash_string(image, project_settings):
    assets_dir = project_settings[rafx_project.ART_DIR]
    filepath = ""
    if image.library:
        filepath = bpy.path.abspath(image.library.filepath)
    else:
        filepath = bpy.data.filepath
    
    filepath = os.path.relpath(filepath, assets_dir)
    return "{}-{}".format(image.name, filepath)

# This is tricky, we need to generate an image with the data in the correct channels, put it somewhere
# and include it in the material
def setup_pbr_texture(bsdf_node: bpy.types.ShaderNodeBsdfPrincipled, attributes, project_settings, export_dir):
    roughness_result = find_path_from_image_to_input_socket(bsdf_node.inputs["Roughness"])
    metallic_result = find_path_from_image_to_input_socket(bsdf_node.inputs["Metallic"])

    if roughness_result or metallic_result:
        export_image = gltf_blender_image.ExportImage()

        # this is just descriptive/helpful
        filename_fragments = []
        # these must uniquely identify the image and blend file it's linked from, relative to the art root dir
        roughness_hash_string = ""
        metallic_hash_string = ""

        # Occlusion
        if roughness_result["image"] == metallic_result["image"]:
            # Just copy the existing channel, this may avoid us having to generate a texture and speed
            # up exporting
            export_image.fill_image(roughness_result["image"], Channel.R, Channel.R)
        else:
            # We're going to have to generate a texture so just fill R with white
            export_image.fill_white(Channel.R)

        if roughness_result:
            export_image.fill_image(roughness_result["image"], Channel.G, get_channel(roughness_result["src_channel"]))
            image = roughness_result["image"]
            filename_fragments.append("R_{}".format(image.name))
            roughness_hash_string = create_image_hash_string(image, project_settings)
        else:
            export_image.fill_white(Channel.G)
            
        if metallic_result:
            export_image.fill_image(metallic_result["image"], Channel.B, get_channel(metallic_result["src_channel"]))
            image = metallic_result["image"]
            filename_fragments.append("M_{}".format(image.name))
            metallic_hash_string = create_image_hash_string(image, project_settings)
        else:
            export_image.fill_white(Channel.B)
        
        blender_image = export_image.blender_image()
        if blender_image:
            image_export_path = rafx_blender_paths.find_export_path_for_blender_data_block(project_settings, blender_image)
        else:
            png_bytes = export_image.encode("PNG")
            
            # combine the hash strings and hash it
            pbr_hash_string = "{}--{}".format(roughness_hash_string, metallic_hash_string).encode("UTF-8")
            pbr_hash = hashlib.md5(pbr_hash_string).hexdigest()[0:12]
            
            # write out the image
            image_export_file_name = "{}-{}.png".format("-".join(filename_fragments), pbr_hash)
            image_export_path = os.path.join(project_settings[rafx_project.ASSETS_DIR], "_generated", "pbr_textures", image_export_file_name)
            rafx_utils.write_bytes_to_file(image_export_path, png_bytes)

        # Add it to the material file
        relpath = rafx_blender_paths.make_cross_platform_relative_path(image_export_path, export_dir)
        attributes["metallic_roughness_texture"] = relpath

    return None

    # GLTF convention.. planning to follow it unless we find there's a good reason not to
    #if socket.name == 'Metallic':
    #    dst_channel = "B"
    #elif socket.name == 'Roughness':
    #    dst_channel = "G"
    #elif socket.name == 'Occlusion':
    #    dst_channel = "R"
    #elif socket.name == 'Alpha':
    #    dst_channel = "A"
    # elif socket.name == 'Clearcoat':
    #     dst_channel = "R"
    # elif socket.name == 'Clearcoat Roughness':
    #     dst_channel = "G"


def export(export_context: ExportContext, material: bpy.types.Material):
    if not export_context.visit_material(material):
        return

    log_str = "Exporting material {}".format(material.name_full)
    export_context.info(log_str)

    export_path = rafx_blender_paths.find_export_path_for_blender_data_block(export_context.project_settings, material)
    export_dir = os.path.dirname(export_path)
    if not material.use_nodes:
        raise Exception("ERROR: Material " + str(material) + " does not use nodes and can't be exported")
    bsdf = find_bsdf_node(material)
        
    attributes = {}
    base_color_factor = gltf_export.get_factor_from_socket(bsdf.inputs["Base Color"], 'RGB', [1.0,1.0,1.0])
    base_color_factor.append(gltf_export.get_factor_from_socket(bsdf.inputs["Alpha"], 'VALUE', 1.0))
    attributes['base_color_factor'] = base_color_factor
    attributes['roughness_factor'] = gltf_export.get_factor_from_socket(bsdf.inputs["Roughness"], 'VALUE', 1.0)
    attributes['metallic_factor'] = gltf_export.get_factor_from_socket(bsdf.inputs["Metallic"], 'VALUE', 1.0)
    attributes['emissive_factor'] = gltf_export.get_factor_from_socket(bsdf.inputs["Emission"], 'RGB', [0.0,0.0,0.0])
    attributes['normal_texture_scale'] = 1.0
    #attributes['occlusion_texture_strength'] = 1.0
    #attributes['alpha_cutoff'] = 0.5
    #attributes['use_alpha'] = False
    #attributes['color_texture'] = ""
    #attributes['metallic_roughness_texture'] = ""
    #attributes['normal_texture'] = ""
    #attributes['emissive_texture'] = ""

    attributes['shadow_method'] = material.shadow_method
    attributes['blend_method'] = material.blend_method
    attributes['alpha_threshold'] = material.alpha_threshold
    attributes['backface_culling'] = material.use_backface_culling

    base_color_node = gltf_export.previous_node_typed(bsdf.inputs["Base Color"], bpy.types.ShaderNodeTexImage)
    alpha_node = gltf_export.previous_node_typed(bsdf.inputs["Alpha"], bpy.types.ShaderNodeTexImage)

    if base_color_node:
        if not base_color_node.image:
            raise Exception("ERROR: Material " + str(material) + " has a ShaderNodeTexImage used as a color map but has no image selected")

        image_path = rafx_blender_paths.find_export_path_for_blender_data_block(export_context.project_settings, base_color_node.image)
        image_path = rafx_blender_paths.make_cross_platform_relative_path(image_path, export_dir)

        attributes['color_texture'] = image_path
        attributes['color_texture_has_alpha_channel'] = False

        if alpha_node:
            if not alpha_node.image:
                raise Exception("ERROR: Material " + str(material) + " has a ShaderNodeTexImage used as an alpha map but has no image selected")
            elif alpha_node.image.filepath != base_color_node.image.filepath:
                raise Exception("ERROR: Material " + str(material) + " alpha image is not the same as color image")
            else:
                attributes['color_texture_has_alpha_channel'] = True
    elif alpha_node:
        if alpha_node.image:
            raise Exception("ERROR: Material " + str(material) + " has a ShaderNodeTexImage used as an alpha map but no color map is specified")
        else:
            raise Exception("ERROR: Material " + str(material) + " has a ShaderNodeTexImage used as an alpha map but has no image selected")

    # Support connecting a texture directly to the normal output (technically wrong) AND image -> normal map -> bsdf
    normal_map_node = gltf_export.previous_node_typed(bsdf.inputs["Normal"], bpy.types.ShaderNodeNormalMap)
    normal_image_node = gltf_export.previous_node_typed(bsdf.inputs["Normal"], bpy.types.ShaderNodeTexImage)
    if normal_map_node or normal_image_node:
        # If we found a ShaderNodeNormalMap, find the image node connected to it, and error if it doesn't exist
        if not normal_image_node:
            normal_image_node = gltf_export.previous_node_typed(normal_map_node.inputs["Color"], bpy.types.ShaderNodeTexImage)
            if not normal_image_node:
                raise rafx_errors.RafxUnsupportedMaterialGraphNode("ShaderNodeNormalMap does not have a ShaderNodeTexImage color input")
        
        if normal_image_node.image:
            image_path = rafx_blender_paths.find_export_path_for_blender_data_block(export_context.project_settings, normal_image_node.image)
            image_path = rafx_blender_paths.make_cross_platform_relative_path(image_path, export_dir)
            attributes['normal_texture'] = image_path
        else:
            raise rafx_errors.RafxUnsupportedMaterialGraphNode("ERROR: Material " + str(material) + "has a ShaderNodeTexImage used as normal map has no image selected")
    
    setup_pbr_texture(bsdf, attributes, export_context.project_settings, export_dir)
     
    json_data = json.dumps(attributes, indent = 4)
    rafx_utils.write_string_to_file(export_path, json_data)

