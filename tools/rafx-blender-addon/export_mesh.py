
import json
import numpy as np
import struct
import bpy
import os

from . import rafx_blender_paths, rafx_errors, rafx_utils
from .rafx_export_types import ExportContext

def group_loops_by_materials(mesh):
    # Create an array of loop indices
    loop_indices = np.empty(len(mesh.loop_triangles) * 3, dtype=np.uint32)
    mesh.loop_triangles.foreach_get('loops', loop_indices)
    
    # Create an array of material indices (1/3 the size of the loop_indices array)
    tri_material_idxs = np.empty(len(mesh.loop_triangles), dtype=np.uint32)
    mesh.loop_triangles.foreach_get('material_index', tri_material_idxs)
    
    # Expand the list so that it is parallel with loop_indices
    loop_material_idxs = np.repeat(tri_material_idxs, 3)
    
    # Find unique material indices currently in use
    unique_material_idxs = np.unique(tri_material_idxs)
    del tri_material_idxs

    # Bucket the loop index by material
    grouped = {}
    for material_idx in unique_material_idxs:
        grouped[material_idx] = loop_indices[loop_material_idxs == material_idx]
    
    return grouped

def export(export_context: ExportContext, object: bpy.types.Object):
    assert(object.type == "MESH")
    if not export_context.visit_mesh(object):
        return
    
    log_str = "Exporting mesh {}".format(object.name_full)
    export_context.info(log_str)
    
    export_path = rafx_blender_paths.find_export_path_for_blender_data_block(export_context.project_settings, object)
    export_dir = os.path.dirname(export_path)

    mesh = object.data
    use_normals = True
    if use_normals:
        mesh.calc_normals_split()
    
    # We now calculate these in rafx
    use_tangents = False
    if use_normals and use_tangents:
        try:
            mesh.calc_tangents()
            use_tangents = True
        except Exception:
            raise rafx_errors.RafxCantCalculateTangents('WARNING', 'Could not calculate tangents. Please try to triangulate the mesh first.')
            use_tangents = False

    use_tex_coords = True
    tex_coord_max = 0
    if use_tex_coords:
        if mesh.uv_layers.active:
            tex_coord_max = len(mesh.uv_layers)
    
    use_color = True
    color_max = 0
    if use_color:
        color_max = len(mesh.vertex_colors)    
    
    mesh.calc_loop_triangles()
    
    # mesh.loop_trianges (returns collection of MeshLoopTriangle)
    # mesh.loops (returns collection of MeshLoop)
    # mesh.polygons (returns collection of MeshPolygon)
    # mesh.uv_layers
    # mesh.vertex_colors
    # mesh.vertices (returns collection of MeshVertex)
        
    vert_fields = [('vertex_index', np.uint32)]
    if use_normals:
        vert_fields += [('n', np.float32, (3,))]
    if use_tangents:
        vert_fields += [('t', np.float32, (4,))]
    for uv_i in range(tex_coord_max):
        vert_fields += [('uv%d' % uv_i, np.float32, (2,))]
    for color_i in range(color_max):
        vert_fields += [('color%d' % color_i, np.float32, (4,))]
            
    loops_by_material = group_loops_by_materials(mesh)
        
    # Get an array of all positions
    vertex_positions = np.empty(len(mesh.vertices) * 3, dtype=np.float32)
    mesh.vertices.foreach_get('co', vertex_positions)
    vertex_positions = vertex_positions.reshape(len(mesh.vertices), 3)
    
    mesh_parts = []
    binary_blobs = []
    
    for material_index, loop_indices in loops_by_material.items():
        i = 0
        verts = np.empty(len(loop_indices), dtype=np.dtype(vert_fields))
        for loop_index in loop_indices:
            loop = mesh.loops[loop_index]
            #print(loop.vertex_index)
            #pos = mesh.vertices[loop.vertex_index].co
            #print(pos)
            #print(loop.normal)
            #print(loop.tangent)
            #print(loop.bitangent)
            #print(loop.bitangent_sign)
            #print(tri.material_index)
        
            #for uv_layer in mesh.uv_layers:
            #    uv = uv_layer.data[loop_index]
        
            #for vertex_color_layer in mesh.vertex_colors:
            #    vertex_color = vertex_color_layer.data[loop_index]

            
            #print("mesh.vertices[loop.vertex_index].co")
            #print(mesh.vertices[loop.vertex_index].co)
            verts[i]['vertex_index'] = loop.vertex_index
            
            if use_normals:
                verts[i]['n'] = loop.normal
            
            if use_tangents:
                verts[i]['t'][0:3] = loop.tangent
                verts[i]['t'][3] = loop.bitangent_sign
            
            for uv_i in range(tex_coord_max):
                uv = mesh.uv_layers[uv_i].data[loop_index].uv.copy()
                uv.y = 1 - uv.y
                verts[i]['uv%d' % uv_i] = uv
            
            for color_i in range(color_max):
                color = mesh.vertex_colors[color_i].data[loop_index].color
                verts[i]['color%d' % color_i] = color
            
            i += 1
        
        verts, indices = np.unique(verts, return_inverse=True)
        vert_count = len(verts)
        if vert_count > 0xFFFFFFFF:
            raise Exception('ERROR', 'Mesh cannot be converted to use a u32 index buffer')
        elif vert_count > 0xFFFF:
            index_type = "U32"
            indices = indices.astype(np.uint32)
        else:
            index_type = "U16"
            indices = indices.astype(np.uint16)
        
        mesh_part_fields = {}
        
        mesh_part_fields['position'] = len(binary_blobs) + 1
        binary_blobs.append(vertex_positions[verts['vertex_index']].tobytes())

        #print(vertex_positions[verts['vertex_index']])
        
        if use_normals:
            mesh_part_fields['normal'] = len(binary_blobs) + 1
            binary_blobs.append(verts['n'].tobytes())
            #print(verts['n'])
            
        if use_tangents:
            mesh_part_fields['tangent'] = len(binary_blobs) + 1
            binary_blobs.append(verts['t'].tobytes())
            #print(verts['t'])
        
        uv_indices = []
        for uv_i in range(tex_coord_max):
            uv = np.empty((len(verts), 2), dtype=np.float32)
            uv_indices.append(len(binary_blobs) + 1)
            binary_blobs.append(verts['uv%d' % uv_i].tobytes())
            #print(verts['uv%d' % uv_i])
        
        if uv_indices:
            mesh_part_fields['uv'] = uv_indices
            
        color_indices = []
        for color_i in range(color_max):
            color = np.empty((len(verts), 4), dtype=np.float32)
            color_indices.append(len(binary_blobs) + 1)
            binary_blobs.append(verts['color%d' % color_i].tobytes())
            #print(verts['color%d' % color_i])
            
        if color_indices:
            mesh_part_fields['color'] = color_indices
        
        indices_blob_index = len(binary_blobs) + 1
        binary_blobs.append(indices.tobytes())
        #print(indices)
        
        mesh_part_fields['material'] = ""
        if int(material_index) < len(object.material_slots):
            material = object.material_slots[int(material_index)].material
            if material:
                material_path = rafx_blender_paths.find_export_path_for_blender_data_block(export_context.project_settings, material)
                material_path = rafx_blender_paths.make_cross_platform_relative_path(material_path, export_dir)
                mesh_part_fields['material'] = material_path

        mesh_part_fields['indices'] = indices_blob_index
        mesh_part_fields['index_type'] = index_type

        mesh_parts.append(mesh_part_fields)
    
    mesh_obj = {
        'mesh_parts': mesh_parts
    }
    
    json_header = json.dumps(mesh_obj, indent = 4)
    
    #TODO: Create an interface for writing binary files in this format
    b = bytearray()
    
    b.extend(struct.pack('I', 0xBB33FF00))

    b.extend(struct.pack('c', b'M'))
    b.extend(struct.pack('c', b'E'))
    b.extend(struct.pack('c', b'S'))
    b.extend(struct.pack('c', b'H'))

    # File schema version
    b.extend(struct.pack('I', 1))
    # Number of blocks
    b.extend(struct.pack('I', 1 + len(binary_blobs)))

    # Encode ending offset of all blocks
    total_bytes = 0
    b.extend(struct.pack('Q', total_bytes))  
    
    # Encode the first json block
    blob_len = len(json_header)
    total_bytes += blob_len
    b.extend(struct.pack('Q', total_bytes))
    total_bytes = ((total_bytes + 15)//16)*16

    # next block is implied to start on a 16-byte interval
    for blob in binary_blobs:
        blob_len = len(blob)
        total_bytes += blob_len
        #print(total_bytes)
        b.extend(struct.pack('Q', total_bytes))   
        total_bytes = ((total_bytes + 15)//16)*16

    json_bytes = json_header.encode('utf-8')
    
    if len(b) % 16 != 0:
        b.extend(b"\0" * (16 - (len(b) % 16)))
    assert(len(b) % 16 == 0)
    
    b.extend(json_bytes)
    for blob in binary_blobs:
        if len(b) % 16 != 0:
            b.extend(b"\0" * (16 - (len(b) % 16)))
        assert(len(b) % 16 == 0)
        
        b.extend(blob)
    
    rafx_utils.write_bytes_to_file(export_path, bytes(b))
