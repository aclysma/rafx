//=================================================================================================
//  The following code is licensed under the MIT license and has been ported from hlsl code
//  in Baking Lab by MJP and David Neubelt
//  http://mynameismjp.wordpress.com/
//=================================================================================================

// The two functions below were based on code and explanations provided by Padraic Hennessy (@PadraicHennessy).
// See this for more info: https://placeholderart.wordpress.com/2014/11/21/implementing-a-physically-based-camera-manual-exposure/


// Scale factor used for storing physical light units in fp16 floats (equal to 2^-10).
const float FP16Scale = 0.0009765625f;

const float APERTURE_F_NUMBER = 0.01;
const float ISO = 1.0;
const float SHUTTER_SPEED_VALUE = 1.0;
const float KEY_VALUE = 0.1150;
const float MANUAL_EXPOSURE = -16.0;
const float WHITE_POINT_HABLE = 6.0;
const float WHITE_POINT_HEJL = 1.0;
const int EXPOSURE_MODE_AUTO = 1;
const int EXPOSURE_MODE_MANUAL_SBS = 2;
const int EXPOSURE_MODE_MANUAL_SOS = 3;
const int EXPOSURE_MODE = EXPOSURE_MODE_MANUAL_SBS;

float saturation_based_exposure()
{
    float maxLuminance = (7800.0f / 65.0f) * (APERTURE_F_NUMBER * APERTURE_F_NUMBER) / (ISO * SHUTTER_SPEED_VALUE);
    return log2(1.0f / maxLuminance);
}

float standard_output_based_exposure(float middleGrey /* = 0.18f */)
{
    float lAvg = (1000.0f / 65.0f) * (APERTURE_F_NUMBER * APERTURE_F_NUMBER) / (ISO * SHUTTER_SPEED_VALUE);
    return log2(middleGrey / lAvg);
}
float default_standard_output_based_exposure()
{
    return standard_output_based_exposure(0.18);
}

float log2_exposure(float avgLuminance)
{
    float exposure = 0.0f;

    if(EXPOSURE_MODE == EXPOSURE_MODE_AUTO)
    {
        avgLuminance = max(avgLuminance, 0.00001f);
        float linearExposure = (KEY_VALUE / avgLuminance);
        exposure = log2(max(linearExposure, 0.00001f));
    }
    else if(EXPOSURE_MODE == EXPOSURE_MODE_MANUAL_SBS)
    {
        exposure = saturation_based_exposure();
        exposure -= log2(FP16Scale);
    }
    else if(EXPOSURE_MODE == EXPOSURE_MODE_MANUAL_SOS)
    {
        exposure = default_standard_output_based_exposure();
        exposure -= log2(FP16Scale);
    }
    else
    {
        exposure = MANUAL_EXPOSURE;
        exposure -= log2(FP16Scale);
    }

    return exposure;
}

float linear_exposure(float avgLuminance)
{
    return exp2(log2_exposure(avgLuminance));
}

// Determines the color based on exposure settings
vec3 calc_exposed_color(vec3 color, float avgLuminance, float offset, out float exposure)
{
    exposure = log2_exposure(avgLuminance);
    exposure += offset;
    return exp2(exposure) * color;
}