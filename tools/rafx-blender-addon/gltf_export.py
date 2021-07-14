# Copyright 2018-2021 The glTF-Blender-IO authors.
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

# This file is an excerpt of code from the GLTF importer/exporter, used
# under apache/MIT
# https://github.com/KhronosGroup/glTF-Blender-IO

import bpy
import numpy as np
import json
import struct
import os

def check_if_is_linked_to_active_output(shader_socket):
    for link in shader_socket.links:
        if isinstance(link.to_node, bpy.types.ShaderNodeOutputMaterial) and link.to_node.is_active_output is True:
            return True

        if len(link.to_node.outputs) > 0: # ignore non active output, not having output sockets
            ret = check_if_is_linked_to_active_output(link.to_node.outputs[0]) # recursive until find an output material node
            if ret is True:
                return True

    return False

def get_socket(blender_material: bpy.types.Material, name: str):
    """
    For a given material input name, retrieve the corresponding node tree socket.

    :param blender_material: a blender material for which to get the socket
    :param name: the name of the socket
    :return: a blender NodeSocket
    """
    if blender_material.node_tree and blender_material.use_nodes:
        #i = [input for input in blender_material.node_tree.inputs]
        #o = [output for output in blender_material.node_tree.outputs]
        
        type = bpy.types.ShaderNodeBsdfPrincipled
        nodes = [n for n in blender_material.node_tree.nodes if isinstance(n, type) and not n.mute]
        nodes = [node for node in nodes if check_if_is_linked_to_active_output(node.outputs[0])]
        inputs = sum([[input for input in node.inputs if input.name == name] for node in nodes], [])
        if inputs:
            return inputs[0]

    return None


def get_factor_from_socket(socket, kind):
    """
    For baseColorFactor, metallicFactor, etc.
    Get a constant value from a socket, or a constant value
    from a MULTIPLY node just before the socket.
    kind is either 'RGB' or 'VALUE'.
    """
    fac = get_const_from_socket(socket, kind)
    if fac is not None:
        return fac

    node = previous_node(socket)
    if node is not None:
        x1, x2 = None, None
        if kind == 'RGB':
            if node.type == 'MIX_RGB' and node.blend_type == 'MULTIPLY':
                # TODO: handle factor in inputs[0]?
                x1 = get_const_from_socket(node.inputs[1], kind)
                x2 = get_const_from_socket(node.inputs[2], kind)
        if kind == 'VALUE':
            if node.type == 'MATH' and node.operation == 'MULTIPLY':
                x1 = get_const_from_socket(node.inputs[0], kind)
                x2 = get_const_from_socket(node.inputs[1], kind)
        if x1 is not None and x2 is None: return x1
        if x2 is not None and x1 is None: return x2

    return None


def get_const_from_socket(socket, kind):
    if not socket.is_linked:
        if kind == 'RGB':
            if socket.type != 'RGBA': return None
            return list(socket.default_value)[:3]
        if kind == 'VALUE':
            if socket.type != 'VALUE': return None
            return socket.default_value

    # Handle connection to a constant RGB/Value node
    prev_node = previous_node(socket)
    if prev_node is not None:
        if kind == 'RGB' and prev_node.type == 'RGB':
            return list(prev_node.outputs[0].default_value)[:3]
        if kind == 'VALUE' and prev_node.type == 'VALUE':
            return prev_node.outputs[0].default_value

    return None


def previous_socket(socket):
    while True:
        if not socket.is_linked:
            return None

        from_socket = socket.links[0].from_socket

        # Skip over reroute nodes
        if from_socket.node.type == 'REROUTE':
            socket = from_socket.node.inputs[0]
            continue

        return from_socket


def previous_node(socket):
    prev_socket = previous_socket(socket)
    if prev_socket is not None:
        return prev_socket.node
    return None


def previous_node_typed(socket, ty):
    previous = previous_node(socket)
    if isinstance(previous, ty):
        return previous
    return None


def find_previous_nodes_of_type(socket, node_type):
    matching_nodes = []
    
    for link in socket.links:
        # follow the link to a shader node
        linked_node = link.from_node
        # check if the node matches the filter
        if isinstance(linked_node, node_type):
            matching_nodes.append(linked_node)
        # traverse into inputs of the node
        for input_socket in linked_node.inputs:
            linked_results = find_previous_nodes_of_type(input_socket, node_type)
            if linked_results:
                # add the link to the current path
                matching_nodes.extend(linked_results)
    
    return matching_nodes


