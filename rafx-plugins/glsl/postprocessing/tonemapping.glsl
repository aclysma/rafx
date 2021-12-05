#include "exposure.glsl"
#include "tonemapping_bgfx.glsl"

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
    return vf.xyz / vf.www;
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

    return numerator / denominator;
}

//////////////////// BRUNO OPSENICA //////////////////////
// Based on tutorial at https://bruop.github.io/tonemapping/
//
// Uses color correction functions from bgfx

float reinhard2(float x, float whitepoint) {
    return (x * (1.0 + (x / (whitepoint * whitepoint)))) / (1.0 + x);
}


vec3 old_autoexposure_tonemapping(vec3 in_color, float histogram_result_average_luminosity_interpolated) {
    // Below color conversion won't work on pure black, but just map black to black to avoid NaN
    if (dot(in_color, vec3(1.0)) < 0.0001) {
        return vec3(0.0);
    }

    float average_luma = clamp(histogram_result_average_luminosity_interpolated, 0.0005, 0.7);

    vec3 Yxy = convertRGB2Yxy(in_color);
    float gray = 0.03;
    float white_squared = 1.0;

    float lp = Yxy.x * gray / (average_luma + 0.0001);
    Yxy.x = reinhard2(lp, white_squared);

    return convertYxy2RGB(Yxy);
}

//////////////////// Karl Bergström Tonemapper //////////////////////
// A hue-preserving tonemapper which maps a luminance range 
// onto an output range. It supports HDR output ranges.
// It uses a modified reinhard function in the bottom and upper parts
// to ensure a continuous function, and desaturates colors as they
// approach a maximum luminance value to avoid changing channel ratios.

const mat3 sRGB_to_Oklab_LMS = mat3(
    0.41224208, 0.21194293, 0.08835888, 
    0.53626156, 0.68070215, 0.28184745,
    0.051428035, 0.10737409, 0.63012964);

const mat3 Oklab_LMS_to_sRGB = mat3(
    4.0765376, -1.2686057, -0.0041975603,
    -3.307096, 2.6097474, -0.70356846,
    0.23082244, -0.34116364, 1.7072057);

const mat3 OKLAB_M_2 =
        mat3(0.2104542553,1.9779984951,0.02599040371,
             0.7936177850,-2.4285922050,0.7827717662,
             -0.0040720468,0.4505937099,-0.8086757660);

const mat3 OKLAB_M_2_INVERSE =
        mat3(0.99998146, 1.0000056, 1.0001117,
             0.39633045, -0.10555917, -0.08943998,
             0.21579975, -0.06385299, -1.2914615);

vec3 Oklab_lms_to_Oklab(vec3 lms) {
    lms = pow(max(lms, 0.0), vec3(1.0/3.0));
    return OKLAB_M_2 * lms;
}

vec3 Oklab_to_Oklab_lms(vec3 oklab) {
    vec3 lms = OKLAB_M_2_INVERSE * oklab;
    return pow(lms, vec3(3.0));
}

// modified reinhard with derivative control (k)
float modified_reinhard(float x, float m, float k) {
    float kx = k * x;
    return (kx * (1 + (x / (k*m*m)))) / (1 + kx);
}

vec3 oklab_to_linear_srgb(vec3 oklab) {
    return Oklab_LMS_to_sRGB * Oklab_to_Oklab_lms(oklab);
}
vec3 linear_srgb_to_oklab(vec3 rgb) {
    return Oklab_lms_to_Oklab(sRGB_to_Oklab_LMS * rgb);
}

vec3 tonemap_bergstrom(
    vec3 in_color,
    float max_component_value,
    float histogram_result_low_luminosity_interpolated,
    float histogram_result_high_luminosity_interpolated,
    float histogram_result_max_luminosity_interpolated
) {
    // Range of luminance we are mapping from.
    float l_low = histogram_result_low_luminosity_interpolated;
    float l_high = max(histogram_result_high_luminosity_interpolated, l_low + 0.01);
    // The upper range's modified reinhard converges on 1 at l_max,
    // which we set to the max of the most luminant pixel on the screen and l_high * l_max_scale
    // The value of l_max_scale is fairly arbitrary
    float l_max_scale = 5;
    float l_max = max(histogram_result_max_luminosity_interpolated, l_high * l_max_scale);

    // Range of linear srgb we are mapping into. Values between [l_low, l_high] are linearly mapped to [k_low, k_high]
    const float k_low = 0.01; // srgb_eotf(k_low) = 0.1 
    float k_max = max_component_value; // this could be the monitor max brightness
    // srgb_eotf(0.214) = 0.5, meaning we map the [l_low,l_high] luminance range into 0.1-0.5 post-sRGB
    const float k_high = 0.214; 
    float k_desaturation = mix(k_high, k_max, 0.5);

    float v = (k_high - k_low) / (l_high - l_low);

    float luminance = dot(in_color, vec3(0.2126, 0.7152, 0.0722));

    // Piecewise function maps from luminance to value to emit to screen
    float adjusted_luminance = 0.0;
    if (luminance < l_low) {
        // lower range is an inverted modified reinhard
        adjusted_luminance = k_low - k_low * modified_reinhard(l_low - luminance, l_low, v / k_low);
    } else if (luminance < l_high) {
        // luminance range [l_low, l_high] is linearly mapped to [k_low, k_high]
        adjusted_luminance = k_low + (luminance - l_low) * v;
    } else {
        // upper range is modified reinhard towards k_max, converging on k_max at l_max
        adjusted_luminance = k_high + (k_max - k_high) * modified_reinhard(luminance - l_high, l_max, v/(k_max - k_high));
    }

    if (adjusted_luminance < 0.0001) {
        return vec3(0.0);
    } else {
        vec3 out_color = in_color * (adjusted_luminance / (luminance + 0.000001));
        float max_element = max(out_color.x, max(out_color.y, out_color.z));
        
        // conversions to oklab don't work well outside 0-1 sRGB range
        // so scale back into range, then scale back after we're done in oklab
        if (max_element > 1) {
            out_color = (out_color / max_element);
        }
        vec3 oklab = linear_srgb_to_oklab(out_color);
        // desaturate between k_desaturation and k_max to make things feel brighter
        oklab.yz *= 1.0 - clamp((adjusted_luminance - k_desaturation) / (k_max - k_desaturation), 0.0, 1.0);
        out_color = oklab_to_linear_srgb(oklab);
        if (max_element > 1) {
            out_color *= max_element;
        }

        return out_color;
    }
}

//////////////////// DEBUG TONEMAPPING //////////////////////

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
const int TM_None = 0;
const int TM_StephenHillACES = 1;
const int TM_SimplifiedLumaACES = 2;
const int TM_Hejl2015 = 3;
const int TM_Hable = 4;
const int TM_FilmicALU = 5;
const int TM_LogDerivative = 6;
const int TM_VisualizeRGBMax = 7;
const int TM_VisualizeLuma = 8;
const int TM_AutoExposureOld = 9;
const int TM_Bergstrom = 10;



vec3 tonemap(
    vec3 color,
    int tonemapper_type,
    float max_component_value,
    float histogram_result_low_luminosity_interpolated,
    float histogram_result_average_luminosity_interpolated,
    float histogram_result_high_luminosity_interpolated,
    float histogram_result_max_luminosity_interpolated
) {
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
        case TM_AutoExposureOld: {
            return old_autoexposure_tonemapping(
                color,
                histogram_result_average_luminosity_interpolated
            );
        } break;
        case TM_Bergstrom: {
            return tonemap_bergstrom(
                color,
                max_component_value,
                histogram_result_low_luminosity_interpolated,
                histogram_result_high_luminosity_interpolated,
                histogram_result_max_luminosity_interpolated
            );
        } break;
        default: {
            return color;
        } break;
    }
}