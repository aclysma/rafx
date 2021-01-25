#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct PerFrameUbo
{
    float4x4 view_proj;
};

struct spvDescriptorSetBuffer0
{
    constant PerFrameUbo* per_frame_data [[id(0)]];
};

struct main0_out
{
    float4 out_color [[user(locn0)]];
    float4 gl_Position [[position]];
};

struct main0_in
{
    float3 in_pos [[attribute(0)]];
    float4 in_color [[attribute(1)]];
};

vertex main0_out main0(main0_in in [[stage_in]], constant spvDescriptorSetBuffer0& spvDescriptorSet0 [[buffer(0)]])
{
    main0_out out = {};
    out.out_color = in.in_color;
    out.gl_Position = (*spvDescriptorSet0.per_frame_data).view_proj * float4(in.in_pos.x, in.in_pos.y, in.in_pos.z, 1.0);
    return out;
}

