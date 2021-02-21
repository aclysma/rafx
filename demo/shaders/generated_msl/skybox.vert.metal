#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct Args
{
    float4x4 inverse_projection;
    float4x4 inverse_view;
};

struct spvDescriptorSetBuffer1
{
    constant Args* uniform_buffer [[id(0)]];
};

struct main0_out
{
    float3 out_texcoord [[user(locn0)]];
    float4 gl_Position [[position]];
};

vertex main0_out main0(constant spvDescriptorSetBuffer1& spvDescriptorSet1 [[buffer(1)]], uint gl_VertexIndex [[vertex_id]])
{
    main0_out out = {};
    out.gl_Position = float4((float((int(gl_VertexIndex) << 1) & 2) * 2.0) - 1.0, (float(int(gl_VertexIndex) & 2) * 2.0) - 1.0, 0.0, 1.0);
    out.out_texcoord = float3x3((*spvDescriptorSet1.uniform_buffer).inverse_view[0].xyz, (*spvDescriptorSet1.uniform_buffer).inverse_view[1].xyz, (*spvDescriptorSet1.uniform_buffer).inverse_view[2].xyz) * ((*spvDescriptorSet1.uniform_buffer).inverse_projection * out.gl_Position).xyz;
    return out;
}

