#version 450
#extension GL_ARB_separate_shader_objects : enable

// @[immutable_samplers([
//         (
//             mag_filter: Linear,
//             min_filter: Linear,
//             address_mode_u: Repeat,
//             address_mode_v: Repeat,
//             address_mode_w: Repeat,
//             anisotropy_enable: false,
//             max_anisotropy: 1.0,
//             border_color: IntOpaqueBlack,
//             unnormalized_coordinates: false,
//             compare_enable: false,
//             compare_op: Always,
//             mipmap_mode: Linear,
//             mip_lod_bias: 0,
//             min_lod: 0,
//             max_lod: 0
//         )
// ])]
layout (set = 0, binding = 1) uniform sampler smp;

layout (set = 1, binding = 0) uniform texture2D tex;

layout(location = 0) in vec2 uv;
layout(location = 1) in vec4 color;

layout(location = 0) out vec4 out_color;

void main() {
    out_color = texture(sampler2D(tex, smp), uv) * color;
}