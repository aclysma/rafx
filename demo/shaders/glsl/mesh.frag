#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

// References:
// https://www.3dgep.com/forward-plus/
// - Basic framework for forward/deferred/forward+ in non-PBR
// https://learnopengl.com/PBR/Theory
// - PBR
// https://cdn2.unrealengine.com/Resources/files/2013SiggraphPresentationsNotes-26915738.pdf
// https://google.github.io/filament/Filament.md.html
//
// The unreal paper is straightforward and practical for PBR. The filament one is more thorough and covers some slower
// but more accurate approaches.

const float PI = 3.14159265359;

//TODO: Shader processor can't handle consts
//const int MAX_POINT_LIGHTS = 16;
//const int MAX_DIRECTIONAL_LIGHTS = 16;
//const int MAX_SPOT_LIGHTS = 16;
//const int MAX_SHADOWS = MAX_DIRECTIONAL_LIGHTS * MAX_POINT_LIGHTS * MAX_SPOT_LIGHTS;

// These are determined by trial and error. A deeper Z projection requires lower numbers here so this may need to be
// per light based on Z-depth
//
// Also, currently the depth values from ortho projections are compressed near 1.0 and perspective near 0.0. Don't know
// why this is happening and it may be why these need such different values
//
// Make sure to tune these with 1-sample PCF rather than multi-sample PCF as multi-sample will hide shadow acne
//
// If the numbers are high, you'll get peter-panning
// If the nubers are low, you'll get noise (shadow acne)
//
// These were tuned with near/far distances of 0.1 to 100.0 reversed Z
//
const float SPOT_LIGHT_SHADOW_MAP_BIAS_MULTIPLIER = 0.01;
const float DIRECTIONAL_LIGHT_SHADOW_MAP_BIAS_MULTIPLIER = 1.0;

// The max is used when light is hitting at an angle (near orthogonal to normal). Min is used when light is hitting
// directly
//
// Tuning steps:
// - Use a single directional light (DIRECTIONAL_LIGHT_SHADOW_MAP_BIAS_MULTIPLIER = 1.0)
//   - Test a light almost vertical with the ground (almost parallel to normal). Raise SHADOW_MAP_BIAS_MIN until
//     shadow acne is gone
//   - Test a light almost horizontal with the ground (almost orthogonal to normal). Raise SHADOW_MAP_BIAS_MAX until
//     shadow acne is gone
//     - There may be a repeating pattern on the ground, the frequency this repeats at in distance is how much peter-panning
//       there will be once the min is high enough to get rid of the noise
//   - Test angles at in-between angles. You can play with the bias function to square or cube the bias_angle_factor
// - Test other lights and adjust their bias
// - NOTE: Factors at play here are (probably) Z depth of projection and resolution of shadow texture
//
// TODO: Investigate if per-light bias multiplier (i.e. SPOT_LIGHT_SHADOW_MAP_BIAS_MULTIPLIER) should apply to SHADOW_MAP_BIAS_MIN
const float SHADOW_MAP_BIAS_MAX = 0.01;
const float SHADOW_MAP_BIAS_MIN = 0.0005;

//#define PCF_DISABLED
//#define PCF_SAMPLE_1
//#define PCF_SAMPLE_9
#define PCF_SAMPLE_25

//
// Per-Frame Pass
//
struct PointLight {
    vec3 position_ws;
    vec3 position_vs;
    vec4 color;
    float range;
    float intensity;
    int shadow_map;
};

struct DirectionalLight {
    vec3 direction_ws;
    vec3 direction_vs;
    vec4 color;
    float intensity;
    int shadow_map;
};

struct SpotLight {
    vec3 position_ws;
    vec3 direction_ws;
    vec3 position_vs;
    vec3 direction_vs;
    vec4 color;
    float spotlight_half_angle;
    float range;
    float intensity;
    int shadow_map;
};

struct ShadowMapData {
    mat4 shadow_map_view_proj;
    vec3 shadow_map_light_dir;
};

// @[export]
// @[internal_buffer]
layout (set = 0, binding = 0) uniform PerViewData {
    vec4 ambient_light;
    uint point_light_count;
    uint directional_light_count;
    uint spot_light_count;
    PointLight point_lights[16];
    DirectionalLight directional_lights[16];
    SpotLight spot_lights[16];
    uint shadow_map_count;
    ShadowMapData shadow_maps[48];
} per_view_data;


// @[immutable_samplers([
//     (
//         mag_filter: Linear,
//         min_filter: Linear,
//         address_mode_u: Repeat,
//         address_mode_v: Repeat,
//         address_mode_w: Repeat,
//         anisotropy_enable: true,
//         max_anisotropy: 16.0,
//         border_color: IntOpaqueBlack,
//         unnormalized_coordinates: false,
//         compare_enable: false,
//         compare_op: Always,
//         mipmap_mode: Linear,
//         mip_lod_bias: 0,
//         min_lod: 0,
//         max_lod: 1000
//     )
// ])]
layout (set = 0, binding = 1) uniform sampler smp;

// @[immutable_samplers([
//     (
//         mag_filter: Linear,
//         min_filter: Linear,
//         address_mode_u: ClampToBorder,
//         address_mode_v: ClampToBorder,
//         address_mode_w: ClampToBorder,
//         anisotropy_enable: true,
//         max_anisotropy: 16.0,
//         border_color: IntOpaqueBlack,
//         unnormalized_coordinates: false,
//         compare_enable: true,
//         compare_op: Greater,
//         mipmap_mode: Linear,
//         mip_lod_bias: 0,
//         min_lod: 0,
//         max_lod: 1000
//     )
// ])]

// @[export]
layout (set = 0, binding = 2) uniform sampler smp_depth;
// @[export]
layout (set = 0, binding = 3) uniform texture2D shadow_map_images[48];

//
// Per-Material Bindings
//
struct MaterialData {
    vec4 base_color_factor;
    vec3 emissive_factor;
    float metallic_factor;
    float roughness_factor;
    float normal_texture_scale;
    float occlusion_texture_strength;
    float alpha_cutoff;
    bool has_base_color_texture;
    bool has_metallic_roughness_texture;
    bool has_normal_texture;
    bool has_occlusion_texture;
    bool has_emissive_texture;
};

// @[export]
// @[internal_buffer]
// @[slot_name("per_material_data")]
layout (set = 1, binding = 0) uniform MaterialDataUbo {
    MaterialData data;
} per_material_data;

// @[export]
// @[slot_name("base_color_texture")]
layout (set = 1, binding = 1) uniform texture2D base_color_texture;

// @[export]
// @[slot_name("metallic_roughness_texture")]
layout (set = 1, binding = 2) uniform texture2D metallic_roughness_texture;

// @[export]
// @[slot_name("normal_texture")]
layout (set = 1, binding = 3) uniform texture2D normal_texture;

// @[export]
// @[slot_name("occlusion_texture")]
layout (set = 1, binding = 4) uniform texture2D occlusion_texture;

// @[export]
// @[slot_name("emissive_texture")]
layout (set = 1, binding = 5) uniform texture2D emissive_texture;

// set = 2, binding = 0
#include "mesh_common_bindings.glsl"

layout (location = 0) in vec3 in_position_vs;
layout (location = 1) in vec3 in_normal_vs;
// w component is a sign value (-1 or +1) indicating handedness of the tangent basis
// see GLTF spec for more info
layout (location = 2) in vec3 in_tangent_vs;
layout (location = 3) in vec3 in_binormal_vs;
layout (location = 4) in vec2 in_uv;
layout (location = 5) in vec4 in_position_ws;

layout (location = 0) out vec4 out_color;

//TODO: It seems like passing texture/sampler through like this breaks reflection metadata
vec4 normal_map(
    mat3 tangent_binormal_normal,
    //texture2D t,
    //sampler s,
    vec2 uv
) {
    // Sample the normal and unflatten it from the texture (i.e. convert
    // range of [0, 1] to [-1, 1])
    vec3 normal = texture(sampler2D(normal_texture, smp), uv).xyz;
    normal = normal * 2.0 - 1.0;

    // Transform the normal from the texture with the TNB matrix, which will put
    // it into the TNB's space (view space))
    normal = tangent_binormal_normal * normal;
    return normalize(vec4(normal, 0.0));
}

float do_calculate_percent_lit(vec3 normal, int index, float bias_multiplier) {
    vec4 shadow_map_pos = per_view_data.shadow_maps[index].shadow_map_view_proj * in_position_ws;
    vec3 light_dir = mat3(per_object_data.model_view) * per_view_data.shadow_maps[index].shadow_map_light_dir;

    // homogenous coordinate normalization
    vec3 projected = shadow_map_pos.xyz / shadow_map_pos.w;

    // Z is 0..1 already, so depth is simply z
    float distance_from_light = projected.z;

    // Convert [-1, 1] range to [0, 1] range so we can sample the shadow map
    vec2 sample_location = projected.xy * 0.5 + 0.5;

    // I found in practice this a constant value was working fine in my scene, so can consider removing this later
    vec3 surface_to_light_dir = -light_dir;

    // Tune with single-PCF
    // bias_angle_factor is high when the light angle is almost orthogonal to the normal
    float bias_angle_factor = 1.0 - dot(normal, surface_to_light_dir);
    float bias = max(SHADOW_MAP_BIAS_MAX * bias_angle_factor * bias_angle_factor * bias_angle_factor, SHADOW_MAP_BIAS_MIN) * bias_multiplier;

    // Non-PCF form
#ifdef PCF_DISABLED
    float distance_from_closest_object_to_light = texture(
        sampler2D(shadow_map_images[index], smp_depth),
        sample_location
    ).r;
    float shadow = distance_from_light + bias < distance_from_closest_object_to_light ? 1.0 : 0.0;
#endif


    // PCF form single sample
#ifdef PCF_SAMPLE_1
    float shadow = texture(
        sampler2DShadow(shadow_map_images[index], smp_depth),
        vec3(
            sample_location,
            distance_from_light + bias
        )
    ).r;
#endif


    // PCF reasonable sample count
#ifdef PCF_SAMPLE_9
    float shadow = 0.0;
    vec2 texelSize = 1.0 / textureSize(sampler2DShadow(shadow_map_images[index], smp_depth), 0);
    for(int x = -1; x <= 1; ++x)
    {
        for(int y = -1; y <= 1; ++y)
        {
            shadow += texture(
                sampler2DShadow(shadow_map_images[index], smp_depth),
                vec3(
                    sample_location + vec2(x, y) * texelSize,
                    distance_from_light + bias
                )
            ).r;
        }
    }
    shadow /= 9.0;
#endif


    // PCF probably too many samples
#ifdef PCF_SAMPLE_25
    float shadow = 0.0;
    vec2 texelSize = 1.0 / textureSize(sampler2DShadow(shadow_map_images[index], smp_depth), 0);
    for(int x = -2; x <= 2; ++x)
    {
        for(int y = -2; y <= 2; ++y)
        {
            shadow += texture(
                sampler2DShadow(shadow_map_images[index], smp_depth),
                vec3(
                    sample_location + vec2(x, y) * texelSize,
                    distance_from_light + bias
                )
            ).r;
        }
    }
    shadow /= 25.0;
#endif

    return shadow;
}


float calculate_percent_lit(vec3 normal, int index, float bias_multiplier) {
    if (index == -1) {
        return 1.0;
    }

    return do_calculate_percent_lit(normal, index, bias_multiplier);
}

//
// Basic non-pbr lighting
//
float attenuate_light(
    float light_range,
    float distance
) {
    // Full lighting until 75% away, then step down to no lighting
    return 1.0 - smoothstep(light_range * .75, light_range, distance);
}

vec4 diffuse_light(
    vec3 surface_to_light_dir, 
    vec3 normal, 
    vec4 light_color
) {
    // Diffuse light - just dot the normal vector with the surface to light dir.
    float NdotL = max(dot(surface_to_light_dir, normal), 0);
    return light_color * NdotL;
}

vec4 specular_light_phong(
    vec3 surface_to_light_dir, 
    vec3 surface_to_eye_dir, 
    vec3 normal, 
    vec4 light_color
) {
    // Calculate the angle that light might reflect at
    vec3 reflect_dir = normalize(reflect(-surface_to_light_dir, normal));

    // Dot the reflection with the view angle
    float RdotV = max(dot(reflect_dir, surface_to_eye_dir), 0);

    // Raise to a power to get the specular effect on a narrow viewing angle
    return light_color * pow(RdotV, 4.0); // hardcode a spec power, will switch to BSDF later
}

vec4 specular_light_blinn_phong(
    vec3 surface_to_light_dir,
    vec3 surface_to_eye_dir,
    vec3 normal,
    vec4 light_color
) {
    // Calculate the angle that light might reflect at
    vec3 halfway_dir = normalize(surface_to_light_dir + surface_to_eye_dir);

    // Dot the reflection with the view angle
    float RdotV = max(dot(normal, halfway_dir), 0);

    // Raise to a power to get the specular effect on a narrow viewing angle
    return light_color * pow(RdotV, 4.0); // hardcode a spec power, will switch to BSDF later
}

vec4 specular_light(
    vec3 surface_to_light_dir,
    vec3 surface_to_eye_dir,
    vec3 normal,
    vec4 light_color
) {
    return specular_light_blinn_phong(
        surface_to_light_dir,
        surface_to_eye_dir,
        normal,
        light_color
    );
}

vec4 shade_diffuse_specular(
    vec3 surface_to_light_dir,
    vec3 surface_to_eye_dir,
    vec3 normal,
    vec4 light_color,
    float intensity // should include attenuation
) {
    vec4 diffuse = diffuse_light(surface_to_light_dir, normal, light_color) * intensity;
    vec4 specular = specular_light(surface_to_light_dir, surface_to_eye_dir, normal, light_color) * intensity;
    return diffuse + specular;
}

vec4 point_light(
    PointLight light,
    vec3 surface_to_eye_dir_vs,
    vec3 surface_position_vs,
    vec3 normal_vs
) {
    // Get the distance to the light and normalize the surface_to_light direction. (Not
    // using normalize since we want the distance too)
    vec3 surface_to_light_dir = light.position_vs - surface_position_vs;
    float distance = length(surface_to_light_dir);
    surface_to_light_dir = surface_to_light_dir / distance;

    // Figure out the falloff of light intensity due to distance from light source
    float attenuation = attenuate_light(light.range, distance);

    return shade_diffuse_specular(surface_to_light_dir, surface_to_eye_dir_vs, normal_vs, light.color, attenuation * light.intensity);
}

float spotlight_cone_falloff(
    vec3 surface_to_light_dir,
    vec3 spotlight_dir,
    float spotlight_half_angle
) {
    // If we dot -spotlight_dir with surface_to_light_dir:
    // - the result will be 1 if the spotlight is pointed straight at the surface position
    // - the result will be 0 if the spotlight direction is orthogonal to the surface position
    float cos_angle = dot(-spotlight_dir, surface_to_light_dir);

    // spotlight_half_angle will indicate the minimum result necessary to receive lighting contribution
    // this indicates the "edge" of the ring on the surface formed by the spotlight where there is no longer a lighting
    // contribution
    float min_cos = cos(spotlight_half_angle);

    // Pick an angle at which to start reducing lighting contribution based on direction of the spotlight
    float max_cos = mix(min_cos, 1, 0.5); // mix is lerp

    // based on the angle found in cos_angle, calculate the contribution
    return smoothstep(min_cos, max_cos, cos_angle);
}

vec4 spot_light(
    SpotLight light,
    vec3 surface_to_eye_dir_vs,
    vec3 surface_position_vs,
    vec3 normal_vs
) {
    // Get the distance to the light and normalize the surface_to_light direction. (Not
    // using normalize since we want the distance too)
    vec3 surface_to_light_dir = light.position_vs - surface_position_vs;
    float distance = length(surface_to_light_dir);
    surface_to_light_dir = surface_to_light_dir / distance;

    // Figure out the falloff of light intensity due to distance from light source
    float attenuation = attenuate_light(light.range, distance);
    float spotlight_direction_intensity = spotlight_cone_falloff(
        surface_to_light_dir,
        light.direction_vs,
        light.spotlight_half_angle
    );

    return shade_diffuse_specular(surface_to_light_dir, surface_to_eye_dir_vs, normal_vs, light.color, attenuation * light.intensity * spotlight_direction_intensity);
}

vec4 directional_light(
    DirectionalLight light,
    vec3 surface_to_eye_dir_vs,
    vec3 surface_position_vs,
    vec3 normal_vs
) {
    vec3 surface_to_light_dir = -light.direction_vs;
    return shade_diffuse_specular(surface_to_light_dir, surface_to_eye_dir_vs, normal_vs, light.color, light.intensity);
}

//
// Normal distribution function approximates the relative surface area where microfacets are aligned to the halfway
// vector, producing specular-like results. (GGX/Trowbridge-Reitz)
//
float ndf_ggx(
    vec3 n,
    vec3 h,
    float roughness
) {
    // disney/epic remap alpha, squaring roughness as it produces better results
    // https://cdn2.unrealengine.com/Resources/files/2013SiggraphPresentationsNotes-26915738.pdf
    float a = roughness * roughness;
    float a2 = a * a;

    float n_dot_h = max(dot(n, h), 0.0);
    float bottom_part = (n_dot_h * n_dot_h * (a2 - 1.0) + 1.0);
    float bottom = PI * bottom_part * bottom_part;
    return a2 / bottom;
}

//
// geometric attenuation, approximates light rays being trapped within micro-surfaces of the material
//
float geometric_attenuation_schlick_ggx(
    float dot_product,
    float k
) {
    float bottom = (dot_product * (1.0 - k)) + k;
    return dot_product / bottom;
}

float geometric_attenuation_smith(
    vec3 n,
    vec3 v,
    vec3 l,
    float roughness
) {
    // This is appropriate for analytic lights, not image-based
    float r_plus_1 = (roughness + 1.0);
    float k = r_plus_1 * r_plus_1 / 8.0;

    float v_factor = geometric_attenuation_schlick_ggx(max(dot(n, v), 0.0), k);
    float l_factor = geometric_attenuation_schlick_ggx(max(dot(n, l), 0.0), k);
    return v_factor * l_factor;
}

//
// fresnel_base is specular reflectance at normal incidence
//
vec3 fresnel_schlick(
    vec3 v,
    vec3 h,
    vec3 fresnel_base
) {
    float v_dot_h = max(dot(v, h), 0.0);

    // approximation for pow(1 - v_dot_h, 5)
    // https://seblagarde.wordpress.com/2012/06/03/spherical-gaussien-approximation-for-blinn-phong-phong-and-fresnel/
    // https://cdn2.unrealengine.com/Resources/files/2013SiggraphPresentationsNotes-26915738.pdf
    // See https://google.github.io/filament/Filament.md.html for alternatives
    return fresnel_base + (1.0 - fresnel_base) * exp2((-5.55473 * v_dot_h - 6.98316) * v_dot_h);
}

vec3 shade_pbr(
    vec3 surface_to_light_dir_vs,
    vec3 surface_to_eye_dir_vs,
    vec3 normal_vs,
    vec3 F0,
    vec3 base_color,
    float roughness,
    float metalness,
    vec3 radiance
) {
    vec3 halfway_dir_vs = normalize(surface_to_light_dir_vs + surface_to_eye_dir_vs);

    float NDF = ndf_ggx(normal_vs, halfway_dir_vs, roughness);
    float G = geometric_attenuation_smith(normal_vs, surface_to_eye_dir_vs, surface_to_light_dir_vs, roughness);
    vec3 F = fresnel_schlick(surface_to_eye_dir_vs, halfway_dir_vs, F0);

    // fresnel defines ratio of light energy that contributes to specular lighting
    vec3 fresnel_specular = F;
    vec3 fresnel_diffuse = vec3(1.0) - fresnel_specular;

    // As the surface becomes more metallic, remove the diffuse term
    fresnel_diffuse *= 1.0 - metalness;

    // Cook-Torrance specular BRDF
    float n_dot_l = max(dot(normal_vs, surface_to_light_dir_vs), 0.0);
    float n_dot_v = max(dot(normal_vs, surface_to_eye_dir_vs), 0.0);
    vec3 top = NDF * G * F;
    float bottom = 4.0 * n_dot_v * n_dot_l;
    vec3 specular = top / max(bottom, 0.001);

    return ((fresnel_diffuse * base_color / PI) + specular) * radiance * n_dot_l;
}

vec3 point_light_pbr(
    PointLight light,
    vec3 surface_to_eye_dir_vs,
    vec3 surface_position_vs,
    vec3 normal_vs,
    vec3 F0,
    vec3 base_color,
    float roughness,
    float metalness
) {
    // Get the distance to the light and normalize the surface_to_light direction. (Not
    // using normalize since we want the distance too)
    vec3 surface_to_light_dir_vs = light.position_vs - surface_position_vs;
    float distance = length(surface_to_light_dir_vs);
    surface_to_light_dir_vs = surface_to_light_dir_vs / distance;

    // Figure out the falloff of light intensity due to distance from light source
    float attenuation = 1.0 / (distance * distance);

    vec3 radiance = light.color.rgb * attenuation * light.intensity;

    return shade_pbr(
        surface_to_light_dir_vs,
        surface_to_eye_dir_vs,
        normal_vs,
        F0,
        base_color,
        roughness,
        metalness,
        radiance
    );
}


vec3 spot_light_pbr(
    SpotLight light,
    vec3 surface_to_eye_dir_vs,
    vec3 surface_position_vs,
    vec3 normal_vs,
    vec3 F0,
    vec3 base_color,
    float roughness,
    float metalness
) {
    // Get the distance to the light and normalize the surface_to_light direction. (Not
    // using normalize since we want the distance too)
    vec3 surface_to_light_dir_vs = light.position_vs - surface_position_vs;
    float distance = length(surface_to_light_dir_vs);
    surface_to_light_dir_vs = surface_to_light_dir_vs / distance;

    // Figure out the falloff of light intensity due to distance from light source
    float attenuation = 1.0 / (distance * distance);

    // Figure out the falloff of light intensity around the projected cone of light
    float spotlight_direction_intensity = spotlight_cone_falloff(
        surface_to_light_dir_vs,
        light.direction_vs,
        light.spotlight_half_angle
    );

    vec3 radiance = light.color.rgb * attenuation * light.intensity * spotlight_direction_intensity;

    return shade_pbr(
        surface_to_light_dir_vs,
        surface_to_eye_dir_vs,
        normal_vs,
        F0,
        base_color,
        roughness,
        metalness,
        radiance
    );
}

vec3 directional_light_pbr(
    DirectionalLight light,
    vec3 surface_to_eye_dir_vs,
    vec3 surface_position_vs,
    vec3 normal_vs,
    vec3 F0,
    vec3 base_color,
    float roughness,
    float metalness
) {
    vec3 surface_to_light_dir_vs = -light.direction_vs;

    // directional lights are infinitely far away and have no fall-off
    float attenuation = 1.0;

    vec3 radiance = light.color.rgb * attenuation * light.intensity;

    return shade_pbr(
        surface_to_light_dir_vs,
        surface_to_eye_dir_vs,
        normal_vs,
        F0,
        base_color,
        roughness,
        metalness,
        radiance
    );
}

vec4 non_pbr_path(
    vec3 surface_to_eye_vs,
    vec4 base_color,
    vec4 emissive_color,
    vec3 normal_vs
) {
    // Point Lights
    vec3 total_light = vec3(0.0);
    for (uint i = 0; i < per_view_data.point_light_count; ++i) {
        // TODO: Early out by distance?

        // Need to use cube maps to detect percent lit
        //float percent_lit = 1.0;
        total_light += /*percent_lit * */ point_light(
            per_view_data.point_lights[i],
            surface_to_eye_vs,
            in_position_vs,
            normal_vs
        ).rgb;
    }

    // Spot Lights
    for (uint i = 0; i < per_view_data.spot_light_count; ++i) {
        // TODO: Early out by distance?

        float percent_lit = calculate_percent_lit(
            normal_vs,
            per_view_data.spot_lights[i].shadow_map,
            SPOT_LIGHT_SHADOW_MAP_BIAS_MULTIPLIER
        );

        total_light += percent_lit * spot_light(
            per_view_data.spot_lights[i],
            surface_to_eye_vs,
            in_position_vs,
            normal_vs
        ).rgb;
    }

    // directional Lights
    for (uint i = 0; i < per_view_data.directional_light_count; ++i) {
        float percent_lit = calculate_percent_lit(
            normal_vs,
            per_view_data.directional_lights[i].shadow_map,
            DIRECTIONAL_LIGHT_SHADOW_MAP_BIAS_MULTIPLIER
        );

        total_light += percent_lit * directional_light(
            per_view_data.directional_lights[i],
            surface_to_eye_vs,
            in_position_vs,
            normal_vs
        ).rgb;
    }

    vec3 rgb_color = base_color.rgb;
    rgb_color *= (per_view_data.ambient_light.rgb + vec3(total_light));
    return vec4(emissive_color.rgb + rgb_color, 1.0);
}

//TODO: Light range is not being considered. Will want a method of tapering it to zero
vec4 pbr_path(
    vec3 surface_to_eye_vs,
    vec4 base_color,
    vec4 emissive_color,
    float metalness,
    float roughness,
    vec3 normal_vs
) {
    // used in fresnel, non-metals use 0.04 and metals use the base color
    vec3 fresnel_base = vec3(0.04);
    fresnel_base = mix(fresnel_base, base_color.rgb, vec3(metalness));

    // Point Lights
    vec3 total_light = vec3(0.0);
    for (uint i = 0; i < per_view_data.point_light_count; ++i) {
        // TODO: Early out by distance?
        // Need to use cube maps to detect percent lit
        //float percent_lit = 1.0;
        total_light += /* percent_lit * */ point_light_pbr(
            per_view_data.point_lights[i],
            surface_to_eye_vs,
            in_position_vs,
            normal_vs,
            fresnel_base,
            base_color.rgb,
            roughness,
            metalness
        );
    }

    // Spot Lights
    for (uint i = 0; i < per_view_data.spot_light_count; ++i) {
        // TODO: Early out by distance?
        float percent_lit = calculate_percent_lit(
            normal_vs,
            per_view_data.spot_lights[i].shadow_map,
            SPOT_LIGHT_SHADOW_MAP_BIAS_MULTIPLIER
        );
        total_light += percent_lit * spot_light_pbr(
            per_view_data.spot_lights[i],
            surface_to_eye_vs,
            in_position_vs,
            normal_vs,
            fresnel_base,
            base_color.rgb,
            roughness,
            metalness
        );
    }

    // directional Lights
    for (uint i = 0; i < per_view_data.directional_light_count; ++i) {
        float percent_lit = calculate_percent_lit(
            normal_vs,
            per_view_data.directional_lights[i].shadow_map,
            DIRECTIONAL_LIGHT_SHADOW_MAP_BIAS_MULTIPLIER
        );
        total_light += percent_lit * directional_light_pbr(
            per_view_data.directional_lights[i],
            surface_to_eye_vs,
            in_position_vs,
            normal_vs,
            fresnel_base,
            base_color.rgb,
            roughness,
            metalness
        );
    }

    //
    // There are still issues here, not sure how alpha interacts and gamma looks terrible
    //
    vec3 ambient = per_view_data.ambient_light.rgb * base_color.rgb; //TODO: Multiply ao in here
    vec3 color = ambient + total_light + emissive_color.rgb;
    return vec4(color, base_color.a);

    // tonemapping
    //vec3 mapped = color.rgb / (color.rgb + vec3(1.0));

    // gamma correction
    //const float gamma = 2.2;
    //mapped = pow(mapped, vec3(1.0 / gamma));

    // output
    //return vec4(mapped, base_color.a);
}


void main() {
    // Sample the base color, if it exists
    vec4 base_color = per_material_data.data.base_color_factor;
    if (per_material_data.data.has_base_color_texture) {
        base_color *= texture(sampler2D(base_color_texture, smp), in_uv);
    }

    // Sample the emissive color, if it exists
    vec4 emissive_color = vec4(per_material_data.data.emissive_factor, 1);
    if (per_material_data.data.has_emissive_texture) {
        emissive_color *= texture(sampler2D(emissive_texture, smp), in_uv);
        base_color = vec4(1.0, 1.0, 0.0, 1.0);
    }

    // Sample metalness/roughness
    float metalness = per_material_data.data.metallic_factor;
    float roughness = per_material_data.data.roughness_factor;
    if (per_material_data.data.has_metallic_roughness_texture) {
        vec4 sampled = texture(sampler2D(metallic_roughness_texture, smp), in_uv);
        metalness *= sampled.r;
        roughness *= sampled.g;
    }

    // Extremely smooth surfaces can produce sharp reflections that, while accurate for point lights, can produce
    // very sharp contrasts in color which look "weird" - and with bloom can produce flickering.
    const float specularity_reduction = 0.00;
    roughness = (roughness + specularity_reduction) / (1.0 + specularity_reduction);

    // Calculate the normal (use the normal map if it exists)
    vec3 normal_vs;
    if (per_material_data.data.has_normal_texture) {
        mat3 tbn = mat3(in_tangent_vs, in_binormal_vs, in_normal_vs);
        normal_vs = normal_map(
            tbn,
            //normal_texture,
            //smp,
            in_uv
        ).xyz;
    } else {
        normal_vs = normalize(vec4(in_normal_vs, 0)).xyz;
    }

    //TOOD: AO

    vec3 eye_position_vs = vec3(0, 0, 0);
    vec3 surface_to_eye_vs = normalize(eye_position_vs - in_position_vs);

    //float percent_lit = calculate_percent_lit(normal_vs);

//    out_color = non_pbr_path(
//        surface_to_eye_vs,
//        base_color,
//        emissive_color,
//        normal_vs
//    );

    //out_color = per_material_data.data.base_color_factor;

    out_color = pbr_path(
        surface_to_eye_vs,
        base_color,
        emissive_color,
        metalness,
        roughness,
        normal_vs
    );
    //out_color = vec4(vec3(dot(normal_vs, -in_shadow_map_light_dir_vs)), 1.0);
}
