#version 450
#extension GL_ARB_separate_shader_objects : enable

#include "../mesh_adv/mesh_adv_types.glsl"
#include "depth_velocity.glsl"
#include "../util/taa_jitter.glsl"

layout(location = 0) in vec4 in_old_position_clip;
layout(location = 1) in vec4 in_new_position_clip;

layout(location = 0) out vec2 out_velocity;

void main() {
    // Perspective divide
    vec2 old_position_ndc = (in_old_position_clip.xy/abs(in_old_position_clip.w));
    vec2 new_position_ndc = (in_new_position_clip.xy/abs(in_new_position_clip.w));
    out_velocity = new_position_ndc - old_position_ndc;
}
