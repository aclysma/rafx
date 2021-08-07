#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct Positions
{
    float2 pos[100];
};

struct Velocity
{
    float2 vel[100];
};

struct spvDescriptorSetBuffer0
{
    device Positions* positions [[id(0)]];
    const device Velocity* velocities [[id(1)]];
};

kernel void main0(constant spvDescriptorSetBuffer0& spvDescriptorSet0 [[buffer(0)]], uint3 gl_GlobalInvocationID [[thread_position_in_grid]])
{
    float2 current_pos = (*spvDescriptorSet0.positions).pos[gl_GlobalInvocationID.x];
    float2 velocity = (*spvDescriptorSet0.velocities).vel[gl_GlobalInvocationID.x];
    current_pos += velocity;
    bool _45 = current_pos.x > 0.949999988079071044921875;
    bool _53;
    if (!_45)
    {
        _53 = current_pos.x < (-0.949999988079071044921875);
    }
    else
    {
        _53 = _45;
    }
    bool _61;
    if (!_53)
    {
        _61 = current_pos.y > 0.949999988079071044921875;
    }
    else
    {
        _61 = _53;
    }
    bool _68;
    if (!_61)
    {
        _68 = current_pos.y < (-0.949999988079071044921875);
    }
    else
    {
        _68 = _61;
    }
    if (_68)
    {
        current_pos = (velocity * (-2.0)) + (current_pos * 0.0500000007450580596923828125);
    }
    (*spvDescriptorSet0.positions).pos[gl_GlobalInvocationID.x] = current_pos;
}

