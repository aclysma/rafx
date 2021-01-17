#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct PerViewData
{
    float4 uniform_color;
};

struct main0_out
{
    float4 out_color [[color(0)]];
};

fragment main0_out main0(constant PerViewData& uniform_data [[buffer(0)]])
{
    main0_out out = {};
    out.out_color = uniform_data.uniform_color;
    return out;
}

