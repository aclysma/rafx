#pragma clang diagnostic ignored "-Wmissing-prototypes"

#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct PointLight
{
    float3 position_ws;
    float range;
    float3 position_vs;
    float intensity;
    float4 color;
    int shadow_map;
};

struct SpotLight
{
    float3 position_ws;
    float spotlight_half_angle;
    float3 direction_ws;
    float range;
    float3 position_vs;
    float intensity;
    float4 color;
    float3 direction_vs;
    int shadow_map;
};

struct DirectionalLight
{
    float3 direction_ws;
    float intensity;
    float4 color;
    float3 direction_vs;
    int shadow_map;
};

struct PointLight_1
{
    packed_float3 position_ws;
    float range;
    packed_float3 position_vs;
    float intensity;
    float4 color;
    int shadow_map;
    char _m0_final_padding[12];
};

struct DirectionalLight_1
{
    packed_float3 direction_ws;
    float intensity;
    float4 color;
    packed_float3 direction_vs;
    int shadow_map;
};

struct SpotLight_1
{
    packed_float3 position_ws;
    float spotlight_half_angle;
    packed_float3 direction_ws;
    float range;
    packed_float3 position_vs;
    float intensity;
    float4 color;
    packed_float3 direction_vs;
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
    float4x4 view;
    float4x4 view_proj;
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

struct MaterialData
{
    float4 base_color_factor;
    packed_float3 emissive_factor;
    float metallic_factor;
    float roughness_factor;
    float normal_texture_scale;
    float alpha_threshold;
    uint enable_alpha_blend;
    uint enable_alpha_clip;
    uint has_base_color_texture;
    uint base_color_texture_has_alpha_channel;
    uint has_metallic_roughness_texture;
    uint has_normal_texture;
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
    texture2d<float> emissive_texture [[id(4)]];
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
    float3 in_model_view_0 [[user(locn6)]];
    float3 in_model_view_1 [[user(locn7)]];
    float3 in_model_view_2 [[user(locn8)]];
};

static inline __attribute__((always_inline))
float4 normal_map(thread const float3x3& tangent_binormal_normal, thread const float2& uv, thread texture2d<float> normal_texture, thread sampler smp)
{
    float3 normal = normal_texture.sample(smp, uv).xyz;
    normal = (normal * 2.0) - float3(1.0);
    normal.z = 0.0;
    normal.z = sqrt(1.0 - dot(normal, normal));
    normal.x = -normal.x;
    normal.y = -normal.y;
    normal = tangent_binormal_normal * normal;
    return normalize(float4(normal, 0.0));
}

static inline __attribute__((always_inline))
float DeferredLightingNDFRoughnessFilter(thread const float3& normal, thread const float& roughness2)
{
    float SIGMA2 = 0.15915493667125701904296875;
    float KAPPA = 0.180000007152557373046875;
    float3 dndu = dfdx(normal);
    float3 dndv = dfdy(normal);
    float kernelRoughness2 = (2.0 * SIGMA2) * (dot(dndu, dndu) + dot(dndv, dndv));
    float clampedKernelRoughness2 = fast::min(kernelRoughness2, KAPPA);
    return fast::clamp(roughness2 + clampedKernelRoughness2, 0.0, 1.0);
}

static inline __attribute__((always_inline))
float attenuate_light_for_range(thread const float& light_range, thread const float& _distance)
{
    return 1.0 - smoothstep(light_range * 0.75, light_range, _distance);
}

static inline __attribute__((always_inline))
float ndf_ggx(thread const float3& n, thread const float3& h, thread const float& roughness_squared)
{
    float a = roughness_squared;
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
float3 shade_pbr(
    thread const float3& surface_to_light_dir_vs,
    thread const float3& surface_to_eye_dir_vs,
    thread const float3& normal_vs,
    thread const float3& F0,
    thread const float3& base_color,
    thread const float& roughness,
    thread const float& roughness_ndf_filtered_squared,
    thread const float& metalness,
    thread const float3& radiance
)
{
    float3 halfway_dir_vs = normalize(surface_to_light_dir_vs + surface_to_eye_dir_vs);
    float3 param = normal_vs;
    float3 param_1 = halfway_dir_vs;
    float param_2 = roughness_ndf_filtered_squared;
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
float3 point_light_pbr(
    thread const PointLight& light,
    thread const float3& surface_to_eye_dir_vs,
    thread const float3& surface_position_vs,
    thread const float3& normal_vs,
    thread const float3& F0,
    thread const float3& base_color,
    thread const float& roughness,
    thread const float& roughness_ndf_filtered_squared,
    thread const float& metalness
)
{
    float3 surface_to_light_dir_vs = light.position_vs - surface_position_vs;
    float _distance = length(surface_to_light_dir_vs);
    surface_to_light_dir_vs /= float3(_distance);
    float attenuation = 1.0 / (0.001000000047497451305389404296875 + (_distance * _distance));
    float3 radiance = (light.color.xyz * attenuation) * light.intensity;
    float3 param = surface_to_light_dir_vs;
    float3 param_1 = surface_to_eye_dir_vs;
    float3 param_2 = normal_vs;
    float3 param_3 = F0;
    float3 param_4 = base_color;
    float param_5 = roughness;
    float param_6 = roughness_ndf_filtered_squared;
    float param_7 = metalness;
    float3 param_8 = radiance;
    return shade_pbr(param, param_1, param_2, param_3, param_4, param_5, param_6, param_7, param_8);
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
float do_calculate_percent_lit_cube(
    constant spvDescriptorSetBuffer0& spvDescriptorSet0,
    thread const float3& light_position_ws,
    thread const float3& light_position_vs,
    thread const float3& normal_vs,
    thread const int& index,
    thread const float& bias_multiplier,
    thread float4& in_position_ws,
    thread float3& in_position_vs,
    thread float3& in_normal_vs,
    thread sampler smp_depth
)
{
    float near_plane = spvDescriptorSet0.per_view_data->shadow_map_cube_data[index].cube_map_projection_near_z;
    float far_plane = spvDescriptorSet0.per_view_data->shadow_map_cube_data[index].cube_map_projection_far_z;
    float3 light_to_surface_ws = in_position_ws.xyz - light_position_ws;
    float3 surface_to_light_dir_vs = normalize(light_position_vs - in_position_vs);
    float bias_angle_factor = 1.0 - fast::max(0.0, dot(in_normal_vs, surface_to_light_dir_vs));
    bias_angle_factor = pow(bias_angle_factor, 3.0);
    float bias0 = 0.000600000028498470783233642578125 + (0.006000000052154064178466796875 * bias_angle_factor);
    float3 param = light_to_surface_ws;
    float param_1 = near_plane;
    float param_2 = far_plane;
    float depth_of_surface = calculate_cubemap_equivalent_depth(param, param_1, param_2);
    float4 _338 = float4(light_to_surface_ws, depth_of_surface + bias0);
    float shadow = spvDescriptorSet0.shadow_map_images_cube[index].sample_compare(smp_depth, _338.xyz, _338.w);
    return shadow;
}

static inline __attribute__((always_inline))
float calculate_percent_lit_cube(
    constant spvDescriptorSetBuffer0& spvDescriptorSet0,
    thread const float3& light_position_ws,
    thread const float3& light_position_vs,
    thread const float3& normal_vs,
    thread const int& index,
    thread const float& bias_multiplier,
    thread float4& in_position_ws,
    thread float3& in_position_vs,
    thread float3& in_normal_vs,
    thread sampler smp_depth
)
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
    return do_calculate_percent_lit_cube(spvDescriptorSet0, param, param_1, param_2, param_3, param_4, in_position_ws, in_position_vs, in_normal_vs, smp_depth);
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
float3 spot_light_pbr(
    thread const SpotLight& light,
    thread const float3& surface_to_eye_dir_vs,
    thread const float3& surface_position_vs,
    thread const float3& normal_vs,
    thread const float3& F0,
    thread const float3& base_color,
    thread const float& roughness,
    thread const float& roughness_ndf_filtered_squared,
    thread const float& metalness
)
{
    float3 surface_to_light_dir_vs = light.position_vs - surface_position_vs;
    float _distance = length(surface_to_light_dir_vs);
    surface_to_light_dir_vs /= float3(_distance);
    float attenuation = 1.0 / (0.001000000047497451305389404296875 + (_distance * _distance));
    float3 param = surface_to_light_dir_vs;
    float3 param_1 = light.direction_vs;
    float param_2 = light.spotlight_half_angle;
    float spotlight_direction_intensity = spotlight_cone_falloff(param, param_1, param_2);
    float radiance = (attenuation * light.intensity) * spotlight_direction_intensity;
    if (radiance > 0.0)
    {
        float3 param_3 = surface_to_light_dir_vs;
        float3 param_4 = surface_to_eye_dir_vs;
        float3 param_5 = normal_vs;
        float3 param_6 = F0;
        float3 param_7 = base_color;
        float param_8 = roughness;
        float param_9 = roughness_ndf_filtered_squared;
        float param_10 = metalness;
        float3 param_11 = light.color.xyz * radiance;
        return shade_pbr(param_3, param_4, param_5, param_6, param_7, param_8, param_9, param_10, param_11);
    }
    else
    {
        return float3(0.0);
    }
}

static inline __attribute__((always_inline))
float do_calculate_percent_lit(
    constant spvDescriptorSetBuffer0& spvDescriptorSet0,
    thread const float3& normal_vs,
    thread const int& index,
    thread const float& bias_multiplier,
    thread float4& in_position_ws,
    thread sampler smp_depth,
    thread float3x3& in_model_view
)
{
    float4 shadow_map_pos = spvDescriptorSet0.per_view_data->shadow_map_2d_data[index].shadow_map_view_proj * in_position_ws;
    float3 projected = shadow_map_pos.xyz / float3(shadow_map_pos.w);
    float2 sample_location_uv = (projected.xy * 0.5) + float2(0.5);
    sample_location_uv.y = 1.0 - sample_location_uv.y;
    float depth_of_surface = projected.z;
    float3 light_dir_vs = in_model_view * spvDescriptorSet0.per_view_data->shadow_map_2d_data[index].shadow_map_light_dir;
    float3 surface_to_light_dir_vs = -light_dir_vs;
    float bias_angle_factor = 1.0 - dot(normal_vs, surface_to_light_dir_vs);
    float bias0 = fast::max(((0.00999999977648258209228515625 * bias_angle_factor) * bias_angle_factor) * bias_angle_factor, 0.0005000000237487256526947021484375) * bias_multiplier;
    float shadow = 0.0;
    float2 texelSize = float2(1.0) / float2(int2(spvDescriptorSet0.shadow_map_images[index].get_width(), spvDescriptorSet0.shadow_map_images[index].get_height()));
    for (int x = -1; x <= 1; x++)
    {
        for (int y = -1; y <= 1; y++)
        {
            float3 _475 = float3(sample_location_uv + (float2(float(x), float(y)) * texelSize), depth_of_surface + bias0);
            shadow += spvDescriptorSet0.shadow_map_images[index].sample_compare(smp_depth, _475.xy, _475.z);
        }
    }
    shadow /= 9.0;
    return shadow;
}

static inline __attribute__((always_inline))
float calculate_percent_lit(
    constant spvDescriptorSetBuffer0& spvDescriptorSet0,
    thread const float3& normal,
    thread const int& index,
    thread const float& bias_multiplier,
    thread float4& in_position_ws,
    thread sampler smp_depth,
    thread float3x3& in_model_view
)
{
    if (index == (-1))
    {
        return 1.0;
    }
    float3 param = normal;
    int param_1 = index;
    float param_2 = bias_multiplier;
    return do_calculate_percent_lit(spvDescriptorSet0, param, param_1, param_2, in_position_ws, smp_depth, in_model_view);
}

static inline __attribute__((always_inline))
float3 directional_light_pbr(
    thread const DirectionalLight& light,
    thread const float3& surface_to_eye_dir_vs,
    thread const float3& surface_position_vs,
    thread const float3& normal_vs,
    thread const float3& F0,
    thread const float3& base_color,
    thread const float& roughness,
    thread const float& roughness_ndf_filtered_squared,
    thread const float& metalness
)
{
    float3 surface_to_light_dir_vs = -light.direction_vs;
    float3 radiance = light.color.xyz * light.intensity;
    float3 param = surface_to_light_dir_vs;
    float3 param_1 = surface_to_eye_dir_vs;
    float3 param_2 = normal_vs;
    float3 param_3 = F0;
    float3 param_4 = base_color;
    float param_5 = roughness;
    float param_6 = roughness_ndf_filtered_squared;
    float param_7 = metalness;
    float3 param_8 = radiance;
    return shade_pbr(param, param_1, param_2, param_3, param_4, param_5, param_6, param_7, param_8);
}

static inline __attribute__((always_inline))
float4 pbr_path(
    constant spvDescriptorSetBuffer0& spvDescriptorSet0,
    thread const float3& surface_to_eye_vs,
    thread const float4& base_color,
    thread const float4& emissive_color,
    thread const float& metalness,
    thread const float& roughness,
    thread const float3& normal_vs,
    thread float4& in_position_ws,
    thread float3& in_position_vs,
    thread float3& in_normal_vs,
    thread sampler smp_depth,
    thread float3x3& in_model_view
)
{
    float3 fresnel_base = float3(0.039999999105930328369140625);
    fresnel_base = mix(fresnel_base, base_color.xyz, float3(metalness));
    float3 param = normal_vs;
    float param_1 = roughness * roughness;
    float roughness_ndf_filtered_squared = DeferredLightingNDFRoughnessFilter(param, param_1);
    float3 total_light = float3(0.0);
    PointLight param_4;
    for (uint i = 0u; i < spvDescriptorSet0.per_view_data->point_light_count; i++)
    {
        float light_surface_distance = distance(float3(spvDescriptorSet0.per_view_data->point_lights[i].position_ws), in_position_ws.xyz);
        float range = spvDescriptorSet0.per_view_data->point_lights[i].range;
        if (light_surface_distance <= range)
        {
            float param_2 = range;
            float param_3 = light_surface_distance;
            float soft_falloff_factor = attenuate_light_for_range(param_2, param_3);
            param_4.position_ws = float3(spvDescriptorSet0.per_view_data->point_lights[i].position_ws);
            param_4.range = spvDescriptorSet0.per_view_data->point_lights[i].range;
            param_4.position_vs = float3(spvDescriptorSet0.per_view_data->point_lights[i].position_vs);
            param_4.intensity = spvDescriptorSet0.per_view_data->point_lights[i].intensity;
            param_4.color = spvDescriptorSet0.per_view_data->point_lights[i].color;
            param_4.shadow_map = spvDescriptorSet0.per_view_data->point_lights[i].shadow_map;
            float3 param_5 = surface_to_eye_vs;
            float3 param_6 = in_position_vs;
            float3 param_7 = normal_vs;
            float3 param_8 = fresnel_base;
            float3 param_9 = base_color.xyz;
            float param_10 = roughness;
            float param_11 = roughness_ndf_filtered_squared;
            float param_12 = metalness;
            float3 pbr = point_light_pbr(param_4, param_5, param_6, param_7, param_8, param_9, param_10, param_11, param_12) * soft_falloff_factor;
            float percent_lit = 0.0;
            if (any(pbr > float3(0.0)))
            {
                float3 param_13 = float3(spvDescriptorSet0.per_view_data->point_lights[i].position_ws);
                float3 param_14 = float3(spvDescriptorSet0.per_view_data->point_lights[i].position_vs);
                float3 param_15 = normal_vs;
                int param_16 = spvDescriptorSet0.per_view_data->point_lights[i].shadow_map;
                float param_17 = 1.0;
                percent_lit = calculate_percent_lit_cube(spvDescriptorSet0, param_13, param_14, param_15, param_16, param_17, in_position_ws, in_position_vs, in_normal_vs, smp_depth);
            }
            total_light += (pbr * percent_lit);
        }
    }
    SpotLight param_20;
    for (uint i_1 = 0u; i_1 < spvDescriptorSet0.per_view_data->spot_light_count; i_1++)
    {
        float light_surface_distance_1 = distance(float3(spvDescriptorSet0.per_view_data->spot_lights[i_1].position_ws), in_position_ws.xyz);
        float range_1 = spvDescriptorSet0.per_view_data->spot_lights[i_1].range;
        if (light_surface_distance_1 <= range_1)
        {
            float param_18 = range_1;
            float param_19 = light_surface_distance_1;
            float soft_falloff_factor_1 = attenuate_light_for_range(param_18, param_19);
            param_20.position_ws = float3(spvDescriptorSet0.per_view_data->spot_lights[i_1].position_ws);
            param_20.spotlight_half_angle = spvDescriptorSet0.per_view_data->spot_lights[i_1].spotlight_half_angle;
            param_20.direction_ws = float3(spvDescriptorSet0.per_view_data->spot_lights[i_1].direction_ws);
            param_20.range = spvDescriptorSet0.per_view_data->spot_lights[i_1].range;
            param_20.position_vs = float3(spvDescriptorSet0.per_view_data->spot_lights[i_1].position_vs);
            param_20.intensity = spvDescriptorSet0.per_view_data->spot_lights[i_1].intensity;
            param_20.color = spvDescriptorSet0.per_view_data->spot_lights[i_1].color;
            param_20.direction_vs = float3(spvDescriptorSet0.per_view_data->spot_lights[i_1].direction_vs);
            param_20.shadow_map = spvDescriptorSet0.per_view_data->spot_lights[i_1].shadow_map;
            float3 param_21 = surface_to_eye_vs;
            float3 param_22 = in_position_vs;
            float3 param_23 = normal_vs;
            float3 param_24 = fresnel_base;
            float3 param_25 = base_color.xyz;
            float param_26 = roughness;
            float param_27 = roughness_ndf_filtered_squared;
            float param_28 = metalness;
            float3 pbr_1 = spot_light_pbr(param_20, param_21, param_22, param_23, param_24, param_25, param_26, param_27, param_28) * soft_falloff_factor_1;
            float percent_lit_1 = 0.0;
            if (any(pbr_1 > float3(0.0)))
            {
                float3 param_29 = normal_vs;
                int param_30 = spvDescriptorSet0.per_view_data->spot_lights[i_1].shadow_map;
                float param_31 = 1.0;
                percent_lit_1 = calculate_percent_lit(spvDescriptorSet0, param_29, param_30, param_31, in_position_ws, smp_depth, in_model_view);
            }
            total_light += (pbr_1 * percent_lit_1);
        }
    }
    DirectionalLight param_32;
    for (uint i_2 = 0u; i_2 < spvDescriptorSet0.per_view_data->directional_light_count; i_2++)
    {
        param_32.direction_ws = float3(spvDescriptorSet0.per_view_data->directional_lights[i_2].direction_ws);
        param_32.intensity = spvDescriptorSet0.per_view_data->directional_lights[i_2].intensity;
        param_32.color = spvDescriptorSet0.per_view_data->directional_lights[i_2].color;
        param_32.direction_vs = float3(spvDescriptorSet0.per_view_data->directional_lights[i_2].direction_vs);
        param_32.shadow_map = spvDescriptorSet0.per_view_data->directional_lights[i_2].shadow_map;
        float3 param_33 = surface_to_eye_vs;
        float3 param_34 = in_position_vs;
        float3 param_35 = normal_vs;
        float3 param_36 = fresnel_base;
        float3 param_37 = base_color.xyz;
        float param_38 = roughness;
        float param_39 = roughness_ndf_filtered_squared;
        float param_40 = metalness;
        float3 pbr_2 = directional_light_pbr(param_32, param_33, param_34, param_35, param_36, param_37, param_38, param_39, param_40);
        float percent_lit_2 = 0.0;
        if (any(pbr_2 > float3(0.0)))
        {
            float3 param_41 = normal_vs;
            int param_42 = spvDescriptorSet0.per_view_data->directional_lights[i_2].shadow_map;
            float param_43 = 1.0;
            percent_lit_2 = calculate_percent_lit(spvDescriptorSet0, param_41, param_42, param_43, in_position_ws, smp_depth, in_model_view);
        }
        total_light += (pbr_2 * percent_lit_2);
    }
    float3 ambient = per_view_data.ambient_light.xyz * base_color.xyz;
    float alpha = 1.0;
    if (per_material_data.data.enable_alpha_blend != 0u)
    {
        alpha = base_color.w;
    }
    else
    {
        bool _1199 = per_material_data.data.enable_alpha_clip != 0u;
        bool _1207;
        if (_1199)
        {
            _1207 = base_color.w < per_material_data.data.alpha_threshold;
        }
        else
        {
            _1207 = _1199;
        }
        if (_1207)
        {
            alpha = 0.0;
        }
    }
    float3 color = (ambient + total_light) + emissive_color.xyz;
    return float4(color, base_color.w);
}

static inline __attribute__((always_inline))
float4 pbr_main(
    constant spvDescriptorSetBuffer0& spvDescriptorSet0,
    thread texture2d<float> normal_texture,
    thread sampler smp,
    thread float4& in_position_ws,
    thread float3& in_position_vs,
    thread float3& in_normal_vs,
    thread sampler smp_depth,
    thread float3x3& in_model_view,
    constant MaterialDataUbo& per_material_data,
    thread texture2d<float> base_color_texture,
    thread float2& in_uv,
    thread texture2d<float> emissive_texture,
    thread texture2d<float> metallic_roughness_texture,
    thread float3& in_tangent_vs,
    thread float3& in_binormal_vs
)
{
    float4 base_color = per_material_data.data.base_color_factor;
    if (per_material_data.data.has_base_color_texture != 0u)
    {
        float4 sampled_color = base_color_texture.sample(smp, in_uv);
        if (per_material_data.data.base_color_texture_has_alpha_channel != 0u)
        {
            base_color *= sampled_color;
        }
        else
        {
            base_color = float4(base_color.xyz * sampled_color.xyz, base_color.w);
        }
    }
    float4 emissive_color = float4(per_material_data.data.emissive_factor[0], per_material_data.data.emissive_factor[1], per_material_data.data.emissive_factor[2], 1.0);
    if (per_material_data.data.has_emissive_texture != 0u)
    {
        emissive_color *= emissive_texture.sample(smp, in_uv);
        base_color = float4(1.0, 1.0, 0.0, 1.0);
    }
    float metalness = per_material_data.data.metallic_factor;
    float roughness = per_material_data.data.roughness_factor;
    if (per_material_data.data.has_metallic_roughness_texture != 0u)
    {
        float4 sampled = metallic_roughness_texture.sample(smp, in_uv);
        metalness *= sampled.z;
        roughness *= sampled.y;
    }
    metalness = fast::clamp(metalness, 0.0, 1.0);
    roughness = fast::clamp(roughness, 0.0, 1.0);
    float3 normal_vs;
    if (per_material_data.data.has_normal_texture != 0u)
    {
        float3x3 tbn = float3x3(float3(in_tangent_vs), float3(in_binormal_vs), float3(in_normal_vs));
        float3x3 param = tbn;
        float2 param_1 = in_uv;
        normal_vs = normal_map(param, param_1, normal_texture, smp).xyz;
    }
    else
    {
        normal_vs = normalize(float4(in_normal_vs, 0.0)).xyz;
    }
    float3 eye_position_vs = float3(0.0);
    float3 surface_to_eye_vs = normalize(eye_position_vs - in_position_vs);
    float3 param_2 = surface_to_eye_vs;
    float4 param_3 = base_color;
    float4 param_4 = emissive_color;
    float param_5 = metalness;
    float param_6 = roughness;
    float3 param_7 = normal_vs;
    float4 out_color = pbr_path(
        spvDescriptorSet0,
        param_2,
        param_3,
        param_4,
        param_5,
        param_6,
        param_7,
        in_position_ws,
        in_position_vs,
        in_normal_vs,
        smp_depth,
        in_model_view
    );
    return out_color;
}

fragment main0_out main0(main0_in in [[stage_in]], constant spvDescriptorSetBuffer0& spvDescriptorSet0 [[buffer(0)]], constant spvDescriptorSetBuffer1& spvDescriptorSet1 [[buffer(1)]])
{
    constexpr sampler smp(filter::linear, mip_filter::linear, address::repeat, compare_func::never, max_anisotropy(16));
    constexpr sampler smp_depth(filter::linear, mip_filter::linear, compare_func::greater, max_anisotropy(1));
    main0_out out = {};
    float3x3 in_model_view = {};
    in_model_view[0] = in.in_model_view_0;
    in_model_view[1] = in.in_model_view_1;
    in_model_view[2] = in.in_model_view_2;
    out.out_color = pbr_main(
        spvDescriptorSet0,
        spvDescriptorSet1.normal_texture,
        smp,
        in.in_position_ws,
        in.in_position_vs,
        in.in_normal_vs,
        smp_depth,
        in_model_view,
        (*spvDescriptorSet1.per_material_data),
        spvDescriptorSet1.base_color_texture,
        in.in_uv,
        spvDescriptorSet1.emissive_texture,
        spvDescriptorSet1.metallic_roughness_texture,
        in.in_tangent_vs,
        in.in_binormal_vs
    );
    return out;
}

