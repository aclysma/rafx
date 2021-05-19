#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

#define PBR_TEXTURES
#include "mesh_pbr_uniform.glsl"
#include "mesh_pbr_textures.glsl"
#include "mesh_pbr_frag.glsl"

layout (location = 0) out vec4 out_color;

void main() {
    out_color = pbr_main();
}
