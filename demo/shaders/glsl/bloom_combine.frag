#version 450
#extension GL_ARB_separate_shader_objects : enable
#include "tonemapping.glsl"

// @[export]
layout (set = 0, binding = 0) uniform texture2D in_color;

// @[export]
layout (set = 0, binding = 1) uniform texture2D in_blur;

// @[immutable_samplers([
//         (
//             mag_filter: Nearest,
//             min_filter: Nearest,
//             mip_map_mode: Linear,
//             address_mode_u: ClampToEdge,
//             address_mode_v: ClampToEdge,
//             address_mode_w: ClampToEdge,
//         )
// ])]
layout (set = 0, binding = 2) uniform sampler smp;

// @[export]
// @[internal_buffer]
layout (set = 0, binding = 3) uniform Config {
    int tonemapper_type;
} config;

layout (location = 0) in vec2 inUV;

layout (location = 0) out vec4 out_sdr;


void main()
{
    vec4 color = texture(sampler2D(in_color, smp), inUV) + texture(sampler2D(in_blur, smp), inUV);

    out_sdr = vec4(tonemap(color.rgb, config.tonemapper_type), color.a);
}
