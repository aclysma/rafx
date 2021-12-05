#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

#include "mesh_basic_wireframe.glsl"

// @[semantic("POSITION")]
layout (location = 0) in vec3 in_pos;

// @[semantic("MODELMATRIX")]
layout (location = 1) in mat4 in_model_matrix; // Uses locations 1-4. The semantic will be named `MODELMATRIX0` through `MODELMATRIX3`.
// layout (location = 2) in mat4 in_model_matrix;
// layout (location = 3) in mat4 in_model_matrix;
// layout (location = 4) in mat4 in_model_matrix;

void main() {
    mat4 model_view_proj = per_view_data.view_proj * in_model_matrix;
    gl_Position = model_view_proj * vec4(in_pos, 1.0);
}
