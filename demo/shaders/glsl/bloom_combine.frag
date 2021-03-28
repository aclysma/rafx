#version 450
#extension GL_ARB_separate_shader_objects : enable

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

// The code for ACESFitted was originally written by Stephen Hill (@self_shadow), who deserves all
// credit for coming up with this fit and implementing it. Buy him a beer next time you see him. :)
// The code is licensed under the MIT license

// sRGB => XYZ => D65_2_D60 => AP1 => RRT_SAT
const mat3 ACESInputMat =
{
    {0.59719, 0.35458, 0.04823},
    {0.07600, 0.90834, 0.01566},
    {0.02840, 0.13383, 0.83777}
};

// ODT_SAT => XYZ => D60_2_D65 => sRGB
const mat3 ACESOutputMat =
{
    { 1.60475, -0.53108, -0.07367},
    {-0.10208,  1.10813, -0.00605},
    {-0.00327, -0.07276,  1.07602}
};

vec3 RRT_and_ODT_fit(vec3 v)
{
    vec3 a = v * (v + 0.0245786f) - 0.000090537f;
    vec3 b = v * (0.983729f * v + 0.4329510f) + 0.238081f;
    return a / b;
}

vec3 tonemap_aces_fitted(vec3 color)
{
    color = ACESInputMat * color;

    // Apply RRT and ODT
    color = RRT_and_ODT_fit(color);

    color = ACESOutputMat * color;

    // Clamp to [0, 1]
    color = clamp(color, 0, 1);

    return color;
}

// source: https://knarkowicz.wordpress.com/2016/01/06/aces-filmic-tone-mapping-curve/
vec3 tonemap_aces_film_simple(vec3 x)
{
    float a = 2.51f;
    float b = 0.03f;
    float c = 2.43f;
    float d = 0.59f;
    float e = 0.14f;
    return clamp((x*(a*x+b))/(x*(c*x+d)+e), 0.0, 1.0);
}

float luma(vec3 color) {
  return dot(color, vec3(0.299, 0.587, 0.114));
}

vec3 visualize_value(float val) {
    // parabolic curves visualize the range
    // blue is used to visualize 0-1 exclusively
    // green covers 0-5
    // red is 3+
    float g = 1 - 0.2 * (val - 3.23605) * (val - 3.23605);
    float b = 1 - 1 * (val - 1) * (val - 1);
    float r = 1 - 1 / (0.5 * val - 0.5);
    if (val > 1.0) { 
        b = 0;
    }
    if (val < 3.0) {
        r = 0;
    }
    return clamp(vec3(r, g, b), 0, 1);
}

void main()
{
    vec4 color = texture(sampler2D(in_color, smp), inUV) + texture(sampler2D(in_blur, smp), inUV);

    // tonemapping.. TODO: implement auto-exposure
    if (config.tonemapper_type == 1) {
        out_sdr = vec4(tonemap_aces_fitted(color.rgb), color.a);
    } else if (config.tonemapper_type == 2) {
        out_sdr = vec4(tonemap_aces_film_simple(color.rgb), color.a);
    } else if (config.tonemapper_type == 3) {
        out_sdr = vec4(color.rgb / (color.rgb + vec3(1.0)), color.a);
    } else if (config.tonemapper_type == 4) {
        float max_val = max(color.r, max(color.g, color.b));
        out_sdr = vec4(visualize_value(max_val), color.a);
    } else if (config.tonemapper_type == 5) {
        float l = luma(color.rgb);
        out_sdr = vec4(visualize_value(l), color.a);
    } else {
        out_sdr = color;
    }
    // out_sdr = vec4(mapped, color.a);
}
