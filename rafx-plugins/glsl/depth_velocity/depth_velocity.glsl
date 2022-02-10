// @[export]
// @[internal_buffer]
layout (set = 0, binding = 0) uniform PerViewData {
    mat4 current_view_proj;
    mat4 current_view_proj_inv;
    // If the previous view_proj is not specified, it will be set to the current view's state
    mat4 previous_view_proj;
    uint viewport_width;
    uint viewport_height;
    vec2 jitter_amount;
} per_view_data;

layout (set = 1, binding = 0) buffer AllTransforms {
    TransformWithHistory transforms[];
} all_transforms;

layout (set = 1, binding = 1) buffer AllDrawData {
    DrawData draw_data[];
} all_draw_data;
