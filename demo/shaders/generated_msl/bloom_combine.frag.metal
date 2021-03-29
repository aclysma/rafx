#pragma clang diagnostic ignored "-Wmissing-prototypes"

#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct Config
{
    int tonemapper_type;
};

struct spvDescriptorSetBuffer0
{
    texture2d<float> in_color [[id(0)]];
    texture2d<float> in_blur [[id(1)]];
    constant Config* config [[id(3)]];
};

struct main0_out
{
    float4 out_sdr [[color(0)]];
};

struct main0_in
{
    float2 inUV [[user(locn0)]];
};

static inline __attribute__((always_inline))
float3 RRT_and_ODT_fit(thread const float3& v)
{
    float3 a = (v * (v + float3(0.02457859925925731658935546875))) - float3(9.0537003416102379560470581054688e-05);
    float3 b = (v * ((v * 0.98372900485992431640625) + float3(0.4329510033130645751953125))) + float3(0.23808099329471588134765625);
    return a / b;
}

static inline __attribute__((always_inline))
float3 tonemap_aces_fitted(thread float3& color)
{
    color = float3x3(float3(0.59719002246856689453125, 0.075999997556209564208984375, 0.0284000001847743988037109375), float3(0.354579985141754150390625, 0.908339977264404296875, 0.13382999598979949951171875), float3(0.048229999840259552001953125, 0.0156599991023540496826171875, 0.837769985198974609375)) * color;
    float3 param = color;
    color = RRT_and_ODT_fit(param);
    color = float3x3(float3(1.60475003719329833984375, -0.10208000242710113525390625, -0.00326999998651444911956787109375), float3(-0.5310800075531005859375, 1.108129978179931640625, -0.07276000082492828369140625), float3(-0.0736699998378753662109375, -0.00604999996721744537353515625, 1.0760200023651123046875)) * color;
    color = fast::clamp(color, float3(0.0), float3(1.0));
    return color;
}

static inline __attribute__((always_inline))
float3 linear_to_srgb(thread const float3& linearRGB)
{
    bool3 cutoff = linearRGB < float3(0.003130800090730190277099609375);
    float3 higher = (float3(1.05499994754791259765625) * pow(linearRGB, float3(0.4166666567325592041015625))) - float3(0.054999999701976776123046875);
    float3 lower = linearRGB * float3(12.9200000762939453125);
    return select(higher, lower, cutoff);
}

static inline __attribute__((always_inline))
float3 tonemap_aces_film_simple(thread const float3& x)
{
    float a = 2.5099999904632568359375;
    float b = 0.02999999932944774627685546875;
    float c = 2.4300000667572021484375;
    float d = 0.589999973773956298828125;
    float e = 0.14000000059604644775390625;
    return fast::clamp((x * ((x * a) + float3(b))) / ((x * ((x * c) + float3(d))) + float3(e)), float3(0.0), float3(1.0));
}

static inline __attribute__((always_inline))
float3 tonemap_Hejl2015(thread const float3& hdr)
{
    float4 vh = float4(hdr, 1.0);
    float4 va = (vh * 1.434999942779541015625) + float4(0.0500000007450580596923828125);
    float4 vf = (((vh * va) + float4(0.0040000001899898052215576171875)) / ((vh * (va + float4(0.550000011920928955078125))) + float4(0.0491000004112720489501953125))) - float4(0.082099996507167816162109375);
    float3 param = vf.xyz / vf.www;
    return linear_to_srgb(param);
}

static inline __attribute__((always_inline))
float3 hable_function(thread const float3& x)
{
    return (((x * ((x * 4.0) + float3(0.60000002384185791015625))) + float3(0.12999999523162841796875)) / ((x * ((x * 4.0) + float3(5.0))) + float3(3.900000095367431640625))) - float3(0.0333333350718021392822265625);
}

static inline __attribute__((always_inline))
float3 tonemap_hable(thread const float3& color)
{
    float3 param = color;
    float3 numerator = hable_function(param);
    float3 param_1 = float3(6.0);
    float3 denominator = hable_function(param_1);
    float3 param_2 = numerator / denominator;
    return linear_to_srgb(param_2);
}

static inline __attribute__((always_inline))
float3 tonemap_filmic_alu(thread const float3& color_in)
{
    float3 color = fast::max(color_in - float3(0.0040000001899898052215576171875), float3(0.0));
    color = (color * ((color * 6.19999980926513671875) + float3(0.5))) / ((color * ((color * 6.19999980926513671875) + float3(1.7000000476837158203125))) + float3(0.0599999986588954925537109375));
    return color;
}

static inline __attribute__((always_inline))
float3 visualize_value(thread const float& val)
{
    float g = 1.0 - ((0.20000000298023223876953125 * (val - 3.2360498905181884765625)) * (val - 3.2360498905181884765625));
    float b = val;
    float r = 1.0 - (1.0 / ((0.5 * val) - 0.5));
    if (val > 1.0)
    {
        b = 0.0;
    }
    if (val < 3.0)
    {
        r = 0.0;
    }
    return fast::clamp(float3(r, g, b), float3(0.0), float3(1.0));
}

static inline __attribute__((always_inline))
float luma(thread const float3& color)
{
    return dot(color, float3(0.2989999949932098388671875, 0.58700001239776611328125, 0.114000000059604644775390625));
}

static inline __attribute__((always_inline))
float3 tonemap(thread const float3& color, thread const int& tonemapper_type)
{
    switch (tonemapper_type)
    {
        case 1:
        {
            float3 param = color;
            float3 _357 = tonemap_aces_fitted(param);
            float3 param_1 = _357;
            return linear_to_srgb(param_1);
        }
        case 2:
        {
            float3 param_2 = color * 0.60000002384185791015625;
            float3 param_3 = tonemap_aces_film_simple(param_2);
            return linear_to_srgb(param_3);
        }
        case 3:
        {
            float3 param_4 = color;
            return tonemap_Hejl2015(param_4);
        }
        case 4:
        {
            float3 param_5 = color;
            return tonemap_hable(param_5);
        }
        case 5:
        {
            float3 param_6 = color;
            return tonemap_filmic_alu(param_6);
        }
        case 6:
        {
            return color / (color + float3(1.0));
        }
        case 7:
        {
            float max_val = fast::max(color.x, fast::max(color.y, color.z));
            float param_7 = max_val;
            return visualize_value(param_7);
        }
        case 8:
        {
            float3 param_8 = color;
            float l = luma(param_8);
            float param_9 = l;
            return visualize_value(param_9);
        }
        default:
        {
            return color;
        }
    }
}

fragment main0_out main0(main0_in in [[stage_in]], constant spvDescriptorSetBuffer0& spvDescriptorSet0 [[buffer(0)]])
{
    constexpr sampler smp(mip_filter::linear, compare_func::never, max_anisotropy(1));
    main0_out out = {};
    float4 color = spvDescriptorSet0.in_color.sample(smp, in.inUV) + spvDescriptorSet0.in_blur.sample(smp, in.inUV);
    float3 param = color.xyz;
    int param_1 = (*spvDescriptorSet0.config).tonemapper_type;
    out.out_sdr = float4(tonemap(param, param_1), color.w);
    return out;
}

