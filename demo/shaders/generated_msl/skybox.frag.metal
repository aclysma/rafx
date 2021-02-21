#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct Args
{
    float4x4 inverse_projection;
    float4x4 inverse_view;
};

struct spvDescriptorSetBuffer0
{
    texturecube<float> skybox_tex [[id(1)]];
};

struct main0_out
{
    float4 out_color [[color(0)]];
};

struct main0_in
{
    float3 in_texcoord [[user(locn0)]];
};

fragment main0_out main0(main0_in in [[stage_in]], constant spvDescriptorSetBuffer0& spvDescriptorSet0 [[buffer(0)]])
{
    constexpr sampler smp(filter::linear, mip_filter::linear, compare_func::never, max_anisotropy(1));
    main0_out out = {};
    out.out_color = spvDescriptorSet0.skybox_tex.sample(smp, in.in_texcoord);
    return out;
}

