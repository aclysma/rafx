#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct Args
{
    float4x4 mvp;
};

struct spvDescriptorSetBuffer0
{
    constant Args* uniform_buffer [[id(0)]];
};

struct spvDescriptorSetBuffer1
{
    texture2d<float> tex [[id(0)]];
};

struct main0_out
{
    float4 out_color [[color(0)]];
};

struct main0_in
{
    float2 uv [[user(locn0)]];
    float4 color [[user(locn1)]];
};

fragment main0_out main0(main0_in in [[stage_in]], constant spvDescriptorSetBuffer0& spvDescriptorSet0 [[buffer(0)]], constant spvDescriptorSetBuffer1& spvDescriptorSet1 [[buffer(1)]])
{
    constexpr sampler smp(filter::linear, mip_filter::linear, compare_func::never, max_anisotropy(1));
    main0_out out = {};
    out.out_color = spvDescriptorSet1.tex.sample(smp, in.uv).xxxx * in.color;
    return out;
}

