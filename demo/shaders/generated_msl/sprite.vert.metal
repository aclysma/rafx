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
    float2 o_uv [[user(locn0)]];
    float4 o_color [[user(locn1)]];
    float4 gl_Position [[position]];
};

struct main0_in
{
    float3 pos [[attribute(0)]];
    float2 uv [[attribute(1)]];
    float4 color [[attribute(2)]];
};

vertex main0_out main0(main0_in in [[stage_in]], constant spvDescriptorSetBuffer0& spvDescriptorSet0 [[buffer(0)]], constant spvDescriptorSetBuffer1& spvDescriptorSet1 [[buffer(1)]])
{
    constexpr sampler smp(filter::linear, mip_filter::linear, address::mirrored_repeat, compare_func::never, max_anisotropy(1));
    main0_out out = {};
    out.o_uv = in.uv;
    out.o_color = in.color;
    out.gl_Position = (*spvDescriptorSet0.uniform_buffer).mvp * float4(in.pos, 1.0);
    return out;
}

