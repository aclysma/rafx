#pragma clang diagnostic ignored "-Wmissing-prototypes"

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

struct DirectionalLight
{
    float3 direction_ws;
    float3 direction_vs;
    float4 color;
    float intensity;
    int shadow_map;
};

struct PointLight_1
{
    float3 position_ws;
    float3 position_vs;
    float4 color;
    float range;
    float intensity;
    int shadow_map;
    char _m0_final_padding[4];
};

struct DirectionalLight_1
{
    float3 direction_ws;
    float3 direction_vs;
    float4 color;
    float intensity;
    int shadow_map;
    char _m0_final_padding[8];
};

struct SpotLight_1
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
    float4 ambient_light;
    uint point_light_count;
    uint directional_light_count;
    uint spot_light_count;
    PointLight_1 point_lights[16];
    DirectionalLight_1 directional_lights[16];
    SpotLight_1 spot_lights[16];
    ShadowMap2DData shadow_map_2d_data[32];
    ShadowMapCubeData shadow_map_cube_data[16];
};

struct PerObjectData
{
    float4x4 model;
    float4x4 model_view;
    float4x4 model_view_proj;
};

struct MaterialData
{
    float4 base_color_factor;
    packed_float3 emissive_factor;
    float metallic_factor;
    float roughness_factor;
    float normal_texture_scale;
    float occlusion_texture_strength;
    float alpha_cutoff;
    uint has_base_color_texture;
    uint has_metallic_roughness_texture;
    uint has_normal_texture;
    uint has_occlusion_texture;
    uint has_emissive_texture;
};

struct MaterialDataUbo
{
    MaterialData data;
};

struct spvDescriptorSetBuffer0
{
    constant PerViewData* per_view_data [[id(0)]];
    array<depth2d<float>, 32> shadow_map_images [[id(3)]];
    array<depthcube<float>, 16> shadow_map_images_cube [[id(35)]];
};

struct spvDescriptorSetBuffer1
{
    constant MaterialDataUbo* per_material_data [[id(0)]];
    texture2d<float> base_color_texture [[id(1)]];
    texture2d<float> metallic_roughness_texture [[id(2)]];
    texture2d<float> normal_texture [[id(3)]];
    texture2d<float> emissive_texture [[id(5)]];
};

struct spvDescriptorSetBuffer2
{
    constant PerObjectData* per_object_data [[id(0)]];
};

struct main0_out
{
    float4 out_color [[color(0)]];
};

struct main0_in
{
    float3 in_position_vs [[user(locn0)]];
    float3 in_normal_vs [[user(locn1)]];
    float3 in_tangent_vs [[user(locn2)]];
    float3 in_binormal_vs [[user(locn3)]];
    float2 in_uv [[user(locn4)]];
    float4 in_position_ws [[user(locn5)]];
};

static inline __attribute__((always_inline))
float4 normal_map(thread const float3x3& tangent_binormal_normal, thread const float2& uv, thread texture2d<float> normal_texture, thread sampler smp)
{
    float3 normal = normal_texture.sample(smp, uv).xyz;
    normal = (normal * 2.0) - float3(1.0);
    normal = tangent_binormal_normal * normal;
    return normalize(float4(normal, 0.0));
}

static inline __attribute__((always_inline))
float calculate_cubemap_equivalent_depth(thread const float3& light_to_surface_ws, thread const float& near, thread const float& far)
{
    float3 light_to_surface_ws_abs = abs(light_to_surface_ws);
    float face_local_z_depth = fast::max(light_to_surface_ws_abs.x, fast::max(light_to_surface_ws_abs.y, light_to_surface_ws_abs.z));
    float depth_value = ((far + near) / (far - near)) - ((((2.0 * far) * near) / (far - near)) / face_local_z_depth);
    return (depth_value + 1.0) * 0.5;
}

static inline __attribute__((always_inline))
float do_calculate_percent_lit_cube(thread const float3& light_position_ws, thread const float3& light_position_vs, thread const float3& normal_vs, thread const int& index, thread const float& bias_multiplier, constant PerViewData& per_view_data, thread float4& in_position_ws, thread float3& in_position_vs, thread float3& in_normal_vs, thread const array<depthcube<float>, 16> shadow_map_images_cube, thread sampler smp_depth)
{
    float near_plane = per_view_data.shadow_map_cube_data[index].cube_map_projection_near_z;
    float far_plane = per_view_data.shadow_map_cube_data[index].cube_map_projection_far_z;
    float3 light_to_surface_ws = in_position_ws.xyz - light_position_ws;
    float3 surface_to_light_dir_vs = normalize(light_position_vs - in_position_vs);
    float bias_angle_factor = 1.0 - fast::max(0.0, dot(in_normal_vs, surface_to_light_dir_vs));
    bias_angle_factor = pow(bias_angle_factor, 3.0);
    float bias0 = 0.00019999999494757503271102905273438 + (0.001000000047497451305389404296875 * bias_angle_factor);
    float3 param = light_to_surface_ws;
    float param_1 = near_plane;
    float param_2 = far_plane;
    float depth_of_surface = calculate_cubemap_equivalent_depth(param, param_1, param_2);
    float4 _303 = float4(light_to_surface_ws, depth_of_surface + bias0);
    float shadow = shadow_map_images_cube[index].sample_compare(smp_depth, _303.xyz, _303.w);
    return shadow;
}

static inline __attribute__((always_inline))
float calculate_percent_lit_cube(thread const float3& light_position_ws, thread const float3& light_position_vs, thread const float3& normal_vs, thread const int& index, thread const float& bias_multiplier, constant PerViewData& per_view_data, thread float4& in_position_ws, thread float3& in_position_vs, thread float3& in_normal_vs, thread const array<depthcube<float>, 16> shadow_map_images_cube, thread sampler smp_depth)
{
    if (index == (-1))
    {
        return 1.0;
    }
    float3 param = light_position_ws;
    float3 param_1 = light_position_vs;
    float3 param_2 = normal_vs;
    int param_3 = index;
    float param_4 = bias_multiplier;
    return do_calculate_percent_lit_cube(param, param_1, param_2, param_3, param_4, per_view_data, in_position_ws, in_position_vs, in_normal_vs, shadow_map_images_cube, smp_depth);
}

static inline __attribute__((always_inline))
float ndf_ggx(thread const float3& n, thread const float3& h, thread const float& roughness)
{
    float a = roughness * roughness;
    float a2 = a * a;
    float n_dot_h = fast::max(dot(n, h), 0.0);
    float bottom_part = ((n_dot_h * n_dot_h) * (a2 - 1.0)) + 1.0;
    float bottom = (3.1415927410125732421875 * bottom_part) * bottom_part;
    return a2 / bottom;
}

static inline __attribute__((always_inline))
float geometric_attenuation_schlick_ggx(thread const float& dot_product, thread const float& k)
{
    float bottom = (dot_product * (1.0 - k)) + k;
    return dot_product / bottom;
}

static inline __attribute__((always_inline))
float geometric_attenuation_smith(thread const float3& n, thread const float3& v, thread const float3& l, thread const float& roughness)
{
    float r_plus_1 = roughness + 1.0;
    float k = (r_plus_1 * r_plus_1) / 8.0;
    float param = fast::max(dot(n, v), 0.0);
    float param_1 = k;
    float v_factor = geometric_attenuation_schlick_ggx(param, param_1);
    float param_2 = fast::max(dot(n, l), 0.0);
    float param_3 = k;
    float l_factor = geometric_attenuation_schlick_ggx(param_2, param_3);
    return v_factor * l_factor;
}

static inline __attribute__((always_inline))
float3 fresnel_schlick(thread const float3& v, thread const float3& h, thread const float3& fresnel_base)
{
    float v_dot_h = fast::max(dot(v, h), 0.0);
    return fresnel_base + ((float3(1.0) - fresnel_base) * exp2((((-5.554729938507080078125) * v_dot_h) - 6.9831600189208984375) * v_dot_h));
}

static inline __attribute__((always_inline))
float3 shade_pbr(thread const float3& surface_to_light_dir_vs, thread const float3& surface_to_eye_dir_vs, thread const float3& normal_vs, thread const float3& F0, thread const float3& base_color, thread const float& roughness, thread const float& metalness, thread const float3& radiance)
{
    float3 halfway_dir_vs = normalize(surface_to_light_dir_vs + surface_to_eye_dir_vs);
    float3 param = normal_vs;
    float3 param_1 = halfway_dir_vs;
    float param_2 = roughness;
    float NDF = ndf_ggx(param, param_1, param_2);
    float3 param_3 = normal_vs;
    float3 param_4 = surface_to_eye_dir_vs;
    float3 param_5 = surface_to_light_dir_vs;
    float param_6 = roughness;
    float G = geometric_attenuation_smith(param_3, param_4, param_5, param_6);
    float3 param_7 = surface_to_eye_dir_vs;
    float3 param_8 = halfway_dir_vs;
    float3 param_9 = F0;
    float3 F = fresnel_schlick(param_7, param_8, param_9);
    float3 fresnel_specular = F;
    float3 fresnel_diffuse = float3(1.0) - fresnel_specular;
    fresnel_diffuse *= (1.0 - metalness);
    float n_dot_l = fast::max(dot(normal_vs, surface_to_light_dir_vs), 0.0);
    float n_dot_v = fast::max(dot(normal_vs, surface_to_eye_dir_vs), 0.0);
    float3 top = F * (NDF * G);
    float bottom = (4.0 * n_dot_v) * n_dot_l;
    float3 specular = top / float3(fast::max(bottom, 0.001000000047497451305389404296875));
    return ((((fresnel_diffuse * base_color) / float3(3.1415927410125732421875)) + specular) * radiance) * n_dot_l;
}

static inline __attribute__((always_inline))
float3 point_light_pbr(thread const PointLight& light, thread const float3& surface_to_eye_dir_vs, thread const float3& surface_position_vs, thread const float3& normal_vs, thread const float3& F0, thread const float3& base_color, thread const float& roughness, thread const float& metalness)
{
    float3 surface_to_light_dir_vs = light.position_vs - surface_position_vs;
    float _distance = length(surface_to_light_dir_vs);
    surface_to_light_dir_vs /= float3(_distance);
    float attenuation = 1.0 / (_distance * _distance);
    float3 radiance = (light.color.xyz * attenuation) * light.intensity;
    float3 param = surface_to_light_dir_vs;
    float3 param_1 = surface_to_eye_dir_vs;
    float3 param_2 = normal_vs;
    float3 param_3 = F0;
    float3 param_4 = base_color;
    float param_5 = roughness;
    float param_6 = metalness;
    float3 param_7 = radiance;
    return shade_pbr(param, param_1, param_2, param_3, param_4, param_5, param_6, param_7);
}

static inline __attribute__((always_inline))
float do_calculate_percent_lit(thread const float3& normal_vs, thread const int& index, thread const float& bias_multiplier, constant PerViewData& per_view_data, thread float4& in_position_ws, thread sampler smp_depth, constant PerObjectData& per_object_data, thread const array<depth2d<float>, 32> shadow_map_images)
{
    float4 shadow_map_pos = per_view_data.shadow_map_2d_data[index].shadow_map_view_proj * in_position_ws;
    float3 projected = shadow_map_pos.xyz / float3(shadow_map_pos.w);
    float2 sample_location_uv = (projected.xy * 0.5) + float2(0.5);
    sample_location_uv.y = 1.0 - sample_location_uv.y;
    float depth_of_surface = projected.z;
    float3 light_dir_vs = float3x3(per_object_data.model_view[0].xyz, per_object_data.model_view[1].xyz, per_object_data.model_view[2].xyz) * per_view_data.shadow_map_2d_data[index].shadow_map_light_dir;
    float3 surface_to_light_dir_vs = -light_dir_vs;
    float bias_angle_factor = 1.0 - dot(normal_vs, surface_to_light_dir_vs);
    float bias0 = fast::max(((0.00999999977648258209228515625 * bias_angle_factor) * bias_angle_factor) * bias_angle_factor, 0.0005000000237487256526947021484375) * bias_multiplier;
    float shadow = 0.0;
    float2 texelSize = float2(1.0) / float2(int2(shadow_map_images[index].get_width(), shadow_map_images[index].get_height()));
    for (int x = -2; x <= 2; x++)
    {
        for (int y = -2; y <= 2; y++)
        {
            float3 _451 = float3(sample_location_uv + (float2(float(x), float(y)) * texelSize), depth_of_surface + bias0);
            shadow += shadow_map_images[index].sample_compare(smp_depth, _451.xy, _451.z);
        }
    }
    shadow /= 25.0;
    return shadow;
}

static inline __attribute__((always_inline))
float calculate_percent_lit(thread const float3& normal, thread const int& index, thread const float& bias_multiplier, constant PerViewData& per_view_data, thread float4& in_position_ws, thread sampler smp_depth, constant PerObjectData& per_object_data, thread const array<depth2d<float>, 32> shadow_map_images)
{
    if (index == (-1))
    {
        return 1.0;
    }
    float3 param = normal;
    int param_1 = index;
    float param_2 = bias_multiplier;
    return do_calculate_percent_lit(param, param_1, param_2, per_view_data, in_position_ws, smp_depth, per_object_data, shadow_map_images);
}

static inline __attribute__((always_inline))
float spotlight_cone_falloff(thread const float3& surface_to_light_dir, thread const float3& spotlight_dir, thread const float& spotlight_half_angle)
{
    float cos_angle = dot(-spotlight_dir, surface_to_light_dir);
    float min_cos = cos(spotlight_half_angle);
    float max_cos = mix(min_cos, 1.0, 0.5);
    return smoothstep(min_cos, max_cos, cos_angle);
}

static inline __attribute__((always_inline))
float3 spot_light_pbr(thread const SpotLight& light, thread const float3& surface_to_eye_dir_vs, thread const float3& surface_position_vs, thread const float3& normal_vs, thread const float3& F0, thread const float3& base_color, thread const float& roughness, thread const float& metalness)
{
    float3 surface_to_light_dir_vs = light.position_vs - surface_position_vs;
    float _distance = length(surface_to_light_dir_vs);
    surface_to_light_dir_vs /= float3(_distance);
    float attenuation = 1.0 / (_distance * _distance);
    float3 param = surface_to_light_dir_vs;
    float3 param_1 = light.direction_vs;
    float param_2 = light.spotlight_half_angle;
    float spotlight_direction_intensity = spotlight_cone_falloff(param, param_1, param_2);
    float3 radiance = ((light.color.xyz * attenuation) * light.intensity) * spotlight_direction_intensity;
    float3 param_3 = surface_to_light_dir_vs;
    float3 param_4 = surface_to_eye_dir_vs;
    float3 param_5 = normal_vs;
    float3 param_6 = F0;
    float3 param_7 = base_color;
    float param_8 = roughness;
    float param_9 = metalness;
    float3 param_10 = radiance;
    return shade_pbr(param_3, param_4, param_5, param_6, param_7, param_8, param_9, param_10);
}

static inline __attribute__((always_inline))
float3 directional_light_pbr(thread const DirectionalLight& light, thread const float3& surface_to_eye_dir_vs, thread const float3& surface_position_vs, thread const float3& normal_vs, thread const float3& F0, thread const float3& base_color, thread const float& roughness, thread const float& metalness)
{
    float3 surface_to_light_dir_vs = -light.direction_vs;
    float attenuation = 1.0;
    float3 radiance = (light.color.xyz * attenuation) * light.intensity;
    float3 param = surface_to_light_dir_vs;
    float3 param_1 = surface_to_eye_dir_vs;
    float3 param_2 = normal_vs;
    float3 param_3 = F0;
    float3 param_4 = base_color;
    float param_5 = roughness;
    float param_6 = metalness;
    float3 param_7 = radiance;
    return shade_pbr(param, param_1, param_2, param_3, param_4, param_5, param_6, param_7);
}

static inline __attribute__((always_inline))
float4 pbr_path(thread const float3& surface_to_eye_vs, thread const float4& base_color, thread const float4& emissive_color, thread const float& metalness, thread const float& roughness, thread const float3& normal_vs, constant PerViewData& per_view_data, thread float4& in_position_ws, thread float3& in_position_vs, thread float3& in_normal_vs, thread const array<depthcube<float>, 16> shadow_map_images_cube, thread sampler smp_depth, constant PerObjectData& per_object_data, thread const array<depth2d<float>, 32> shadow_map_images)
{
    float3 fresnel_base = float3(0.039999999105930328369140625);
    fresnel_base = mix(fresnel_base, base_color.xyz, float3(metalness));
    float3 total_light = float3(0.0);
    PointLight param_5;
    for (uint i = 0u; i < per_view_data.point_light_count; i++)
    {
        float3 param = per_view_data.point_lights[i].position_ws;
        float3 param_1 = per_view_data.point_lights[i].position_vs;
        float3 param_2 = normal_vs;
        int param_3 = per_view_data.point_lights[i].shadow_map;
        float param_4 = 1.0;
        float percent_lit = calculate_percent_lit_cube(param, param_1, param_2, param_3, param_4, per_view_data, in_position_ws, in_position_vs, in_normal_vs, shadow_map_images_cube, smp_depth);
        param_5.position_ws = per_view_data.point_lights[i].position_ws;
        param_5.position_vs = per_view_data.point_lights[i].position_vs;
        param_5.color = per_view_data.point_lights[i].color;
        param_5.range = per_view_data.point_lights[i].range;
        param_5.intensity = per_view_data.point_lights[i].intensity;
        param_5.shadow_map = per_view_data.point_lights[i].shadow_map;
        float3 param_6 = surface_to_eye_vs;
        float3 param_7 = in_position_vs;
        float3 param_8 = normal_vs;
        float3 param_9 = fresnel_base;
        float3 param_10 = base_color.xyz;
        float param_11 = roughness;
        float param_12 = metalness;
        total_light += (point_light_pbr(param_5, param_6, param_7, param_8, param_9, param_10, param_11, param_12) * percent_lit);
    }
    SpotLight param_16;
    for (uint i_1 = 0u; i_1 < per_view_data.spot_light_count; i_1++)
    {
        float3 param_13 = normal_vs;
        int param_14 = per_view_data.spot_lights[i_1].shadow_map;
        float param_15 = 0.4000000059604644775390625;
        float percent_lit_1 = calculate_percent_lit(param_13, param_14, param_15, per_view_data, in_position_ws, smp_depth, per_object_data, shadow_map_images);
        param_16.position_ws = per_view_data.spot_lights[i_1].position_ws;
        param_16.direction_ws = per_view_data.spot_lights[i_1].direction_ws;
        param_16.position_vs = per_view_data.spot_lights[i_1].position_vs;
        param_16.direction_vs = per_view_data.spot_lights[i_1].direction_vs;
        param_16.color = per_view_data.spot_lights[i_1].color;
        param_16.spotlight_half_angle = per_view_data.spot_lights[i_1].spotlight_half_angle;
        param_16.range = per_view_data.spot_lights[i_1].range;
        param_16.intensity = per_view_data.spot_lights[i_1].intensity;
        param_16.shadow_map = per_view_data.spot_lights[i_1].shadow_map;
        float3 param_17 = surface_to_eye_vs;
        float3 param_18 = in_position_vs;
        float3 param_19 = normal_vs;
        float3 param_20 = fresnel_base;
        float3 param_21 = base_color.xyz;
        float param_22 = roughness;
        float param_23 = metalness;
        total_light += (spot_light_pbr(param_16, param_17, param_18, param_19, param_20, param_21, param_22, param_23) * percent_lit_1);
    }
    DirectionalLight param_27;
    for (uint i_2 = 0u; i_2 < per_view_data.directional_light_count; i_2++)
    {
        float3 param_24 = normal_vs;
        int param_25 = per_view_data.directional_lights[i_2].shadow_map;
        float param_26 = 1.0;
        float percent_lit_2 = calculate_percent_lit(param_24, param_25, param_26, per_view_data, in_position_ws, smp_depth, per_object_data, shadow_map_images);
        param_27.direction_ws = per_view_data.directional_lights[i_2].direction_ws;
        param_27.direction_vs = per_view_data.directional_lights[i_2].direction_vs;
        param_27.color = per_view_data.directional_lights[i_2].color;
        param_27.intensity = per_view_data.directional_lights[i_2].intensity;
        param_27.shadow_map = per_view_data.directional_lights[i_2].shadow_map;
        float3 param_28 = surface_to_eye_vs;
        float3 param_29 = in_position_vs;
        float3 param_30 = normal_vs;
        float3 param_31 = fresnel_base;
        float3 param_32 = base_color.xyz;
        float param_33 = roughness;
        float param_34 = metalness;
        total_light += (directional_light_pbr(param_27, param_28, param_29, param_30, param_31, param_32, param_33, param_34) * percent_lit_2);
    }
    float3 ambient = per_view_data.ambient_light.xyz * base_color.xyz;
    float3 color = (ambient + total_light) + emissive_color.xyz;
    return float4(color, base_color.w);
}

fragment main0_out main0(main0_in in [[stage_in]], constant spvDescriptorSetBuffer0& spvDescriptorSet0 [[buffer(0)]], constant spvDescriptorSetBuffer1& spvDescriptorSet1 [[buffer(1)]], constant spvDescriptorSetBuffer2& spvDescriptorSet2 [[buffer(2)]])
{
    constexpr sampler smp(filter::linear, mip_filter::linear, address::repeat, compare_func::never, max_anisotropy(16));
    constexpr sampler smp_depth(filter::linear, mip_filter::linear, address::clamp_to_border, compare_func::greater, border_color::transparent_black, max_anisotropy(16));
    main0_out out = {};
    float4 base_color = (*spvDescriptorSet1.per_material_data).data.base_color_factor;
    if ((*spvDescriptorSet1.per_material_data).data.has_base_color_texture != 0u)
    {
        base_color *= spvDescriptorSet1.base_color_texture.sample(smp, in.in_uv);
    }
    float4 emissive_color = float4((*spvDescriptorSet1.per_material_data).data.emissive_factor[0], (*spvDescriptorSet1.per_material_data).data.emissive_factor[1], (*spvDescriptorSet1.per_material_data).data.emissive_factor[2], 1.0);
    if ((*spvDescriptorSet1.per_material_data).data.has_emissive_texture != 0u)
    {
        emissive_color *= spvDescriptorSet1.emissive_texture.sample(smp, in.in_uv);
        base_color = float4(1.0, 1.0, 0.0, 1.0);
    }
    float metalness = (*spvDescriptorSet1.per_material_data).data.metallic_factor;
    float roughness = (*spvDescriptorSet1.per_material_data).data.roughness_factor;
    if ((*spvDescriptorSet1.per_material_data).data.has_metallic_roughness_texture != 0u)
    {
        float4 sampled = spvDescriptorSet1.metallic_roughness_texture.sample(smp, in.in_uv);
        metalness *= sampled.x;
        roughness *= sampled.y;
    }
    roughness = (roughness + 0.0) / 1.0;
    float3 normal_vs;
    if ((*spvDescriptorSet1.per_material_data).data.has_normal_texture != 0u)
    {
        float3x3 tbn = float3x3(float3(in.in_tangent_vs), float3(in.in_binormal_vs), float3(in.in_normal_vs));
        float3x3 param = tbn;
        float2 param_1 = in.in_uv;
        normal_vs = normal_map(param, param_1, spvDescriptorSet1.normal_texture, smp).xyz;
    }
    else
    {
        normal_vs = normalize(float4(in.in_normal_vs, 0.0)).xyz;
    }
    float3 eye_position_vs = float3(0.0);
    float3 surface_to_eye_vs = normalize(eye_position_vs - in.in_position_vs);
    float3 param_2 = surface_to_eye_vs;
    float4 param_3 = base_color;
    float4 param_4 = emissive_color;
    float param_5 = metalness;
    float param_6 = roughness;
    float3 param_7 = normal_vs;
    out.out_color = pbr_path(param_2, param_3, param_4, param_5, param_6, param_7, (*spvDescriptorSet0.per_view_data), in.in_position_ws, in.in_position_vs, in.in_normal_vs, spvDescriptorSet0.shadow_map_images_cube, smp_depth, (*spvDescriptorSet2.per_object_data), spvDescriptorSet0.shadow_map_images);
    return out;
}

