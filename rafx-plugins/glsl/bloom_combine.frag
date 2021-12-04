#version 450
#extension GL_ARB_separate_shader_objects : enable
#include "tonemapping.glsl"
#include "luma_histogram_types.glsl"

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

// Should be kept in sync with the constants in OutputColorSpace
const int OUTPUT_COLOR_SPACE_SRGB = 0;
const int OUTPUT_COLOR_SPACE_P3 = 1;

// @[export]
// @[internal_buffer]
layout (set = 0, binding = 3) uniform Config {
    int tonemapper_type;
    int output_color_space;
    float max_color_component_value;
} config;

layout(set = 0, binding = 4) buffer HistogramResultBuffer
{
    HistogramResult result;
} histogram_result;

layout (location = 0) in vec2 inUV;

layout (location = 0) out vec4 out_sdr;

// Source: kolor
const mat3 sRGB_to_P3 = mat3(
    0.8224886, 0.033200048, 0.017089065,
    0.17751142, 0.9668, 0.072411515,
    0.00000000000000005551115, -0.000000000000000017347235, 0.9104994
);

void main()
{
    // Combine SDR + blurred HDR
    vec4 rgb = texture(sampler2D(in_color, smp), inUV) + texture(sampler2D(in_blur, smp), inUV);
    vec3 color = rgb.rgb;

    if (color == vec3(0)) {
        out_sdr = vec4(color, 1);
        return;
    }

    vec3 color_srgb_linear = tonemap(
        color,
        config.tonemapper_type,
        config.max_color_component_value,
        histogram_result.result.low_luminosity_interpolated,
        histogram_result.result.average_luminosity_interpolated,
        histogram_result.result.high_luminosity_interpolated,
        histogram_result.result.max_luminosity_interpolated
    );

    switch (config.output_color_space)
    {
        case OUTPUT_COLOR_SPACE_SRGB:
            out_sdr = vec4(color_srgb_linear, 1.0);
            break;
        case OUTPUT_COLOR_SPACE_P3:
            vec3 color_linear_p3 = sRGB_to_P3 * color_srgb_linear;
            out_sdr = vec4(color_linear_p3, 1.0);
            break;
    }
}
