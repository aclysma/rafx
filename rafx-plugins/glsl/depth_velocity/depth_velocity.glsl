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
    // The count is used to avoid a bug on nvidia when GBV is enabled where it seems the push constant is just invalid
    // and walks past the end of the array
    uint count;
    uint pad0;
    uint pad1;
    uint pad2;
    DrawData draw_data[];
} all_draw_data;

#ifdef PLATFORM_DX12
    layout (push_constant) uniform PushConstantData {
        uint instance_offset;
    } push_constants;
#endif // PLATFORM_DX12
