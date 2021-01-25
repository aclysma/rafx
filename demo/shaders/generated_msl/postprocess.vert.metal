#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct main0_out
{
    float2 outUV [[user(locn0)]];
    float4 gl_Position [[position]];
};

vertex main0_out main0(uint gl_VertexIndex [[vertex_id]])
{
    main0_out out = {};
    out.outUV = float2(float((int(gl_VertexIndex) << 1) & 2), float(int(gl_VertexIndex) & 2));
    out.gl_Position = float4((out.outUV * 2.0) - float2(1.0), 0.0, 1.0);
    return out;
}

