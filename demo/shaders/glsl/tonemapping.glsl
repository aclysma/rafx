#include "exposure.glsl"
// The code for ACESFitted was originally written by Stephen Hill (@self_shadow), who deserves all
// credit for coming up with this fit and implementing it. Buy him a beer next time you see him. :)
// The code is licensed under the MIT license. The code has been converted to glsl, and was originally found at:
// https://github.com/TheRealMJP/BakingLab/blob/master/BakingLab/ACES.hlsl

// sRGB => XYZ => D65_2_D60 => AP1 => RRT_SAT
// Original:
// static const float3x3 ACESInputMat =
// {
//     {0.59719, 0.35458, 0.04823},
//     {0.07600, 0.90834, 0.01566},
//     {0.02840, 0.13383, 0.83777}
// };

// Adapted to glsl: 
const mat3 ACESInputMat = 
mat3(
    0.59719, 0.07600,  0.02840,
    0.35458, 0.90834, 0.13383,
    0.04823,0.01566, 0.83777
);

// ODT_SAT => XYZ => D60_2_D65 => sRGB
// Original:
// static const float3x3 ACESOutputMat =
// {
//     { 1.60475, -0.53108, -0.07367},
//     {-0.10208,  1.10813, -0.00605},
//     {-0.00327, -0.07276,  1.07602}
// };
// Adapted to glsl:
const mat3 ACESOutputMat =
mat3(
    1.60475, -0.10208, -0.00327,
    -0.53108, 1.10813, -0.07276,
    -0.07367, -0.00605, 1.07602
);

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


// Applies the filmic curve from John Hable's presentation
vec3 tonemap_filmic_alu(vec3 color_in)
{
    vec3 color = max(color_in - 0.004f, 0);
    color = (color * (6.2f * color + 0.5f)) / (color * (6.2f * color + 1.7f)+ 0.06f);
    return color;
}

// Converts a color from linear light gamma to sRGB gamma
vec3 linear_to_srgb(vec3 linearRGB)
{
    bvec3 cutoff = lessThan(linearRGB, vec3(0.0031308));
    vec3 higher = vec3(1.055)*pow(linearRGB, vec3(1.0/2.4)) - vec3(0.055);
    vec3 lower = linearRGB * vec3(12.92);

    return mix(higher, lower, cutoff);
}

// Converts a color from sRGB gamma to linear light gamma
vec3 srgb_to_linear(vec3 sRGB)
{
    bvec3 cutoff = lessThan(sRGB, vec3(0.04045));
    vec3 higher = pow((sRGB + vec3(0.055))/vec3(1.055), vec3(2.4));
    vec3 lower = sRGB/vec3(12.92);

    return mix(higher, lower, cutoff);
}

const float SHOULDER_STRENGTH = 4.0;
const float LINEAR_STRENGTH = 5.0;
const float LINEAR_ANGLE = 0.1200;
const float TOE_STRENGTH = 13.0;

vec3 tonemap_Hejl2015(vec3 hdr)
{
    vec4 vh = vec4(hdr, WHITE_POINT_HEJL);
    vec4 va = (1.435f * vh) + 0.05;
    vec4 vf = ((vh * va + 0.004f) / ((vh * (va + 0.55f) + 0.0491f))) - 0.0821f;
    return linear_to_srgb(vf.xyz / vf.www);
}

vec3 hable_function(in vec3 x) {
    const float A = SHOULDER_STRENGTH;
    const float B = LINEAR_STRENGTH;
    const float C = LINEAR_ANGLE;
    const float D = TOE_STRENGTH;

    // Not exposed as settings
    const float E = 0.01f;
    const float F = 0.3f;

    return ((x * (A * x + C * B)+ D * E) / (x * (A * x + B) + D * F)) - E / F;
}

vec3 tonemap_hable(in vec3 color) {
    vec3 numerator = hable_function(color);
    vec3 denominator = hable_function(vec3(WHITE_POINT_HABLE));

    return linear_to_srgb(numerator / denominator);
}

float luma(vec3 color) {
  return dot(color, vec3(0.299, 0.587, 0.114));
}

vec3 visualize_value(float val) {
    // blue is used to visualize 0-1 exclusively in a linear fashion.
    // green covers 0-5 and is a parabolic curve, so it transitions over into red.
    // red is 3+ and is a log-ish curve so that it can handle differences in very large values
    float g = 1 - 0.2 * (val - 3.23605) * (val - 3.23605);
    float b = val;
    float r = 1 - 1 / (0.5 * val - 0.5);
    // the transition blue -> green is hard, to make it easier to spot when values go over 1
    if (val > 1.0) { 
        b = 0;
    }
    if (val < 3.0) {
        r = 0;
    }
    return clamp(vec3(r, g, b), 0, 1);
}


// Should be kept in sync with the constants in TonemapperType
const int TM_StephenHillACES = 1;
const int TM_SimplifiedLumaACES = 2;
const int TM_Hejl2015 = 3;
const int TM_Hable = 4;
const int TM_FilmicALU = 5;
const int TM_LogDerivative = 6;
const int TM_VisualizeRGBMax = 7;
const int TM_VisualizeLuma = 8;

vec3 tonemap(vec3 color, int tonemapper_type) {
    // tonemapping.. TODO: implement auto-exposure
    switch (tonemapper_type) {
        case TM_StephenHillACES:  {
            return linear_to_srgb(tonemap_aces_fitted(color));
        } break;
        case TM_SimplifiedLumaACES: {
            return linear_to_srgb(tonemap_aces_film_simple(color * 0.6));
        } break;
        case TM_Hejl2015: {
            return tonemap_Hejl2015(color);
        } break;
        case TM_Hable: {
            return tonemap_hable(color);
        } break;
        case TM_FilmicALU: {
            return tonemap_filmic_alu(color);
        } break;
        case TM_LogDerivative: {
            return color.rgb / (color.rgb + vec3(1.0));
        } break;
        case TM_VisualizeRGBMax: {
            float max_val = max(color.r, max(color.g, color.b));
            return visualize_value(max_val);
        } break;
        case TM_VisualizeLuma: {
            float l = luma(color);
            return visualize_value(l);
        } break;
        default: {
            return color;
        } break;
    }
}