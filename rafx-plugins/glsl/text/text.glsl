// @[export]
// @[internal_buffer]
layout(set = 1, binding = 0) uniform PerViewUbo {
    mat4 view_proj;
} per_view_data;

// @[immutable_samplers([
//         (
//             mag_filter: Linear,
//             min_filter: Linear,
//             mip_map_mode: Linear,
//             address_mode_u: Repeat,
//             address_mode_v: Repeat,
//             address_mode_w: Repeat,
//             max_anisotropy: 16.0,
//         )
// ])]
layout (set = 0, binding = 1) uniform sampler smp;

// @[export]
layout (set = 0, binding = 0) uniform texture2D tex;
