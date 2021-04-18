#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct PointLight
{
    float3 position_ws;
    float3 position_vs;
    float4 color;
    float range;
    float intensity;
    int shadow_map;
    char _m0_final_padding[4];
};

struct DirectionalLight
{
    float3 direction_ws;
    float3 direction_vs;
    float4 color;
    float intensity;
    int shadow_map;
    char _m0_final_padding[8];
};

struct SpotLight
{
    float3 position_ws;
    float3 direction_ws;
    float3 position_vs;
    float3 direction_vs;
    float4 color;
    float spotlight_half_angle;
    float range;
    float intensity;
    int shadow_map;
};

struct ShadowMap2DData
{
    float4x4 shadow_map_view_proj;
    float3 shadow_map_light_dir;
};

struct ShadowMapCubeData
{
    float cube_map_projection_near_z;
    float cube_map_projection_far_z;
    char _m0_final_padding[8];
};

struct PerViewData
{
    float4x4 view[1];
    float4x4 view_proj;
    float4 ambient_light;
    int point_light_count;
    int directional_light_count;
    int spot_light_count;
    PointLight point_lights[16];
    DirectionalLight directional_lights[16];
    SpotLight spot_lights[16];
    ShadowMap2DData shadow_map_2d_data[32];
    ShadowMapCubeData shadow_map_cube_data[16];
};

struct spvDescriptorSetBuffer0
{
    constant PerViewData* per_view_data [[id(0)]];
};

struct main0_out
{
    float4 out_color [[user(locn0)]];
    float4 gl_Position [[position]];
};

struct main0_in
{
    float4 pos [[attribute(0)]];
    float4 in_color [[attribute(1)]];
};

vertex main0_out main0(main0_in in [[stage_in]], constant spvDescriptorSetBuffer0& spvDescriptorSet0 [[buffer(0)]])
{
    main0_out out = {};
    out.out_color = in.in_color;
    out.gl_Position = in.pos;
    return out;
}

