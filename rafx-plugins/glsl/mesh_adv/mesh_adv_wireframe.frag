#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

#include "mesh_adv_wireframe.glsl"

layout (location = 0) out vec4 out_color;

void main() {
    out_color = vec4(1, 1, 1, 1);
}
