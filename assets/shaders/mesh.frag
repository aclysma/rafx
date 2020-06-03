#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

//
// Per-Frame Pass
//
struct PointLight {
    vec3 position_ws;
    vec3 position_vs;
    vec4 color;
    float range;
    float intensity;
};

struct DirectionalLight {
    vec3 direction_ws;
    vec3 direction_vs;
    vec4 color;
    float intensity;
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
};

layout (set = 0, binding = 0) uniform PerFrameData {
    vec4 ambient_light;
    uint point_light_count;
    uint directional_light_count;
    uint spot_light_count;
    PointLight point_lights[16];
    DirectionalLight directional_lights[16];
    SpotLight spot_lights[16];
} per_frame_data;

layout (set = 0, binding = 1) uniform sampler smp;

//
// Per-Material Bindings
//
struct MaterialData {
    vec4 base_color_factor;
    vec3 emissive_factor;
    float pad0;
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

layout (set = 1, binding = 0) uniform MaterialDataUbo {
    MaterialData data;
} material_data_ubo;

layout (set = 1, binding = 1) uniform texture2D base_color_texture;
layout (set = 1, binding = 2) uniform texture2D metallic_roughness_texture;
layout (set = 1, binding = 3) uniform texture2D normal_texture;
layout (set = 1, binding = 4) uniform texture2D occlusion_texture;
layout (set = 1, binding = 5) uniform texture2D emissive_texture;

layout (location = 0) in vec3 in_position_vs;
layout (location = 1) in vec3 in_normal_vs;
// w component is a sign value (-1 or +1) indicating handedness of the tangent basis
// see GLTF spec for more info
layout (location = 2) in vec3 in_tangent_vs;
layout (location = 3) in vec3 in_binormal_vs;
layout (location = 4) in vec2 in_uv;

// Force early depth testing, this is likely not strictly necessary
layout(early_fragment_tests) in;

layout (location = 0) out vec4 out_color;

vec4 normal_map(
    mat3 tangent_normal_binormal, 
    texture2D t, 
    sampler s, 
    vec2 uv
) {
    // Sample the normal and unflatten it from the texture (i.e. convert
    // range of [0, 1] to [-1, 1])
    vec3 normal = texture(sampler2D(t, s), uv).xyz;
    normal = normal * 2.0 - 1.0;

    // Transform the normal from the texture with the TNB matrix, which will put
    // it into the TNB's space (view space))
    normal = normal * tangent_normal_binormal;
    return normalize(vec4(normal, 0.0));
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

vec4 specular_light(
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

float attenuate_light(
    float light_range, 
    float distance
) {
    // Full lighting until 75% away, then step down to no lighting
    return 1.0 - smoothstep(light_range * .75, light_range, distance);
}


struct LightingResult
{
    vec4 diffuse;
    vec4 specular;
};

LightingResult point_light(
    PointLight light,
    MaterialData material,
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

    // Calculate lighting components
    LightingResult result;
    result.diffuse = diffuse_light(surface_to_light_dir, normal_vs, light.color) * attenuation * light.intensity;
    result.specular = specular_light(surface_to_light_dir, surface_to_eye_dir_vs, normal_vs, light.color) * attenuation * light.intensity;
    return result;
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

LightingResult spot_light(
    SpotLight light,
    MaterialData material,
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

    // Calculate lighting components
    LightingResult result;
    result.diffuse = diffuse_light(surface_to_light_dir, normal_vs, light.color) * attenuation * light.intensity * spotlight_direction_intensity;
    result.specular = specular_light(surface_to_light_dir, surface_to_eye_dir_vs, normal_vs, light.color) * attenuation * light.intensity * spotlight_direction_intensity;
    return result;
}

LightingResult directional_light(
    DirectionalLight light,
    MaterialData material,
    vec3 surface_to_eye_dir_vs,
    vec3 surface_position_vs,
    vec3 normal_vs
) {
    vec3 surface_to_light_dir = -light.direction_vs;

    LightingResult result;
    result.diffuse = diffuse_light(surface_to_light_dir, normal_vs, light.color) * light.intensity;
    result.specular = specular_light(surface_to_light_dir, surface_to_eye_dir_vs, normal_vs, light.color) * light.intensity;
    return result;
}


void main() {
    // Sample the base color, if it exists
    vec4 base_color = material_data_ubo.data.base_color_factor;
    if (material_data_ubo.data.has_base_color_texture) {
        base_color *= texture(sampler2D(base_color_texture, smp), in_uv);
    }

    // Sample the emissive color, if it exists
    vec4 emissive_color = vec4(material_data_ubo.data.emissive_factor, 1);
    if (material_data_ubo.data.has_emissive_texture) {
        emissive_color *= texture(sampler2D(emissive_texture, smp), in_uv);
        base_color = vec4(1.0, 1.0, 0.0, 1.0);
    }

    // Calculate the normal (use the normal map if it exists)
    vec4 normal_vs;
    if (material_data_ubo.data.has_normal_texture) {
        mat3 tbn = mat3(in_tangent_vs, in_binormal_vs, in_normal_vs);
        normal_vs = normal_map(tbn, normal_texture, smp, in_uv);
    } else {
        normal_vs = normalize(vec4(in_normal_vs, 0));
    }

    vec3 eye_position_vs = vec3(0, 0, 0);
    vec3 surface_to_eye_vs = normalize(eye_position_vs - in_position_vs);
    LightingResult total_result = {vec4(0,0,0,0), vec4(0,0,0,0)};
    
    // Point Lights
    for (uint i = 0; i < per_frame_data.point_light_count; ++i) {
        // TODO: Early out by distance?

        LightingResult iter_result = point_light(
            per_frame_data.point_lights[i], 
            material_data_ubo.data, 
            surface_to_eye_vs,
            in_position_vs, 
            in_normal_vs
        );

        total_result.diffuse += iter_result.diffuse;
        total_result.specular += iter_result.specular;
    }

    // Spot Lights
    for (uint i = 0; i < per_frame_data.spot_light_count; ++i) {
        // TODO: Early out by distance?

        LightingResult iter_result = spot_light(
            per_frame_data.spot_lights[i],
            material_data_ubo.data,
            surface_to_eye_vs,
            in_position_vs,
            in_normal_vs
        );

        total_result.diffuse += iter_result.diffuse;
        total_result.specular += iter_result.specular;
    }

    // directional Lights
    for (uint i = 0; i < per_frame_data.directional_light_count; ++i) {
        LightingResult iter_result = directional_light(
            per_frame_data.directional_lights[i],
            material_data_ubo.data,
            surface_to_eye_vs,
            in_position_vs,
            in_normal_vs
        );

        total_result.diffuse += iter_result.diffuse;
        total_result.specular += iter_result.specular;
    }

    base_color *= (per_frame_data.ambient_light + total_result.diffuse + total_result.specular);
    out_color = emissive_color + base_color;
}
