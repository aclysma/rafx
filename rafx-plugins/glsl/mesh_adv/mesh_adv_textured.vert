#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_ARB_shader_draw_parameters : enable

#define PBR_TEXTURES
#include "mesh_adv_pbr_bindings.glsl"
#include "mesh_adv_pbr_vert.glsl"

void main() {
    pbr_main();
}
