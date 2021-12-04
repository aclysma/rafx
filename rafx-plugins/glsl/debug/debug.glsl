// @[export]
// @[internal_buffer]
layout(set = 0, binding = 0) uniform PerFrameUbo {
    mat4 view_proj;
} per_frame_data;
