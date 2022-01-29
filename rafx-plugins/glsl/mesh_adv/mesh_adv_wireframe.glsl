
#include "mesh_adv_types.glsl"

// @[export]
// @[internal_buffer]
layout (set = 0, binding = 0) uniform PerViewData {
    mat4 view;
    mat4 view_proj;
} per_view_data;

layout (set = 1, binding = 0) buffer AllTransforms {
    Transform transforms[];
} all_transforms;

layout (set = 1, binding = 1) buffer AllDrawData {
    DrawData draw_data[];
} all_draw_data;

// @[export]
layout (push_constant) uniform PushConstants {
    uint draw_data_index;
} constants;
