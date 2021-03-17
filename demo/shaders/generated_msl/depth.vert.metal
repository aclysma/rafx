#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct PerViewData
{
    float4x4 view;
    float4x4 view_proj;
};

struct PerObjectData
{
    float4x4 model;
};

struct spvDescriptorSetBuffer0
{
    constant PerViewData* per_view_data [[id(0)]];
};

struct spvDescriptorSetBuffer2
{
    constant PerObjectData* per_object_data [[id(0)]];
};

struct main0_out
{
    float4 gl_Position [[position]];
};

struct main0_in
{
    float3 in_pos [[attribute(0)]];
};

vertex main0_out main0(main0_in in [[stage_in]], constant spvDescriptorSetBuffer0& spvDescriptorSet0 [[buffer(0)]], constant spvDescriptorSetBuffer2& spvDescriptorSet2 [[buffer(2)]])
{
    main0_out out = {};
    float4x4 model_view_proj = (*spvDescriptorSet0.per_view_data).view_proj * (*spvDescriptorSet2.per_object_data).model;
    out.gl_Position = model_view_proj * float4(in.in_pos, 1.0);
    return out;
}

