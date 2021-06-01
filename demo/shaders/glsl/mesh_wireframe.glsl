// @[export]
// @[internal_buffer]
layout (set = 0, binding = 0) uniform PerViewData {
    mat4 view;
    mat4 view_proj;
} per_view_data;