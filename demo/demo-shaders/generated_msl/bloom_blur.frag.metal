#pragma clang diagnostic ignored "-Wmissing-prototypes"
#pragma clang diagnostic ignored "-Wmissing-braces"

#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

template<typename T, size_t Num>
struct spvUnsafeArray
{
    T elements[Num ? Num : 1];
    
    thread T& operator [] (size_t pos) thread
    {
        return elements[pos];
    }
    constexpr const thread T& operator [] (size_t pos) const thread
    {
        return elements[pos];
    }
    
    device T& operator [] (size_t pos) device
    {
        return elements[pos];
    }
    constexpr const device T& operator [] (size_t pos) const device
    {
        return elements[pos];
    }
    
    constexpr const constant T& operator [] (size_t pos) const constant
    {
        return elements[pos];
    }
    
    threadgroup T& operator [] (size_t pos) threadgroup
    {
        return elements[pos];
    }
    constexpr const threadgroup T& operator [] (size_t pos) const threadgroup
    {
        return elements[pos];
    }
};

struct Config
{
    uint horizontal;
};

struct spvDescriptorSetBuffer0
{
    texture2d<float> tex [[id(0)]];
    constant Config* config [[id(2)]];
};

constant spvUnsafeArray<float, 5> _17 = spvUnsafeArray<float, 5>({ 0.227026998996734619140625, 0.19459460675716400146484375, 0.121621601283550262451171875, 0.054053999483585357666015625, 0.01621600054204463958740234375 });

struct main0_out
{
    float4 out_blur [[color(0)]];
};

struct main0_in
{
    float2 inUV [[user(locn0)]];
};

fragment main0_out main0(main0_in in [[stage_in]], constant spvDescriptorSetBuffer0& spvDescriptorSet0 [[buffer(0)]])
{
    constexpr sampler smp(mip_filter::linear, compare_func::never, max_anisotropy(1));
    main0_out out = {};
    float2 tex_offset = float2(1.0) / float2(int2(spvDescriptorSet0.tex.get_width(), spvDescriptorSet0.tex.get_height()));
    float3 result = spvDescriptorSet0.tex.sample(smp, in.inUV).xyz * _17[0];
    if ((*spvDescriptorSet0.config).horizontal != 0u)
    {
        for (int i = 1; i < 5; i++)
        {
            result += (spvDescriptorSet0.tex.sample(smp, (in.inUV + float2(tex_offset.x * float(i), 0.0))).xyz * _17[i]);
            result += (spvDescriptorSet0.tex.sample(smp, (in.inUV - float2(tex_offset.x * float(i), 0.0))).xyz * _17[i]);
        }
    }
    else
    {
        for (int i_1 = 1; i_1 < 5; i_1++)
        {
            result += (spvDescriptorSet0.tex.sample(smp, (in.inUV + float2(0.0, tex_offset.y * float(i_1)))).xyz * _17[i_1]);
            result += (spvDescriptorSet0.tex.sample(smp, (in.inUV - float2(0.0, tex_offset.y * float(i_1)))).xyz * _17[i_1]);
        }
    }
    out.out_blur = float4(result, 1.0);
    return out;
}

