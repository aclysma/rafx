// @[export]
// @[internal_buffer]
layout (set = 0, binding = 0) uniform PerViewData {
    mat4 view;
    mat4 view_proj;
} per_view_data;

// @[export]
// @[internal_buffer]
layout(set = 2, binding = 0) uniform PerObjectData {
    mat4 model;
} per_object_data;