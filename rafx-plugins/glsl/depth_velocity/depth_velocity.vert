#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

#include "depth_velocity.glsl"
#include "../util/taa_jitter.glsl"

// @[semantic("POSITION")]
layout (location = 0) in vec3 in_pos;

// @[semantic("CURRENTMODELMATRIX")]
layout (location = 1) in mat4 in_current_model_matrix; // Uses locations 1-4. The semantic will be named `CURRENTMODELMATRIX0` through `CURRENTMODELMATRIX3`.
// layout (location = 2) in mat4 in_current_model_matrix;
// layout (location = 3) in mat4 in_current_model_matrix;
// layout (location = 4) in mat4 in_current_model_matrix;

// If the previous frame is not specified, it will be set to the current frame's state
// @[semantic("PREVIOUSMODELMATRIX")]
layout (location = 5) in mat4 in_previous_model_matrix; // Uses locations 5-8. The semantic will be named `PREVIOUSMODELMATRIX0` through `PREVIOUSMODELMATRIX3`.
// layout (location = 6) in mat4 in_previous_model_matrix;
// layout (location = 7) in mat4 in_previous_model_matrix;
// layout (location = 8) in mat4 in_previous_model_matrix;

layout (location = 0) out vec4 out_old_position_clip;
layout (location = 1) out vec4 out_new_position_clip;

void main() {
    out_old_position_clip = per_view_data.previous_view_proj * in_previous_model_matrix * vec4(in_pos, 1.0);

    vec4 new_position_clip = per_view_data.current_view_proj * in_current_model_matrix * vec4(in_pos, 1.0);
    out_new_position_clip = new_position_clip;
    gl_Position = add_jitter(new_position_clip, per_view_data.jitter_amount);
}
