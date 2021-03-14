#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

// set = 2, binding = 0
#include "mesh.glsl"

// @[semantic("POSITION")]
layout (location = 0) in vec3 in_pos;

void main() {
    mat4 model_view_proj = per_view_data.view_proj * per_object_data.model;
    gl_Position = model_view_proj * vec4(in_pos, 1.0);
}
