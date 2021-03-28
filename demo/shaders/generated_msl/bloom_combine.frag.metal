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
    color = float3x3(float3(0.59719002246856689453125, 0.354579985141754150390625, 0.048229999840259552001953125), float3(0.075999997556209564208984375, 0.908339977264404296875, 0.0156599991023540496826171875), float3(0.0284000001847743988037109375, 0.13382999598979949951171875, 0.837769985198974609375)) * color;
    float3 param = color;
    color = RRT_and_ODT_fit(param);
    color = float3x3(float3(1.60475003719329833984375, -0.5310800075531005859375, -0.0736699998378753662109375), float3(-0.10208000242710113525390625, 1.108129978179931640625, -0.00604999996721744537353515625), float3(-0.00326999998651444911956787109375, -0.07276000082492828369140625, 1.0760200023651123046875)) * color;
    color = fast::clamp(color, float3(0.0), float3(1.0));
    return color;
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
float3 visualize_value(thread const float& val)
{
    float g = 1.0 - ((0.20000000298023223876953125 * (val - 3.2360498905181884765625)) * (val - 3.2360498905181884765625));
    float b = 1.0 - ((1.0 * (val - 1.0)) * (val - 1.0));
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

fragment main0_out main0(main0_in in [[stage_in]], constant spvDescriptorSetBuffer0& spvDescriptorSet0 [[buffer(0)]])
{
    constexpr sampler smp(mip_filter::linear, compare_func::never, max_anisotropy(1));
    main0_out out = {};
    float4 color = spvDescriptorSet0.in_color.sample(smp, in.inUV) + spvDescriptorSet0.in_blur.sample(smp, in.inUV);
    if ((*spvDescriptorSet0.config).tonemapper_type == 1)
    {
        float3 param = color.xyz;
        float3 _227 = tonemap_aces_fitted(param);
        out.out_sdr = float4(_227, color.w);
    }
    else
    {
        if ((*spvDescriptorSet0.config).tonemapper_type == 2)
        {
            float3 param_1 = color.xyz;
            out.out_sdr = float4(tonemap_aces_film_simple(param_1), color.w);
        }
        else
        {
            if ((*spvDescriptorSet0.config).tonemapper_type == 3)
            {
                out.out_sdr = float4(color.xyz / (color.xyz + float3(1.0)), color.w);
            }
            else
            {
                if ((*spvDescriptorSet0.config).tonemapper_type == 4)
                {
                    float max_val = fast::max(color.x, fast::max(color.y, color.z));
                    float param_2 = max_val;
                    out.out_sdr = float4(visualize_value(param_2), color.w);
                }
                else
                {
                    if ((*spvDescriptorSet0.config).tonemapper_type == 5)
                    {
                        float3 param_3 = color.xyz;
                        float l = luma(param_3);
                        float param_4 = l;
                        out.out_sdr = float4(visualize_value(param_4), color.w);
                    }
                    else
                    {
                        out.out_sdr = color;
                    }
                }
            }
        }
    }
    return out;
}

