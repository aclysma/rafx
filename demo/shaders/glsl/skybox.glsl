// @[immutable_samplers([
//     (
//         mag_filter: Linear,
//         min_filter: Linear,
//         mip_map_mode: Linear,
//         address_mode_u: ClampToEdge,
//         address_mode_v: ClampToEdge,
//         address_mode_w: ClampToEdge,
//     )
// ])]
layout (set = 0, binding = 0) uniform sampler smp;

// @[export]
layout (set = 0, binding = 1) uniform textureCube skybox_tex;

// @[export]
// @[internal_buffer]
layout(set = 0, binding = 2) uniform Args {
    mat4 inverse_projection;
    mat4 inverse_view;
} uniform_buffer;
