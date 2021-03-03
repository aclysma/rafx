#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct PerViewUbo
{
    float4x4 view_proj;
};

struct spvDescriptorSetBuffer0
{
    texture2d<float> tex [[id(0)]];
};

struct spvDescriptorSetBuffer1
{
    constant PerViewUbo* per_view_data [[id(0)]];
};

struct main0_out
{
    float2 uv [[user(locn0)]];
    float4 color [[user(locn1)]];
    float4 gl_Position [[position]];
};

struct main0_in
{
    float3 pos [[attribute(0)]];
    float2 in_uv [[attribute(1)]];
    float4 in_color [[attribute(2)]];
};

vertex main0_out main0(main0_in in [[stage_in]], constant spvDescriptorSetBuffer0& spvDescriptorSet0 [[buffer(0)]], constant spvDescriptorSetBuffer1& spvDescriptorSet1 [[buffer(1)]])
{
    constexpr sampler smp(filter::linear, mip_filter::linear, address::repeat, compare_func::never, max_anisotropy(16));
    main0_out out = {};
    out.uv = in.in_uv;
    out.color = in.in_color;
    out.gl_Position = (*spvDescriptorSet1.per_view_data).view_proj * float4(in.pos.x, in.pos.y, in.pos.z, 1.0);
    return out;
}

