// Based on CAS sample
//
// Copyright(c) 2019 Advanced Micro Devices, Inc.All rights reserved.
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files(the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions :
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

cbuffer Config : register(b0, space0)
{
    uint config_image_width : packoffset(c0);
    uint config_image_height : packoffset(c0.y);
    float config_sharpen_amount : packoffset(c0.z);
};

RWTexture2D<unorm float4> img_src : register(u1, space0);
RWTexture2D<unorm float4> img_dst : register(u2, space0);








#define A_GPU 1
#define A_HLSL 1

#if CAS_SAMPLE_FP16

#define A_HALF 1
#define CAS_PACKED_ONLY 1

#endif

#include "ffx_a.h"

#if CAS_SAMPLE_FP16

AH3 CasLoadH(ASW2 p)
{
    return img_src.Load(ASU3(p, 0)).rgb;
}

// Lets you transform input from the load into a linear color space between 0 and 1. See ffx_cas.h
// In this case, our input is already linear and between 0 and 1
void CasInputH(inout AH2 r, inout AH2 g, inout AH2 b) {}

#else

AF3 CasLoad(ASU2 p)
{
    return img_src.Load(int3(p, 0)).rgb;
}

// Lets you transform input from the load into a linear color space between 0 and 1. See ffx_cas.h
// In this case, our input is already linear and between 0 and 1
void CasInput(inout AF1 r, inout AF1 g, inout AF1 b) {}

#endif

#include "ffx_cas.h"

[numthreads(64, 1, 1)]
void main(uint3 LocalThreadId : SV_GroupThreadID, uint3 WorkGroupId : SV_GroupID)
{
    AU4 const0;
    AU4 const1;
    CasSetup(const0, const1, config_sharpen_amount, config_image_width, config_image_width, config_image_width, config_image_width);

    // Do remapping of local xy in workgroup for a more PS-like swizzle pattern.
    AU2 gxy = ARmp8x8(LocalThreadId.x) + AU2(WorkGroupId.x << 4u, WorkGroupId.y << 4u);
    bool sharpenOnly = true;

#if CAS_SAMPLE_FP16

    // Filter.
    AH4 c0, c1;
    AH2 cR, cG, cB;

    CasFilterH(cR, cG, cB, gxy, const0, const1, sharpenOnly);
    CasDepack(c0, c1, cR, cG, cB);
    img_dst[ASU2(gxy)] = AF4(c0);
    img_dst[ASU2(gxy) + ASU2(8, 0)] = AF4(c1);
    gxy.y += 8u;

    CasFilterH(cR, cG, cB, gxy, const0, const1, sharpenOnly);
    CasDepack(c0, c1, cR, cG, cB);
    img_dst[ASU2(gxy)] = AF4(c0);
    img_dst[ASU2(gxy) + ASU2(8, 0)] = AF4(c1);

#else

    // Filter.
    AF3 c;
    CasFilter(c.r, c.g, c.b, gxy, const0, const1, sharpenOnly);
    img_dst[ASU2(gxy)] = AF4(c, 1);
    gxy.x += 8u;

    CasFilter(c.r, c.g, c.b, gxy, const0, const1, sharpenOnly);
    img_dst[ASU2(gxy)] = AF4(c, 1);
    gxy.y += 8u;

    CasFilter(c.r, c.g, c.b, gxy, const0, const1, sharpenOnly);
    img_dst[ASU2(gxy)] = AF4(c, 1);
    gxy.x -= 8u;

    CasFilter(c.r, c.g, c.b, gxy, const0, const1, sharpenOnly);
    img_dst[ASU2(gxy)] = AF4(c, 1);

#endif
}