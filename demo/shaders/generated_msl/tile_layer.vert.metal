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
    texture2d<float> tex [[id(1)]];
};

struct main0_out
{
    float2 o_uv [[user(locn0)]];
    float4 gl_Position [[position]];
};

struct main0_in
{
    float4 pos [[attribute(0)]];
    float2 uv [[attribute(1)]];
};

vertex main0_out main0(main0_in in [[stage_in]], constant spvDescriptorSetBuffer0& spvDescriptorSet0 [[buffer(0)]], constant spvDescriptorSetBuffer1& spvDescriptorSet1 [[buffer(1)]])
{
    constexpr sampler smp(mip_filter::nearest, address::mirrored_repeat, compare_func::never, max_anisotropy(1), lod_clamp(0.0, 0.0));
    main0_out out = {};
    out.o_uv = in.uv;
    out.gl_Position = (*spvDescriptorSet0.uniform_buffer).mvp * in.pos;
    return out;
}

