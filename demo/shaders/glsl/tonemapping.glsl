// The code for ACESFitted was originally written by Stephen Hill (@self_shadow), who deserves all
// credit for coming up with this fit and implementing it. Buy him a beer next time you see him. :)
// The code is licensed under the MIT license. The code has been converted to glsl, and was originally found at:
// https://github.com/TheRealMJP/BakingLab/blob/master/BakingLab/ACES.hlsl

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
const int TM_LogDerivative = 3;
const int TM_VisualizeRGBMax = 4;
const int TM_VisualizeLuma = 5;

vec3 tonemap(vec3 color, int tonemapper_type) {
    // tonemapping.. TODO: implement auto-exposure
    switch (tonemapper_type) {
        case TM_StephenHillACES:  {
            return tonemap_aces_fitted(color.rgb);
        } break;
        case TM_SimplifiedLumaACES: {
            return tonemap_aces_film_simple(color.rgb);
        } break;
        case TM_LogDerivative: {
            return color.rgb / (color.rgb + vec3(1.0));
        } break;
        case TM_VisualizeRGBMax: {
            float max_val = max(color.r, max(color.g, color.b));
            return visualize_value(max_val);
        } break;
        case TM_VisualizeLuma: {
            float l = luma(color.rgb);
            return visualize_value(l);
        } break;
        default: {
            return color;
        } break;
    }
}