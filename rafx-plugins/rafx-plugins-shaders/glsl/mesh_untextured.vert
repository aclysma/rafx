#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

#include "mesh_pbr_uniform.glsl"
#include "mesh_pbr_vert.glsl"

void main() {
    pbr_main();
}
