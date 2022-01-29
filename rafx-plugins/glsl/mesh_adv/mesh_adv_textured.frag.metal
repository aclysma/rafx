#pragma clang diagnostic ignored "-Wmissing-prototypes"

#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct DirectionalLight
{
    float3 direction_ws;
    float intensity;
    float4 color;
    float3 direction_vs;
    int shadow_map;
};

struct DirectionalLight_1
{
    packed_float3 direction_ws;
    float intensity;
    float4 color;
    packed_float3 direction_vs;
    int shadow_map;
};

struct ShadowMap2DData
{
    float2 uv_min;
    float2 uv_max;
    float4x4 shadow_map_view_proj;
    float3 shadow_map_light_dir;
};

struct ShadowMapCubeData
{
    float4 uv_min_uv_max[6];
    float cube_map_projection_near_z;
    float cube_map_projection_far_z;
    char _m0_final_padding[8];
};

struct PerViewData
{
    float4x4 view;
    float4x4 view_proj;
    float4 ambient_light;
    float2 jitter_amount;
    uint viewport_width;
    uint viewport_height;
    float mip_bias;
    float ndf_filter_amount;
    uint directional_light_count;
    uint use_clustered_lighting;
    DirectionalLight_1 directional_lights[8];
    ShadowMap2DData shadow_map_2d_data[96];
    ShadowMapCubeData shadow_map_cube_data[32];
};

struct LightInList
{
    packed_float3 position_ws;
    float range;
    packed_float3 position_vs;
    float intensity;
    float4 color;
    packed_float3 spotlight_direction_ws;
    float spotlight_half_angle;
    packed_float3 spotlight_direction_vs;
    int shadow_map;
};

struct AllLights
{
    uint light_count;
    LightInList data[512];
};

struct LightInList_1
{
    float3 position_ws;
    float range;
    float3 position_vs;
    float intensity;
    float4 color;
    float3 spotlight_direction_ws;
    float spotlight_half_angle;
    float3 spotlight_direction_vs;
    int shadow_map;
};

struct ClusterMeta
{
    uint count;
    uint first_light;
};

struct LightBinningOutput
{
    uint data_write_ptr;
    uint pad0;
    uint pad1;
    uint pad2;
    ClusterMeta offsets[3072];
    uint data[786432];
};

struct LightBinOutput
{
    LightBinningOutput data;
};

struct DrawData
{
    uint transform_index;
    uint material_index;
};

struct AllDrawData
{
    DrawData draw_data[1];
};

struct PushConstants
{
    uint draw_data_index;
};

struct MaterialDbEntry
{
    float4 base_color_factor;
    float3 emissive_factor;
    float metallic_factor;
    float roughness_factor;
    float normal_texture_scale;
    float alpha_threshold;
    bool enable_alpha_blend;
    bool enable_alpha_clip;
    int color_texture;
    bool base_color_texture_has_alpha_channel;
    int metallic_roughness_texture;
    int normal_texture;
    int emissive_texture;
};

struct MaterialDbEntry_1
{
    float4 base_color_factor;
    packed_float3 emissive_factor;
    float metallic_factor;
    float roughness_factor;
    float normal_texture_scale;
    float alpha_threshold;
    uint enable_alpha_blend;
    uint enable_alpha_clip;
    int color_texture;
    uint base_color_texture_has_alpha_channel;
    int metallic_roughness_texture;
    int normal_texture;
    int emissive_texture;
    char _m0_final_padding[8];
};

struct AllMaterials
{
    MaterialDbEntry_1 materials[1];
};

struct Transform
{
    float4x4 model_matrix;
};

struct AllTransforms
{
    Transform transforms[1];
};

struct spvDescriptorSetBuffer0
{
    constant PerViewData* per_view_data [[id(0)]];
    depth2d<float> shadow_map_atlas [[id(4)]];
    device LightBinOutput* light_bin_output [[id(5)]];
    device AllLights* all_lights [[id(6)]];
};

struct spvDescriptorSetBuffer1
{
    texture2d<float> ssao_texture [[id(0)]];
};

struct spvDescriptorSetBuffer2
{
    device AllTransforms* all_transforms [[id(0)]];
    device AllDrawData* all_draw_data [[id(1)]];
};

struct spvDescriptorSetBuffer3
{
    device AllMaterials* all_materials [[id(0)]];
    array<texture2d<float>, 256> all_material_textures [[id(1)]];
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
uint get_light_cluster_index(constant PerViewData& per_view_data, thread float3& in_position_vs, thread float4& gl_FragCoord)
{
    float NEAR_Z = 5.0;
    float FAR_Z = 10000.0;
    int X_BINS = 16;
    int Y_BINS = 8;
    int Z_BINS = 24;
    uint cluster_coord_x = min(uint((gl_FragCoord.x / float(per_view_data.viewport_width)) * float(X_BINS)), uint(X_BINS - 1));
    uint cluster_coord_y = min(uint((1.0 - (gl_FragCoord.y / float(per_view_data.viewport_height))) * float(Y_BINS)), uint(Y_BINS - 1));
    float top = float(Z_BINS - 1) * log((-in_position_vs.z) / NEAR_Z);
    float bottom = log(FAR_Z / NEAR_Z);
    uint cluster_coord_z = uint(fast::clamp((top / bottom) + 1.0, 0.0, float(Z_BINS - 1)));
    uint linear_index = ((uint(X_BINS * Y_BINS) * cluster_coord_z) + (uint(X_BINS) * cluster_coord_y)) + cluster_coord_x;
    return linear_index;
}

static inline __attribute__((always_inline))
float4 normal_map(constant spvDescriptorSetBuffer3& spvDescriptorSet3, thread const int& normal_texture, thread const float3x3& tangent_binormal_normal, thread const float2& uv, thread sampler smp, constant PerViewData& per_view_data)
{
    float3 normal = spvDescriptorSet3.all_material_textures[normal_texture].sample(smp, uv, bias(per_view_data.mip_bias)).xyz;
    normal = (normal * 2.0) - float3(1.0);
    normal.z = 0.0;
    normal.z = sqrt(1.0 - dot(normal, normal));
    normal.x = -normal.x;
    normal.y = -normal.y;
    normal = tangent_binormal_normal * normal;
    return normalize(float4(normal, 0.0));
}

static inline __attribute__((always_inline))
float DeferredLightingNDFRoughnessFilter(thread const float3& normal, thread const float& roughness2, thread const float& ndf_filter_amount)
{
    float SIGMA2 = 0.15915493667125701904296875;
    float KAPPA = 0.180000007152557373046875;
    float3 dndu = dfdx(normal);
    float3 dndv = dfdy(normal);
    float kernelRoughness2 = (2.0 * SIGMA2) * (dot(dndu, dndu) + dot(dndv, dndv));
    float clampedKernelRoughness2 = fast::min(kernelRoughness2, KAPPA);
    return fast::clamp(roughness2 + (clampedKernelRoughness2 * ndf_filter_amount), 0.0, 1.0);
}

static inline __attribute__((always_inline))
float attenuate_light_for_range(thread const float& light_range, thread const float& _distance)
{
    return 1.0 - smoothstep(light_range * 0.75, light_range, _distance);
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
float3 shade_pbr(thread const float3& surface_to_light_dir_vs, thread const float3& surface_to_eye_dir_vs, thread const float3& normal_vs, thread const float3& F0, thread const float3& base_color, thread const float& roughness, thread const float& roughness_ndf_filtered_squared, thread const float& metalness, thread const float3& radiance)
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
float3 spot_light_pbr(thread const float3& light_position_vs, thread const float3& light_color, thread const float& light_intensity, thread const float3& light_direction_vs, thread const float& light_spotlight_half_angle, thread const float3& surface_to_eye_dir_vs, thread const float3& surface_position_vs, thread const float3& normal_vs, thread const float3& F0, thread const float3& base_color, thread const float& roughness, thread const float& roughness_ndf_filtered_squared, thread const float& metalness)
{
    float3 surface_to_light_dir_vs = light_position_vs - surface_position_vs;
    float _distance = length(surface_to_light_dir_vs);
    surface_to_light_dir_vs /= float3(_distance);
    float attenuation = 1.0 / (0.001000000047497451305389404296875 + (_distance * _distance));
    float3 param = surface_to_light_dir_vs;
    float3 param_1 = light_direction_vs;
    float param_2 = light_spotlight_half_angle;
    float spotlight_direction_intensity = spotlight_cone_falloff(param, param_1, param_2);
    float radiance = (attenuation * light_intensity) * spotlight_direction_intensity;
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
        float3 param_11 = light_color * radiance;
        return shade_pbr(param_3, param_4, param_5, param_6, param_7, param_8, param_9, param_10, param_11);
    }
    else
    {
        return float3(0.0);
    }
}

static inline __attribute__((always_inline))
float do_calculate_percent_lit(thread const float3& normal_vs, thread const int& index, thread const float& bias_multiplier, constant PerViewData& per_view_data, thread float4& in_position_ws, thread depth2d<float> shadow_map_atlas, thread float3x3& in_model_view, thread sampler smp_depth_linear)
{
    float4 shadow_map_pos = per_view_data.shadow_map_2d_data[index].shadow_map_view_proj * in_position_ws;
    float3 projected = shadow_map_pos.xyz / float3(shadow_map_pos.w);
    float2 sample_location_uv = (projected.xy * 0.5) + float2(0.5);
    sample_location_uv.y = 1.0 - sample_location_uv.y;
    float2 uv_min = per_view_data.shadow_map_2d_data[index].uv_min;
    float2 uv_max = per_view_data.shadow_map_2d_data[index].uv_max;
    sample_location_uv = mix(uv_min, uv_max, sample_location_uv);
    float depth_of_surface = projected.z;
    float3 light_dir_vs = in_model_view * per_view_data.shadow_map_2d_data[index].shadow_map_light_dir;
    float3 surface_to_light_dir_vs = -light_dir_vs;
    float bias_angle_factor = 1.0 - dot(normal_vs, surface_to_light_dir_vs);
    float bias0 = fast::max(((0.00999999977648258209228515625 * bias_angle_factor) * bias_angle_factor) * bias_angle_factor, 0.0005000000237487256526947021484375) * bias_multiplier;
    float4 uv_min_max_compare = float4(uv_min, -uv_max);
    float percent_lit = 0.0;
    float2 texelSize = float2(int2(1) / int2(shadow_map_atlas.get_width(), shadow_map_atlas.get_height()));
    for (int x = -1; x <= 1; x++)
    {
        for (int y = -1; y <= 1; y++)
        {
            float4 uv = float4(sample_location_uv + (float2(float(x), float(y)) * texelSize), 0.0, 0.0);
            float2 _666 = -uv.xy;
            uv = float4(uv.x, uv.y, _666.x, _666.y);
            if (all(uv >= uv_min_max_compare))
            {
                float3 _686 = float3(uv.xy, depth_of_surface + bias0);
                percent_lit += shadow_map_atlas.sample_compare(smp_depth_linear, _686.xy, _686.z);
            }
            else
            {
                percent_lit += 1.0;
            }
        }
    }
    percent_lit /= 9.0;
    return percent_lit;
}

static inline __attribute__((always_inline))
float calculate_percent_lit(thread const float3& normal, thread const int& index, thread const float& bias_multiplier, constant PerViewData& per_view_data, thread float4& in_position_ws, thread depth2d<float> shadow_map_atlas, thread float3x3& in_model_view, thread sampler smp_depth_linear)
{
    if (index == (-1))
    {
        return 1.0;
    }
    float3 param = normal;
    int param_1 = index;
    float param_2 = bias_multiplier;
    return do_calculate_percent_lit(param, param_1, param_2, per_view_data, in_position_ws, shadow_map_atlas, in_model_view, smp_depth_linear);
}

static inline __attribute__((always_inline))
float3 point_light_pbr(thread const float3& light_position_vs, thread const float3& light_color, thread const float& light_intensity, thread const float3& surface_to_eye_dir_vs, thread const float3& surface_position_vs, thread const float3& normal_vs, thread const float3& F0, thread const float3& base_color, thread const float& roughness, thread const float& roughness_ndf_filtered_squared, thread const float& metalness)
{
    float3 surface_to_light_dir_vs = light_position_vs - surface_position_vs;
    float _distance = length(surface_to_light_dir_vs);
    surface_to_light_dir_vs /= float3(_distance);
    float attenuation = 1.0 / (0.001000000047497451305389404296875 + (_distance * _distance));
    float3 radiance = (light_color * attenuation) * light_intensity;
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
float3 cube_sample_to_uv_and_face_index(thread const float3& dir)
{
    float3 dirAbs = abs(dir);
    bool _320 = dirAbs.z >= dirAbs.x;
    bool _328;
    if (_320)
    {
        _328 = dirAbs.z >= dirAbs.y;
    }
    else
    {
        _328 = _320;
    }
    float faceIndex;
    float ma;
    float2 uv;
    if (_328)
    {
        faceIndex = (dir.z < 0.0) ? 5.0 : 4.0;
        ma = 0.5 / dirAbs.z;
        float _346;
        if (dir.z < 0.0)
        {
            _346 = -dir.x;
        }
        else
        {
            _346 = dir.x;
        }
        uv = float2(_346, -dir.y);
    }
    else
    {
        if (dirAbs.y >= dirAbs.x)
        {
            faceIndex = (dir.y < 0.0) ? 3.0 : 2.0;
            ma = 0.5 / dirAbs.y;
            float _381;
            if (dir.y < 0.0)
            {
                _381 = -dir.z;
            }
            else
            {
                _381 = dir.z;
            }
            uv = float2(dir.x, _381);
        }
        else
        {
            faceIndex = float(dir.x < 0.0);
            ma = 0.5 / dirAbs.x;
            float _403;
            if (dir.x < 0.0)
            {
                _403 = dir.z;
            }
            else
            {
                _403 = -dir.z;
            }
            uv = float2(_403, -dir.y);
        }
    }
    return float3((uv * ma) + float2(0.5), faceIndex);
}

static inline __attribute__((always_inline))
float do_calculate_percent_lit_cube(thread const float3& light_position_ws, thread const float3& light_position_vs, thread const float3& normal_vs, thread const int& index, thread const float& bias_multiplier, constant PerViewData& per_view_data, thread float4& in_position_ws, thread float3& in_position_vs, thread float3& in_normal_vs, thread depth2d<float> shadow_map_atlas, thread sampler smp_depth_nearest)
{
    float near_plane = per_view_data.shadow_map_cube_data[index].cube_map_projection_near_z;
    float far_plane = per_view_data.shadow_map_cube_data[index].cube_map_projection_far_z;
    float3 light_to_surface_ws = in_position_ws.xyz - light_position_ws;
    float3 surface_to_light_dir_vs = normalize(light_position_vs - in_position_vs);
    float bias_angle_factor = 1.0 - fast::max(0.0, dot(in_normal_vs, surface_to_light_dir_vs));
    bias_angle_factor = pow(bias_angle_factor, 3.0);
    float bias0 = 0.000600000028498470783233642578125 + (0.006000000052154064178466796875 * bias_angle_factor);
    float3 param = light_to_surface_ws;
    float param_1 = near_plane;
    float param_2 = far_plane;
    float depth_of_surface = calculate_cubemap_equivalent_depth(param, param_1, param_2);
    float3 param_3 = light_to_surface_ws;
    float3 uv_and_face = cube_sample_to_uv_and_face_index(param_3);
    float4 uv_min_uv_max = per_view_data.shadow_map_cube_data[index].uv_min_uv_max[int(uv_and_face.z)];
    if (uv_min_uv_max.x < 0.0)
    {
        return 1.0;
    }
    float2 uv_to_sample = mix(uv_min_uv_max.xy, uv_min_uv_max.zw, uv_and_face.xy);
    float3 _517 = float3(uv_to_sample, depth_of_surface + bias0);
    float shadow = shadow_map_atlas.sample_compare(smp_depth_nearest, _517.xy, _517.z);
    return shadow;
}

static inline __attribute__((always_inline))
float calculate_percent_lit_cube(thread const float3& light_position_ws, thread const float3& light_position_vs, thread const float3& normal_vs, thread const int& index, thread const float& bias_multiplier, constant PerViewData& per_view_data, thread float4& in_position_ws, thread float3& in_position_vs, thread float3& in_normal_vs, thread depth2d<float> shadow_map_atlas, thread sampler smp_depth_nearest)
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
    return do_calculate_percent_lit_cube(param, param_1, param_2, param_3, param_4, per_view_data, in_position_ws, in_position_vs, in_normal_vs, shadow_map_atlas, smp_depth_nearest);
}

static inline __attribute__((always_inline))
float3 iterate_point_and_spot_lights_clustered(thread const float3& surface_to_eye_vs, thread const float4& base_color, thread const float& metalness, thread const float& roughness, thread const float3& normal_vs, thread const float3& fresnel_base, thread const float& roughness_ndf_filtered_squared, thread const uint& light_cluster_index, constant PerViewData& per_view_data, thread float4& in_position_ws, thread float3& in_position_vs, thread float3& in_normal_vs, thread depth2d<float> shadow_map_atlas, thread sampler smp_depth_nearest, thread float3x3& in_model_view, thread sampler smp_depth_linear, device AllLights& all_lights, device LightBinOutput& light_bin_output)
{
    float3 total_light = float3(0.0);
    uint light_first = light_bin_output.data.offsets[light_cluster_index].first_light;
    uint light_last = light_first + light_bin_output.data.offsets[light_cluster_index].count;
    LightInList_1 light;
    for (uint light_list_index = light_first; light_list_index < light_last; light_list_index++)
    {
        uint light_index = light_bin_output.data.data[light_list_index];
        light.position_ws = float3(all_lights.data[light_index].position_ws);
        light.range = all_lights.data[light_index].range;
        light.position_vs = float3(all_lights.data[light_index].position_vs);
        light.intensity = all_lights.data[light_index].intensity;
        light.color = all_lights.data[light_index].color;
        light.spotlight_direction_ws = float3(all_lights.data[light_index].spotlight_direction_ws);
        light.spotlight_half_angle = all_lights.data[light_index].spotlight_half_angle;
        light.spotlight_direction_vs = float3(all_lights.data[light_index].spotlight_direction_vs);
        light.shadow_map = all_lights.data[light_index].shadow_map;
        if (dot(light.spotlight_direction_vs, light.spotlight_direction_vs) > 0.00999999977648258209228515625)
        {
            float light_surface_distance = distance(light.position_ws, in_position_ws.xyz);
            float range = light.range;
            if (light_surface_distance <= range)
            {
                float param = range;
                float param_1 = light_surface_distance;
                float soft_falloff_factor = attenuate_light_for_range(param, param_1);
                float3 param_2 = light.position_vs;
                float3 param_3 = light.color.xyz;
                float param_4 = light.intensity;
                float3 param_5 = light.spotlight_direction_vs;
                float param_6 = light.spotlight_half_angle;
                float3 param_7 = surface_to_eye_vs;
                float3 param_8 = in_position_vs;
                float3 param_9 = normal_vs;
                float3 param_10 = fresnel_base;
                float3 param_11 = base_color.xyz;
                float param_12 = roughness;
                float param_13 = roughness_ndf_filtered_squared;
                float param_14 = metalness;
                float3 pbr = spot_light_pbr(param_2, param_3, param_4, param_5, param_6, param_7, param_8, param_9, param_10, param_11, param_12, param_13, param_14) * soft_falloff_factor;
                float percent_lit = 1.0;
                if (any(pbr > float3(0.0)))
                {
                    float3 param_15 = normal_vs;
                    int param_16 = light.shadow_map;
                    float param_17 = 1.0;
                    percent_lit = calculate_percent_lit(param_15, param_16, param_17, per_view_data, in_position_ws, shadow_map_atlas, in_model_view, smp_depth_linear);
                }
                total_light += (pbr * percent_lit);
            }
        }
        else
        {
            float light_surface_distance_1 = distance(light.position_ws, in_position_ws.xyz);
            float range_1 = light.range;
            if (light_surface_distance_1 <= range_1)
            {
                float param_18 = range_1;
                float param_19 = light_surface_distance_1;
                float soft_falloff_factor_1 = attenuate_light_for_range(param_18, param_19);
                float3 param_20 = light.position_vs;
                float3 param_21 = light.color.xyz;
                float param_22 = light.intensity;
                float3 param_23 = surface_to_eye_vs;
                float3 param_24 = in_position_vs;
                float3 param_25 = normal_vs;
                float3 param_26 = fresnel_base;
                float3 param_27 = base_color.xyz;
                float param_28 = roughness;
                float param_29 = roughness_ndf_filtered_squared;
                float param_30 = metalness;
                float3 pbr_1 = point_light_pbr(param_20, param_21, param_22, param_23, param_24, param_25, param_26, param_27, param_28, param_29, param_30) * soft_falloff_factor_1;
                float percent_lit_1 = 1.0;
                if (any(pbr_1 > float3(0.0)))
                {
                    float3 param_31 = light.position_ws;
                    float3 param_32 = light.position_vs;
                    float3 param_33 = normal_vs;
                    int param_34 = light.shadow_map;
                    float param_35 = 1.0;
                    percent_lit_1 = calculate_percent_lit_cube(param_31, param_32, param_33, param_34, param_35, per_view_data, in_position_ws, in_position_vs, in_normal_vs, shadow_map_atlas, smp_depth_nearest);
                }
                total_light += (pbr_1 * percent_lit_1);
            }
        }
    }
    return total_light;
}

static inline __attribute__((always_inline))
float3 iterate_point_and_spot_lights_all(thread const float3& surface_to_eye_vs, thread const float4& base_color, thread const float& metalness, thread const float& roughness, thread const float3& normal_vs, thread const float3& fresnel_base, thread const float& roughness_ndf_filtered_squared, thread const uint& light_cluster_index, constant PerViewData& per_view_data, thread float4& in_position_ws, thread float3& in_position_vs, thread float3& in_normal_vs, thread depth2d<float> shadow_map_atlas, thread sampler smp_depth_nearest, thread float3x3& in_model_view, thread sampler smp_depth_linear, device AllLights& all_lights)
{
    float3 total_light = float3(0.0);
    LightInList_1 light;
    for (uint light_index = 0u; light_index < all_lights.light_count; light_index++)
    {
        light.position_ws = float3(all_lights.data[light_index].position_ws);
        light.range = all_lights.data[light_index].range;
        light.position_vs = float3(all_lights.data[light_index].position_vs);
        light.intensity = all_lights.data[light_index].intensity;
        light.color = all_lights.data[light_index].color;
        light.spotlight_direction_ws = float3(all_lights.data[light_index].spotlight_direction_ws);
        light.spotlight_half_angle = all_lights.data[light_index].spotlight_half_angle;
        light.spotlight_direction_vs = float3(all_lights.data[light_index].spotlight_direction_vs);
        light.shadow_map = all_lights.data[light_index].shadow_map;
        if (dot(light.spotlight_direction_vs, light.spotlight_direction_vs) > 0.00999999977648258209228515625)
        {
            float light_surface_distance = distance(light.position_ws, in_position_ws.xyz);
            float range = light.range;
            if (light_surface_distance <= range)
            {
                float param = range;
                float param_1 = light_surface_distance;
                float soft_falloff_factor = attenuate_light_for_range(param, param_1);
                float3 param_2 = light.position_vs;
                float3 param_3 = light.color.xyz;
                float param_4 = light.intensity;
                float3 param_5 = light.spotlight_direction_vs;
                float param_6 = light.spotlight_half_angle;
                float3 param_7 = surface_to_eye_vs;
                float3 param_8 = in_position_vs;
                float3 param_9 = normal_vs;
                float3 param_10 = fresnel_base;
                float3 param_11 = base_color.xyz;
                float param_12 = roughness;
                float param_13 = roughness_ndf_filtered_squared;
                float param_14 = metalness;
                float3 pbr = spot_light_pbr(param_2, param_3, param_4, param_5, param_6, param_7, param_8, param_9, param_10, param_11, param_12, param_13, param_14) * soft_falloff_factor;
                float percent_lit = 1.0;
                if (any(pbr > float3(0.0)))
                {
                    float3 param_15 = normal_vs;
                    int param_16 = light.shadow_map;
                    float param_17 = 1.0;
                    percent_lit = calculate_percent_lit(param_15, param_16, param_17, per_view_data, in_position_ws, shadow_map_atlas, in_model_view, smp_depth_linear);
                }
                total_light += (pbr * percent_lit);
            }
        }
        else
        {
            float light_surface_distance_1 = distance(light.position_ws, in_position_ws.xyz);
            float range_1 = light.range;
            if (light_surface_distance_1 <= range_1)
            {
                float param_18 = range_1;
                float param_19 = light_surface_distance_1;
                float soft_falloff_factor_1 = attenuate_light_for_range(param_18, param_19);
                float3 param_20 = light.position_vs;
                float3 param_21 = light.color.xyz;
                float param_22 = light.intensity;
                float3 param_23 = surface_to_eye_vs;
                float3 param_24 = in_position_vs;
                float3 param_25 = normal_vs;
                float3 param_26 = fresnel_base;
                float3 param_27 = base_color.xyz;
                float param_28 = roughness;
                float param_29 = roughness_ndf_filtered_squared;
                float param_30 = metalness;
                float3 pbr_1 = point_light_pbr(param_20, param_21, param_22, param_23, param_24, param_25, param_26, param_27, param_28, param_29, param_30) * soft_falloff_factor_1;
                float percent_lit_1 = 1.0;
                if (any(pbr_1 > float3(0.0)))
                {
                    float3 param_31 = light.position_ws;
                    float3 param_32 = light.position_vs;
                    float3 param_33 = normal_vs;
                    int param_34 = light.shadow_map;
                    float param_35 = 1.0;
                    percent_lit_1 = calculate_percent_lit_cube(param_31, param_32, param_33, param_34, param_35, per_view_data, in_position_ws, in_position_vs, in_normal_vs, shadow_map_atlas, smp_depth_nearest);
                }
                total_light += (pbr_1 * percent_lit_1);
            }
        }
    }
    return total_light;
}

static inline __attribute__((always_inline))
float3 directional_light_pbr(thread const DirectionalLight& light, thread const float3& surface_to_eye_dir_vs, thread const float3& surface_position_vs, thread const float3& normal_vs, thread const float3& F0, thread const float3& base_color, thread const float& roughness, thread const float& roughness_ndf_filtered_squared, thread const float& metalness)
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
float4 pbr_path(thread const float3& surface_to_eye_vs, thread const float4& base_color, thread const float4& emissive_color, thread const float& metalness, thread const float& roughness, thread const float3& normal_vs, thread const uint& light_cluster_index, thread const float& ambient_factor, constant PerViewData& per_view_data, thread float4& in_position_ws, thread float3& in_position_vs, thread float3& in_normal_vs, thread depth2d<float> shadow_map_atlas, thread sampler smp_depth_nearest, thread float3x3& in_model_view, thread sampler smp_depth_linear, device AllLights& all_lights, device LightBinOutput& light_bin_output, device AllDrawData& all_draw_data, constant PushConstants& constants, device AllMaterials& all_materials)
{
    float3 fresnel_base = float3(0.039999999105930328369140625);
    fresnel_base = mix(fresnel_base, base_color.xyz, float3(metalness));
    float3 param = normal_vs;
    float param_1 = roughness * roughness;
    float param_2 = per_view_data.ndf_filter_amount;
    float roughness_ndf_filtered_squared = DeferredLightingNDFRoughnessFilter(param, param_1, param_2);
    float3 total_light = float3(0.0);
    if (per_view_data.use_clustered_lighting != 0u)
    {
        float3 param_3 = surface_to_eye_vs;
        float4 param_4 = base_color;
        float param_5 = metalness;
        float param_6 = roughness;
        float3 param_7 = normal_vs;
        float3 param_8 = fresnel_base;
        float param_9 = roughness_ndf_filtered_squared;
        uint param_10 = light_cluster_index;
        total_light = iterate_point_and_spot_lights_clustered(param_3, param_4, param_5, param_6, param_7, param_8, param_9, param_10, per_view_data, in_position_ws, in_position_vs, in_normal_vs, shadow_map_atlas, smp_depth_nearest, in_model_view, smp_depth_linear, all_lights, light_bin_output);
    }
    else
    {
        float3 param_11 = surface_to_eye_vs;
        float4 param_12 = base_color;
        float param_13 = metalness;
        float param_14 = roughness;
        float3 param_15 = normal_vs;
        float3 param_16 = fresnel_base;
        float param_17 = roughness_ndf_filtered_squared;
        uint param_18 = light_cluster_index;
        total_light = iterate_point_and_spot_lights_all(param_11, param_12, param_13, param_14, param_15, param_16, param_17, param_18, per_view_data, in_position_ws, in_position_vs, in_normal_vs, shadow_map_atlas, smp_depth_nearest, in_model_view, smp_depth_linear, all_lights);
    }
    DirectionalLight param_19;
    for (uint i = 0u; i < per_view_data.directional_light_count; i++)
    {
        param_19.direction_ws = float3(per_view_data.directional_lights[i].direction_ws);
        param_19.intensity = per_view_data.directional_lights[i].intensity;
        param_19.color = per_view_data.directional_lights[i].color;
        param_19.direction_vs = float3(per_view_data.directional_lights[i].direction_vs);
        param_19.shadow_map = per_view_data.directional_lights[i].shadow_map;
        float3 param_20 = surface_to_eye_vs;
        float3 param_21 = in_position_vs;
        float3 param_22 = normal_vs;
        float3 param_23 = fresnel_base;
        float3 param_24 = base_color.xyz;
        float param_25 = roughness;
        float param_26 = roughness_ndf_filtered_squared;
        float param_27 = metalness;
        float3 pbr = directional_light_pbr(param_19, param_20, param_21, param_22, param_23, param_24, param_25, param_26, param_27);
        float percent_lit = 1.0;
        if (any(pbr > float3(0.0)))
        {
            float3 param_28 = normal_vs;
            int param_29 = per_view_data.directional_lights[i].shadow_map;
            float param_30 = 1.0;
            percent_lit = calculate_percent_lit(param_28, param_29, param_30, per_view_data, in_position_ws, shadow_map_atlas, in_model_view, smp_depth_linear);
        }
        total_light += (pbr * percent_lit);
    }
    float3 ambient = (per_view_data.ambient_light.xyz * base_color.xyz) * ambient_factor;
    uint material_index = all_draw_data.draw_data[constants.draw_data_index].material_index;
    MaterialDbEntry per_material_data;
    per_material_data.base_color_factor = all_materials.materials[material_index].base_color_factor;
    per_material_data.emissive_factor = float3(all_materials.materials[material_index].emissive_factor);
    per_material_data.metallic_factor = all_materials.materials[material_index].metallic_factor;
    per_material_data.roughness_factor = all_materials.materials[material_index].roughness_factor;
    per_material_data.normal_texture_scale = all_materials.materials[material_index].normal_texture_scale;
    per_material_data.alpha_threshold = all_materials.materials[material_index].alpha_threshold;
    per_material_data.enable_alpha_blend = all_materials.materials[material_index].enable_alpha_blend != 0u;
    per_material_data.enable_alpha_clip = all_materials.materials[material_index].enable_alpha_clip != 0u;
    per_material_data.color_texture = all_materials.materials[material_index].color_texture;
    per_material_data.base_color_texture_has_alpha_channel = all_materials.materials[material_index].base_color_texture_has_alpha_channel != 0u;
    per_material_data.metallic_roughness_texture = all_materials.materials[material_index].metallic_roughness_texture;
    per_material_data.normal_texture = all_materials.materials[material_index].normal_texture;
    per_material_data.emissive_texture = all_materials.materials[material_index].emissive_texture;
    float alpha = 1.0;
    if (per_material_data.enable_alpha_blend)
    {
        alpha = base_color.w;
    }
    else
    {
        bool _1734;
        if (per_material_data.enable_alpha_clip)
        {
            _1734 = base_color.w < per_material_data.alpha_threshold;
        }
        else
        {
            _1734 = per_material_data.enable_alpha_clip;
        }
        if (_1734)
        {
            alpha = 0.0;
        }
    }
    float3 color = (ambient + total_light) + emissive_color.xyz;
    return float4(color, alpha);
}

static inline __attribute__((always_inline))
float4 pbr_main(thread sampler smp, constant PerViewData& per_view_data, thread float4& in_position_ws, thread float3& in_position_vs, thread float3& in_normal_vs, thread depth2d<float> shadow_map_atlas, thread sampler smp_depth_nearest, thread float3x3& in_model_view, thread sampler smp_depth_linear, device AllLights& all_lights, device LightBinOutput& light_bin_output, device AllDrawData& all_draw_data, constant PushConstants& constants, constant spvDescriptorSetBuffer3& spvDescriptorSet3, thread float4& gl_FragCoord, thread float2& in_uv, thread texture2d<float> ssao_texture, thread float3& in_tangent_vs, thread float3& in_binormal_vs)
{
    uint material_index = all_draw_data.draw_data[constants.draw_data_index].material_index;
    MaterialDbEntry per_material_data;
    per_material_data.base_color_factor = spvDescriptorSet3.all_materials->materials[material_index].base_color_factor;
    per_material_data.emissive_factor = float3(spvDescriptorSet3.all_materials->materials[material_index].emissive_factor);
    per_material_data.metallic_factor = spvDescriptorSet3.all_materials->materials[material_index].metallic_factor;
    per_material_data.roughness_factor = spvDescriptorSet3.all_materials->materials[material_index].roughness_factor;
    per_material_data.normal_texture_scale = spvDescriptorSet3.all_materials->materials[material_index].normal_texture_scale;
    per_material_data.alpha_threshold = spvDescriptorSet3.all_materials->materials[material_index].alpha_threshold;
    per_material_data.enable_alpha_blend = spvDescriptorSet3.all_materials->materials[material_index].enable_alpha_blend != 0u;
    per_material_data.enable_alpha_clip = spvDescriptorSet3.all_materials->materials[material_index].enable_alpha_clip != 0u;
    per_material_data.color_texture = spvDescriptorSet3.all_materials->materials[material_index].color_texture;
    per_material_data.base_color_texture_has_alpha_channel = spvDescriptorSet3.all_materials->materials[material_index].base_color_texture_has_alpha_channel != 0u;
    per_material_data.metallic_roughness_texture = spvDescriptorSet3.all_materials->materials[material_index].metallic_roughness_texture;
    per_material_data.normal_texture = spvDescriptorSet3.all_materials->materials[material_index].normal_texture;
    per_material_data.emissive_texture = spvDescriptorSet3.all_materials->materials[material_index].emissive_texture;
    float4 base_color = per_material_data.base_color_factor;
    float ambient_factor = 1.0;
    uint light_cluster_index = get_light_cluster_index(per_view_data, in_position_vs, gl_FragCoord);
    if (per_material_data.color_texture != (-1))
    {
        float4 sampled_color = spvDescriptorSet3.all_material_textures[per_material_data.color_texture].sample(smp, in_uv, bias(per_view_data.mip_bias));
        if (per_material_data.base_color_texture_has_alpha_channel)
        {
            base_color *= sampled_color;
        }
        else
        {
            base_color = float4(base_color.xyz * sampled_color.xyz, base_color.w);
        }
    }
    float screen_coord_x = gl_FragCoord.x / float(per_view_data.viewport_width);
    float screen_coord_y = gl_FragCoord.y / float(per_view_data.viewport_height);
    ambient_factor = ssao_texture.sample(smp, float2(screen_coord_x, screen_coord_y)).x;
    float4 emissive_color = float4(per_material_data.emissive_factor, 1.0);
    if (per_material_data.emissive_texture != (-1))
    {
        emissive_color *= spvDescriptorSet3.all_material_textures[per_material_data.emissive_texture].sample(smp, in_uv, bias(per_view_data.mip_bias));
    }
    float metalness = per_material_data.metallic_factor;
    float roughness = per_material_data.roughness_factor;
    if (per_material_data.metallic_roughness_texture != (-1))
    {
        float4 sampled = spvDescriptorSet3.all_material_textures[per_material_data.metallic_roughness_texture].sample(smp, in_uv, bias(per_view_data.mip_bias));
        metalness *= sampled.z;
        roughness *= sampled.y;
    }
    metalness = fast::clamp(metalness, 0.0, 1.0);
    roughness = fast::clamp(roughness, 0.0, 1.0);
    float3 normal_vs;
    if (per_material_data.normal_texture != (-1))
    {
        float3x3 tbn = float3x3(float3(in_tangent_vs), float3(in_binormal_vs), float3(in_normal_vs));
        int param = per_material_data.normal_texture;
        float3x3 param_1 = tbn;
        float2 param_2 = in_uv;
        normal_vs = normal_map(spvDescriptorSet3, param, param_1, param_2, smp, per_view_data).xyz;
    }
    else
    {
        normal_vs = normalize(float4(in_normal_vs, 0.0)).xyz;
    }
    float3 eye_position_vs = float3(0.0);
    float3 surface_to_eye_vs = normalize(eye_position_vs - in_position_vs);
    float3 param_3 = surface_to_eye_vs;
    float4 param_4 = base_color;
    float4 param_5 = emissive_color;
    float param_6 = metalness;
    float param_7 = roughness;
    float3 param_8 = normal_vs;
    uint param_9 = light_cluster_index;
    float param_10 = ambient_factor;
    float4 out_color = pbr_path(param_3, param_4, param_5, param_6, param_7, param_8, param_9, param_10, per_view_data, in_position_ws, in_position_vs, in_normal_vs, shadow_map_atlas, smp_depth_nearest, in_model_view, smp_depth_linear, all_lights, light_bin_output, all_draw_data, constants, *spvDescriptorSet3.all_materials);
    return out_color;
}

fragment main0_out main0(main0_in in [[stage_in]], constant spvDescriptorSetBuffer0& spvDescriptorSet0 [[buffer(0)]], constant spvDescriptorSetBuffer1& spvDescriptorSet1 [[buffer(1)]], constant spvDescriptorSetBuffer2& spvDescriptorSet2 [[buffer(2)]], constant spvDescriptorSetBuffer3& spvDescriptorSet3 [[buffer(3)]], constant PushConstants& constants [[buffer(4)]], float4 gl_FragCoord [[position]])
{
    constexpr sampler smp(filter::linear, mip_filter::linear, address::repeat, compare_func::never, max_anisotropy(16));
    constexpr sampler smp_depth_nearest(mip_filter::nearest, compare_func::greater, max_anisotropy(1), lod_clamp(0.0, 0.0));
    constexpr sampler smp_depth_linear(filter::linear, mip_filter::linear, compare_func::greater, max_anisotropy(1));
    main0_out out = {};
    float3x3 in_model_view = {};
    in_model_view[0] = in.in_model_view_0;
    in_model_view[1] = in.in_model_view_1;
    in_model_view[2] = in.in_model_view_2;
    out.out_color = pbr_main(smp, (*spvDescriptorSet0.per_view_data), in.in_position_ws, in.in_position_vs, in.in_normal_vs, spvDescriptorSet0.shadow_map_atlas, smp_depth_nearest, in_model_view, smp_depth_linear, (*spvDescriptorSet0.all_lights), (*spvDescriptorSet0.light_bin_output), (*spvDescriptorSet2.all_draw_data), constants, spvDescriptorSet3, gl_FragCoord, in.in_uv, spvDescriptorSet1.ssao_texture, in.in_tangent_vs, in.in_binormal_vs);
    return out;
}

