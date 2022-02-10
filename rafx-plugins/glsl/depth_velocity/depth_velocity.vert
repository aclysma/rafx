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

void main() {

    // draw_data_index push constant can be replaced by gl_DrawID
    DrawData draw_data = all_draw_data.draw_data[gl_InstanceIndex];
    mat4 previous_model_matrix = all_transforms.transforms[draw_data.transform_index].previous_model_matrix;
    mat4 current_model_matrix = all_transforms.transforms[draw_data.transform_index].current_model_matrix;

    out_old_position_clip = per_view_data.previous_view_proj * previous_model_matrix * vec4(in_pos, 1.0);

    vec4 new_position_clip = per_view_data.current_view_proj * current_model_matrix * vec4(in_pos, 1.0);
    out_new_position_clip = new_position_clip;
    gl_Position = add_jitter(new_position_clip, per_view_data.jitter_amount);
}
