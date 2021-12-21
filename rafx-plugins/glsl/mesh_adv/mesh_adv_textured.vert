#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

#define PBR_TEXTURES
#include "mesh_adv_pbr_uniform.glsl"
#include "mesh_adv_pbr_textures.glsl"
#include "mesh_adv_pbr_vert.glsl"

void main() {
    pbr_main();
}
