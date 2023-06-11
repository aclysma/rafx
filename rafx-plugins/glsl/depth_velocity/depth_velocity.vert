#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

#include "../mesh_adv/mesh_adv_types.glsl"
#include "depth_velocity.glsl"
#include "../util/taa_jitter.glsl"

// @[semantic("POSITION")]
layout (location = 0) in vec3 in_pos;

layout (location = 0) out vec4 out_old_position_clip;
layout (location = 1) out vec4 out_new_position_clip;

invariant gl_Position;

//TODO: Remove hack if (data_array_index < 786432) { - did I bump to 512 lights per cluster and forget?

void main() {
    // WARNING: The operations that contribute to computing gl_Position must be kept in EXACT sync with other places to
    // ensure deterministic values are generated between depth prepass and rendering
#ifdef PLATFORM_DX12
    uint instance_index = push_constants.instance_offset;
    // HACK: GBV seems to cause instance_index to be bad values, this protects from causing a crash
    if (instance_index > all_draw_data.count) {
        instance_index = 0;
    }
#else
    uint instance_index = gl_InstanceIndex;
#endif

    DrawData draw_data = all_draw_data.draw_data[instance_index];
    mat4 current_model_matrix = all_transforms.transforms[draw_data.transform_index].current_model_matrix;
    mat4 current_model_view_proj = per_view_data.current_view_proj * current_model_matrix;

    vec4 current_position_clip = current_model_view_proj * vec4(in_pos, 1.0);
    gl_Position = add_jitter(current_position_clip, per_view_data.jitter_amount);
    // End deterministic path

    out_new_position_clip = current_position_clip;

    mat4 previous_model_matrix = all_transforms.transforms[draw_data.transform_index].previous_model_matrix;
    out_old_position_clip = per_view_data.previous_view_proj * previous_model_matrix * vec4(in_pos, 1.0);
}
