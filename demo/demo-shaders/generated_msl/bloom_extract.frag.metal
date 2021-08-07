#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct spvDescriptorSetBuffer0
{
    texture2d<float> tex [[id(0)]];
};

struct main0_out
{
    float4 out_sdr [[color(0)]];
    float4 out_bloom [[color(1)]];
};

struct main0_in
{
    float2 inUV [[user(locn0)]];
};

fragment main0_out main0(main0_in in [[stage_in]], constant spvDescriptorSetBuffer0& spvDescriptorSet0 [[buffer(0)]])
{
    constexpr sampler smp(mip_filter::linear, compare_func::never, max_anisotropy(1));
    main0_out out = {};
    float3 color = spvDescriptorSet0.tex.sample(smp, in.inUV).xyz;
    float brightness = dot(color, float3(0.2125999927520751953125, 0.715200006961822509765625, 0.072200000286102294921875));
    if (brightness > 1.0)
    {
        out.out_bloom = float4(color, 1.0);
    }
    else
    {
        out.out_bloom = float4(0.0, 0.0, 0.0, 1.0);
    }
    out.out_sdr = float4(color, 1.0);
    return out;
}

