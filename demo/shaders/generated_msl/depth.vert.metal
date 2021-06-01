#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct PerViewData
{
    float4x4 view;
    float4x4 view_proj;
};

struct spvDescriptorSetBuffer0
{
    constant PerViewData* per_view_data [[id(0)]];
};

struct main0_out
{
    float4 gl_Position [[position]];
};

struct main0_in
{
    float3 in_pos [[attribute(0)]];
    float4 in_model_matrix_0 [[attribute(1)]];
    float4 in_model_matrix_1 [[attribute(2)]];
    float4 in_model_matrix_2 [[attribute(3)]];
    float4 in_model_matrix_3 [[attribute(4)]];
};

vertex main0_out main0(main0_in in [[stage_in]], constant spvDescriptorSetBuffer0& spvDescriptorSet0 [[buffer(0)]])
{
    main0_out out = {};
    float4x4 in_model_matrix = {};
    in_model_matrix[0] = in.in_model_matrix_0;
    in_model_matrix[1] = in.in_model_matrix_1;
    in_model_matrix[2] = in.in_model_matrix_2;
    in_model_matrix[3] = in.in_model_matrix_3;
    float4x4 model_view_proj = (*spvDescriptorSet0.per_view_data).view_proj * in_model_matrix;
    out.gl_Position = model_view_proj * float4(in.in_pos, 1.0);
    return out;
}

