#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

#include "mesh_adv_types.glsl"
#include "shadow_atlas_depth.glsl"

// @[semantic("POSITION")]
layout (location = 0) in vec3 in_pos;

out float gl_ClipDistance[4];

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

    vec4 clip_space = model_view_proj * vec4(in_pos, 1.0);

    // We implicitly clip 0 < w < 1, we also clip -w < x < w to be in the view frustum
    gl_ClipDistance[0] = clip_space.x + clip_space.w;
    gl_ClipDistance[1] = clip_space.w - clip_space.x;
    gl_ClipDistance[2] = clip_space.y + clip_space.w;
    gl_ClipDistance[3] = clip_space.w - clip_space.y;

    // 2d coordinates with perspective divide
    vec2 ndc_xy = clip_space.xy / clip_space.w;

    // [-1, 1] -> [0, 1]
    vec2 unit_xy = (ndc_xy + 1.0) / 2.0;

    // [0, 1] -> uv coordinates
    unit_xy.x = mix(per_view_data.uv_min.x, per_view_data.uv_max.x, unit_xy.x);
    unit_xy.y = 1 - mix(per_view_data.uv_min.y, per_view_data.uv_max.y, 1 - unit_xy.y);

    // back to clip space
    vec2 clip_xy = (unit_xy * 2.0 - 1.0) * clip_space.w;

    gl_Position = vec4(clip_xy.x, clip_xy.y, clip_space.z, clip_space.w);
}

// When passed a correct view_proj_atlassed matrix, the output of this version is the same as the above version but
// has more math ops in it
/*
void main() {
    mat4 model_view_proj = per_view_data.view_proj * in_model_matrix;
    vec4 clip_space = model_view_proj * vec4(in_pos, 1.0);
    gl_ClipDistance[0] = clip_space.x + clip_space.w;
    gl_ClipDistance[1] = clip_space.w - clip_space.x;
    gl_ClipDistance[2] = clip_space.y + clip_space.w;
    gl_ClipDistance[3] = clip_space.w - clip_space.y;

    mat4 model_view_proj_atlassed = per_view_data.view_proj_atlassed * in_model_matrix;
    vec4 clip_space_atlassed = model_view_proj_atlassed * vec4(in_pos, 1.0);
    gl_Position = clip_space_atlassed;
}
*/