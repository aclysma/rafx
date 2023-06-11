#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

#include "mesh_adv_wireframe.glsl"

// @[semantic("POSITION")]
layout (location = 0) in vec3 in_pos;

void main() {
#ifdef PLATFORM_DX12
    uint instance_index = push_constants.instance_offset;
    // HACK: GBV seems to cause instance_index to be bad values, this protects from causing a crash
    if (instance_index > all_draw_data.count) {
        instance_index = 0;
    }
#else
    uint instance_index = gl_InstanceIndex;
#endif

    // draw_data_index push constant can be replaced by gl_DrawID
    DrawData draw_data = all_draw_data.draw_data[instance_index];
    mat4 model_matrix = all_transforms.transforms[draw_data.transform_index].model_matrix;

    mat4 model_view_proj = per_view_data.view_proj * model_matrix;
    gl_Position = model_view_proj * vec4(in_pos, 1.0);
}
